//! Slimes and magma cubes get about by hopping, as BDS's (`minecraft:movement.jump`;
//! research/captures/bds-kinds2.txt): on the ground one waits out its `jump_delay`, then jumps
//! 0.42 up and ahead the way it faces, and keeps going ahead in the air. A calm one turns another
//! way now and then; one with prey (hunting.rs, `fights=hops`) faces it and waits less. Its viewers
//! see it squish as it jumps and as it lands (`EntityAge` 2 and 1), and a player its box touches is
//! hurt by its size (sizes.rs), at most once in [`TOUCHES`] ticks.
// TODO: a split one's AddEntity shows its kind's box and a velocity, then its whole dictionary, as
// BDS's; it splits only once dead, not when despawned; a magma cube's higher hop; it hops in lava; its touch hurts golems.

pub(super) mod sizes;

use acacia_proto::packets::SetEntityData;
use acacia_proto::types::{EntityProperties, MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain};
use bevy_ecs::prelude::*;

use super::genes::Gene;
use crate::body::Body;
use crate::components::{InLayer, Position, Tick, in_world};
use crate::sync::Told;
use crate::{entities, late_sounds, vitals};

/// Ticks on the ground between hops, least and most, of a slime and a magma cube, calm and with
/// prey: their packs' `jump_delay`s.
const CALM: [[u32; 2]; 2] = [[10, 30], [40, 120]];
const AGGRESSIVE: [[u32; 2]; 2] = [[3, 10], [13, 40]];
/// Of its pace, what a hop throws it ahead and what it adds in the air each tick after
/// (bds-kinds2.txt: a medium slime's 0.0856 a tick at the jump, about 0.082 after).
const HOP: f32 = 0.214;
const AIR: f32 = 0.0185;
/// Ticks between a calm one's turns, least and most.
const TURNS: [u32; 2] = [40, 100];
/// Its touch: how near boxes come, the ticks between two, and how fast it throws a player away
/// and up (the packs' `area_attack`; bds-kinds2.txt).
const TOUCH: f32 = 0.15;
const TOUCHES: u64 = 10;
const THROW: f32 = 0.4;
const PLAYER: [f32; 2] = [0.6, 1.8];
const MIDDLE: f32 = 0.5;
/// `EntityAge` as it jumps and as it lands.
const SQUISHED: i8 = 2;
const LANDED: i8 = 1;
/// How many a dead one splits into, least and most, and how far above its feet.
const SPLITS: [u32; 2] = [2, 4];
const SPLIT_UP: f32 = 0.5;

/// A slime's hopping, with a random state of its own: drawing on the shard's every tick would
/// shift every other draw.
#[derive(Component, Clone, Copy)]
pub(super) struct Hops {
    wait: u32,
    yaw: f32,
    turn_in: u32,
    state: u64,
    prey: Option<[f32; 3]>,
    touchable: u64,
    /// In the air from its own hop: only that landing squishes it, not its first.
    hopping: bool,
}

impl Hops {
    /// A number from `low` to `high`, both kept.
    fn roll(&mut self, [low, high]: [u32; 2]) -> u32 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        low + (self.state % u64::from(high - low + 1)) as u32
    }
}

/// A new slime's or magma cube's hopping, if `kind` is one.
pub(super) fn arm(world: &mut World, entity: Entity, kind: &str) {
    if sizes::KINDS.contains(&kind) {
        world.entity_mut(entity).insert(Hops { wait: CALM[0][0], yaw: 0.0, turn_in: 0, state: entity.to_bits() | 1, prey: None, touchable: 0, hopping: false });
    }
}

/// Before its move, as hunting.rs steers: it hops at the feet of its prey, if it has any.
pub(super) fn steer(world: &mut World, entity: Entity, prey: Option<[f32; 3]>) -> bool {
    if let Some(mut hops) = world.get_mut::<Hops>(entity) {
        hops.prey = prey;
    }
    true
}

fn size(world: &World, entity: Entity, kind: &str) -> Option<&'static sizes::Size> {
    sizes::of(kind, world.get::<Gene>(entity).copied())
}

