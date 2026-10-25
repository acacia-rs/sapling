//! Lightning, as BDS's `LightningBolt` struck (docs/research/wither-lightning-fireworks.md,
//! "Lightning"): a bolt a thunderstorm (storms.rs), `/summon lightning_bolt` or a trident's
//! Channeling calls down is heard as thunder and a blast, strikes the block it lands in (a rod,
//! rods.rs) and scrapes the copper under it, lights fire on normal and hard, and hurts once whom
//! it reaches: a player 5 and 8 seconds alight, a mob as its kind takes it (creatures/struck.rs).
//! It flashes again one to three times, lighting fire each time, then goes.

mod rods;
mod storms;

use acacia_proto::packets::LevelSoundEvent;
use acacia_proto::types::{MetadataFlags1, Vec3f};
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Difficulty, Weather};
use sapling_data::AddEntityOptions;

use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{InLayer, Layer, NetId, Position, Settings, in_world};
use crate::{entities, mobs, vitals};

pub(crate) const KIND: &str = "minecraft:lightning_bolt";
/// A bolt's life before its first flash ends, and its flashes after: one more than a roll of this.
const LIFE: i32 = 2;
const FLASHES: u32 = 3;
/// A flash comes when its life has run down past -3 and up to this much more.
const REFLASH: u32 = 12;
/// It reaches this far across, and from where it landed this far up.
const REACH: f32 = 3.0;
const UP: f32 = 9.0;
pub(crate) const HARM: f32 = 5.0;
pub(crate) const BURNS: u16 = 160;
/// Beside the fire where it lands it tries this many more, each up to a block off.
const MORE_FIRES: usize = 4;
const FIRE_RULE: &str = "doFireTick";

/// A bolt: its life, the flashes it has left, and whether it hurts.
#[derive(Component, Clone, Copy)]
struct Bolt {
    life: i32,
    flashes: u32,
    harms: bool,
}

fn sound(name: &str, [x, y, z]: [f32; 3]) -> bytes::Bytes {
    let sound = LevelSoundEvent { sound_id: name.into(), position: Vec3f { x, y, z }, extra_data: -1, entity_type: KIND.into(), is_baby_mob: false, is_global: false, entity_unique_id: -1, fire_at_position: None };
    sapling_net::encode(&sound)
}

/// Whether it thunders on `layer`.
pub(crate) fn thunders(world: &World, layer: Entity) -> bool {
    crate::weather::of(world, layer) == Weather::Thunder
}

/// A bolt comes down on `at` in `layer`, hurting where it reaches if it `harms`; the bolt.
pub(crate) fn strike(world: &mut World, layer: Entity, at: [f32; 3], harms: bool) -> Option<Entity> {
    let announce = |id: u64| {
        let options = AddEntityOptions { id, identifier: KIND, feet: at, yaw: 0.0, pitch: 0.0, variant: 0, mark_variant: 0, color: 0, baby: false, flags: MetadataFlags1(0), health: None, name: "" };
        Some(vec![sapling_net::encode(&sapling_data::add_entity(&options))])
    };
    let bolt = entities::spawn(world, layer, KIND, Body::new(at, [0.0; 3], [0.0, 0.0]), at, announce)?;
    let flashes = world.get_resource_or_insert_with(Dice::default).roll(FLASHES) + 1;
    world.entity_mut(bolt).insert(Bolt { life: LIFE, flashes, harms });
    let cell = at.map(|c| c.floor() as i32);
    entities::after_announcing(world, layer, cell, vec![sound("thunder", at), sound("explode", at)]);
    rods::struck(world, layer, cell);
    kindle(world, layer, cell, harms, MORE_FIRES);
    if harms {
        hurt(world, layer, at);
    }
    Some(bolt)
}

