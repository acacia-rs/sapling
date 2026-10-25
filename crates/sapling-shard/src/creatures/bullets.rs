//! A shulker's bullets, as BDS's flew (research/captures/bds-shulker-findings.md): the Java game's
//! `ShulkerBullet`, which flies along one axis at a time towards its target and turns where it is
//! level with it, or after 10 to 50 ticks; whom it meets takes 4 and floats up for ten seconds.
// TODO: a bullet is not shown hidden on its first tick nor turned as it flies; it hits no mob and
// cannot be shot down; a shulker shows no idle peeks and does not teleport.

use acacia_proto::packets::{LevelSoundEvent, SetEntityData};
use acacia_proto::types::{EntityProperties, MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain, MetadataFlags1, Vec3f};
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_data::AddEntityOptions;
use sapling_world::World as Level;

use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{InLayer, Layer, NetId, Position, in_world};
use crate::sync::Told;
use crate::{entities, late_sounds, mobs, vitals};

/// Its name in `mob-traits.tsv` (`shot=`).
pub(super) const NAME: &str = "shulker_bullet";
const KIND: &str = "minecraft:shulker_bullet";
const SHULKER: &str = "minecraft:shulker";
const SIZE: [f32; 2] = [0.3125, 0.3125];
const PLAYER: [f32; 2] = [0.6, 1.8];
/// It leaves this far up its shulker and this far towards its target; `shoot` is heard this far up.
const HELD: f32 = 0.692;
const AHEAD: f32 = 1.0;
const SHOOT_UP: f32 = 0.99;
/// The speed it is drawn to on a new leg, growing by this much a tick, and how much of the way
/// there it gets in a tick.
const PULL: f32 = 0.15;
const GROWS: f32 = 1.025;
const EASES: f32 = 0.2;
/// This near its target's cell it makes straight for it; a leg lasts this many ticks, so many times over.
const NEAR: f32 = 2.0;
const LEG: u32 = 10;
const LEGS: u32 = 5;
/// It falls this much a tick more once its target is gone.
const FALLS: f32 = 0.04;
/// Whom it meets takes this and floats for this many ticks; a box is met this much wider each way.
const HARM: f32 = 4.0;
const LIFTS: u32 = 200;
const REACH: f32 = 0.3;
/// A shulker with prey is open this far.
const OPEN: i32 = 100;

#[derive(Component)]
struct Bullet {
    by: [f32; 3],
    target: Entity,
    pull: [f32; 3],
    /// The axis it flies along and which way, if it is not making straight for its target.
    heading: Option<(usize, i32)>,
    steps: u32,
    /// Where it met someone or something, the tick before it goes.
    spent: Option<[f32; 3]>,
}

fn heard(name: &str, [x, y, z]: [f32; 3], of: &str, id: i64) -> Bytes {
    sapling_net::encode(&LevelSoundEvent { sound_id: name.into(), position: Vec3f { x, y, z }, extra_data: -1, entity_type: of.into(), is_baby_mob: false, is_global: false, entity_unique_id: id, fire_at_position: None })
}

