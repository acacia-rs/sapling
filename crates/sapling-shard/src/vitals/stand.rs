//! What the place a living player stands in does to it each tick: a fall, fire, lava, water over
//! its head (less often with Respiration), a block around it, thorns and the void (hazards.rs),
//! powder snow (frost.rs), and the hunger its moves cost; boots with Frost Walker freeze the water
//! around, with Soul Speed quicken it on soul sand (soles.rs).

use acacia_proto::packets::LevelSoundEvent;
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_data::EYE_HEIGHT;
use sapling_data::enchant;

use super::{frost, hazards, soles};
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
/// A landing that costs more than this is a big one.
const BIG_FALL: f32 = 4.0;

/// The sounds of a player's landing, heard by the others a tick later as on BDS
/// (research/captures/bds-around.txt, bds-hurt.txt): from where the landing tick began, `land`
/// naming the block landed on, or for a landing that hurts `fall.small` or `fall.big` and `fall`;
/// with fall damage off a hurting one is `fall` alone.
fn sound_landing(world: &mut World, player: Entity, layer: Entity, from: [f32; 3], feet: [f32; 3], harm: Option<f32>, spared: bool) {
    let (Some(id), Some(level)) = (world.get::<NetId>(player).map(|id| id.0), world.get::<Layer>(layer).map(|layer| &layer.world)) else { return };
    let cell = [feet[0].floor() as i32, (feet[1] - 0.5).floor() as i32, feet[2].floor() as i32];
    let under = level.registry().network_hash(level.block(cell[0], cell[1], cell[2])) as i32;
    let [x, y, z] = from;
    let sound = |sound_id: &str, extra_data: i32| {
        sapling_net::encode(&LevelSoundEvent { sound_id: sound_id.into(), position: Vec3f { x, y, z }, extra_data, entity_type: "minecraft:player".into(), is_baby_mob: false, is_global: false, entity_unique_id: id as i64, fire_at_position: None })
    };
    let sounds = match harm {
        Some(harm) if !spared => vec![sound(if harm > BIG_FALL { "fall.big" } else { "fall.small" }, -1), sound("fall", under)],
        Some(_) => vec![sound("fall", under)],
        None => vec![sound("land", under)],
    };
    for sound in sounds {
        crate::late_sounds::play_later_but(world, 1, layer, cell, Some(player), sound);
    }
}

/// Whether a player's hit now is critical: it falls, not in a liquid (which ends a fall), riding or
/// blind (research/captures/bds-crit.txt).
// TODO: nor climbing.
pub(crate) fn falls_freely(world: &World, player: Entity) -> bool {
    let (Some(vitals), Some(position)) = (world.get::<Vitals>(player), world.get::<Position>(player)) else { return false };
    let blind = effects::level(world, player, sapling_api::Effect::Blindness).is_some();
    let feet = position.feet[1];
    !position.on_ground && vitals.health.is_airborne() && vitals.health.dropped(feet, feet) > 0.0 && !blind && world.get::<crate::riding::Riding>(player).is_none()
}

/// How far a player off the ground has fallen; none on it or riding.
pub(crate) fn fallen(world: &World, player: Entity) -> f32 {
    let (Some(vitals), Some(position)) = (world.get::<Vitals>(player), world.get::<Position>(player)) else { return 0.0 };
    let feet = position.feet[1];
    let falls = !position.on_ground && vitals.health.is_airborne() && world.get::<crate::riding::Riding>(player).is_none();
    if falls { vitals.health.dropped(feet, feet) } else { 0.0 }
}

/// A player's fall so far no longer counts: one from where it is starts anew.
pub(crate) fn break_fall(world: &mut World, player: Entity) {
    let Some(feet) = world.get::<Position>(player).map(|position| position.feet[1]) else { return };
    if let Some(mut vitals) = world.get_mut::<Vitals>(player) {
        vitals.health.break_fall(feet);
    }
}

