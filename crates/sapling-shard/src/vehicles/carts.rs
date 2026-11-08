//! Minecarts: placed on a rail from their item, rolled along rails (rails.rs) and falling off
//! them as any body does, pushed by those who walk into them; a detector rail is pressed under
//! one (detectors.rs), an activator rail throws out a minecart's rider and primes a TNT minecart, which bursts at
//! the end of its fuse or when it falls hard (docs/research/vehicles.md "Minecarts").

use acacia_proto::packets::{EntityEvent, EntityEventEventId, LevelEvent, LevelEventEvent, SetEntityData};
use acacia_proto::types::Vec3f;
use acacia_proto::types::{EntityProperties, MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain};
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};

use super::rails::{self, Rail, RailKind, Roll};
use super::{MINECART, TNT_MINECART, is_crowded, spawn};
use crate::body::{Body, InLiquid, Physics};
use crate::components::{InLayer, Layer, NetId, Position};
use crate::creatures;
use crate::entities::{self, Kind};
use crate::riding::{Reins, Riding};
use crate::sync::Told;

/// The wire puts a minecart this far above its box's bottom.
pub(super) const LIFT: f32 = 0.35;
/// Off a rail a cart falls without drag either way, through water and lava alike (bds-p3lava.txt:
/// 0.04, 0.08, ... a tick to a pool's floor); [`loosen`] caps and halves it before the move.
const LOOSE: Physics = Physics { gravity: 0.04, air_drag: 1.0, fall_drag: 1.0, ground_drag: 1.0, liquid: InLiquid::Ignores, burns: false, passes: false, water_drag: 0.8, flutter: 1.0, step: 0.0 };
/// Off a rail each axis across is capped at this, then halved if the cart stood on the ground off a
/// rail a tick before (bds-r3cart.txt: 1.18 off a powered runway went 0.4, 0.2 on, 0.1 once landed).
const LOOSE_CAP: f32 = 0.4;
const LOOSE_GROUND_DRAG: f32 = 0.5;
const RIDDEN_DRAG: f32 = 0.997;
const EMPTY_DRAG: f32 = 0.96;
/// A cart with goods keeps more of its motion, the less full it is: this much per step its comparator
/// signal is below 15 (bds-ffcart.txt: a full hopper minecart slowed 0.98 a tick, a plain one 0.96).
const LADEN_DRAG: f32 = 0.98;
const LADEN_EASE: f32 = 0.001;
/// The squared move across a tick past which a cart on a rail faces the way it went, `atan2(dz, dx)`:
/// BDS's turned 0, 290, 315 then 270 round a curve east to north (research/captures/bds-u3yaw.txt).
const TURNS_PAST: f32 = 0.001;
pub(super) const FUSE: i32 = 80;
/// The fuse of one a blast reached: from this many ticks, to this many more.
pub(super) const SHORT_FUSE: [i32; 2] = [10, 20];
const BLAST: f32 = 3.0;
/// A TNT minecart falling further than this bursts as it lands.
const HARD_FALL: f32 = 3.5;
/// Others push a cart from this near, by this much (BDS `legacy_minecart` push).
const NEAR: f32 = 0.2;
const PUSH: f32 = 0.1;
const PUSHED_SHARE: f32 = 0.5;
const PUSHER_SHARE: f32 = 0.25;

/// A minecart's own part: the fuse of a primed TNT minecart, how far it fell, and whether it rolled
/// on a rail its last tick.
#[derive(Component, Clone, Copy, Default)]
pub(super) struct Cart {
    fuse: Option<i32>,
    fallen: f32,
    railed: bool,
}

/// A cart's motion off a rail before its move: each axis across capped, halved on the ground unless
/// it came off a rail this tick (BDS's first tick off a runway went the whole 0.4, bds-r3cart.txt).
fn loosen(world: &mut World, cart: Entity) {
    let railed = world.get::<Cart>(cart).is_some_and(|state| state.railed);
    let Some(mut body) = world.get_mut::<Body>(cart) else { return };
    let drag = if body.on_ground && !railed { LOOSE_GROUND_DRAG } else { 1.0 };
    for axis in [0, 2] {
        body.motion[axis] = body.motion[axis].clamp(-LOOSE_CAP, LOOSE_CAP) * drag;
    }
}

