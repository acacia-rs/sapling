//! Budding amethyst growing buds by random tick (`BuddingAmethystBlock` subscribes to
//! `BlockRandomTickEvent`; the rule is the Java game's, docs/research/world-ticks.md "Amethyst"):
//! one tick in five a face at random, where a bud on that face grows a stage or an empty cell gets
//! a small one.

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};

use super::super::{level, send};
use super::Picks;

pub(super) const BUDDING: &str = "minecraft:budding_amethyst";
const ODDS: u32 = 5;
const STAGES: [&str; 4] = ["minecraft:small_amethyst_bud", "minecraft:medium_amethyst_bud", "minecraft:large_amethyst_bud", "minecraft:amethyst_cluster"];
const FACE: &str = "minecraft:block_face";

fn face_name(face: Face) -> &'static str {
    match face {
        Face::Down => "down",
        Face::Up => "up",
        Face::North => "north",
        Face::South => "south",
        Face::West => "west",
        Face::East => "east",
    }
}

/// What a random tick does to the budding amethyst at `pos`.
pub(super) fn ticked(world: &mut World, layer: Entity, pos: BlockPos, picks: &mut Picks) {
    if picks.below(ODDS) != 0 {
        return;
    }
    let face = Face::ALL[picks.below(6) as usize];
    let Some(level) = level(world, layer) else { return };
    let cell @ [x, y, z] = face.neighbour(pos);
    if !level.in_height(y) || !level.is_loaded(send::column(cell)) {
        return;
    }
    let Some(state) = level.registry().get(level.block(x, y, z)) else { return };
    let facing = format!("{FACE}={}", face_name(face));
    let next = if state.is_air() || state.name == "minecraft:water" && state.liquid_depth == 0 {
        STAGES[0]
    } else {
        let Some(stage) = STAGES.iter().position(|stage| *stage == state.name).filter(|_| state.properties.contains(&facing)) else { return };
        let Some(next) = STAGES.get(stage + 1) else { return };
        next
    };
    let Some(id) = level.registry().find(next, &facing) else { return };
    send::set(world, layer, cell, id);
}
