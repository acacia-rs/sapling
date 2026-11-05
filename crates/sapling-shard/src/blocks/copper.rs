//! Copper's four stages of weathering and its wax, read off and written into a block's name, for
//! the axe, honeycomb, the random tick (growth/copper.rs) and lightning (docs/research/copper.md).

use acacia_proto::packets::{LevelEvent, LevelEventEvent};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_world::BlockRegistry;

use super::{Dice, doors, level, send};

/// Cells a lightning walk tries for its next step before it ends.
const WALK_TRIES: usize = 10;

const STAGES: [&str; 4] = ["", "exposed_", "weathered_", "oxidized_"];
pub(crate) const OXIDIZED: usize = 3;
const WAXED: &str = "waxed_";
/// The block of copper, which alone is named otherwise unwaxed and new.
const BLOCK: (&str, &str) = ("copper", "copper_block");
/// What weathers, by its name with no stage and no wax.
const BASES: &[&str] = &[
    "copper", "cut_copper", "cut_copper_slab", "cut_copper_stairs", "double_cut_copper_slab", "chiseled_copper", "copper_door", "copper_trapdoor", "copper_grate",
    "copper_bulb", "lightning_rod", "copper_bars", "copper_chain", "copper_lantern", "copper_chest", "copper_golem_statue",
];

/// A copper block by its name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Copper {
    pub waxed: bool,
    pub stage: usize,
    pub base: &'static str,
}

impl Copper {
    /// The copper block named `name`, with or without `minecraft:`.
    pub(crate) fn of(name: &str) -> Option<Copper> {
        let bare = name.strip_prefix("minecraft:").unwrap_or(name);
        let (waxed, rest) = bare.strip_prefix(WAXED).map_or((false, bare), |rest| (true, rest));
        let (stage, base) = (1..STAGES.len()).find_map(|stage| Some((stage, rest.strip_prefix(STAGES[stage])?))).unwrap_or((0, rest));
        let base = if !waxed && stage == 0 && base == BLOCK.1 { BLOCK.0 } else if base == BLOCK.1 { return None } else { base };
        let base = BASES.iter().find(|known| **known == base)?;
        Some(Copper { waxed, stage, base })
    }

    pub(crate) fn name(self) -> String {
        if !self.waxed && self.stage == 0 && self.base == BLOCK.0 {
            return format!("minecraft:{}", BLOCK.1);
        }
        format!("minecraft:{}{}{}", if self.waxed { WAXED } else { "" }, STAGES[self.stage], self.base)
    }

    pub(crate) fn is_door(self) -> bool {
        self.base == "copper_door"
    }

    /// The state of this block in the state `id` of another stage or wax, its properties kept.
    pub(crate) fn state(self, registry: &BlockRegistry, id: u32) -> Option<u32> {
        registry.find(&self.name(), registry.get(id)?.properties)
    }
}

/// Whether `held` changes the copper block `name`: honeycomb waxes, an axe unwaxes or scrapes
/// (docs/research/copper.md, W1-W2), before a door or bulb's own use.
pub(crate) fn is_worked_by(name: &str, held: &str) -> bool {
    Copper::of(name).is_some_and(|copper| match held {
        "minecraft:honeycomb" => !copper.waxed,
        _ => held.ends_with("_axe") && (copper.waxed || copper.stage > 0),
    })
}

/// The copper block of `id`, if it is one.
pub(crate) fn of_state(registry: &BlockRegistry, id: u32) -> Option<Copper> {
    Copper::of(registry.get(id)?.name)
}

/// Makes the copper at `pos` `into` (another stage or wax): a door both its halves. Whether it changed.
pub(crate) fn turn(world: &mut World, layer: Entity, pos: BlockPos, into: Copper) -> bool {
    let other = doors::other_half(world, layer, pos).filter(|_| into.is_door());
    let mut changed = false;
    for at in [Some(pos), other].into_iter().flatten() {
        let Some(level) = level(world, layer) else { return changed };
        let Some(id) = into.state(level.registry(), level.block(at[0], at[1], at[2])) else { continue };
        changed |= send::set(world, layer, at, id);
    }
    changed
}

fn copper_at(world: &World, layer: Entity, [x, y, z]: BlockPos) -> Option<Copper> {
    let level = level(world, layer).filter(|level| level.in_height(y))?;
    of_state(level.registry(), level.block(x, y, z))
}

