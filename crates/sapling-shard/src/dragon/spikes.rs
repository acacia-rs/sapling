//! A spike built again (`SpikeFeature::place` 149d63340, as the End's generation builds it,
//! sapling-worldgen `feature/end.rs`), and its crystal (`placeCrystal` 149d65290): obsidian within
//! the circle below the height, air elsewhere in the box from y 66 to ten over the top, the iron
//! bar cage of a guarded one, then bedrock under the crystal. A respawn builds them
//! (research/captures/bds-dragon-again.txt).

use sapling_api::BlockPos;
use sapling_worldgen::end::Spike;

const OBSIDIAN: (&str, &str) = ("minecraft:obsidian", "");
const AIR: (&str, &str) = ("minecraft:air", "");
/// The lowest y of a spike's air.
const AIR_FROM: i32 = 66;
/// How far over its top a spike's box goes.
const CLEARED: i32 = 10;

/// The cage's bars, joined to the bars beside them as BDS left them.
fn bars(spike: &Spike, [x, y, z]: BlockPos) -> (&'static str, String) {
    let cage = |dx: i32, dy: i32, dz: i32| (dx.abs() == 2 || dz.abs() == 2 || dy == 3) && dx.abs() <= 2 && dz.abs() <= 2 && (0..=3).contains(&dy);
    let (dx, dy, dz) = (x - spike.x, y - spike.height, z - spike.z);
    let joined = |ox: i32, oz: i32| u8::from(cage(dx + ox, dy, dz + oz));
    let state = format!("minecraft:connection_east={},minecraft:connection_north={},minecraft:connection_south={},minecraft:connection_west={}", joined(1, 0), joined(0, -1), joined(0, 1), joined(-1, 0));
    ("minecraft:iron_bars", state)
}

/// What building `spike` writes, in its order.
pub(super) fn cells(spike: &Spike) -> Vec<(BlockPos, (&'static str, String))> {
    let r = spike.radius;
    let mut cells = Vec::new();
    for x in spike.x - r..=spike.x + r {
        for y in 0..=spike.height + CLEARED {
            for z in spike.z - r..=spike.z + r {
                let d2 = (x - spike.x).pow(2) + (z - spike.z).pow(2);
                if d2 <= r * r + 1 && y < spike.height {
                    cells.push(([x, y, z], (OBSIDIAN.0, OBSIDIAN.1.to_owned())));
                } else if y >= AIR_FROM {
                    cells.push(([x, y, z], (AIR.0, AIR.1.to_owned())));
                }
            }
        }
    }
    if spike.guarded {
        for dx in -2..=2i32 {
            for dz in -2..=2i32 {
                for dy in 0..=3 {
                    if dx.abs() == 2 || dz.abs() == 2 || dy == 3 {
                        let pos = [spike.x + dx, spike.height + dy, spike.z + dz];
                        cells.push((pos, bars(spike, pos)));
                    }
                }
            }
        }
    }
    cells
}

/// Whether the spike's top layer has lost any of its obsidian (`isSpikeDamaged` 149d64ef0).
pub(super) fn is_damaged(spike: &Spike, obsidian_at: impl Fn(BlockPos) -> bool) -> bool {
    let r = spike.radius;
    (spike.x - r..=spike.x + r).flat_map(|x| (spike.z - r..=spike.z + r).map(move |z| (x, z))).filter(|(x, z)| (x - spike.x).pow(2) + (z - spike.z).pow(2) <= r * r + 1).any(|(x, z)| !obsidian_at([x, spike.height - 1, z]))
}

/// The cell of the bedrock a spike's crystal stands on, and where the crystal's feet are.
pub(super) fn top(spike: &Spike) -> (BlockPos, [f32; 3]) {
    ([spike.x, spike.height, spike.z], [spike.x as f32 + 0.5, (spike.height + 1) as f32, spike.z as f32 + 0.5])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_guarded_spikes_roof_is_joined_every_way_and_its_corners_two() {
        let spike = Spike { x: -12, z: 39, radius: 2, height: 79, guarded: true };
        assert_eq!(bars(&spike, [-12, 82, 39]).1, "minecraft:connection_east=1,minecraft:connection_north=1,minecraft:connection_south=1,minecraft:connection_west=1");
        assert_eq!(bars(&spike, [-14, 80, 37]).1, "minecraft:connection_east=1,minecraft:connection_north=0,minecraft:connection_south=1,minecraft:connection_west=0");
        let cells = cells(&spike);
        assert!(cells.iter().any(|(pos, (name, _))| *pos == [-12, 78, 39] && name.ends_with("obsidian")));
        assert!(cells.iter().any(|(pos, (name, _))| *pos == [-12, 79, 39] && name.ends_with("air")), "the crystal's bedrock comes after");
    }
}
