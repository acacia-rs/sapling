//! Lightning, as BDS's `LightningBolt` struck (docs/research/wither-lightning-fireworks.md,
//! "Lightning"): a bolt a thunderstorm (storms.rs), `/summon lightning_bolt` or a trident's
//! Channeling calls down is heard as thunder and a blast, strikes the block it lands in (a rod or
//! copper, blocks/rods.rs; rods draw it, rods.rs), lights fire on normal and hard, and hurts once whom
//! it reaches: a player 5 and 8 seconds alight, a mob as its kind takes it (creatures/struck.rs).
//! It flashes again one to three times, lighting fire each time, then goes.

mod rods;
mod storms;

use acacia_proto::packets::LevelSoundEvent;
use acacia_proto::types::{MetadataFlags1, Vec3f};
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Difficulty};
use sapling_data::AddEntityOptions;

use crate::blocks::{Dice, LightningStrike};
use crate::body::Body;
use crate::components::{InLayer, Layer, NetId, Position, Settings, Tick, in_world};
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

/// A bolt: its life, the flashes it has left, whether it hurts, and whether it is still to land
/// (`call_down`) in a tick after the one it was `born` in.
#[derive(Component, Clone, Copy)]
struct Bolt {
    life: i32,
    flashes: u32,
    harms: bool,
    lands: bool,
    born: u64,
}

/// What a bolt still to land will do to the blocks about it.
#[derive(Component)]
struct Lands(LightningStrike);

fn sound(name: &str, [x, y, z]: [f32; 3]) -> bytes::Bytes {
    let sound = LevelSoundEvent { sound_id: name.into(), position: Vec3f { x, y, z }, extra_data: -1, entity_type: KIND.into(), is_baby_mob: false, is_global: false, entity_unique_id: -1, fire_at_position: None };
    sapling_net::encode(&sound)
}

/// Whether bolts come down on `layer`: some 95 ticks into a storm, as its rain and lightning near
/// their fullest (`weather::storms`).
pub(crate) fn thunders(world: &World, layer: Entity) -> bool {
    crate::weather::storms(world, layer)
}

/// A storm's bolt comes down on `at` in `layer` and lands at once, hurting where it reaches if it
/// `harms`; the bolt.
pub(crate) fn strike(world: &mut World, layer: Entity, at: [f32; 3], harms: bool) -> Option<Entity> {
    let bolt = spawn(world, layer, at, harms, false)?;
    let strike = crate::blocks::lightning_sparks(world, layer, cell(at));
    land(world, layer, bolt, at, &strike, harms);
    Some(bolt)
}

/// A summoned or Channeling bolt: shown now with its sparks, it lands in its first tick, a tick later,
/// as BDS's (research/captures/bds-m3struck.txt, bds-m3rod.txt, bds-rsdevices.txt, bds-o3rod.txt).
pub(crate) fn call_down(world: &mut World, layer: Entity, at: [f32; 3], harms: bool) -> Option<Entity> {
    let bolt = spawn(world, layer, at, harms, true)?;
    let strike = crate::blocks::lightning_sparks(world, layer, cell(at));
    world.entity_mut(bolt).insert(Lands(strike));
    Some(bolt)
}

fn spawn(world: &mut World, layer: Entity, at: [f32; 3], harms: bool, lands: bool) -> Option<Entity> {
    let announce = |id: u64| {
        let options = AddEntityOptions { id, identifier: KIND, feet: at, yaw: 0.0, pitch: 0.0, variant: 0, mark_variant: 0, color: 0, baby: false, flags: MetadataFlags1(0), health: None, name: "" };
        Some(vec![sapling_net::encode(&sapling_data::add_entity(&options))])
    };
    let bolt = entities::spawn(world, layer, KIND, Body::new(at, [0.0; 3], [0.0, 0.0]), at, announce)?;
    let flashes = world.get_resource_or_insert_with(Dice::default).roll(FLASHES) + 1;
    let born = world.resource::<Tick>().0;
    world.entity_mut(bolt).insert(Bolt { life: LIFE, flashes, harms, lands, born });
    Some(bolt)
}

/// The bolt's own cell, whose block its strike reaches first (docs/research/copper.md L3; a bolt in
/// a rod's cell powers it, research/captures/bds-o3rod.txt).
fn cell(at: [f32; 3]) -> BlockPos {
    at.map(|c| c.floor() as i32)
}

