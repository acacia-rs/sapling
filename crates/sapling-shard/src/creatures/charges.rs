//! A breeze's wind charges, as BDS's flew (research/captures/bds-breeze-findings.md,
//! bds-d3breeze.txt): straight at 0.7 a tick, hidden its first tick (flight/vanish.rs); one that
//! meets a player stops and bursts 6 ticks later, hurting it 1; one that meets a mob bursts there
//! at once, hurting it 1 and throwing it from the shooter and by the burst; one that meets a block
//! bursts there. A burst throws every player within 6 away from it, pushes mobs (its breeze too)
//! and, the tick after, swings the doors, levers and buttons its rays reach (blocks/gusts.rs). A
//! hit turns it back (struck_back.rs). BDS's flew on out of sight; here one goes after
//! [`FLIES`] ticks, so that misses do not pile up.

use acacia_proto::packets::{LevelEvent, LevelEventEvent, LevelSoundEvent};
use acacia_proto::types::{MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain, MetadataFlags1, Vec3f};
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_data::AddEntityOptions;

use super::flight::{self, vanish};
use super::health::{Cause, Health};
use super::struck_back::{self, Strikable};
use super::{Creature, Dying, harm, hits, hunting};
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{InLayer, Layer, NetId, Position, Tick, in_world};
use crate::sync::Told;
use crate::{entities, mobs, vitals};

const KIND: &str = "minecraft:breeze_wind_charge_projectile";
const SIZE: [f32; 2] = [0.3125, 0.3125];
const PLAYER: [f32; 2] = [0.6, 1.8];
/// It leaves this far up its breeze at this speed, aimed this far up its prey, each part of its
/// speed off by up to this; `shoot` is heard this far up, `disappeared` this far.
const HELD: f32 = 1.416;
const SPEED: f32 = 0.7;
const AIMED: f32 = 0.9;
const SPREAD: f32 = 0.0075;
const SHOOT_UP: f32 = 0.885;
const HIDDEN_UP: f32 = 0.9;
/// Ticks from meeting a player to bursting.
const HOLDS: u64 = 6;
const HARM: f32 = 1.0;
/// A burst reaches this far and throws this hard, at 0 off, towards a player's eyes.
const REACH: f32 = 6.0;
const PUSH: f32 = 0.6;
const EYES: f32 = 1.62;
/// Its pack's `explode` power, whose rays find the blocks it swings.
const POWER: f32 = 3.0;
/// A mob it meets is thrown this fast away from its shooter, and up, as well as by the burst.
const THROW: f32 = 0.4;
const FLIES: u64 = 200;
/// It is drawn back this far from a block it meets (a player's charge too, bds-q3wind.txt).
pub(super) const BACK: f32 = 0.25;

#[derive(Component)]
struct Charge {
    /// Its breeze, or whoever turned it, and that one's kind.
    owner: Entity,
    kind: String,
    born: u64,
    /// The tick it met a player and whom.
    met: Option<(u64, Entity)>,
    /// Where it burst, and whether on a mob.
    burst: Option<([f32; 3], bool)>,
}

fn heard(name: &str, [x, y, z]: [f32; 3], of: &str, id: i64) -> Bytes {
    sapling_net::encode(&LevelSoundEvent { sound_id: name.into(), position: Vec3f { x, y, z }, extra_data: -1, entity_type: of.into(), is_baby_mob: false, is_global: false, entity_unique_id: id, fire_at_position: None })
}

