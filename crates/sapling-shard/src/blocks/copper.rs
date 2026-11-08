//! Copper's four stages of weathering and its wax, read off and written into a block's name, for
//! the axe, honeycomb, the random tick (growth/copper.rs) and lightning (docs/research/copper.md).

use std::collections::HashMap;

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_world::BlockRegistry;

use super::{Dice, doors, level, send};

pub(crate) mod chests;

/// Cells a lightning walk tries for its next step before it ends.
const WALK_TRIES: usize = 10;
const ROD: &str = "lightning_rod";

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

    pub(crate) fn is_chest(self) -> bool {
        self.base == "copper_chest"
    }

    /// The state of this block in the state `id` of another stage or wax, its properties kept.
    pub(crate) fn state(self, registry: &BlockRegistry, id: u32) -> Option<u32> {
        registry.find(&self.name(), registry.get(id)?.properties)
    }
}

/// Whether `held` changes the copper block `name`: honeycomb waxes, an axe unwaxes or scrapes
/// (docs/research/copper.md, W1-W2), before a door or bulb's own use. A chest opens unless its
/// player sneaks (W7).
pub(crate) fn is_worked_by(name: &str, held: &str) -> bool {
    Copper::of(name).filter(|copper| !copper.is_chest()).is_some_and(|copper| match held {
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

/// Whether lightning's hook does something at `at`, a stage it loses counted in `stages`: a rod is
/// powered whatever its stage or wax, other unwaxed copper while it has a stage to lose; each shows a
/// spark (BDS 148b39420, 148b5c410 by way of the rod's hit 148b5b4f0).
fn hooks_at(world: &World, layer: Entity, at: BlockPos, stages: &mut HashMap<BlockPos, usize>) -> bool {
    let Some(copper) = copper_at(world, layer, at) else { return false };
    let rod = copper.base == ROD;
    if copper.waxed {
        return rod;
    }
    let stage = stages.entry(at).or_insert(copper.stage);
    let had = *stage > 0;
    *stage = stage.saturating_sub(1);
    rod || had
}

/// Lightning's hook calls for a bolt in `cell`, rolled now (docs/research/copper.md, L3-L5): `cell`
/// itself, then, if `start` is unwaxed copper, 3 to 5 walks from it, each hooking it and stepping up
/// to 1 to 8 times onto unwaxed copper about. The cells hooked, a cell once for each call.
pub(crate) fn hooks(world: &mut World, layer: Entity, cell: BlockPos, start: BlockPos) -> Vec<BlockPos> {
    let mut stages = HashMap::new();
    let mut hooked: Vec<BlockPos> = Vec::new();
    let mut call = |world: &World, at: BlockPos, stages: &mut HashMap<BlockPos, usize>| {
        if hooks_at(world, layer, at, stages) {
            hooked.push(at);
        }
    };
    call(world, cell, &mut stages);
    if !copper_at(world, layer, start).is_some_and(|copper| !copper.waxed) {
        return hooked;
    }
    let walks = 3 + world.get_resource_or_insert_with(Dice::default).roll(3);
    for _ in 0..walks {
        call(world, start, &mut stages);
        let steps = 1 + world.get_resource_or_insert_with(Dice::default).roll(8);
        let mut at = start;
        for _ in 0..steps {
            let mut next = None;
            for _ in 0..WALK_TRIES {
                let tried = neighbour(world, at);
                if copper_at(world, layer, tried).is_some_and(|copper| !copper.waxed) {
                    next = Some(tried);
                    break;
                }
            }
            let Some(next) = next else { break };
            call(world, next, &mut stages);
            at = next;
        }
    }
    hooked
}

/// A hook call coming down on `pos`: unwaxed copper there loses a stage, quietly (its spark was shown
/// with [`hooks`]); whether it is a rod, to be powered.
pub(crate) fn hooked(world: &mut World, layer: Entity, pos: BlockPos) -> bool {
    let Some(copper) = copper_at(world, layer, pos) else { return false };
    let into = Copper { stage: copper.stage.saturating_sub(1), ..copper };
    if !copper.waxed && copper.stage > 0 && copper.is_chest() {
        chests::turn(world, layer, pos, into, true);
    } else if !copper.waxed && copper.stage > 0 {
        turn(world, layer, pos, into);
    }
    copper.base == ROD
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