fn is_bare(world: &World, layer: Entity, pos: BlockPos) -> bool {
    copper_at(world, layer, pos).is_some_and(|copper| !copper.waxed)
}

/// Unwaxed copper at `pos` goes back one stage (BDS 148b39420, 148b5c410).
pub(super) fn clean(world: &mut World, layer: Entity, pos: BlockPos) {
    if let Some(copper) = copper_at(world, layer, pos).filter(|copper| !copper.waxed && copper.stage > 0) {
        turn(world, layer, pos, Copper { stage: copper.stage - 1, ..copper });
    }
}

/// The cells lightning's hook runs on, in order, for a bolt on `pos` whose walks start at `start`: `pos`,
/// then 3 to 5 walks from `start` over unwaxed copper (docs/research/copper.md, L3-L5).
pub(super) fn walked(world: &mut World, layer: Entity, pos: BlockPos, start: BlockPos) -> Vec<BlockPos> {
    let mut visits = vec![pos];
    if !is_bare(world, layer, start) {
        return visits;
    }
    let walks = 3 + world.get_resource_or_insert_with(Dice::default).roll(3);
    for _ in 0..walks {
        visits.push(start);
        let steps = 1 + world.get_resource_or_insert_with(Dice::default).roll(8);
        let mut at = start;
        for _ in 0..steps {
            let Some(next) = (0..WALK_TRIES).find_map(|_| Some(neighbour(world, at)).filter(|cell| is_bare(world, layer, *cell))) else { break };
            visits.push(next);
            at = next;
        }
    }
    visits
}

/// Which of the hook's `visits` spark, each at its cell's corner: a rod every time, other unwaxed copper
/// while it has a stage left to lose (research/captures/bds-p2copper.txt).
pub(super) fn sparking(world: &World, layer: Entity, visits: &[BlockPos]) -> Vec<BlockPos> {
    let mut lost: Vec<BlockPos> = Vec::new();
    let mut sparks = Vec::new();
    for &pos in visits {
        let Some(copper) = copper_at(world, layer, pos).filter(|copper| !copper.waxed) else { continue };
        let stages_left = copper.stage - lost.iter().filter(|cell| **cell == pos).count().min(copper.stage);
        if copper.base == "lightning_rod" || stages_left > 0 {
            sparks.push(pos);
        }
        lost.push(pos);
    }
    sparks
}

/// Lightning's spark at the corner of `pos`.
pub(super) fn spark_at(pos: BlockPos) -> bytes::Bytes {
    let position = Vec3f { x: pos[0] as f32, y: pos[1] as f32, z: pos[2] as f32 };
    sapling_net::encode(&LevelEvent { event: LevelEventEvent::ParticleElectricSpark, position, data: 0 })
}

/// A cell at most one off `at` along each axis, drawn x, y then z.
fn neighbour(world: &mut World, at: BlockPos) -> BlockPos {
    let mut dice = world.get_resource_or_insert_with(Dice::default);
    std::array::from_fn(|axis| at[axis] + dice.roll(3) as i32 - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copper_names_read_and_write_back() {
        for name in ["copper_block", "waxed_copper", "exposed_copper", "waxed_oxidized_cut_copper_stairs", "lightning_rod", "weathered_copper_bulb", "waxed_exposed_double_cut_copper_slab"] {
            let copper = Copper::of(name).unwrap_or_else(|| panic!("{name}"));
            assert_eq!(copper.name(), format!("minecraft:{name}"));
        }
        assert_eq!(Copper::of("waxed_copper").map(|copper| (copper.waxed, copper.stage)), Some((true, 0)));
        assert_eq!(Copper::of("minecraft:oxidized_copper_door").map(|copper| (copper.waxed, copper.stage, copper.is_door())), Some((false, 3, true)));
        assert_eq!([Copper::of("copper_ore"), Copper::of("raw_copper_block"), Copper::of("waxed_copper_block"), Copper::of("copper_torch")], [None; 4]);
        let scraped = Copper { stage: 0, ..Copper::of("exposed_copper").unwrap() };
        assert_eq!(scraped.name(), "minecraft:copper_block");
    }

    #[test]
    fn every_copper_name_is_a_block() {
        let registry = BlockRegistry::vanilla();
        for base in BASES {
            for waxed in [false, true] {
                for stage in 0..STAGES.len() {
                    let name = Copper { waxed, stage, base }.name();
                    assert!(registry.states_of(&name).next().is_some(), "{name}");
                }
            }
        }
    }
}
