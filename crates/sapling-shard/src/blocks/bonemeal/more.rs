//! Bone meal on cocoa, nether vines, rooted dirt and moss, as BDS grew them in survival
//! (research/captures/bds-t3meal.txt): `None` for what takes none.

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::super::{Dice, level, with_property};
use super::{is_air, name_at, state};

const RIPE_COCOA: u8 = 2;
const VINE_AGE: u8 = 25;
/// Java's `NetherVines.getBlocksToGrowWhenBonemealed` odds, which BDS's 2, 2, 1 and 3 fit.
const VINE_KEEPS: f64 = 0.826;
/// Java's `moss_patch_bonemeal`, which BDS's patch fits: radius 1 or 2, edges kept 3 in 4.
const MOSS_REACH: [i32; 2] = [1, 2];
const MOSS_EDGE: f32 = 0.75;
const MOSS_PLANTS: f32 = 0.6;
const MOSS_DEPTH: i32 = 5;
/// `moss_vegetation`: what grows on a patch, by weight.
const MOSS_GROWTH: [(&str, u32); 5] = [("flowering_azalea", 4), ("azalea", 7), ("moss_carpet", 25), ("short_grass", 50), ("tall_grass", 10)];
const MOSS_TAKES: &[&str] = &[
    "dirt", "coarse_dirt", "podzol", "dirt_with_roots", "grass_block", "mycelium", "stone", "granite", "diorite", "andesite", "deepslate", "tuff", "moss_block", "mud",
];

/// The cells bone meal on `pos` sets, and whether it took the meal; `None` for a block not of these.
pub(super) fn grown(world: &mut World, layer: Entity, pos: BlockPos) -> Option<Option<Vec<(BlockPos, u32)>>> {
    let level = level(world, layer)?;
    let [x, y, z] = pos;
    let id = level.block(x, y, z);
    let name = name_at(level, pos)?.to_owned();
    let age = |id: u32| level.registry().get(id).and_then(|state| state.properties.split(',').find_map(|pair| pair.split_once('=')?.1.parse::<u8>().ok()));
    Some(match name.as_str() {
        "cocoa" => age(id).filter(|age| *age < RIPE_COCOA).and_then(|age| with_property(level.registry(), id, "age", &(age + 1).to_string())).map(|id| vec![(pos, id)]),
        "dirt_with_roots" => is_air(level, [x, y - 1, z]).then(|| state(level, "hanging_roots", "")).flatten().map(|roots| vec![([x, y - 1, z], roots)]),
        "twisting_vines" | "weeping_vines" => {
            let up = if name == "twisting_vines" { 1 } else { -1 };
            let tip = (0..).map(|step| y + step * up).take_while(|y| name_at(level, [x, *y, z]) == Some(name.as_str())).last()?;
            let tip_age = age(level.block(x, tip, z)).unwrap_or(0);
            let count = vine_count(world);
            let level = super::super::level(world, layer)?;
            let mut age = (tip_age + 1).min(VINE_AGE);
            let mut grown = Vec::new();
            for cell in (1..=count).map(|step| [x, tip + step * up, z]).take_while(|cell| is_air(level, *cell)) {
                grown.push((cell, state(level, &name, &format!("{name}_age={age}"))?));
                age = (age + 1).min(VINE_AGE);
            }
            Some(grown)
        }
        "moss_block" if is_air(level, [x, y + 1, z]) => Some(moss(world, layer, pos)),
        "moss_block" => None,
        _ => return None,
    })
}

fn vine_count(world: &mut World) -> i32 {
    let mut dice = world.get_resource_or_insert_with(Dice::default);
    let (mut keeps, mut count) = (1.0, 0);
    while f64::from(dice.fraction()) < keeps {
        keeps *= VINE_KEEPS;
        count += 1;
    }
    count
}

/// A patch of moss around the moss block at `pos`, with plants on some of it.
fn moss(world: &mut World, layer: Entity, [x, y, z]: BlockPos) -> Vec<(BlockPos, u32)> {
    let rolls: Vec<f32> = {
        let mut dice = world.get_resource_or_insert_with(Dice::default);
        (0..128).map(|_| dice.fraction()).collect()
    };
    let mut rolls = rolls.into_iter().cycle();
    let Some(level) = level(world, layer) else { return Vec::new() };
    let reach = MOSS_REACH[(rolls.next().unwrap_or(0.0) * 2.0) as usize % 2];
    let mut grown = Vec::new();
    for dx in -reach..=reach {
        for dz in -reach..=reach {
            let (edge_x, edge_z) = (dx.abs() == reach, dz.abs() == reach);
            if edge_x && edge_z || (edge_x || edge_z) && rolls.next().unwrap_or(0.0) > MOSS_EDGE {
                continue;
            }
            let (cx, cz) = (x + dx, z + dz);
            let Some(floor) = (y - MOSS_DEPTH..=y).rev().find(|fy| !is_air(level, [cx, *fy, cz]) && is_air(level, [cx, fy + 1, cz])) else { continue };
            if !name_at(level, [cx, floor, cz]).is_some_and(|name| MOSS_TAKES.contains(&name)) {
                continue;
            }
            grown.extend(state(level, "moss_block", "").map(|moss| ([cx, floor, cz], moss)));
            if rolls.next().unwrap_or(1.0) < MOSS_PLANTS {
                let mut pick = (rolls.next().unwrap_or(0.0) * MOSS_GROWTH.iter().map(|(_, weight)| *weight).sum::<u32>() as f32) as u32;
                let plant = MOSS_GROWTH.iter().find(|(_, weight)| pick.checked_sub(*weight).map(|left| pick = left).is_none()).map_or("short_grass", |(name, _)| *name);
                grown.extend(plant_cells(level, [cx, floor + 1, cz], plant));
            }
        }
    }
    grown
}

fn plant_cells(level: &sapling_world::World, at: BlockPos, plant: &str) -> Vec<(BlockPos, u32)> {
    if plant != "tall_grass" {
        return state(level, plant, "").map(|id| vec![(at, id)]).unwrap_or_default();
    }
    let above = [at[0], at[1] + 1, at[2]];
    match (is_air(level, above), state(level, plant, "upper_block_bit=0"), state(level, plant, "upper_block_bit=1")) {
        (true, Some(lower), Some(upper)) => vec![(at, lower), (above, upper)],
        _ => Vec::new(),
    }
}
