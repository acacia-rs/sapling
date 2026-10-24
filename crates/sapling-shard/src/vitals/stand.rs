//! What the place a living player stands in does to it each tick: a fall, fire, lava, water over
//! its head (less often with Respiration), a block around it, and the hunger its moves cost;
//! boots with Frost Walker freeze the water around.

use bevy_ecs::prelude::*;
use sapling_data::EYE_HEIGHT;
use sapling_data::enchant;

use super::{
    AIR, BREATH, DROWN_HARM, FIERY, GASP, JUMP_TIRES, LAVA, LEAP_TIRES, LONGEST_STEP, SIZE, SPRINT_TIRES, STARVED, Vitals, WALLED, WATER, dress, effects, flash, hunger,
    settle, tell,
};
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{Layer, NetId, Position, Tick};
use crate::creatures::{Cause, Health, holds, is_full_block, touches};
use crate::movement::Motion;

const NATURAL_REGENERATION: &str = "naturalRegeneration";

pub(super) fn stand(world: &mut World, player: Entity, layer: Entity) {
    dress(world, player);
    let warding = crate::enchants::warding(world, player);
    let gilled = effects::level(world, player, sapling_api::Effect::WaterBreathing).is_some();
    let (Some(position), Some(mut vitals)) = (world.get::<Position>(player).copied(), world.get::<Vitals>(player).copied()) else { return };
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return };
    let rules = level.level().rules.clone();
    let body = Body::new(position.feet, [0.0; 3], SIZE);
    let (fire, lava, water) = (touches(level, &body, FIERY), touches(level, &body, LAVA), touches(level, &body, WATER));
    let [low, eyes] = [0.0, EYE_HEIGHT].map(|up| [position.feet[0], position.feet[1] + up, position.feet[2]].map(|c| c.floor() as i32));
    // BDS lays a player with only its head in a block down, unhurt.
    let (under_water, in_wall) = (holds(level, eyes, WATER) && !gilled, is_full_block(level, eyes) && is_full_block(level, low));
    // A teleport is no fall: until the client follows it, the height starts anew.
    let (carried, sprinting, jumps) = world.get_mut::<Motion>(player).map_or((false, false, 0), |mut motion| (motion.awaits_teleport(), motion.pose.sprinting, std::mem::take(&mut motion.jumps)));
    let from = vitals.was.replace(position.feet).unwrap_or(position.feet);
    let run = (position.feet[0] - from[0]).hypot(position.feet[2] - from[2]);
    let ran = if sprinting && position.on_ground && !carried && run <= LONGEST_STEP { run } else { 0.0 };
    let fed = vitals.hunger;
    if rules.on(sapling_data::HUNGER) {
        vitals.hunger.tire(f32::from(jumps) * if sprinting { LEAP_TIRES } else { JUMP_TIRES } + ran * SPRINT_TIRES);
    }
    let (health, breath) = (&mut vitals.health, vitals.air);
    let mut hits = Vec::new();
    health.begin();
    let spared = *health;
    // A hit the world's rules spare is undone, its timers kept (rules::spares_player).
    let mut take = |health: &mut Health, before: Health, hit: Option<Cause>| match hit {
        Some(cause) if crate::rules::spares_player(&rules, cause) => health.reweigh(&before, None),
        Some(cause) => hits.push(cause),
        None => {}
    };
    // BDS does not count the move a player lands in: 12 blocks cost it 8 (bds-hurt.txt).
    let fallen_to = if position.on_ground { position.feet[1].max(from[1]) } else { position.feet[1] };
    let dropped = if position.on_ground && !(water || lava || carried) { health.dropped(from[1], fallen_to) } else { 0.0 };
    let before = *health;
    let fell = health.fall(from, fallen_to, position.on_ground, water || lava || carried).map(|_| Cause::Fall);
    take(health, before, fell);
    let (lit, before) = (health.is_burning(), *health);
    let burnt = health.burn(fire, water).map(|cause| if lit { Cause::FireTick } else { cause });
    take(health, before, burnt);
    let before = *health;
    let scalded = (lava && health.scald()).then_some(Cause::Lava);
    take(health, before, scalded);
    let before = *health;
    let walled = health.smother(in_wall, WALLED).then_some(Cause::Suffocation);
    take(health, before, walled);
    vitals.air = match under_water || in_wall {
        true if warding.respiration > 0 && world.get_resource_or_insert_with(Dice::default).fraction() < enchant::holds_breath(warding.respiration) => breath,
        true => breath - 1,
        false => (breath + BREATH).min(AIR),
    };
    if vitals.air <= -GASP {
        vitals.air = 0;
        let before = vitals.health;
        let drowned = (under_water && vitals.health.hurt(DROWN_HARM)).then_some(Cause::Drowning);
        take(&mut vitals.health, before, drowned);
    }
    let left = vitals.health.left();
    match vitals.hunger.tick(rules.on(NATURAL_REGENERATION) && left > 0.0 && left < vitals.health.max()) {
        Some(hunger::Turn::Mend) => vitals.health.mend(1.0),
        Some(hunger::Turn::Starve) => hits.extend((left > STARVED && vitals.health.hurt(1.0)).then_some(Cause::Starving)),
        None => {}
    }
    // Plugins may spare the player what its surroundings took this tick, or change how much.
    if let Some(&cause) = hits.last().filter(|_| vitals.health.lost_since(&spared) > 0.0)
        && !crate::damage::weigh(world, player, &mut vitals.health, &spared, cause)
    {
        hits.clear();
    }
    let is_alive = vitals.health.left() > 0.0;
    *world.get_mut::<Vitals>(player).expect("read above") = vitals;
    if let Some(id) = world.get::<NetId>(player).map(|id| id.0).filter(|_| vitals.air != breath) {
        tell(world, player, &sapling_data::mob_air(id, vitals.air));
    }
    if let Some(id) = world.get::<NetId>(player).map(|id| id.0).filter(|_| vitals.hunger.shown() != fed.shown()) {
        let tick = world.resource::<Tick>().0;
        tell(world, player, &vitals.hunger.told(id, tick));
    }
    for cause in hits.iter().filter(|_| is_alive) {
        flash(world, player, *cause);
    }
    settle(world, player, hits.last().copied().unwrap_or(Cause::Fire));
    crate::blocks::trample(world, layer, position.feet, dropped);
    if position.on_ground {
        crate::blocks::tread_ore(world, layer, position.feet);
    }
    if run > 0.0 && !carried {
        crate::blocks::frost_walk(world, layer, position.feet, warding.frost_walker);
    }
}
