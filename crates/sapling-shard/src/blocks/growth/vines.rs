//! A vine's random tick: the Java 1.12 `BlockVine.updateTick` BDS keeps (`VineBlock::randomTick`
//! 1425e0490), with `vine_direction_bits` for the faces and one draw of sixteen for the faces a
//! new vine keeps where Java draws four coins.

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};

use super::super::{level, send};
use super::Picks;

const VINE: &str = "minecraft:vine";
const BITS: &str = "vine_direction_bits";
const CROWDED: usize = 5;
/// The faces by the order of the game's draw of six (down, up, north, south, west, east), and
/// the bit each has in `vine_direction_bits`.
const FACES: [Face; 6] = [Face::Down, Face::Up, Face::North, Face::South, Face::West, Face::East];
const HORIZONTAL: [(Face, u8); 4] = [(Face::South, 1), (Face::West, 2), (Face::North, 4), (Face::East, 8)];

type Level = sapling_world::World;

fn bit(face: Face) -> u8 {
    HORIZONTAL.iter().find(|(at, _)| *at == face).map_or(0, |(_, bit)| *bit)
}

/// The face a quarter turn clockwise and counter-clockwise from `face`, seen from above.
fn turns(face: Face) -> (Face, Face) {
    match face {
        Face::North => (Face::East, Face::West),
        Face::East => (Face::South, Face::North),
        Face::South => (Face::West, Face::East),
        _ => (Face::North, Face::South),
    }
}

fn state(level: &Level, [x, y, z]: BlockPos) -> Option<(&'static str, &'static str)> {
    level.in_height(y).then(|| level.registry().get(level.block(x, y, z))).flatten().map(|state| (state.name, state.properties))
}

fn bits_at(level: &Level, pos: BlockPos) -> Option<u8> {
    let (_, properties) = state(level, pos).filter(|(name, _)| *name == VINE)?;
    properties.split(',').filter_map(|pair| pair.split_once('=')).find(|(key, _)| *key == BITS).and_then(|(_, value)| value.parse().ok())
}

fn is_air(level: &Level, pos: BlockPos) -> bool {
    level.is_loaded(send::column(pos)) && state(level, pos).is_some_and(|(name, _)| name == "minecraft:air")
}

/// Whether a vine holds to the block at `pos`: a whole solid one, not glass.
fn holds_to(level: &Level, [x, y, z]: BlockPos) -> bool {
    level.in_height(y) && level.registry().get(level.block(x, y, z)).is_some_and(|state| state.is_solid() && !state.name.ends_with("glass"))
}

/// The bits of `bits` whose face has a block to hold to from `pos`.
fn held(level: &Level, pos: BlockPos, bits: u8) -> u8 {
    HORIZONTAL.iter().filter(|(face, bit)| bits & bit != 0 && holds_to(level, face.neighbour(pos))).fold(0, |kept, (_, bit)| kept | bit)
}

fn is_crowded(level: &Level, [x, y, z]: BlockPos) -> bool {
    let cells = (-4..=4).flat_map(|dx| (-1..=1).flat_map(move |dy| (-4..=4).map(move |dz| [x + dx, y + dy, z + dz])));
    cells.filter(|cell| bits_at(level, *cell).is_some()).take(CROWDED).count() >= CROWDED
}

fn put(world: &mut World, layer: Entity, pos: BlockPos, bits: u8) {
    let Some(id) = level(world, layer).and_then(|level| level.registry().find(VINE, &format!("{BITS}={bits}"))) else { return };
    send::set(world, layer, pos, id);
}

/// One in four: up into air over it, holding where it can; across to a face it lacks; else down.
pub(super) fn ticked(world: &mut World, layer: Entity, pos: BlockPos, picks: &mut Picks) {
    let Some(level) = level(world, layer) else { return };
    let Some(bits) = bits_at(level, pos) else { return };
    if picks.below(4) != 0 {
        return;
    }
    let face = FACES[picks.below(6) as usize];
    let up = Face::Up.neighbour(pos);
    if face == Face::Up && level.in_height(pos[1] + 1) && is_air(level, up) {
        if is_crowded(level, pos) {
            return;
        }
        let grown = held(level, up, bits & picks.below(16) as u8);
        if grown != 0 {
            put(world, layer, up, grown);
        }
        return;
    }
    if face != Face::Up && face != Face::Down && bits & bit(face) == 0 {
        if !is_crowded(level, pos) {
            sideways(world, layer, pos, bits, face);
        }
        return;
    }
    down(world, layer, pos, bits, picks);
}

/// Into the air beside, holding round a corner or to the block past it; onto a solid block
/// beside, by a face more.
fn sideways(world: &mut World, layer: Entity, pos: BlockPos, bits: u8, face: Face) {
    let Some(level) = level(world, layer) else { return };
    let beside = face.neighbour(pos);
    if holds_to(level, beside) {
        put(world, layer, pos, bits | bit(face));
        return;
    }
    if !is_air(level, beside) {
        return;
    }
    let (clockwise, counter) = turns(face);
    let back = bit(super::super::opposite(face));
    let choice = if bits & bit(clockwise) != 0 && holds_to(level, clockwise.neighbour(beside)) {
        Some((beside, bit(clockwise)))
    } else if bits & bit(counter) != 0 && holds_to(level, counter.neighbour(beside)) {
        Some((beside, bit(counter)))
    } else if bits & bit(clockwise) != 0 && is_air(level, clockwise.neighbour(beside)) && holds_to(level, clockwise.neighbour(pos)) {
        Some((clockwise.neighbour(beside), back))
    } else if bits & bit(counter) != 0 && is_air(level, counter.neighbour(beside)) && holds_to(level, counter.neighbour(pos)) {
        Some((counter.neighbour(beside), back))
    } else {
        None
    };
    if let Some((at, bits)) = choice {
        put(world, layer, at, bits);
    }
}

/// Down into the air under it with some of its faces, or more faces onto the vine under it.
fn down(world: &mut World, layer: Entity, pos: BlockPos, bits: u8, picks: &mut Picks) {
    let Some(level) = level(world, layer) else { return };
    let below = Face::Down.neighbour(pos);
    if !level.in_height(pos[1] - 2) {
        return;
    }
    let kept = bits & picks.below(16) as u8;
    if is_air(level, below) {
        if kept != 0 {
            put(world, layer, below, kept);
        }
    } else if let Some(under) = bits_at(level, below)
        && under | kept != under
    {
        put(world, layer, below, under | kept);
    }
}
