//! Mobs that blow up, as BDS's creeper did (docs/research/mob-attacks.md, "Creepers"): lit near
//! its prey, it burns a fuse down and bursts, hurting and throwing who is near and breaking blocks.
// TODO: a fuse does not go out for a prey out of sight; nothing between a blast and a body shields it; mobs are hurt
// and not thrown; the blocks a creeper breaks leave nothing; what a block withstands is guessed from how hard
// it is (`withstands`), for want of the game's table.

use acacia_proto::packets::{LevelEvent, LevelEventEvent, LevelSoundEvent};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::Difficulty;

use super::harm;
use super::health::{Cause, Health};
use crate::blocks::{self, Dice};
use crate::body::Body;
use crate::components::{InLayer, Layer, NetId, Position, Settings};
use crate::{drops, entities, late_sounds, mobs, vitals};

/// A fuse is this many ticks long, and hisses this many ticks after it is lit.
const FUSE: i32 = 30;
const HISSES: i32 = 2;
const HEAD: f32 = 0.85;
const GRIEFING: &str = "mobGriefing";
/// A blast's rays: from its middle to each cell of the faces of a cube this many across, in
/// steps this long, each step costing this much and what the block it is in withstands.
const RAYS: i32 = 16;
const STEP: f32 = 0.3;
const FADES: f32 = 0.225;
/// A blast's worst, at its middle, for each unit of its power.
const HARM: f32 = 7.0;

/// A lit fuse: the ticks left of it.
#[derive(Component)]
pub(super) struct Fuse(i32);

pub(super) fn is_lit(world: &World, entity: Entity) -> bool {
    world.get::<Fuse>(entity).is_some()
}

fn tell(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, body: &Body, left: i32) {
    let told = sapling_data::mob_fuse(id, kind, left);
    entities::tell_viewers(world, entity, layer, body.at.map(|c| c.floor() as i32), vec![sapling_net::encode(&told)]);
}

/// Lights the mob's fuse, if it is not lit.
pub(super) fn light(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, body: &Body) {
    if !is_lit(world, entity) {
        world.entity_mut(entity).insert(Fuse(FUSE - 1));
        tell(world, entity, layer, id, kind, body, FUSE - 1);
    }
}

/// Puts the mob's fuse out, if it is lit: its prey is gone or far.
pub(super) fn put_out(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, body: &Body) {
    if world.entity_mut(entity).take::<Fuse>().is_some() {
        let unlit = sapling_data::mob_fire_flags(id, kind, false);
        entities::tell_viewers(world, entity, layer, body.at.map(|c| c.floor() as i32), vec![sapling_net::encode(&unlit)]);
    }
}

/// What a block of this hardness withstands of a blast: the Java game gives most blocks their
/// hardness, stone and its like 6, and obsidian far more than any blast has.
fn withstands(hardness: f32, needs_pickaxe: bool) -> f32 {
    match hardness {
        hard if hard < 0.0 || hard >= 50.0 => f32::INFINITY,
        hard if needs_pickaxe && hard <= 2.0 => 6.0,
        hard => hard,
    }
}

/// The cells a blast of `power` at `at` breaks: the Java game's rays, which broke the one stone
/// under BDS's creeper.
fn broken(world: &mut World, layer: Entity, at: [f32; 3], power: f32) -> Vec<[i32; 3]> {
    let strengths: Vec<f32> = {
        let mut dice = world.get_resource_or_insert_with(Dice::default);
        (0..RAYS * RAYS * RAYS).map(|_| power * (0.7 + dice.fraction() * 0.6)).collect()
    };
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return Vec::new() };
    let mut cells = Vec::new();
    let edge = |i: i32| i == 0 || i == RAYS - 1;
    for (ray, [i, j, k]) in (0..RAYS).flat_map(|i| (0..RAYS).flat_map(move |j| (0..RAYS).map(move |k| [i, j, k]))).enumerate().filter(|(_, [i, j, k])| edge(*i) || edge(*j) || edge(*k)) {
        let towards = [i, j, k].map(|c| c as f32 / (RAYS - 1) as f32 * 2.0 - 1.0);
        let long = towards.iter().map(|c| c * c).sum::<f32>().sqrt();
        let (mut left, mut here) = (strengths[ray], at);
        while left > 0.0 {
            let [x, y, z] = here.map(|c| c.floor() as i32);
            let state = level.in_height(y).then(|| level.registry().get(level.block(x, y, z))).flatten();
            if let Some(state) = state.filter(|state| !state.is_air()) {
                let solid = !state.is_liquid();
                left -= (if solid { withstands(state.mining.hardness, state.mining.harvest_tools != 0) } else { 100.0 } + STEP) * STEP;
                if left > 0.0 && solid && !cells.contains(&[x, y, z]) {
                    cells.push([x, y, z]);
                }
            }
            here = std::array::from_fn(|axis| here[axis] + towards[axis] / long * STEP);
            left -= FADES;
        }
    }
    cells
}