pub(super) fn stand(world: &mut World, player: Entity, layer: Entity) {
    dress(world, player);
    let warding = crate::enchants::warding(world, player);
    // bds-fx2.txt: under conduit power a drowning player's air filled again as out of water.
    let gilled = [sapling_api::Effect::WaterBreathing, sapling_api::Effect::ConduitPower].into_iter().any(|effect| effects::level(world, player, effect).is_some());
    let into_wall = hazards::into_wall(world, player);
    let (Some(position), Some(mut vitals)) = (world.get::<Position>(player).copied(), world.get::<Vitals>(player).copied()) else { return };
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return };
    let rules = level.level().rules.clone();
    let body = Body::new(position.feet, [0.0; 3], SIZE);
    let (fire, lava, water) = (touches(level, &body, FIERY), touches(level, &body, LAVA), touches(level, &body, WATER));
    let (thorns, in_void, in_snow) = (hazards::thorns(level, &body), hazards::in_void(level, position.feet[1]), touches(level, &body, frost::POWDER_SNOW));
    let on_soul = soles::on_soul(level, position);
    let [low, eyes] = [0.0, EYE_HEIGHT].map(|up| [position.feet[0], position.feet[1] + up, position.feet[2]].map(|c| c.floor() as i32));
    // BDS lays a player with only its head in a block down, unhurt.
    let (under_water, in_wall) = (holds(level, eyes, WATER) && !gilled, is_full_block(level, eyes) && is_full_block(level, low));
    // A teleport is no fall: until the client follows it, the height starts anew.
    let (carried, sprinting, jumps) = world.get_mut::<Motion>(player).map_or((false, false, 0), |mut motion| (motion.awaits_teleport(), motion.pose.sprinting, std::mem::take(&mut motion.jumps)));
    let freezes = frost::chill(world, player, in_snow);
    soles::tread(world, player, on_soul.then_some(warding.soul_speed));
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
    let landed = position.on_ground && health.is_airborne() && !(water || lava || carried);
    let before = *health;
    let fell = health.fall(from, fallen_to, position.on_ground, water || lava || carried).map(|_| Cause::Fall);
    let landing = landed.then(|| (health.landing(), crate::rules::spares_player(&rules, Cause::Fall)));
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
    let before = *health;
    let moved = [position.feet[0] - from[0], position.feet[2] - from[2]];
    let pricked = (hazards::pricks(thorns, moved) && health.hurt(health.warded(health.shielded(hazards::PRICKS), Cause::Contact))).then_some(Cause::Contact);
    take(health, before, pricked);
    let before = *health;
    let fell_out = (in_void && health.hurt(hazards::VOID_HARM)).then_some(Cause::Void);
    take(health, before, fell_out);
    let before = *health;
    let flown = into_wall.is_some_and(|harm| health.hurt(harm)).then_some(Cause::FlyIntoWall);
    take(health, before, flown);
    let before = *health;
    let frozen = (freezes && health.hurt(frost::FREEZE_HARM)).then_some(Cause::Freezing);
    take(health, before, frozen);
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
        Some(hunger::Turn::Mend) => vitals.health.mend(crate::life_events::healed(world, player, 1.0, sapling_api::HealSource::Natural)),
        Some(hunger::Turn::Starve) => hits.extend((left > STARVED && vitals.health.hurt(1.0)).then_some(Cause::Starving)),
        None => {}
    }
    if !crate::life_events::may_lose_food(world, player, fed.shown().0, vitals.hunger.shown().0) {
        vitals.hunger.keep_food(fed.shown().0);
    }
    // Plugins may spare the player what its surroundings took this tick, or change how much.
    if let Some(&cause) = hits.last().filter(|_| vitals.health.lost_since(&spared) > 0.0)
        && !crate::damage::weigh(world, player, &mut vitals.health, &spared, cause)
    {
        hits.clear();
    }
    *world.get_mut::<Vitals>(player).expect("read above") = vitals;
    if let Some(&cause) = hits.last() {
        super::totem::rescue(world, player, cause);
    }
    let is_alive = world.get::<Vitals>(player).is_some_and(|vitals| vitals.health.left() > 0.0);
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
    if let Some((harm, spared)) = landing {
        sound_landing(world, player, layer, from, position.feet, harm, spared);
    }
    if let Some(harm) = into_wall.filter(|_| hits.contains(&Cause::FlyIntoWall)) {
        hazards::sound_wall(world, player, layer, harm);
    }
    crate::blocks::trample(world, layer, position.feet, dropped);
    let sneaking = world.get::<Motion>(player).is_some_and(|motion| motion.pose.sneaking);
    crate::blocks::tread_turtle_eggs(world, layer, "minecraft:player", position.feet, position.on_ground, landed, sneaking);
    if position.on_ground {
        crate::blocks::tread_ore(world, layer, position.feet);
    }
    if run > 0.0 && !carried {
        crate::blocks::frost_walk(world, layer, position.feet, warding.frost_walker);
    }
}