pub(super) fn fit(world: &mut World, cart: Entity) {
    world.entity_mut(cart).insert(Cart::default());
}

/// Places a minecart of `kind` on the rail at `pos`; false off a rail or where a mob or vehicle is
/// in the rail's cell (BDS `MinecartItem::_useOn`).
pub(super) fn place(world: &mut World, layer: Entity, pos: BlockPos, kind: &str) -> bool {
    let Some(rail) = world.get::<Layer>(layer).and_then(|layer| rails::rail_at(&layer.world, pos)) else { return false };
    let [x, y, z] = rail.pos.map(|c| c as f32);
    if is_crowded(world, layer, ([x, y, z], [x + 1.0, y + 1.0, z + 1.0])) {
        return false;
    }
    spawn(world, layer, kind, [x + 0.5, y, z + 0.5], 0.0, 0, None).is_some()
}

/// Primes a TNT minecart to burst in `fuse` ticks; false for any other vehicle, or one primed.
pub(super) fn prime(world: &mut World, cart: Entity, fuse: i32) -> bool {
    let is_tnt = world.get::<Kind>(cart).is_some_and(|kind| kind.0 == TNT_MINECART);
    let Some(mut state) = world.get_mut::<Cart>(cart).filter(|state| is_tnt && state.fuse.is_none()) else { return false };
    state.fuse = Some(fuse);
    true
}

/// [`prime`] by a hurt from fire or lava.
pub(super) fn prime_burning(world: &mut World, cart: Entity) {
    prime_lit(world, cart, FUSE);
}

/// [`prime`] by fire, which BDS showed with `MinecartTntPrimeFuse` and `SoundIgnite` at the wire point (bds-fftnt.txt).
pub(super) fn prime_lit(world: &mut World, cart: Entity, fuse: i32) {
    let (Some(id), Some(layer), Some(body)) = (world.get::<NetId>(cart).map(|id| id.0), world.get::<InLayer>(cart).map(|layer| layer.0), world.get::<Body>(cart).copied()) else { return };
    if prime(world, cart, fuse) {
        let shown = EntityEvent { runtime_entity_id: id, event_id: EntityEventEventId::MinecartTntPrimeFuse, data: 0, fire_at_position: None };
        let [x, y, z] = body.at;
        let lit = LevelEvent { event: LevelEventEvent::SoundIgnite, position: Vec3f { x, y: y + LIFT, z }, data: 0 };
        entities::tell_viewers(world, cart, layer, body.at.map(|c| c.floor() as i32), vec![sapling_net::encode(&shown), sapling_net::encode(&lit)]);
    }
}

/// The pack's `ignite` of a TNT minecart lit by hand, at the cart (research/captures/bds-fftnt.txt).
pub(super) fn ignite_heard(world: &mut World, cart: Entity) {
    let (Some(layer), Some(body), Some(id)) = (world.get::<InLayer>(cart).map(|layer| layer.0), world.get::<Body>(cart).copied(), world.get::<NetId>(cart).map(|id| id.0)) else { return };
    let [x, y, z] = body.at;
    let position = Vec3f { x, y: y + LIFT, z };
    let heard = acacia_proto::packets::LevelSoundEvent { sound_id: "ignite".into(), position, extra_data: -1, entity_type: TNT_MINECART.into(), is_baby_mob: false, is_global: false, entity_unique_id: id as i64, fire_at_position: None };
    entities::tell_viewers(world, cart, layer, body.at.map(|c| c.floor() as i32), vec![sapling_net::encode(&heard)]);
}

