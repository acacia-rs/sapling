//! Mobs that hunt players, as BDS's were seen to (docs/research/mob-attacks.md): one picks
//! the nearest player it could kill, walks straight at it and hits it when it is within reach.
// TODO: no path around a block; prey once seen is kept behind a
// wall; nothing pushes another aside; a spider hunts by day too and none holds what it hits with; peaceful worlds keep their
// monsters, idle; viewers far off are told every move, where BDS told one of every other.

use acacia_proto::packets::{EntityEvent, EntityEventEventId, LevelSoundEvent, SetEntityData};
use acacia_proto::types::{EntityProperties, MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain, Vec3f};
use bevy_ecs::prelude::*;
use sapling_api::Difficulty;

use super::{arrows, blast, legs};
use crate::body::Body;
use crate::components::{InLayer, Layer, NetId, Position, Settings, Tick, in_world};
use crate::{entities, late_sounds, vitals};

/// Ticks between two looks for a player.
const LOOKS: u64 = 10;
/// An archer comes this near its prey and no nearer.
const SHOOTS_FROM: f32 = 10.0;
/// One that blows up lights its fuse this near its prey, and puts it out this far from it.
const IGNITES: f32 = 2.5;
const GOES_OUT: f32 = 6.0;
/// A hunter's box reaches this much further across for its hit, and it hits once in this many ticks.
const REACH: f32 = 0.8;
const COOLDOWN: u64 = 20;
/// The speed its hit throws a player at, away from it and up.
const THROW: f32 = 0.4;
/// Nearer than this to its prey it stands: their boxes touch.
const TOUCHING: f32 = 0.6;
const PLAYER: [f32; 2] = [0.6, 1.8];
/// How far apart a line of sight is looked at.
const GLANCE: f32 = 0.25;
/// Its sounds come from this much of its height up: its growl, and its hit.
const HEAD: f32 = 0.85;
const MIDDLE: f32 = 0.5;

#[derive(Component, Default)]
pub(super) struct Hunter {
    prey: Option<Entity>,
    ready: u64,
}

/// Makes the mob a hunter if its kind is one.
pub(super) fn arm(world: &mut World, entity: Entity, kind: &str) {
    if sapling_data::mob_traits(kind).hunts.is_some() {
        world.entity_mut(entity).insert(Hunter::default());
    }
}

fn sound(name: &str, id: u64, kind: &str, body: &Body, up: f32) -> bytes::Bytes {
    let position = Vec3f { x: body.at[0], y: body.at[1] + up * body.size[1], z: body.at[2] };
    sapling_net::encode(&LevelSoundEvent { sound_id: name.into(), position, extra_data: -1, entity_type: kind.into(), is_baby_mob: false, is_global: false, entity_unique_id: id as i64, fire_at_position: None })
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    (0..3).map(|axis| (a[axis] - b[axis]).powi(2)).sum::<f32>().sqrt()
}

/// The feet of the player if a hunter in `layer` at `from` that sees `sight` far may hunt it.
fn seen(world: &World, player: Entity, layer: Entity, from: [f32; 3], sight: f32) -> Option<[f32; 3]> {
    let feet = world.get::<Position>(player)?.feet;
    (world.get::<InLayer>(player)?.0 == layer && vitals::is_mortal(world, player) && distance(from, feet) <= sight).then_some(feet)
}

/// Whether no full block, glass too, is between a hunter's eyes and the player's
/// (research/captures/bds-around.txt).
fn in_sight(world: &World, layer: Entity, body: &Body, feet: [f32; 3]) -> bool {
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return false };
    let (from, to) = ([body.at[0], body.at[1] + HEAD * body.size[1], body.at[2]], [feet[0], feet[1] + sapling_data::EYE_HEIGHT, feet[2]]);
    let glances = (distance(from, to) / GLANCE).ceil().max(1.0);
    let hides = |[x, y, z]: [i32; 3]| level.in_height(y) && level.registry().get(level.block(x, y, z)).is_some_and(|state| state.is_full_cube());
    (0..=glances as usize).all(|glance| !hides(std::array::from_fn(|axis| (from[axis] + (to[axis] - from[axis]) * glance as f32 / glances).floor() as i32)))
}

