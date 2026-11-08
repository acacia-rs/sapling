//! A ghast's fireballs, as BDS's flew (research/captures/bds-ghast-findings.md, bds-d3ghast.txt,
//! bds-d3ghast2.txt): straight at 1.6 a tick, hidden and alight from its first tick
//! (flight/vanish.rs), pushed off its ghast as it leaves it. One that meets a player ends its
//! whole step, stands 6 ticks while the players it overlaps push it as they push mobs
//! (crowding.rs, once a tick; BDS's took 5 or 6 pushes, research/captures/bds-tt1ghast.txt), then
//! hurts it 6, sets it alight, moves one more step and blows up there with power 1 the tick after;
//! one that meets a mob does so at once and blows up the tick after; one that meets a block blows up
//! where it meets it. Its blast lights fires where mobs may grief (blast.rs). A hit turns it back (struck_back.rs).

use acacia_proto::types::{MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain, MetadataFlags1, Vec3f};
use bevy_ecs::prelude::*;
use sapling_data::AddEntityOptions;

use super::blast::{self, Wreck};
use super::flight::{self, vanish};
use super::health::{Cause, Health};
use super::struck_back::{self, Strikable};
use super::{Creature, Dying, harm};
use crate::body::Body;
use crate::components::{InLayer, Layer, NetId, Position, Tick, in_world};
use crate::sync::Told;
use crate::{entities, mobs, vitals};

const KIND: &str = "minecraft:fireball";
const SIZE: [f32; 2] = [1.0, 1.0];
const PLAYER: [f32; 2] = [0.6, 1.8];
/// It leaves this far up its ghast and this far towards its prey, at this speed, aimed this far
/// up its prey; `disappeared` is heard this far up its ghast.
const HELD: f32 = 0.5;
const AHEAD: f32 = 1.0;
const SPEED: f32 = 1.6;
const AIMED: f32 = 0.9;
/// What it costs whom it meets, the ticks it sets them alight, the ticks it stands after meeting a
/// player, and its blast.
const HARM: f32 = 6.0;
const BURNS: u16 = 100;
const STANDS: u64 = 6;
const POWER: f32 = 1.0;
/// The Java game's `Entity.pushAwayFrom`: this hard at most, between bodies whose boxes meet.
const PUSHED: f32 = 0.05;
/// BDS's flew on; here one that met nothing goes after this many ticks, so that misses do not pile up.
const FLIES: u64 = 200;
const GRIEFING: &str = "mobGriefing";

#[derive(Component)]
struct Fireball {
    /// Its ghast, or whoever turned it, and the kind that names in a death message.
    owner: Entity,
    kind: String,
    born: u64,
    /// The tick whom it met is hurt, the player still to hurt then, and the blast the tick after.
    met: Option<(u64, Option<Entity>)>,
}

/// How much a body at `from` pushes one at `to` across, as the Java game's `pushAwayFrom`.
fn pushed_from(from: [f32; 3], to: [f32; 3]) -> [f32; 2] {
    let (dx, dz) = (to[0] - from[0], to[2] - from[2]);
    let most = dx.abs().max(dz.abs());
    if most < 0.01 {
        return [0.0; 2];
    }
    let root = most.sqrt();
    let scale = (1.0 / root).min(1.0) / root * PUSHED;
    [dx * scale, dz * scale]
}