/// Fire where the bolt landed and `more` tries about it, on normal and hard where fire ticks.
fn kindle(world: &mut World, layer: Entity, cell: BlockPos, harms: bool, more: usize) {
    let difficulty = world.resource::<Settings>().0.difficulty;
    let ticks = world.get::<Layer>(layer).is_some_and(|level| level.world.level().rules.on(FIRE_RULE));
    if !harms || !ticks || !matches!(difficulty, Difficulty::Normal | Difficulty::Hard) {
        return;
    }
    crate::blocks::light_fire(world, layer, cell);
    for _ in 0..more {
        let off: [i32; 3] = {
            let mut dice = world.get_resource_or_insert_with(Dice::default);
            std::array::from_fn(|_| dice.roll(3) as i32 - 1)
        };
        crate::blocks::light_fire(world, layer, std::array::from_fn(|axis| cell[axis] + off[axis]));
    }
}

/// Strikes, once, each player and mob of `layer` whose feet are within reach of `at`.
fn hurt(world: &mut World, layer: Entity, at: [f32; 3]) {
    let reaches = |feet: [f32; 3]| (feet[0] - at[0]).abs() <= REACH && (feet[2] - at[2]).abs() <= REACH && feet[1] >= at[1] && feet[1] <= at[1] + UP;
    let players: Vec<Entity> = in_world(world).into_iter().filter(|player| world.get::<InLayer>(*player).is_some_and(|on| on.0 == layer) && world.get::<Position>(*player).is_some_and(|position| reaches(position.feet))).collect();
    let mut bodies = world.query_filtered::<(Entity, &InLayer, &Body), With<crate::creatures::Health>>();
    let mobs: Vec<Entity> = bodies.iter(world).filter(|(_, on, body)| on.0 == layer && reaches(body.at)).map(|(mob, ..)| mob).collect();
    for player in players {
        vitals::electrocuted(world, player, HARM, BURNS);
    }
    for mob in mobs {
        crate::creatures::struck_by_lightning(world, mob, layer);
    }
}

/// A tick of a bolt: its life runs down, it flashes again while it has flashes, and goes.
fn flash(world: &mut World, bolt: Entity, layer: Entity) -> Option<()> {
    let (Bolt { life, flashes, harms }, at) = (*world.get::<Bolt>(bolt)?, world.get::<Body>(bolt)?.at);
    if life < 1 && flashes == 0 {
        mobs::remove(world, bolt);
        return None;
    }
    let roll = world.get_resource_or_insert_with(Dice::default).roll(REFLASH) as i32;
    let reflashes = life < 1 && life <= -3 - roll;
    if reflashes {
        kindle(world, layer, at.map(|c| c.floor() as i32), harms, 0);
    }
    *world.get_mut::<Bolt>(bolt)? = if reflashes { Bolt { life: 1, flashes: flashes - 1, harms } } else { Bolt { life: life - 1, flashes, harms } };
    Some(())
}

/// A trident with Channeling met `struck` at `feet`: while it thunders, a bolt comes down on it
/// where the sky is open over it.
pub(crate) fn channel(world: &mut World, layer: Entity, feet: [f32; 3]) {
    let cell = feet.map(|c| c.floor() as i32);
    let open = world.get::<Layer>(layer).is_some_and(|level| level.world.surface(cell[0], cell[2]).is_none_or(|top| top < cell[1]));
    if thunders(world, layer) && open {
        let at = cell.map(|c| c as f32);
        strike(world, layer, at, true);
        entities::after_announcing(world, layer, cell, vec![sound("item.trident.thunder", at)]);
    }
}

/// Storms strike, bolts flash and go, struck rods go dark.
pub(crate) fn tick(world: &mut World) {
    storms::tick(world);
    rods::tick(world);
    let bolts: Vec<(Entity, Entity)> = world.query_filtered::<(Entity, &InLayer), (With<Bolt>, With<NetId>)>().iter(world).map(|(bolt, layer)| (bolt, layer.0)).collect();
    for (bolt, layer) in bolts {
        flash(world, bolt, layer);
    }
}