/// The shulker `id` opens for its prey or closes, heard a tick later; nothing for other kinds.
pub(super) fn peek(world: &mut World, layer: Entity, id: u64, kind: &str, feet: [f32; 3], open: bool) -> Option<Bytes> {
    (kind == SHULKER).then(|| {
        let sound = heard(if open { "shulker.open" } else { "shulker.close" }, feet, SHULKER, id as i64);
        late_sounds::play_later(world, 1, layer, feet.map(|c| c.floor() as i32), sound);
        let item = MetadataDictionaryItem { key: Key::ShulkerPeekId, r#type: Type::Int, legacy_type: 2, value: Value::Default(Plain::Int(if open { OPEN } else { 0 })) };
        sapling_net::encode(&SetEntityData { runtime_entity_id: id, metadata: vec![item], properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 })
    })
}

fn is_empty(level: &Level, [x, y, z]: [i32; 3]) -> bool {
    Body::new([x as f32 + 0.5, y as f32 + 0.05, z as f32 + 0.5], [0.0; 3], [0.9, 0.9]).is_free(level)
}

/// The cell of its target's middle, which it turns level with.
fn goal(feet: [f32; 3]) -> [i32; 3] {
    [feet[0], feet[1] + PLAYER[1] / 2.0, feet[2]].map(|c| c.floor() as i32)
}

/// A new leg from `at` towards the target at `feet`, along another axis than `off`.
fn turn(level: &Level, dice: &mut Dice, at: [f32; 3], feet: [f32; 3], off: Option<usize>) -> ([f32; 3], Option<(usize, i32)>, u32) {
    let cell = goal(feet);
    let mut aim = [cell[0] as f32 + 0.5, cell[1] as f32 + PLAYER[1] / 2.0, cell[2] as f32 + 0.5];
    let middle = cell.map(|c| c as f32 + 0.5);
    let heading = ((0..3).map(|axis| (middle[axis] - at[axis]).powi(2)).sum::<f32>() >= NEAR * NEAR).then(|| {
        let own = at.map(|c| c.floor() as i32);
        let step = |(axis, sign): (usize, i32)| std::array::from_fn::<i32, 3, _>(|a| own[a] + if a == axis { sign } else { 0 });
        let open: Vec<(usize, i32)> =
            (0..3).filter(|axis| Some(*axis) != off).filter_map(|axis| Some((axis, (cell[axis] - own[axis]).signum())).filter(|(_, sign)| *sign != 0)).filter(|way| is_empty(level, step(*way))).collect();
        let any = |dice: &mut Dice| { let roll = dice.roll(6) as usize; (roll / 2, if roll % 2 == 0 { -1 } else { 1 }) };
        match open.is_empty() {
            true => {
                let mut way = any(dice);
                for _ in 0..5 {
                    if is_empty(level, step(way)) {
                        break;
                    }
                    way = any(dice);
                }
                way
            }
            false => open[dice.roll(open.len() as u32) as usize],
        }
    });
    if let Some((axis, sign)) = heading {
        aim = std::array::from_fn(|a| at[a] + if a == axis { sign as f32 } else { 0.0 });
    }
    let delta: [f32; 3] = std::array::from_fn(|a| aim[a] - at[a]);
    let far = delta.iter().map(|c| c * c).sum::<f32>().sqrt();
    let pull = if far == 0.0 { [0.0; 3] } else { delta.map(|c| c / far * PULL) };
    (pull, heading, LEG + LEG * dice.roll(LEGS))
}

/// [`turn`] in the level of `layer`, by the shard's dice.
fn turn_in(world: &mut World, layer: Entity, at: [f32; 3], feet: [f32; 3], off: Option<usize>) -> Option<([f32; 3], Option<(usize, i32)>, u32)> {
    world.get_resource_or_insert_with(Dice::default);
    world.resource_scope(|world, mut dice: Mut<Dice>| Some(turn(&world.get::<Layer>(layer)?.world, &mut dice, at, feet, off)))
}

/// A shulker of `kind` with the body `shulker` shoots a bullet at `target`; it flies its first tick at once.
pub(super) fn fire(world: &mut World, layer: Entity, shulker: Entity, kind: &str, body: &Body, target: Entity) {
    let Some(feet) = world.get::<Position>(target).map(|position| position.feet) else { return };
    let (dx, dz) = (feet[0] - body.at[0], feet[2] - body.at[2]);
    let far = dx.hypot(dz).max(1e-4);
    let from = [body.at[0] + dx / far * AHEAD, body.at[1] + HELD, body.at[2] + dz / far * AHEAD];
    let Some((pull, heading, steps)) = turn_in(world, layer, from, feet, None) else { return };
    let owner = world.get::<NetId>(shulker).map_or(-1, |id| id.0 as i64);
    let announce = |id: u64| {
        let options = AddEntityOptions { id, identifier: KIND, feet: from, yaw: 0.0, pitch: 0.0, variant: 0, mark_variant: 0, color: 0, baby: false, flags: MetadataFlags1(0), health: None, name: "" };
        let mut shown = sapling_data::add_entity(&options);
        shown.metadata.retain(|item| item.key != Key::OwnerEid);
        shown.metadata.push(MetadataDictionaryItem { key: Key::OwnerEid, r#type: Type::Long, legacy_type: 7, value: Value::Default(Plain::Long(owner)) });
        Some(vec![sapling_net::encode(&shown)])
    };
    let Some(bullet) = entities::spawn(world, layer, KIND, Body::new(from, [0.0; 3], SIZE), from, announce) else { return };
    world.entity_mut(bullet).insert(Bullet { by: body.at, target, pull, heading, steps, spent: None });
    let up = [body.at[0], body.at[1] + SHOOT_UP, body.at[2]];
    let cell = body.at.map(|c| c.floor() as i32);
    entities::tell_viewers(world, shulker, layer, cell, vec![heard("undefined", up, "", -1), heard("shoot", up, kind, owner)]);
    if let Some(id) = world.get::<NetId>(bullet).map(|id| id.0) {
        fly(world, bullet, layer, id, kind);
    }
}

/// The player of `layer` whose box, `REACH` wider, holds `at`.
fn met(world: &World, layer: Entity, at: [f32; 3]) -> Option<Entity> {
    let across = PLAYER[0] / 2.0 + REACH;
    in_world(world).into_iter().find(|player| {
        let Some(feet) = world.get::<Position>(*player).map(|position| position.feet) else { return false };
        world.get::<InLayer>(*player).map(|layer| layer.0) == Some(layer) && vitals::is_mortal(world, *player) && (at[0] - feet[0]).abs() < across && (at[2] - feet[2]).abs() < across && at[1] > feet[1] - REACH && at[1] < feet[1] + PLAYER[1] + REACH
    })
}

/// A tick of one bullet, shot by a mob of `kind`.
fn fly(world: &mut World, bullet: Entity, layer: Entity, id: u64, kind: &str) -> Option<()> {
    let mut body = *world.get::<Body>(bullet)?;
    let state = world.get::<Bullet>(bullet)?;
    let (by, target, mut pull, heading) = (state.by, state.target, state.pull, state.heading);
    if let Some(point) = state.spent {
        mobs::remove(world, bullet);
        late_sounds::play_later(world, 0, layer, point.map(|c| c.floor() as i32), heard("bullet.hit", point, "", -1));
        return None;
    }
    let feet = world.get::<Position>(target).map(|position| position.feet);
    match feet {
        Some(_) => {
            pull = pull.map(|c| (c * GROWS).clamp(-1.0, 1.0));
            body.motion = std::array::from_fn(|a| body.motion[a] + (pull[a] - body.motion[a]) * EASES);
        }
        None => body.motion[1] -= FALLS,
    }
    let middle = [body.at[0], body.at[1] + SIZE[1] / 2.0, body.at[2]];
    let along = |part: f32| -> [f32; 3] { std::array::from_fn(|a| middle[a] + body.motion[a] * part) };
    let struck = [0.25, 0.5, 0.75, 1.0].into_iter().find_map(|part| met(world, layer, along(part)).map(|player| (player, along(part))));
    let end = along(1.0);
    let walled = !is_empty(&world.get::<Layer>(layer)?.world, end.map(|c| c.floor() as i32));
    if let Some((player, point)) = struck {
        if vitals::shot(world, player, HARM, kind, by, true, [0.0; 3]) {
            vitals::afflict(world, player, "levitation", LIFTS);
        }
        world.get_mut::<Bullet>(bullet)?.spent = Some(point);
        return Some(());
    }
    if walled {
        world.get_mut::<Bullet>(bullet)?.spent = Some(end);
        return Some(());
    }
    body.at = std::array::from_fn(|a| body.at[a] + body.motion[a]);
    *world.get_mut::<Body>(bullet)? = body;
    let packets = world.get_mut::<Told>(bullet)?.update(id, body.at, body.motion, false);
    entities::tell_viewers(world, bullet, layer, body.at.map(|c| c.floor() as i32), packets);
    let Some(feet) = feet else { return Some(()) };
    let own = body.at.map(|c| c.floor() as i32);
    let ahead = heading.map(|(axis, sign)| std::array::from_fn::<i32, 3, _>(|a| own[a] + if a == axis { sign } else { 0 }));
    let level = &world.get::<Layer>(layer)?.world;
    let blocked = ahead.is_some_and(|cell| !is_empty(level, cell));
    let level_with = heading.is_some_and(|(axis, _)| own[axis] == goal(feet)[axis]);
    let mut state = world.get_mut::<Bullet>(bullet)?;
    state.pull = pull;
    state.steps = state.steps.saturating_sub(1);
    if state.steps == 0 || blocked || level_with {
        let (pull, heading, steps) = turn_in(world, layer, body.at, feet, heading.map(|(axis, _)| axis))?;
        let mut state = world.get_mut::<Bullet>(bullet)?;
        (state.pull, state.heading, state.steps) = (pull, heading, steps);
    }
    Some(())
}

/// Moves every bullet a tick.
pub(super) fn tick(world: &mut World) {
    let bullets: Vec<(Entity, Entity, u64)> = world.query_filtered::<(Entity, &InLayer, &NetId), With<Bullet>>().iter(world).map(|(bullet, layer, id)| (bullet, layer.0, id.0)).collect();
    for (bullet, layer, id) in bullets {
        fly(world, bullet, layer, id, SHULKER);
    }
}
