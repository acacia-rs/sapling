//! How much of a region's wrong and missing cells BDS's thread-local generator explains, judged by
//! the `-run2` twin (docs/research/worldgen-bds-features.md, "Floor").

use std::collections::HashMap;

use sapling_worldgen::Block;

use super::BdsCell;
use super::judge::{Context, Verdict, judge};

type At = (i32, i32, i32);

/// Blocks whose placement draws from the generator at `gs:[0x58]` +0xe60: multiface faces and
/// spreads, and the sculk patches whose cursors walk over them.
const THREAD_LOCAL: &[&str] = &["glow_lichen", "sculk_vein", "sculk", "sculk_sensor", "sculk_catalyst", "sculk_shrieker"];
/// Blocks of features whose column scans stop at a lichen or vein cell: speleothem clusters
/// (`ColumnUtils::scan`) and the lush caves' `snap_to_surface` features.
const SCANNING: &[&str] = &[
    "pointed_dripstone",
    "dripstone_block",
    "sulfur_spike",
    "potent_sulfur",
    "sulfur",
    "cinnabar",
    "moss_block",
    "moss_carpet",
    "cave_vines",
    "cave_vines_body_with_berries",
    "cave_vines_head_with_berries",
    "big_dripleaf",
    "small_dripleaf_block",
    "azalea",
    "flowering_azalea",
    "spore_blossom",
    "hanging_roots",
    "dirt_with_roots",
];
/// A sculk cursor is lost beyond distance squared 224 from its patch's origin.
const PATCH_REACH: i32 = 15;
/// `ColumnUtils::scan` reaches 12 cells; a cluster's columns lie within 7 of its origin.
const SCAN_REACH: i32 = 10;
const BUCKET: i32 = 16;

#[derive(Clone, Copy, PartialEq)]
enum Class {
    ThreadLocal,
    Scan,
    Other,
}

#[derive(Default)]
pub struct Floor {
    /// Cells the twin disagrees on with a thread-local block in either run, by bucket.
    twin_seeds: HashMap<At, Vec<At>>,
    /// Stable wrong or missing cells.
    wrong: Vec<(At, Class)>,
}

fn any(names: &[&str], blocks: [&str; 2]) -> bool {
    blocks.iter().any(|b| names.contains(b))
}

fn bucket((x, y, z): At) -> At {
    (x.div_euclid(BUCKET), y.div_euclid(BUCKET), z.div_euclid(BUCKET))
}

impl Floor {
    pub fn add(&mut self, ours: Block, bds: &BdsCell, twin: &BdsCell, at: At, context: Context) {
        if bds != twin {
            if any(THREAD_LOCAL, [&bds.block, &twin.block]) {
                self.twin_seeds.entry(bucket(at)).or_default().push(at);
            }
            return;
        }
        if !matches!(judge(ours, bds, at.1, context), Verdict::Error(_) | Verdict::Missing(_)) {
            return;
        }
        let sides = [ours.name(), bds.block.as_str()];
        let class = if any(THREAD_LOCAL, sides) {
            Class::ThreadLocal
        } else if any(SCANNING, sides) {
            Class::Scan
        } else {
            Class::Other
        };
        self.wrong.push((at, class));
    }

    fn near_seed(&self, at: At, reach: i32) -> bool {
        let (b, span) = (bucket(at), reach.div_euclid(BUCKET) + 1);
        let close = |q: &At| (q.0 - at.0).pow(2) + (q.1 - at.1).pow(2) + (q.2 - at.2).pow(2) <= reach * reach;
        (-span..=span).any(|dx| (-span..=span).any(|dy| (-span..=span).any(|dz| self.twin_seeds.get(&(b.0 + dx, b.1 + dy, b.2 + dz)).is_some_and(|cells| cells.iter().any(close)))))
    }

    /// One line: the split of wrong + missing, and what is left as deterministic.
    pub fn print(&self, world: &str, region: &str) {
        let count = |class: Class, reach: Option<i32>| self.wrong.iter().filter(|(at, c)| *c == class && reach.is_none_or(|r| self.near_seed(*at, r))).count();
        let total = self.wrong.len();
        let (local, local_near) = (count(Class::ThreadLocal, None), count(Class::ThreadLocal, Some(PATCH_REACH)));
        let (scan, scan_near) = (count(Class::Scan, None), count(Class::Scan, Some(SCAN_REACH)));
        println!(
            "  floor: world={world} region=\"{region}\" wrong+missing={total} thread_local_blocks={local} (twin-confirmed {local_near}) scans={scan} (twin-confirmed {scan_near}) deterministic={} (upper bound {})",
            total - local - scan_near,
            total - local_near - scan_near
        );
    }
}
