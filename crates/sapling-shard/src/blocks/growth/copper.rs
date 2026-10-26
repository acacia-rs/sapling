//! Copper weathering by random tick, as BDS's `tryOxidize` (docs/research/copper.md, O1-O5): a
//! block that passes its odds weathers a stage if no unwaxed copper within four blocks is younger,
//! likelier the more of it is older.

use sapling_api::BlockPos;

use super::super::copper::{self, Copper, OXIDIZED};
use super::Picks;
use super::rules::{Change, shown};

type Level = sapling_world::World;

/// One random tick in so many tries to weather at all: 1.2 times the Java game's 64/1125.
const TRIES: f32 = 0.068_266_67;
/// How far, counted along the axes, copper looks for copper that holds it back or hurries it.
const REACH: i32 = 4;
/// New copper weathers at this part of the chance.
const NEW: f32 = 0.75;
const CHANCES: u32 = 1 << 20;

fn draw(picks: &mut Picks) -> f32 {
    picks.below(CHANCES) as f32 / CHANCES as f32
}

fn copper_at(level: &Level, [x, y, z]: BlockPos) -> Option<Copper> {
    level.in_height(y).then(|| copper::of_state(level.registry(), level.block(x, y, z))).flatten()
}

/// Whether the block is copper that still weathers.
pub(super) fn is_ticked(name: &str) -> bool {
    Copper::of(name).is_some_and(|copper| !copper.waxed && copper.stage < OXIDIZED)
}

/// What a random tick makes of the copper at `pos`: the next stage, a door's both halves.
pub(super) fn ticked(level: &Level, pos: BlockPos, picks: &mut Picks) -> Vec<Change> {
    let (tries, odds) = (draw(picks), draw(picks));
    let Some(here) = copper_at(level, pos).filter(|copper| !copper.waxed && copper.stage < OXIDIZED && tries < TRIES) else { return Vec::new() };
    let (mut same, mut older) = (0u32, 0u32);
    for dx in -REACH..=REACH {
        for dy in -(REACH - dx.abs())..=REACH - dx.abs() {
            let left = REACH - dx.abs() - dy.abs();
            for dz in -left..=left {
                let Some(other) = (dx, dy, dz).ne(&(0, 0, 0)).then(|| copper_at(level, [pos[0] + dx, pos[1] + dy, pos[2] + dz])).flatten().filter(|other| !other.waxed) else { continue };
                match other.stage.cmp(&here.stage) {
                    std::cmp::Ordering::Less => return Vec::new(),
                    std::cmp::Ordering::Equal => same += 1,
                    std::cmp::Ordering::Greater => older += 1,
                }
            }
        }
    }
    let share = (older + 1) as f32 / (older + 1 + same) as f32;
    let chance = share * share * if here.stage == 0 { NEW } else { 1.0 };
    if odds >= chance {
        return Vec::new();
    }
    let next = Copper { stage: here.stage + 1, ..here };
    let other_half = here.is_door().then(|| door_half(level, pos)).flatten();
    [Some(pos), other_half].into_iter().flatten().filter_map(|at| next.state(level.registry(), level.block(at[0], at[1], at[2])).map(|id| shown(at, id))).collect()
}

/// The other half of the copper door at `pos`, if it stands.
fn door_half(level: &Level, [x, y, z]: BlockPos) -> Option<BlockPos> {
    let upper = level.registry().get(level.block(x, y, z))?.properties.split(',').any(|pair| pair == "upper_block_bit=1");
    let other = [x, if upper { y - 1 } else { y + 1 }, z];
    copper_at(level, other).filter(|copper| copper.is_door()).map(|_| other)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lone_copper_weathers_at_its_odds_and_younger_copper_beside_holds_it_back() {
        let mut level = Level::void();
        let registry = sapling_world::BlockRegistry::vanilla();
        let id = |name: &str| registry.find(name, "").unwrap();
        level.set_block(0, 64, 0, id("minecraft:exposed_copper"));
        assert!(super::super::rules::is_ticked("minecraft:exposed_copper", "") && !is_ticked("minecraft:waxed_copper"));
        let mut picks = Picks::default();
        let weathered = (0..10_000).filter(|_| !ticked(&level, [0, 64, 0], &mut picks).is_empty()).count();
        assert!((550..820).contains(&weathered), "0.068 a tick: {weathered}");
        level.set_block(3, 64, 0, id("minecraft:copper_block"));
        assert!((0..2000).all(|_| ticked(&level, [0, 64, 0], &mut picks).is_empty()), "new copper four off holds it back");
    }
}