/// The hunter `id` has the entity `target` as its prey, or none for 0.
fn targets(id: u64, target: i64) -> bytes::Bytes {
    let item = MetadataDictionaryItem { key: Key::TargetEid, r#type: Type::Long, legacy_type: 7, value: Value::Default(Plain::Long(target)) };
    sapling_net::encode(&SetEntityData { runtime_entity_id: id, metadata: vec![item], properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 })
}

/// A hunter that died lets its prey go, which its viewers are told (bds-bite.txt).
pub(super) fn let_go(world: &mut World, entity: Entity, layer: Entity, id: u64, body: &Body) {
    if world.get_mut::<Hunter>(entity).is_some_and(|mut hunter| hunter.prey.take().is_some()) {
        entities::tell_viewers(world, entity, layer, body.at.map(|c| c.floor() as i32), vec![targets(id, 0)]);
    }
}

/// Keeps or picks the hunter's prey and tells its viewers of a change; where the prey is.
fn pick(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, body: &Body) -> Option<[f32; 3]> {
    let now = world.resource::<Tick>().0;
    let had = world.get::<Hunter>(entity)?.prey;
    let sight = sapling_data::mob_traits(kind).hunts?.sees;
    let hunts = world.resource::<Settings>().0.difficulty != Difficulty::Peaceful;
    let mut found = had.filter(|_| hunts).and_then(|prey| Some((prey, seen(world, prey, layer, body.at, sight)?)));
    if found.is_none() && hunts && (now + id).is_multiple_of(LOOKS) {
        let near = |(_, feet): &(Entity, [f32; 3])| distance(body.at, *feet);
        found = in_world(world).into_iter().filter_map(|player| Some((player, seen(world, player, layer, body.at, sight)?))).filter(|(_, feet)| in_sight(world, layer, body, *feet)).min_by(|a, b| near(a).total_cmp(&near(b)));
    }
    let prey = found.map(|(prey, _)| prey);
    if prey != had {
        world.get_mut::<Hunter>(entity)?.prey = prey;
        let target = prey.and_then(|prey| world.get::<NetId>(prey)).map_or(0, |id| id.0 as i64);
        let cell = body.at.map(|c| c.floor() as i32);
        entities::tell_viewers(world, entity, layer, cell, vec![targets(id, target)]);
        if prey.is_some() {
            late_sounds::play_later(world, 1, layer, cell, sound("mob.warning", id, kind, body, HEAD));
        }
    }
    found.map(|(_, feet)| feet)
}

/// Before the hunter's move: it walks at its prey, if it has any.
pub(super) fn steer(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str) -> bool {
    let (Some(body), Some(speed)) = (world.get::<Body>(entity).copied(), sapling_data::mob_traits(kind).hunts.map(|hunts| hunts.speed)) else { return false };
    let Some(feet) = pick(world, entity, layer, id, kind, &body) else { return blast::is_lit(world, entity) };
    // A lit creeper stands where it is, and an archer within its range.
    let stands = blast::is_lit(world, entity) || (sapling_data::mob_traits(kind).hunts.is_some_and(|hunts| hunts.shoots.is_some()) && distance(feet, body.at) < SHOOTS_FROM);
    match stands {
        true => legs::rest(world, entity),
        false => legs::walk(world, entity, layer, &body, [feet[0], feet[2]], speed, TOUCHING),
    }
    true
}

/// After its move: a hunter whose reach its prey is in swings and hits it, heard a tick later.
pub(super) fn strike(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, body: &Body) -> Option<()> {
    let now = world.resource::<Tick>().0;
    let hunts = sapling_data::mob_traits(kind).hunts?;
    let harm = hunts.attack;
    let (prey, ready) = world.get::<Hunter>(entity).map(|hunter| (hunter.prey, hunter.ready))?;
    if prey.is_none() && hunts.explodes.is_some() {
        blast::put_out(world, entity, layer, id, kind, body);
    }
    if let Some(every) = hunts.shoots.filter(|_| prey.is_some()) {
        // The first arrow waits as long as the next.
        let feet = world.get::<Position>(prey?)?.feet;
        if ready != 0 && now >= ready {
            arrows::loose(world, layer, kind, body, feet);
        }
        if ready == 0 || now >= ready {
            world.get_mut::<Hunter>(entity)?.ready = now + u64::from(every);
        }
        return None;
    }
    let prey = prey.filter(|_| now >= ready)?;
    let feet = world.get::<Position>(prey)?.feet;
    let (dx, dz) = (feet[0] - body.at[0], feet[2] - body.at[2]);
    if hunts.explodes.is_some() {
        // Between their boxes, as the pack's `target_nearby_sensor` counts.
        match distance(feet, body.at) - (body.size[0] + PLAYER[0]) / 2.0 {
            gap if gap < IGNITES => blast::light(world, entity, layer, id, kind, body),
            gap if gap > GOES_OUT => blast::put_out(world, entity, layer, id, kind, body),
            _ => {}
        }
        return None;
    }
    let across = (body.size[0] + PLAYER[0]) / 2.0 + REACH;
    if dx.abs() >= across || dz.abs() >= across || feet[1] >= body.at[1] + body.size[1] || body.at[1] >= feet[1] + PLAYER[1] {
        return None;
    }
    world.get_mut::<Hunter>(entity)?.ready = now + COOLDOWN;
    let harm = match world.resource::<Settings>().0.difficulty {
        Difficulty::Peaceful => return None,
        Difficulty::Easy => harm.min(harm / 2.0 + 1.0),
        Difficulty::Normal => harm,
        Difficulty::Hard => harm * 1.5,
    };
    let cell = body.at.map(|c| c.floor() as i32);
    let swing = EntityEvent { runtime_entity_id: id, event_id: EntityEventEventId::ArmSwing, data: 0, fire_at_position: None };
    entities::tell_viewers(world, entity, layer, cell, vec![sapling_net::encode(&swing)]);
    let far = dx.hypot(dz);
    let away = if far > 1e-4 { [dx / far, dz / far] } else { [0.0; 2] };
    let bitten = vitals::bitten(world, prey, harm, kind, [away[0] * THROW, THROW, away[1] * THROW]);
    if let Some((effect, ticks)) = hunts.gives.filter(|_| bitten) {
        vitals::afflict(world, prey, effect, ticks);
    }
    late_sounds::play_later(world, 1, layer, cell, sound("attack", id, kind, body, MIDDLE));
    Some(())
}
