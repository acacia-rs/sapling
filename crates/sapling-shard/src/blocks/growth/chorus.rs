//! Chorus flowers growing by random tick (`ChorusFlowerBlock` subscribes to `BlockRandomTickEvent`;
//! the rule is the Java game's `ChorusFlowerBlock.randomTick`, docs/research/world-ticks.md
//! "Chorus"): up while the stalk under it is short, else out to the sides, else it dies at age 5.

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};

use super::super::{level, send};
use super::Picks;

pub(super) const FLOWER: &str = "minecraft:chorus_flower";
const PLANT: &str = "minecraft:chorus_plant";
const END_STONE: &str = "minecraft:end_stone";
const DEAD: u8 = 5;
/// Under this age a flower that cannot grow up branches out.
const BRANCHING: u8 = 4;
const SIDES: [Face; 4] = [Face::North, Face::South, Face::West, Face::East];

type Level = sapling_world::World;

fn name_at(level: &Level, [x, y, z]: BlockPos) -> Option<&'static str> {
    level.in_height(y).then(|| level.registry().get(level.block(x, y, z))).flatten().map(|state| state.name)
}

fn is_empty(level: &Level, pos: BlockPos) -> bool {
    level.is_loaded(send::column(pos)) && name_at(level, pos) == Some("minecraft:air")
}

/// Whether the cells beside `pos` are empty, but `except`.
fn alone(level: &Level, pos: BlockPos, except: Option<BlockPos>) -> bool {
    SIDES.into_iter().map(|face| face.neighbour(pos)).filter(|cell| Some(*cell) != except).all(|cell| is_empty(level, cell))
}

fn age_at(level: &Level, [x, y, z]: BlockPos) -> u8 {
    let properties = level.registry().get(level.block(x, y, z)).map_or("", |state| state.properties);
    properties.split(',').filter_map(|pair| pair.split_once('=')).find(|(key, _)| *key == "age").and_then(|(_, value)| value.parse().ok()).unwrap_or(0)
}

/// What a random tick does to the chorus flower at `pos`.
pub(super) fn ticked(world: &mut World, layer: Entity, pos: BlockPos, picks: &mut Picks) {
    let Some(level) = level(world, layer) else { return };
    let above = Face::Up.neighbour(pos);
    let age = age_at(level, pos);
    if !is_empty(level, above) || age >= DEAD {
        return;
    }
    let below = name_at(level, Face::Down.neighbour(pos));
    let (mut grows, mut on_end_stone) = (false, false);
    if below == Some(END_STONE) {
        grows = true;
    } else if below == Some(PLANT) {
        let mut stalk = 1;
        for _ in 0..4 {
            match name_at(level, [pos[0], pos[1] - stalk - 1, pos[2]]) {
                Some(PLANT) => stalk += 1,
                under => {
                    on_end_stone = under == Some(END_STONE);
                    break;
                }
            }
        }
        grows = stalk < 2 || stalk as u32 <= picks.below(if on_end_stone { 5 } else { 4 });
    } else if below == Some("minecraft:air") {
        grows = true;
    }
    let mut writes: Vec<(BlockPos, u8)> = Vec::new();
    let mut becomes_plant = false;
    if grows && alone(level, above, None) && is_empty(level, Face::Up.neighbour(above)) {
        becomes_plant = true;
        writes.push((above, age));
    } else if age < BRANCHING {
        let tries = picks.below(4) + u32::from(on_end_stone);
        for _ in 0..tries {
            let face = SIDES[picks.below(4) as usize];
            let side = face.neighbour(pos);
            let free = is_empty(level, side) && is_empty(level, Face::Down.neighbour(side)) && alone(level, side, Some(pos));
            if free && !writes.iter().any(|(at, _)| *at == side) {
                writes.push((side, age + 1));
            }
        }
        becomes_plant = !writes.is_empty();
    }
    let registry = level.registry();
    let flower = |age: u8| registry.find(FLOWER, &format!("age={age}"));
    let here = if becomes_plant { registry.states_of(PLANT).next().map(|(id, _)| id) } else { flower(DEAD) };
    let grown: Vec<(BlockPos, u32)> = writes.into_iter().filter_map(|(at, age)| Some((at, flower(age)?))).collect();
    let noise = |name: &str, at: BlockPos, id: u32| send::sound(name, registry, at, id);
    let mut sounds: Vec<(BlockPos, bytes::Bytes)> = grown.iter().map(|&(at, id)| (at, noise("chorusgrow", at, id))).collect();
    if !becomes_plant && let Some(id) = here {
        sounds.push((pos, noise("chorusdeath", pos, id)));
    }
    if let Some(id) = here {
        send::set(world, layer, pos, id);
    }
    for (at, id) in grown {
        send::set(world, layer, at, id);
    }
    for (at, sound) in sounds {
        send::to_viewers(world, layer, at, &[sound]);
    }
}