/// What a blast of `power` at `at` does to a body whose eyes are at `eyes`: what it costs on
/// this difficulty, nothing on peaceful, and the speed it throws at.
fn reaches(world: &World, at: [f32; 3], eyes: [f32; 3], power: f32) -> Option<(f32, [f32; 3])> {
    let away: [f32; 3] = std::array::from_fn(|axis| eyes[axis] - at[axis]);
    let far = away.iter().map(|c| c * c).sum::<f32>().sqrt();
    let impact = 1.0 - far / (2.0 * power);
    let harm = (impact * impact + impact) / 2.0 * HARM * 2.0 * power + 1.0;
    let harm = match world.resource::<Settings>().0.difficulty {
        Difficulty::Peaceful => 0.0,
        Difficulty::Easy => harm.min(harm / 2.0 + 1.0),
        Difficulty::Normal => harm,
        Difficulty::Hard => harm * 1.5,
    };
    (impact > 0.0 && far > 1e-4).then(|| (harm, away.map(|c| c / far * impact)))
}

/// What a blast does to the blocks it reaches: nothing, or it breaks them, and they may leave
/// their items.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Wreck {
    Nothing,
    Breaks,
    Leaves,
}

/// A blast of `power` at `at`: of `source`, if an entity made it, which is gone with it.
pub(crate) fn explode(world: &mut World, source: Option<Entity>, layer: Entity, at: [f32; 3], power: f32, wreck: Wreck) {
    let [x, y, z] = at;
    let cell = at.map(|c| c.floor() as i32);
    let flash = LevelEvent { event: LevelEventEvent::ParticleExplosion, position: Vec3f { x, y, z }, data: power as i32 };
    match source {
        Some(source) => entities::tell_viewers(world, source, layer, cell, vec![sapling_net::encode(&flash)]),
        None => blocks::to_viewers(world, layer, cell, &[sapling_net::encode(&flash)]),
    }
    let bang = LevelSoundEvent { sound_id: "explode".into(), position: Vec3f { x, y, z }, extra_data: -1, entity_type: String::new(), is_baby_mob: false, is_global: false, entity_unique_id: -1, fire_at_position: None };
    late_sounds::play_later(world, 1, layer, cell, sapling_net::encode(&bang));
    if let Some(source) = source {
        mobs::remove(world, source);
    }

    let players: Vec<(Entity, [f32; 3])> = crate::components::in_world(world).into_iter().filter_map(|player| Some((player, world.get::<Position>(player)?.feet))).filter(|(player, _)| world.get::<InLayer>(*player).map(|layer| layer.0) == Some(layer)).collect();
    for (player, feet) in players {
        let eyes = [feet[0], feet[1] + sapling_data::EYE_HEIGHT, feet[2]];
        if let Some((harm, thrown)) = reaches(world, at, eyes, power) {
            vitals::blasted(world, player, harm, at, thrown);
        }
    }
    // Items lying in it are gone, as the stacks BDS's first TNT left were with its second.
    let reach = 2.0 * power;
    for (item, _) in drops::lying_in(world, layer, at.map(|c| c - reach), at.map(|c| c + reach)) {
        let far = world.get::<Body>(item).map_or(f32::MAX, |its| (0..3).map(|axis| (its.at[axis] - at[axis]).powi(2)).sum::<f32>().sqrt());
        if far < reach {
            mobs::remove(world, item);
        }
    }
    let mut mobs = world.query::<(Entity, &InLayer, &Body, &Health, &NetId)>();
    let near: Vec<(Entity, u64, Body)> = mobs.iter(world).filter(|(_, in_layer, ..)| in_layer.0 == layer).map(|(mob, _, body, _, id)| (mob, id.0, *body)).collect();
    for (mob, id, its) in near {
        let middle = [its.at[0], its.at[1] + its.size[1] / 2.0, its.at[2]];
        let Some((harm, _)) = reaches(world, at, middle, power).filter(|(harm, _)| *harm > 0.0) else { continue };
        let Some(mut health) = world.get::<Health>(mob).copied() else { continue };
        if health.hurt(harm) {
            *world.get_mut::<Health>(mob).expect("read above") = health;
            entities::tell_viewers(world, mob, layer, its.at.map(|c| c.floor() as i32), vec![harm::strike(id, &health, Cause::EntityExplosion, None)]);
        }
    }
    if wreck != Wreck::Nothing {
        let cells = broken(world, layer, at, power);
        blocks::blow_out(world, layer, &cells, wreck == Wreck::Leaves);
    }
}

/// A tick of a lit fuse; whether the mob burst, and is gone.
pub(super) fn tick(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, body: &Body) -> bool {
    let Some(left) = world.get::<Fuse>(entity).map(|fuse| fuse.0 - 1) else { return false };
    let Some(power) = sapling_data::mob_traits(kind).hunts.and_then(|hunts| hunts.explodes).filter(|_| left < 0) else {
        world.entity_mut(entity).insert(Fuse(left));
        tell(world, entity, layer, id, kind, body, left);
        if left == FUSE - 1 - HISSES {
            let position = Vec3f { x: body.at[0], y: body.at[1] + HEAD * body.size[1], z: body.at[2] };
            let hiss = LevelSoundEvent { sound_id: "fuse".into(), position, extra_data: -1, entity_type: kind.into(), is_baby_mob: false, is_global: false, entity_unique_id: id as i64, fire_at_position: None };
            entities::tell_viewers(world, entity, layer, body.at.map(|c| c.floor() as i32), vec![sapling_net::encode(&hiss)]);
        }
        return false;
    };
    let griefs = world.get::<Layer>(layer).is_some_and(|layer| layer.world.level().rules.on(GRIEFING));
    explode(world, Some(entity), layer, body.at, power, if griefs { Wreck::Breaks } else { Wreck::Nothing });
    true
}