fn squish(id: u64, age: i8) -> bytes::Bytes {
    let item = MetadataDictionaryItem { key: Key::EntityAge, r#type: Type::Byte, legacy_type: 0, value: Value::Default(Plain::Byte(age)) };
    sapling_net::encode(&SetEntityData { runtime_entity_id: id, metadata: vec![item], properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 })
}

/// Before its move: it turns, waits or hops.
pub(super) fn tick(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, start: &Body) {
    let (Some(mut hops), Some(size)) = (world.get::<Hops>(entity).copied(), size(world, entity, kind)) else { return };
    let magma = usize::from(kind == sizes::KINDS[1]);
    let leap = start.on_ground && hops.wait == 0;
    match hops.prey {
        Some(feet) => hops.yaw = (-(feet[0] - start.at[0])).atan2(feet[2] - start.at[2]).to_degrees(),
        None if hops.turn_in == 0 => {
            hops.yaw = hops.roll([0, 359]) as f32;
            hops.turn_in = hops.roll(TURNS);
        }
        None => {}
    }
    hops.turn_in = hops.turn_in.saturating_sub(1);
    if start.on_ground {
        let delay = if hops.prey.is_some() { AGGRESSIVE[magma] } else { CALM[magma] };
        hops.wait = if leap { hops.roll(delay) } else { hops.wait.min(delay[1]).saturating_sub(1) };
    }
    let (sin, cos) = hops.yaw.to_radians().sin_cos();
    let ahead = if leap { HOP } else if start.on_ground { 0.0 } else { AIR } * size.pace;
    hops.hopping |= leap;
    if let Some(mut kept) = world.get_mut::<Hops>(entity) {
        *kept = hops;
    }
    if leap {
        entities::tell_viewers(world, entity, layer, start.at.map(|c| c.floor() as i32), vec![squish(id, SQUISHED)]);
    }
    if let Some(mut told) = world.get_mut::<Told>(entity) {
        told.face(hops.yaw, hops.yaw);
    }
    if let Some(mut body) = world.get_mut::<Body>(entity) {
        // BDS's comes to a stop where it lands, and moves again with its next hop.
        if start.on_ground && !leap {
            (body.motion[0], body.motion[2]) = (0.0, 0.0);
        }
        body.walk([-sin * ahead, cos * ahead], leap);
    }
}

/// After its move: viewers see it land, and it hurts a player its box touches.
pub(super) fn after_move(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, start: &Body, body: &Body) -> Option<()> {
    let size = size(world, entity, kind)?;
    let cell = body.at.map(|c| c.floor() as i32);
    if !start.on_ground && body.on_ground && std::mem::take(&mut world.get_mut::<Hops>(entity)?.hopping) {
        entities::tell_viewers(world, entity, layer, cell, vec![squish(id, LANDED)]);
    }
    let now = world.resource::<Tick>().0;
    if size.touch <= 0.0 || now < world.get::<Hops>(entity)?.touchable {
        return None;
    }
    let (low, high) = body.bounds();
    let touched = in_world(world).into_iter().find(|&player| {
        let Some(feet) = world.get::<Position>(player).map(|position| position.feet) else { return false };
        let reach = [PLAYER[0] / 2.0, 0.0, PLAYER[0] / 2.0];
        let top = [PLAYER[0] / 2.0, PLAYER[1], PLAYER[0] / 2.0];
        world.get::<InLayer>(player).is_some_and(|at| at.0 == layer) && vitals::is_mortal(world, player) && (0..3).all(|axis| feet[axis] - reach[axis] < high[axis] + TOUCH && feet[axis] + top[axis] > low[axis] - TOUCH)
    })?;
    world.get_mut::<Hops>(entity)?.touchable = now + TOUCHES;
    let harm = super::on_difficulty(world, size.touch)?;
    let feet = world.get::<Position>(touched)?.feet;
    let (dx, dz) = (feet[0] - body.at[0], feet[2] - body.at[2]);
    let far = dx.hypot(dz);
    let away = if far > 1e-4 { [dx / far, dz / far] } else { [0.0; 2] };
    vitals::bitten(world, touched, harm, kind, [away[0] * THROW, THROW, away[1] * THROW]);
    late_sounds::play_later(world, 1, layer, cell, super::hunting::sound("attack", id, kind, body, MIDDLE));
    Some(())
}

/// As a dead one goes: one larger than the smallest leaves [`SPLITS`] of half its size, half a
/// block above its feet, heard `born` (bds-kinds2.txt, after `/kill`).
pub(super) fn split(world: &mut World, entity: Entity) -> Option<()> {
    let (kind, gene, layer, at) = (world.get::<entities::Kind>(entity)?.0.clone(), *world.get::<Gene>(entity)?, world.get::<InLayer>(entity)?.0, world.get::<Body>(entity)?.at);
    let smaller = Gene(gene.0 / 2);
    sizes::of(&kind, Some(smaller))?;
    let count = SPLITS[0] + world.get_resource_or_insert_with(crate::blocks::Dice::default).roll(SPLITS[1] - SPLITS[0] + 1);
    for _ in 0..count {
        super::born(world, layer, &kind, [at[0], at[1] + SPLIT_UP, at[2]], super::Birth { reason: sapling_api::SpawnReason::Split, yaw: 0.0, gene: Some(smaller), sound: "born" });
    }
    Some(())
}

/// What a slain one leaves a player: its size.
pub(super) fn experience(world: &World, entity: Entity, kind: &str) -> Option<(i32, i32)> {
    size(world, entity, kind)?;
    world.get::<Gene>(entity).map(|gene| (gene.0, gene.0))
}
