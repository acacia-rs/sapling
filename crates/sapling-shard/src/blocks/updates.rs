//! What a changed block does to its neighbours: sand and gravel over a gap fall, and what stood
//! on or hung off a block that is gone breaks and leaves its item.
// TODO: a falling block lands at once, without the falling entity; the lists are name endings,
// not block data; levers, vines, wall banners and beds are not looked at; `ctx.set_block`
// settles nothing.

use std::collections::VecDeque;

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};
use sapling_world::BlockRegistry;

use super::place::gives_way;
use super::{doors, level, loot, send, water};

/// Cells looked at for one change: a tower of sand is one cell a step.
const MAX_STEPS: usize = 512;
/// Snow layers fall on Bedrock (`TopSnowBlock` ticks as a falling block), where Java breaks them.
const FALLS: &[&str] = &[":sand", "red_sand", "suspicious_sand", "gravel", "concrete_powder", "dragon_egg", "anvil", "snow_layer"];
/// Blocks that need the block under them (the Java game's `canSurvive`).
const STANDS: &[&str] = &[
    "_door", "carpet", "rail", "sapling", "short_grass", "tall_grass", "fern", "deadbush", "poppy", "dandelion", "tulip", "orchid", "allium", "bluet", "daisy",
    "cornflower", "pressure_plate", "standing_sign", "standing_banner", "redstone_wire", "repeater", "comparator", "wheat", "carrots", "potatoes", "beetroot",
    "reeds", "cactus", "_mushroom", "cake", "flower_pot", "lily_of_the_valley", "wither_rose", "torchflower", "sunflower", "lilac", "rose_bush", "peony",
    "pitcher_plant", "eyeblossom", "pink_petals", "wildflowers", "sweet_berry_bush", "nether_wart", "crimson_roots", "warped_roots", "_fungus", "nether_sprouts",
    ":bamboo", "mangrove_propagule", "melon_stem", "pumpkin_stem", "_crop", "kelp", "seagrass", "sea_pickle", "waterlily", "candle", "trip_wire", "leaf_litter",
    "dry_grass", ":bush", "firefly_bush", "azalea", "dripleaf", "chorus_plant", "chorus_flower",
];
/// Blocks whose `facing_direction` names the face of the block they hang or stand on.
const HANGS: &[&str] = &["ladder", "button", "wall_sign", "wall_banner"];
const FACES: [&str; 6] = ["0", "1", "2", "3", "4", "5"];
/// A torch's `torch_facing_direction` on each face of its block, as facing.rs places it.
const TORCH_ON: [&str; 6] = ["top", "top", "south", "north", "east", "west"];
/// A lever's `lever_direction`s on each face of its block.
const LEVER_ON: [[&str; 2]; 6] =
    [["down_north_south", "down_east_west"], ["up_north_south", "up_east_west"], ["north", "north"], ["south", "south"], ["west", "west"], ["east", "east"]];

fn ends(name: &str, list: &[&str]) -> bool {
    list.iter().any(|end| name.ends_with(end))
}

fn has(properties: &str, key: &str, value: &str) -> bool {
    properties.split(',').filter_map(|pair| pair.split_once('=')).any(|pair| pair == (key, value))
}

/// Whether the block `id`, on `face` of a block that is gone, goes with it.
fn depended(registry: &BlockRegistry, id: u32, face: Face) -> bool {
    let Some(state) = registry.get(id).filter(|state| !state.is_air()) else { return false };
    let (index, is) = (face as usize, |key: &str, value: &str| has(state.properties, key, value));
    if state.name.ends_with("torch") {
        return face != Face::Down && is("torch_facing_direction", TORCH_ON[index]);
    }
    if state.name.ends_with("lever") {
        return LEVER_ON[index].iter().any(|on| is("lever_direction", on));
    }
    ends(state.name, HANGS) && is("facing_direction", FACES[index]) || face == Face::Up && ends(state.name, STANDS)
}

/// Whether the block `id` could not stay at `pos`: what it stands or hangs on is not solid.
// TODO: the game asks the face (a top slab holds a torch, leaves do not); this asks the cell.
pub(super) fn lacks_support(world: &World, layer: Entity, pos: BlockPos, id: u32) -> bool {
    let Some(level) = level(world, layer) else { return false };
    let registry = level.registry();
    Face::ALL.iter().filter(|face| depended(registry, id, **face)).any(|face| {
        let [x, y, z] = [0, 1, 2].map(|axis| pos[axis] - face.offset()[axis]);
        !(level.in_height(y) && registry.get(level.block(x, y, z)).is_some_and(|state| state.is_solid()))
    })
}

fn block(world: &World, layer: Entity, [x, y, z]: BlockPos) -> Option<(&BlockRegistry, u32)> {
    let level = level(world, layer)?;
    (level.in_height(y) && level.is_loaded(send::column([x, y, z]))).then(|| (level.registry(), level.block(x, y, z)))
}

/// Moves the falling block at `pos` to the lowest cell under it that gives way; false when it
/// stays.
fn fall(world: &mut World, layer: Entity, pos: BlockPos) -> bool {
    let Some((registry, id)) = block(world, layer, pos).filter(|(registry, id)| registry.get(*id).is_some_and(|state| ends(state.name, FALLS))) else { return false };
    let air = registry.air_id();
    let free = |y: i32| block(world, layer, [pos[0], y, pos[2]]).is_some_and(|(registry, id)| gives_way(registry, id));
    let Some(lowest) = (1..).map(|down| pos[1] - down).take_while(|y| free(*y)).last() else { return false };
    send::set(world, layer, pos, air) && send::set(world, layer, [pos[0], lowest, pos[2]], id)
}

/// Breaks the block at `pos` as one that lost its hold: it leaves its item.
fn pop(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some((registry, id)) = block(world, layer, pos) else { return };
    let (air, particles) = (registry.air_id(), send::destroy_particles(registry, pos, id));
    let (name, properties) = registry.get(id).map_or(("", ""), |state| (state.name, state.properties));
    let left = loot::thrown(world, name, properties, &loot::Breaker::UNAIDED);
    doors::remove_other_half(world, layer, pos);
    if send::set(world, layer, pos, air) {
        send::to_viewers(world, layer, pos, &[particles]);
        loot::spill(world, layer, pos, &left);
    }
}

/// Lets everything that the change at `pos` unsettled fall or break, and what that unsettles.
pub(super) fn settle(world: &mut World, layer: Entity, pos: BlockPos) {
    let mut changed = VecDeque::from([pos]);
    for _ in 0..MAX_STEPS {
        let Some(pos) = changed.pop_front() else { return };
        water::disturb(world, layer, pos);
        if fall(world, layer, pos) {
            changed.push_back(pos);
            continue;
        }
        if block(world, layer, pos).is_none_or(|(registry, id)| registry.get(id).is_some_and(|state| state.is_solid())) {
            continue;
        }
        for face in Face::ALL {
            let beside = face.neighbour(pos);
            let Some((registry, id)) = block(world, layer, beside) else { continue };
            if depended(registry, id, face) {
                pop(world, layer, beside);
                changed.push_back(beside);
            } else if face == Face::Up && registry.get(id).is_some_and(|state| ends(state.name, FALLS)) {
                changed.push_back(beside);
            }
        }
    }
}