fn land(world: &mut World, layer: Entity, bolt: Entity, at: [f32; 3], strike: &LightningStrike, harms: bool) {
    let cell = cell(at);
    // BDS shows the struck blocks, then the thunder, then the vibration (bds-o3rod.txt, bds-z2bolt.txt).
    crate::blocks::lightning_struck(world, layer, strike);
    entities::tell_viewers(world, bolt, layer, cell, vec![sound("thunder", at), sound("explode", at)]);
    crate::blocks::lightning_felt(world, layer, strike);
    kindle(world, layer, cell, harms, MORE_FIRES);
    if harms {
        hurt(world, layer, at);
    }
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
    // Every orb and item in reach goes, netherite too (research/captures/bds-m3struck.txt).
    let mut lying = world.query_filtered::<(Entity, &InLayer, &Body), Or<(With<crate::drops::Dropped>, With<crate::orbs::Orb>)>>();
    let gone: Vec<Entity> = lying.iter(world).filter(|(_, on, body)| on.0 == layer && reaches(body.at)).map(|(item, ..)| item).collect();
    for item in gone {
        mobs::remove(world, item);
    }
}

/// A tick of a bolt: its life runs down, it flashes again while it has flashes, and goes.
fn flash(world: &mut World, bolt: Entity, layer: Entity) -> Option<()> {
    let (Bolt { life, flashes, harms, lands, born }, at) = (*world.get::<Bolt>(bolt)?, world.get::<Body>(bolt)?.at);
    if lands && world.resource::<Tick>().0 <= born {
        return Some(());
    }
    if lands {
        world.get_mut::<Bolt>(bolt)?.lands = false;
        let strike = world.entity_mut(bolt).take::<Lands>().map(|lands| lands.0).unwrap_or_default();
        land(world, layer, bolt, at, &strike, harms);
    }
    if life < 1 && flashes == 0 {
        mobs::remove(world, bolt);
        return None;
    }
    let roll = world.get_resource_or_insert_with(Dice::default).roll(REFLASH) as i32;
    let reflashes = life < 1 && life <= -3 - roll;
    if reflashes {
        kindle(world, layer, at.map(|c| c.floor() as i32), harms, 0);
    }
    let next = if reflashes { (1, flashes - 1) } else { (life - 1, flashes) };
    *world.get_mut::<Bolt>(bolt)? = Bolt { life: next.0, flashes: next.1, harms, lands: false, born };
    Some(())
}

/// A trident with Channeling met `struck` at `feet`: while it thunders, a bolt comes down on it
/// where the sky is open over it.
pub(crate) fn channel(world: &mut World, layer: Entity, feet: [f32; 3]) {
    let cell = feet.map(|c| c.floor() as i32);
    let at = cell.map(|c| c as f32);
    if channel_at(world, layer, cell, at) {
        entities::after_announcing(world, layer, cell, vec![sound("item.trident.thunder", at)]);
    }
}

/// A trident with Channeling met the lightning rod at `rod` while the sky over it is open: a bolt at
/// the middle of the rod's own cell, landing on the rod, and no `item.trident.thunder`
/// (research/captures/bds-m3rod.txt; `LightningRodBlock` slot 15, Linux 0xc538e10).
pub(crate) fn channel_to_rod(world: &mut World, layer: Entity, rod: [i32; 3]) {
    let at = [rod[0] as f32 + 0.5, rod[1] as f32, rod[2] as f32 + 0.5];
    channel_at(world, layer, [rod[0], rod[1] + 1, rod[2]], at);
}

/// Whether a bolt came down: while it thunders, with nothing above `sky`.
fn channel_at(world: &mut World, layer: Entity, sky: [i32; 3], at: [f32; 3]) -> bool {
    let open = world.get::<Layer>(layer).is_some_and(|level| level.world.surface(sky[0], sky[2]).is_none_or(|top| top < sky[1]));
    thunders(world, layer) && open && call_down(world, layer, at, true).is_some()
}

/// Storms strike, bolts flash and go.
pub(crate) fn tick(world: &mut World) {
    storms::tick(world);
    let bolts: Vec<(Entity, Entity)> = world.query_filtered::<(Entity, &InLayer), (With<Bolt>, With<NetId>)>().iter(world).map(|(bolt, layer)| (bolt, layer.0)).collect();
    for (bolt, layer) in bolts {
        flash(world, bolt, layer);
    }
}