/// A breeze of `kind` with the body `breeze` shoots a charge at a player whose feet are `prey`.
pub(super) fn fire(world: &mut World, layer: Entity, breeze: Entity, kind: &str, body: &Body, prey: [f32; 3]) {
    let from = [body.at[0], body.at[1] + HELD, body.at[2]];
    let aim: [f32; 3] = std::array::from_fn(|axis| prey[axis] + if axis == 1 { AIMED } else { 0.0 } - from[axis]);
    let long = aim.iter().map(|c| c * c).sum::<f32>().sqrt().max(1e-4);
    let mut dice = world.get_resource_or_insert_with(Dice::default);
    let motion = aim.map(|c| c / long * SPEED + (dice.fraction() + dice.fraction() - 1.0) * SPREAD);
    let (yaw, pitch) = ((-motion[0]).atan2(motion[2]).to_degrees(), motion[1].atan2(motion[0].hypot(motion[2])).to_degrees());
    let owner = world.get::<NetId>(breeze).map_or(-1, |id| id.0 as i64);
    let announce = |id: u64| {
        let options = AddEntityOptions { id, identifier: KIND, feet: from, yaw, pitch, variant: 0, mark_variant: 0, color: 0, baby: false, flags: MetadataFlags1::INVISIBLE, health: None, name: "" };
        let mut shown = sapling_data::add_entity(&options);
        shown.velocity = Vec3f { x: motion[0], y: motion[1], z: motion[2] };
        shown.metadata.retain(|item| item.key != Key::OwnerEid);
        shown.metadata.push(MetadataDictionaryItem { key: Key::OwnerEid, r#type: Type::Long, legacy_type: 7, value: Value::Default(Plain::Long(owner)) });
        Some(vec![sapling_net::encode(&shown)])
    };
    let Some(charge) = entities::spawn_mob(world, layer, KIND, Body::new(from, motion, SIZE), from, announce) else { return };
    let born = world.resource::<Tick>().0;
    world.entity_mut(charge).insert((Charge { owner: breeze, kind: kind.to_owned(), born, met: None, burst: None }, Strikable::Turns));
    let Some(id) = world.get::<NetId>(charge).map(|id| id.0) else { return };
    vanish::hide_mobs(world, layer, KIND, id, [body.at[0], body.at[1] + HIDDEN_UP, body.at[2]]);
    let up = [body.at[0], body.at[1] + SHOOT_UP, body.at[2]];
    entities::tell_viewers(world, breeze, layer, body.at.map(|c| c.floor() as i32), vec![heard("shoot", up, kind, owner), heard("undefined", up, "", -1)]);
    fly(world, charge, layer, id);
}

/// Whether the charge's box, its bottom middle at `at`, reaches into a box of `size` at `feet`.
fn reaches(at: [f32; 3], feet: [f32; 3], size: [f32; 2]) -> bool {
    let across = (size[0] + SIZE[0]) / 2.0;
    (at[0] - feet[0]).abs() < across && (at[2] - feet[2]).abs() < across && at[1] + SIZE[1] > feet[1] && at[1] < feet[1] + size[1]
}

/// The player of `layer` other than `owner` whose box the charge's at `at` reaches into.
fn met(world: &World, layer: Entity, owner: Entity, at: [f32; 3]) -> Option<Entity> {
    in_world(world).into_iter().find(|player| {
        *player != owner && world.get::<InLayer>(*player).map(|layer| layer.0) == Some(layer) && world.get::<Position>(*player).is_some_and(|position| reaches(at, position.feet, PLAYER))
    })
}

/// The living mob of `layer` other than `owner` whose box the charge's at `at` reaches into.
fn met_mob(world: &mut World, layer: Entity, owner: Entity, at: [f32; 3]) -> Option<Entity> {
    let mut mobs = world.query_filtered::<(Entity, &InLayer, &Body), (With<Creature>, With<Health>, Without<Dying>)>();
    mobs.iter(world).find(|(mob, on, body)| *mob != owner && on.0 == layer && reaches(at, body.at, body.size)).map(|(mob, ..)| mob)
}

/// Where `who` stands: a mob's body or a player's feet.
fn feet_of(world: &World, who: Entity) -> Option<[f32; 3]> {
    world.get::<Body>(who).map(|body| body.at).or_else(|| world.get::<Position>(who).map(|position| position.feet))
}

/// How a wind burst reaches: this far, throwing this hard at 0 off, and costing whom it struck this.
#[derive(Clone, Copy)]
pub(super) struct Gust {
    pub reach: f32,
    pub push: f32,
    pub harm: f32,
    pub mobs: bool,
}

const BREEZE: Gust = Gust { reach: REACH, push: PUSH, harm: HARM, mobs: true };

/// A wind burst at `at` from a charge shot by `kind` at `from`: `struck`, a player, is hurt and
/// thrown from `from`, every other player within its reach is pushed towards its eyes; mobs but
/// `struck` are pushed if the gust pushes mobs.
pub(super) fn gust(world: &mut World, layer: Entity, at: [f32; 3], struck: Option<Entity>, kind: &str, from: [f32; 3], gust: Gust) {
    let near: Vec<(Entity, [f32; 3])> = in_world(world).into_iter().filter(|player| world.get::<InLayer>(*player).map(|layer| layer.0) == Some(layer)).filter_map(|player| Some((player, world.get::<Position>(player)?.feet))).collect();
    for (player, feet) in near {
        let off = (0..3).map(|axis| (feet[axis] - at[axis]).powi(2)).sum::<f32>().sqrt();
        let towards: [f32; 3] = std::array::from_fn(|axis| feet[axis] + if axis == 1 { EYES } else { 0.0 } - at[axis]);
        let long = towards.iter().map(|c| c * c).sum::<f32>().sqrt().max(1e-4);
        let push = if off < gust.reach { towards.map(|c| c / long * gust.push * (1.0 - off / gust.reach)) } else { [0.0; 3] };
        match Some(player) == struck {
            true => drop(vitals::winded(world, player, gust.harm, kind, from, push)),
            false if off < gust.reach => vitals::pushed(world, player, push),
            false => {}
        }
    }
    if gust.mobs {
        let mut mobs = world.query_filtered::<(Entity, &InLayer, &Body), With<Health>>();
        let near: Vec<(Entity, Body)> = mobs.iter(world).filter(|(mob, in_layer, _)| in_layer.0 == layer && Some(*mob) != struck).map(|(mob, _, body)| (mob, *body)).collect();
        for (mob, its) in near {
            let Some(push) = mob_push(world, layer, at, &its, gust) else { continue };
            if let Some(mut body) = world.get_mut::<Body>(mob) {
                let thrown = std::array::from_fn(|axis| body.motion[axis] + push[axis]);
                body.throw(thrown);
            }
        }
    }
}

/// How a burst at `at` pushes a mob of body `its`: as a blast of its reach, `push` times as hard
/// (bds-q3wind.txt: measured from the feet less half the width, towards 0.9 up).
fn mob_push(world: &World, layer: Entity, at: [f32; 3], its: &Body, gust: Gust) -> Option<[f32; 3]> {
    let impact = super::blast::impact(world, layer, at, its.at, its.size, gust.reach)?;
    Some(super::blast::towards(at, [its.at[0], its.at[1] + its.size[1] * super::blast::EYES, its.at[2]], impact * gust.push))
}

/// A mob the charge met is hurt and thrown away from its shooter at `from` and by the burst at
/// `at` together, and turns on the shooter (bds-d3breeze.txt).
fn strike_mob(world: &mut World, layer: Entity, mob: Entity, owner: Entity, from: [f32; 3], at: [f32; 3]) -> Option<()> {
    let by_player = in_world(world).contains(&owner);
    harm::wound_by(world, mob, layer, HARM, Cause::Projectile, by_player.then_some(owner))?;
    let its = *world.get::<Body>(mob)?;
    let push = mob_push(world, layer, at, &its, BREEZE).unwrap_or([0.0; 3]);
    let (dx, dz) = (its.at[0] - from[0], its.at[2] - from[2]);
    let far = dx.hypot(dz).max(1e-4);
    let plain = [dx / far * THROW, THROW, dz / far * THROW];
    hits::throw_at_once(world, mob, layer, std::array::from_fn(|axis| plain[axis] + push[axis]));
    hunting::hurt_by(world, mob, owner);
    Some(())
}

/// The burst at `at` on `struck`, a player or a mob, if it met one; seen there.
fn burst(world: &mut World, charge: Entity, layer: Entity, at: [f32; 3], struck: Option<(Entity, bool)>) -> Option<()> {
    let (owner, kind) = world.get::<Charge>(charge).map(|charge| (charge.owner, charge.kind.clone()))?;
    let from = feet_of(world, owner).unwrap_or(at);
    let on_mob = matches!(struck, Some((_, false)));
    if let Some((mob, false)) = struck {
        strike_mob(world, layer, mob, owner, from, at);
    }
    gust(world, layer, at, struck.map(|(struck, _)| struck), &kind, from, BREEZE);
    let [x, y, z] = at;
    let flash = LevelEvent { event: LevelEventEvent::ParticleBreezeWindExplosion, position: Vec3f { x, y, z }, data: 3 };
    let mut packets = vec![sapling_net::encode(&flash)];
    if matches!(struck, Some((_, true))) {
        packets.push(heard("undefined", at, "", -1));
    }
    entities::tell_viewers(world, charge, layer, at.map(|c| c.floor() as i32), packets);
    world.get_mut::<Charge>(charge)?.burst = Some((at, on_mob));
    Some(())
}

/// The tick after its burst at `at`: the blocks it reached swing, it goes, and the burst is heard.
fn after_burst(world: &mut World, charge: Entity, layer: Entity, at: [f32; 3], on_mob: bool) {
    let (cells, _) = super::blast::rays::broken(world, None, layer, at, POWER);
    crate::blocks::gusted(world, layer, &cells);
    let mut sounds = vec![heard("breeze_wind_charge.burst", at, "", -1)];
    if on_mob {
        sounds.push(heard("undefined", at, "", -1));
    }
    entities::tell_viewers(world, charge, layer, at.map(|c| c.floor() as i32), sounds);
    mobs::remove(world, charge);
}

/// A tick of one charge: on, turned, stopped by a player, burst, or gone the tick after its burst.
fn fly(world: &mut World, charge: Entity, layer: Entity, id: u64) -> Option<()> {
    let now = world.resource::<Tick>().0;
    let mut body = *world.get::<Body>(charge)?;
    let (met_at, burst_at, born) = world.get::<Charge>(charge).map(|charge| (charge.met, charge.burst, charge.born))?;
    if let Some((at, on_mob)) = burst_at {
        after_burst(world, charge, layer, at, on_mob);
        return None;
    }
    let met_at = match struck_back::turned(world, charge) {
        Some(by) => {
            let kind = world.get::<entities::Kind>(by).map_or_else(|| "minecraft:player".to_owned(), |kind| kind.0.clone());
            let mut state = world.get_mut::<Charge>(charge)?;
            (state.owner, state.kind, state.met) = (by, kind, None);
            None
        }
        None => met_at,
    };
    if let Some((since, player)) = met_at {
        if now >= since + HOLDS {
            burst(world, charge, layer, body.at, Some((player, true)));
        }
        return Some(());
    }
    if now >= born + FLIES {
        mobs::remove(world, charge);
        return None;
    }
    vanish::show_mobs(world, layer, KIND, id, now - born, &body);
    let owner = world.get::<Charge>(charge)?.owner;
    let along = |part: f32| -> [f32; 3] { std::array::from_fn(|axis| body.at[axis] + body.motion[axis] * part) };
    if let Some((mob, at)) = [0.25, 0.5, 0.75, 1.0].into_iter().find_map(|part| met_mob(world, layer, owner, along(part)).map(|mob| (mob, along(part)))) {
        return burst(world, charge, layer, [at[0], at[1] + SIZE[1] / 2.0, at[2]], Some((mob, false)));
    }
    let to = along(1.0);
    let middle = [to[0], to[1] + SIZE[1] / 2.0, to[2]];
    let walled = !Body::new(to, [0.0; 3], SIZE).is_free(&world.get::<Layer>(layer)?.world);
    if walled {
        let long = body.motion.iter().map(|c| c * c).sum::<f32>().sqrt().max(1e-4);
        return burst(world, charge, layer, std::array::from_fn(|axis| middle[axis] - body.motion[axis] / long * BACK), None);
    }
    let gone = body.motion;
    body.at = to;
    *world.get_mut::<Body>(charge)? = body;
    let mut packets = world.get_mut::<Told>(charge)?.update(id, body.at, body.motion, false);
    packets.splice(0..0, flight::flags(world, charge, id, gone, now == born + 1));
    entities::tell_viewers(world, charge, layer, body.at.map(|c| c.floor() as i32), packets);
    if let Some(player) = met(world, layer, owner, to) {
        world.get_mut::<Charge>(charge)?.met = Some((now, player));
    }
    Some(())
}

/// Moves every charge a tick.
pub(super) fn tick(world: &mut World) {
    let charges: Vec<(Entity, Entity, u64)> = world.query_filtered::<(Entity, &InLayer, &NetId), With<Charge>>().iter(world).map(|(charge, layer, id)| (charge, layer.0, id.0)).collect();
    for (charge, layer, id) in charges {
        fly(world, charge, layer, id);
    }
}