/// A ghast of `kind` with the body `ghast` shoots a fireball at a player whose feet are `prey`.
pub(super) fn fire(world: &mut World, layer: Entity, ghast: Entity, kind: &str, body: &Body, prey: [f32; 3]) {
    let mouth = [body.at[0], body.at[1] + HELD, body.at[2]];
    let aim: [f32; 3] = std::array::from_fn(|axis| prey[axis] + if axis == 1 { AIMED } else { 0.0 } - mouth[axis]);
    let long = aim.iter().map(|c| c * c).sum::<f32>().sqrt().max(1e-4);
    let across = aim[0].hypot(aim[2]).max(1e-4);
    let from = [mouth[0] + aim[0] / across * AHEAD, mouth[1], mouth[2] + aim[2] / across * AHEAD];
    let shot = aim.map(|c| c / long * SPEED);
    let owner = world.get::<NetId>(ghast).map_or(-1, |id| id.0 as i64);
    let announce = |id: u64| {
        let options = AddEntityOptions { id, identifier: KIND, feet: from, yaw: 0.0, pitch: 0.0, variant: 0, mark_variant: 0, color: 0, baby: false, flags: MetadataFlags1::INVISIBLE, health: None, name: "" };
        let mut shown = sapling_data::add_entity(&options);
        shown.velocity = Vec3f { x: shot[0], y: shot[1], z: shot[2] };
        shown.metadata.retain(|item| item.key != Key::OwnerEid);
        shown.metadata.push(MetadataDictionaryItem { key: Key::OwnerEid, r#type: Type::Long, legacy_type: 7, value: Value::Default(Plain::Long(owner)) });
        Some(vec![sapling_net::encode(&shown)])
    };
    // It leaves inside its ghast's box, which pushes it off, and it the ghast a tick later (bds-d3ghast.txt).
    let [px, pz] = pushed_from(body.at, from);
    let motion = [shot[0] + px, shot[1], shot[2] + pz];
    let Some(fireball) = entities::spawn_mob(world, layer, KIND, Body::new(from, motion, SIZE), from, announce) else { return };
    if let Some(mut moving) = world.get_mut::<Body>(ghast) {
        moving.jostle([-px, -pz]);
    }
    let born = world.resource::<Tick>().0;
    world.entity_mut(fireball).insert((Fireball { owner: ghast, kind: kind.to_owned(), born, met: None }, Strikable::Turns));
    let Some(id) = world.get::<NetId>(fireball).map(|id| id.0) else { return };
    vanish::hide_mobs(world, layer, KIND, id, mouth);
    let alight = world.get_mut::<Told>(fireball).and_then(|mut told| told.set_flag(MetadataFlags1::ONFIRE, true));
    if let Some(word) = alight {
        entities::after_announcing(world, layer, from.map(|c| c.floor() as i32), vec![crate::sync::flags_packet(id, word)]);
    }
}

/// Whether the fireball's box, its bottom middle at `at`, reaches into a box of `size` at `feet`.
fn reaches(at: [f32; 3], feet: [f32; 3], size: [f32; 2]) -> bool {
    let across = (size[0] + SIZE[0]) / 2.0;
    (at[0] - feet[0]).abs() < across && (at[2] - feet[2]).abs() < across && at[1] + SIZE[1] > feet[1] && at[1] < feet[1] + size[1]
}

/// The player of `layer` other than `owner` whose box the fireball's at `at` reaches into, else the
/// living mob; and whether it is a player.
fn met(world: &mut World, layer: Entity, owner: Entity, at: [f32; 3]) -> Option<(Entity, bool)> {
    let player = in_world(world).into_iter().find(|player| {
        *player != owner && world.get::<InLayer>(*player).map(|layer| layer.0) == Some(layer) && world.get::<Position>(*player).is_some_and(|position| reaches(at, position.feet, PLAYER))
    });
    player.map(|player| (player, true)).or_else(|| {
        let mut mobs = world.query_filtered::<(Entity, &InLayer, &Body), (With<Creature>, With<Health>, Without<Dying>)>();
        mobs.iter(world).find(|(mob, on, body)| *mob != owner && on.0 == layer && reaches(at, body.at, body.size)).map(|(mob, ..)| (mob, false))
    })
}

fn burst(world: &mut World, fireball: Entity, layer: Entity, at: [f32; 3]) {
    let griefs = world.get::<Layer>(layer).is_some_and(|layer| layer.world.level().rules.on(GRIEFING));
    blast::explode_fireball(world, fireball, layer, at, POWER, if griefs { Wreck::Decays } else { Wreck::Nothing });
}

/// A mob the fireball met is hurt and set alight at once (bds-d3ghast2.txt).
fn strike_mob(world: &mut World, layer: Entity, mob: Entity, owner: Entity) {
    let by_player = in_world(world).contains(&owner);
    harm::wound_by(world, mob, layer, HARM, Cause::Projectile, by_player.then_some(owner));
    if let Some(mut health) = world.get_mut::<Health>(mob) {
        health.ignite(BURNS);
    }
}

/// A tick of one fireball.
fn fly(world: &mut World, fireball: Entity, layer: Entity, id: u64) -> Option<()> {
    let now = world.resource::<Tick>().0;
    let mut body = *world.get::<Body>(fireball)?;
    if let Some(by) = struck_back::turned(world, fireball) {
        let kind = world.get::<entities::Kind>(by).map_or_else(|| "minecraft:player".to_owned(), |kind| kind.0.clone());
        let mut state = world.get_mut::<Fireball>(fireball)?;
        (state.owner, state.kind, state.met) = (by, kind, None);
    }
    let (owner, kind, born, met_at) = world.get::<Fireball>(fireball).map(|ball| (ball.owner, ball.kind.clone(), ball.born, ball.met))?;
    // Standing in a player it is pushed; it hurts as it stops, takes one more step, and blows up the tick after.
    if let Some((hurts, player)) = met_at {
        if now <= hurts && player.is_some() {
            let [px, pz] = super::crowding::pushed_by_players(world, layer, &body);
            (body.motion[0], body.motion[2]) = (body.motion[0] + px, body.motion[2] + pz);
            *world.get_mut::<Body>(fireball)? = body;
        }
        if now < hurts {
            return Some(());
        }
        if let Some(player) = player.filter(|_| now == hurts) {
            if vitals::shot(world, player, HARM, &kind, body.at, false, [0.0; 3]) {
                vitals::set_alight(world, player, BURNS);
            }
            // Cut short by blocks in its way (bds-q1ghast.txt: clipped by the wall behind the actor).
            let mut clipped = body;
            clipped.shove(&world.get::<Layer>(layer)?.world, body.motion);
            tell_move(world, fireball, layer, id, &mut body, clipped.at, false)?;
        }
        if now > hurts {
            burst(world, fireball, layer, body.at);
        }
        return Some(());
    }
    if now >= born + FLIES {
        mobs::remove(world, fireball);
        return None;
    }
    vanish::show_mobs(world, layer, KIND, id, now - born, &body);
    // Looked at a quarter of a step at a time: a whole one is as long as a player is met across.
    let along = |part: f32| -> [f32; 3] { std::array::from_fn(|axis| body.at[axis] + body.motion[axis] * part) };
    let struck = [0.25, 0.5, 0.75, 1.0].into_iter().find_map(|part| met(world, layer, owner, along(part)).map(|met| (met, along(part))));
    // One that meets a player still ends its whole step (bds-tt1ghast.txt); one that meets a mob stops there.
    let to = match struck {
        Some(((_, false), at)) => at,
        _ => along(1.0),
    };
    if !Body::new(to, [0.0; 3], SIZE).is_free(&world.get::<Layer>(layer)?.world) {
        burst(world, fireball, layer, body.at);
        return None;
    }
    tell_move(world, fireball, layer, id, &mut body, to, now == born + 1)?;
    let met = match struck {
        Some(((player, true), _)) => (now + STANDS, Some(player)),
        Some(((mob, false), _)) => {
            strike_mob(world, layer, mob, owner);
            (now, None)
        }
        None => return Some(()),
    };
    world.get_mut::<Fireball>(fireball)?.met = Some(met);
    Some(())
}

/// Moves the fireball to `to` and tells its viewers; `first`, it is its first tick in flight.
fn tell_move(world: &mut World, fireball: Entity, layer: Entity, id: u64, body: &mut Body, to: [f32; 3], first: bool) -> Option<()> {
    let gone: [f32; 3] = std::array::from_fn(|axis| to[axis] - body.at[axis]);
    body.at = to;
    *world.get_mut::<Body>(fireball)? = *body;
    let mut packets = world.get_mut::<Told>(fireball)?.update(id, body.at, body.motion, false);
    packets.splice(0..0, flight::flags(world, fireball, id, gone, first));
    entities::tell_viewers(world, fireball, layer, body.at.map(|c| c.floor() as i32), packets);
    Some(())
}

/// Moves every fireball a tick.
pub(super) fn tick(world: &mut World) {
    let fireballs: Vec<(Entity, Entity, u64)> = world.query_filtered::<(Entity, &InLayer, &NetId), With<Fireball>>().iter(world).map(|(ball, layer, id)| (ball, layer.0, id.0)).collect();
    for (fireball, layer, id) in fireballs {
        fly(world, fireball, layer, id);
    }
}