fn fuse_shown(id: u64, fuse: i32) -> bytes::Bytes {
    let item = MetadataDictionaryItem { key: Key::FuseLength, r#type: Type::Int, legacy_type: 2, value: Value::Default(Plain::Int(fuse)) };
    sapling_net::encode(&SetEntityData { runtime_entity_id: id, metadata: vec![item], properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 })
}

/// How a cart rolls this tick: its drag, its rider's push, the solid blocks at its rail's ends.
fn how(world: &World, cart: Entity, kind: &str, rail: &Rail, level: &sapling_world::World) -> Roll {
    let driver = world.get::<crate::riding::Seats>(cart).and_then(|seats| seats.0.first().copied());
    let pushed_to = driver.filter(|driver| world.get::<Reins>(*driver).is_some_and(|reins| reins.keys[1] > 0.0)).and_then(|driver| world.get::<Position>(driver)).map(|at| at.yaw);
    let laden = || LADEN_DRAG + f32::from(15 - super::hopped::signal(world, cart).unwrap_or(0).min(15)) * LADEN_EASE;
    let drag = if driver.is_some() { RIDDEN_DRAG } else if kind == MINECART || kind == TNT_MINECART { EMPTY_DRAG } else { laden() };
    let solid = |face: Face| {
        let [x, y, z] = face.neighbour(rail.pos);
        level.registry().get(level.block(x, y, z)).is_some_and(|state| state.is_full_cube())
    };
    Roll { drag, ridden: driver.is_some(), pushed_to, walls: [solid(Face::North), solid(Face::South), solid(Face::West), solid(Face::East)] }
}

/// Moves each minecart a tick, and lets its rail act on it.
pub(super) fn tick(world: &mut World) {
    super::detectors::tick(world);
    push(world);
    super::bumps::push(world);
    let carts: Vec<(Entity, Entity, u64, String, Body)> = world.query_filtered::<(Entity, &InLayer, &NetId, &Kind, &Body), (With<Cart>, Without<super::burning::BurntThrough>)>().iter(world).map(|(cart, layer, id, kind, body)| (cart, layer.0, id.0, kind.0.clone(), *body)).collect();
    for (cart, layer, id, kind, body) in carts {
        let parting = world.entity_mut(cart).take::<super::bumps::Parting>();
        let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { continue };
        let on = rails::rail_under(level, body.at);
        let railed = on.is_some();
        let (body, packets) = match on {
            Some(rail) => {
                let (mut feet, mut motion) = rails::roll(&rail, body.at, body.motion, &how(world, cart, &kind, &rail, level));
                if rail.shape <= 5 {
                    // Up a slope the box moves as if on its top, clear of the slope's own block (Java `moveAlongTrack`).
                    let bottom = if rail.shape <= 1 { feet[1] } else { rail.pos[1] as f32 + 1.0 };
                    (feet, motion) = super::bumps::walled(level, &body, feet, bottom, motion);
                    feet[1] = rails::height(&rail, [feet[0], feet[2]]);
                }
                // Rolled off a flat rail's end it is left at the rail block's bottom: BDS's dropped 0.15 that tick (bds-r3cart.txt).
                if rail.shape <= 1 && rails::rail_under(level, feet).is_none() {
                    feet[1] = rail.pos[1] as f32;
                }
                if let Some(stopped) = super::bumps::blocked(world, cart, layer, &body, feet) {
                    (feet, motion) = (stopped, [0.0; 3]);
                }
                if let Some(super::bumps::Parting([px, pz])) = parting {
                    (feet[0], feet[2], motion) = (feet[0] + px, feet[2] + pz, [0.0; 3]);
                }
                let Some(mut moved) = world.get_mut::<Body>(cart) else { continue };
                (moved.at, moved.motion, moved.on_ground) = (feet, motion, true);
                let moved = *moved;
                let Some(mut told) = world.get_mut::<Told>(cart) else { continue };
                let (dx, dz) = (feet[0] - body.at[0], feet[2] - body.at[2]);
                if dx * dx + dz * dz > TURNS_PAST {
                    told.face_body(dz.atan2(dx).to_degrees());
                }
                let packets = told.update(id, [feet[0], feet[1] + LIFT, feet[2]], motion, true);
                act(world, cart, &kind, &rail);
                (moved, packets)
            }
            None => {
                loosen(world, cart);
                let Some((moved, packets)) = entities::step(world, cart, layer, id, &LOOSE, LIFT) else { continue };
                (moved, packets)
            }
        };
        entities::tell_viewers(world, cart, layer, body.at.map(|c| c.floor() as i32), packets);
        // Read after the rail acted: an activator rail may have primed it.
        let Some(mut state) = world.get::<Cart>(cart).copied() else { continue };
        let landed_hard = kind == TNT_MINECART && body.on_ground && state.fallen > HARD_FALL;
        state.fallen = if body.on_ground { 0.0 } else { state.fallen - body.motion[1].min(0.0) };
        state.railed = railed;
        // A fuse is shown each tick down to 0, and bursts the tick after (research/captures/bds-fftnt.txt).
        if landed_hard || state.fuse.is_some_and(|fuse| fuse <= 0) {
            crate::riding::forget(world, cart);
            creatures::explode_primed(world, cart, layer, body.at, BLAST, crate::tnt::wreck(world, layer));
            continue;
        }
        state.fuse = state.fuse.map(|fuse| fuse - 1);
        if let Some(fuse) = state.fuse {
            entities::tell_viewers(world, cart, layer, body.at.map(|c| c.floor() as i32), vec![fuse_shown(id, fuse)]);
        }
        if let Ok(mut entity) = world.get_entity_mut(cart) {
            entity.insert((state, super::detectors::Settled));
        }
    }
}

/// What the rail under a cart does to it: a powered activator rail throws out a minecart's riders
/// and primes a TNT minecart.
fn act(world: &mut World, cart: Entity, kind: &str, rail: &Rail) {
    match rail.kind {
        RailKind::Activator if rail.on && kind == MINECART => drop(crate::riding::evict(world, cart)),
        RailKind::Activator if rail.on && kind == super::command_cart::COMMAND_MINECART => {
            let at = world.get::<crate::body::Body>(cart).map_or([0.0; 3], |body| body.at);
            super::command_cart::run(world, cart, at);
        }
        RailKind::Activator if rail.on => drop(prime(world, cart, FUSE)),
        _ => {}
    }
}

/// Mobs and players walking into a cart push it, and a mob is pushed back.
fn push(world: &mut World) {
    let carts: Vec<(Entity, Entity, Body)> = world.query_filtered::<(Entity, &InLayer, &Body), With<Cart>>().iter(world).map(|(cart, layer, body)| (cart, layer.0, *body)).collect();
    if carts.is_empty() {
        return;
    }
    let mut others: Vec<(Option<Entity>, Entity, [f32; 3], [f32; 2])> =
        world.query_filtered::<(Entity, &InLayer, &Body), (With<creatures::Creature>, Without<Riding>)>().iter(world).map(|(mob, layer, body)| (Some(mob), layer.0, body.at, body.size)).collect();
    others.extend(world.query_filtered::<(&InLayer, &Position), Without<Riding>>().iter(world).map(|(layer, at)| (None, layer.0, at.feet, [0.6, 1.8])));
    for (cart, layer, body) in carts {
        let (low, high) = body.bounds();
        for (mob, other_layer, at, [width, height]) in &others {
            let touches = *other_layer == layer && (at[0] - width / 2.0 < high[0] + NEAR && at[0] + width / 2.0 > low[0] - NEAR) && (at[2] - width / 2.0 < high[2] + NEAR && at[2] + width / 2.0 > low[2] - NEAR) && at[1] < high[1] && at[1] + height > low[1];
            let (dx, dz) = (at[0] - body.at[0], at[2] - body.at[2]);
            let far = dx.hypot(dz);
            if !touches || far < 0.01 {
                continue;
            }
            let force = (1.0 / far).min(1.0) * PUSH;
            let away = [dx / far * force, dz / far * force];
            if let Some(mut pushed) = world.get_mut::<Body>(cart) {
                (pushed.motion[0], pushed.motion[2]) = (pushed.motion[0] - away[0] * PUSHED_SHARE, pushed.motion[2] - away[1] * PUSHED_SHARE);
            }
            if let Some(mut pusher) = mob.and_then(|mob| world.get_mut::<Body>(mob)) {
                (pusher.motion[0], pusher.motion[2]) = (pusher.motion[0] + away[0] * PUSHER_SHARE, pusher.motion[2] + away[1] * PUSHER_SHARE);
            }
        }
    }
}
