//! How much of a region's wrong and missing cells BDS's thread-local generator explains
//! (docs/research/worldgen-bds-residue.md, "Floor: what BDS itself does not repeat" and "Reach").
//! A cell is floor when a thread-local draw could have reached it in our own decoration
//! (`OverworldGenerator::thread_local_reach`) and BDS's two runs disagree close by.

use std::collections::HashMap;

use sapling_worldgen::{Block, ReachCells, ReachTries};

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
/// How far one differing vein moves a patch's cells while the patch's draws stay in step (a live
/// patch pair: 29 cells within 4). A patch whose draws shift differs all over, twin included.
const DIRECT_REACH: i32 = 4;
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
    /// Stable wrong or missing cells, and whether our decoration's thread-local reach holds them.
    wrong: Vec<(At, Class, bool)>,
    tries: ReachTries,
}

fn any(names: &[&str], blocks: [&str; 2]) -> bool {
    blocks.iter().any(|b| names.contains(b))
}

fn bucket((x, y, z): At) -> At {
    (x.div_euclid(BUCKET), y.div_euclid(BUCKET), z.div_euclid(BUCKET))
}

impl Floor {
    /// The tries a chunk followed as a source (multiface features, sculk patches, liquid ticks).
    pub fn note_tries(&mut self, tries: ReachTries) {
        self.tries.followed += tries.followed;
        self.tries.tainted += tries.tainted;
    }

    pub fn add(&mut self, ours: Block, bds: &BdsCell, twin: &BdsCell, at: At, context: Context, reach: Option<&ReachCells>) {
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
        let reached = reach.is_some_and(|cells| cells.contains(at.0.rem_euclid(16) as usize, at.1, at.2.rem_euclid(16) as usize));
        self.wrong.push((at, class, reached));
    }

    fn near_seed(&self, at: At, reach: i32) -> bool {
        let (b, span) = (bucket(at), reach.div_euclid(BUCKET) + 1);
        let close = |q: &At| (q.0 - at.0).pow(2) + (q.1 - at.1).pow(2) + (q.2 - at.2).pow(2) <= reach * reach;
        (-span..=span).any(|dx| (-span..=span).any(|dy| (-span..=span).any(|dz| self.twin_seeds.get(&(b.0 + dx, b.1 + dy, b.2 + dz)).is_some_and(|cells| cells.iter().any(close)))))
    }

    /// One line: the thread-local blocks by what supports calling them floor, the scans, and the
    /// deterministic rest (everything but `floor` and the twin-confirmed scans).
    pub fn print(&self, world: &str, region: &str) {
        let count = |keep: &dyn Fn(At, Class, bool) -> bool| self.wrong.iter().filter(|&&(at, class, reached)| keep(at, class, reached)).count();
        let total = self.wrong.len();
        let local = count(&|_, class, _| class == Class::ThreadLocal);
        let local_reached = count(&|_, class, reached| class == Class::ThreadLocal && reached);
        let local_near = count(&|at, class, _| class == Class::ThreadLocal && self.near_seed(at, PATCH_REACH));
        let floor = count(&|at, class, reached| class == Class::ThreadLocal && reached && self.near_seed(at, DIRECT_REACH));
        // Liquids and anything else a followed try wrote beside a differing vein.
        let other = count(&|at, class, reached| class == Class::Other && reached && self.near_seed(at, DIRECT_REACH));
        let scan = count(&|_, class, _| class == Class::Scan);
        let scan_near = count(&|at, class, _| class == Class::Scan && self.near_seed(at, SCAN_REACH));
        println!(
            "  floor: world={world} region=\"{region}\" wrong+missing={total} thread_local_blocks={local} (reached {local_reached}, twin within {PATCH_REACH} {local_near}, floor {floor}) other_floor={other} scans={scan} (twin-confirmed {scan_near}) deterministic={} tries={}/{}",
            total - floor - other - scan_near,
            self.tries.tainted,
            self.tries.followed
        );
    }
}
