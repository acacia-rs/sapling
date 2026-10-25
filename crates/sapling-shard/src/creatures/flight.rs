//! A shot's flight, a mob's (arrows.rs) or a player's (shots/): it flies by its own drag and
//! weight, looked at a quarter of a block at a time for the first player or mob in its way, which
//! it hits (hits.rs). Where it meets a block an arrow or a trident sticks, for a minute, or for
//! the world folder to keep if a player loosed it; anything else lands (landing.rs).

use acacia_proto::packets::{EntityEvent, EntityEventEventId, LevelSoundEvent};
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValueDefault as Plain, MetadataFlags1, Vec3f};
use bevy_ecs::prelude::*;
use sapling_data::AddEntityOptions;
use sapling_world::{ItemStack, ShotRecord};

use super::arrows::{Lands, Shot};
use super::hits::{self, After};
use super::{Creature, Dying, Health, landing};
use crate::body::Body;
use crate::components::{InLayer, Layer, NetId, Position, Tick, in_world};
use crate::dragon::cloud::item;
use crate::sync::Told;
use crate::{entities, mobs, vitals};

const PLAYER: [f32; 2] = [0.6, 1.8];
/// Ticks one lies stuck before it goes, unless a player may pick it up.
const LIES: u64 = 1200;
const SHAKES: i32 = 7;
const WATER: &[&str] = &["minecraft:water", "minecraft:flowing_water"];
/// The wire's own key 18 carries a tipped arrow's potion, one more than its data value.
// TODO: the key as remembered of the Bedrock protocol; no BDS capture of a tipped arrow was read.
const TIP: Key = Key::MinecartHasDisplay;

#[derive(Component)]
pub(crate) struct Arrow {
    /// Who shot it, named if it kills ("player:" and a name for a player), and the shooter itself,
    /// which it passes through.
    archer: String,
    by: Entity,
    /// The effect it gives the player or mob it hits, and for how many ticks (a stray's, a bogged's).
    gives: Option<(&'static str, u32)>,
    shot: Shot,
    born: u64,
    stuck: Option<u64>,
}

/// Who may pick a stuck shot up.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Pickup {
    Never,
    /// A survival player, which is given it, or a creative one.
    Anyone,
    /// A creative player alone, which is given nothing (from creative, or Infinity).
    Creative,
}

/// What a player's shot carries beyond a mob's (shots/).
#[derive(Component, Clone, Debug)]
pub(crate) struct Loosed {
    pub player: Option<Entity>,
    /// Power's level, a full draw, Punch's level.
    pub power: u8,
    pub critical: bool,
    pub punch: u8,
    pub pickup: Pickup,
    /// What it is picked up as: the arrow, tipped or not, or the trident.
    pub item: Option<ItemStack>,
    pub tipped: Option<i16>,
    /// Bodies it may pass through yet (Piercing), and those it passed.
    pub pierces: u8,
    pub pierced: Vec<Entity>,
    pub impaling: u8,
    pub loyalty: u8,
    /// A trident that hit something hurts nothing more; one coming back flies by tridents.rs.
    pub spent: bool,
    pub returning: bool,
}

impl Loosed {
    pub(crate) fn new(player: Entity, pickup: Pickup, item: Option<ItemStack>) -> Self {
        Self { player: Some(player), power: 0, critical: false, punch: 0, pickup, item, tipped: None, pierces: 0, pierced: Vec::new(), impaling: 0, loyalty: 0, spent: false, returning: false }
    }
}

/// A shot leaving `from` at `motion`.
pub(crate) struct Launch {
    pub layer: Entity,
    pub by: Entity,
    pub archer: String,
    pub from: [f32; 3],
    pub motion: [f32; 3],
    pub shot: Shot,
    pub gives: Option<(&'static str, u32)>,
}

pub(crate) fn sound(name: &str, [x, y, z]: [f32; 3], of: &str) -> bytes::Bytes {
    sapling_net::encode(&LevelSoundEvent { sound_id: name.into(), position: Vec3f { x, y, z }, extra_data: -1, entity_type: of.into(), is_baby_mob: false, is_global: false, entity_unique_id: -1, fire_at_position: None })
}

/// The yaw and pitch of something moving at `motion`, as BDS turns a projectile.
pub(crate) fn facing(motion: [f32; 3]) -> [f32; 2] {
    [(-motion[0]).atan2(motion[2]).to_degrees(), motion[1].atan2(motion[0].hypot(motion[2])).to_degrees()]
}

/// Puts the shot into the world, with what a player's carries; the entity.
pub(crate) fn launch(world: &mut World, launch: Launch, loosed: Option<Loosed>) -> Option<Entity> {
    let Launch { layer, by, archer, from, motion, shot, gives } = launch;
    let [yaw, pitch] = facing(motion);
    let owner = loosed.as_ref().and_then(|_| world.get::<NetId>(by)).map(|id| id.0 as i64);
    let (critical, tip) = loosed.as_ref().map_or((false, None), |loosed| (loosed.critical, loosed.tipped));
    let announce = |id: u64| {
        let flags = MetadataFlags1(if critical { MetadataFlags1::CRITICAL.0 } else { 0 });
        let options = AddEntityOptions { id, identifier: shot.kind, feet: from, yaw, pitch, variant: 0, mark_variant: 0, color: 0, baby: false, flags, health: None, name: "" };
        let mut shown = sapling_data::add_entity(&options);
        shown.velocity = Vec3f { x: motion[0], y: motion[1], z: motion[2] };
        let mut items = Vec::new();
        if let Lands::Splash(data) | Lands::Lingers(data) = shot.lands {
            items.push(item(Key::PotionAuxValue, Plain::Short(data)));
        }
        items.extend(owner.map(|owner| item(Key::OwnerEid, Plain::Long(owner))));
        items.extend(tip.map(|data| item(TIP, Plain::Byte((data + 1) as i8))));
        for added in items {
            // The recorded potion already carries one, of 0.
            shown.metadata.retain(|item| item.key != added.key);
            shown.metadata.push(added);
        }
        Some(vec![sapling_net::encode(&shown)])
    };
    let born = world.resource::<Tick>().0;
    let arrow = entities::spawn(world, layer, shot.kind, Body::new(from, motion, shot.size), from, announce)?;
    world.entity_mut(arrow).insert(Arrow { archer, by, gives, shot, born, stuck: None });
    if let Some(loosed) = loosed {
        world.entity_mut(arrow).insert(loosed);
    }
    Some(arrow)
}

/// The player in `layer` whose box the shot's, of `size` with its bottom middle at `at`, reaches
/// into: BDS's arrow flew over a player 8 blocks off, a hand above its head. Not `by`, which shot it.
fn met(world: &World, layer: Entity, by: Entity, at: [f32; 3], size: [f32; 2], passed: &[Entity]) -> Option<Entity> {
    in_world(world).into_iter().filter(|player| *player != by && !passed.contains(player)).find(|player| {
        let Some(feet) = world.get::<Position>(*player).map(|position| position.feet) else { return false };
        let across = (PLAYER[0] + size[0]) / 2.0;
        world.get::<InLayer>(*player).map(|layer| layer.0) == Some(layer) && vitals::is_mortal(world, *player) && (at[0] - feet[0]).abs() < across && (at[2] - feet[2]).abs() < across && at[1] + size[1] > feet[1] && at[1] < feet[1] + PLAYER[1]
    })
}

/// The living mob of `layer` whose box the shot's reaches into.
fn met_mob(world: &mut World, layer: Entity, by: Entity, at: [f32; 3], size: [f32; 2], passed: &[Entity]) -> Option<Entity> {
    let mut mobs = world.query_filtered::<(Entity, &InLayer, &Body), (With<Creature>, With<Health>, Without<Dying>)>();
    mobs.iter(world).filter(|(mob, ..)| *mob != by && !passed.contains(mob)).find(|(_, in_layer, body)| {
        let across = (body.size[0] + size[0]) / 2.0;
        in_layer.0 == layer && (at[0] - body.at[0]).abs() < across && (at[2] - body.at[2]).abs() < across && at[1] + size[1] > body.at[1] && at[1] < body.at[1] + body.size[1]
    }).map(|(mob, ..)| mob)
}

/// The first player or mob on the shot's way from `at` by `motion`, where it met it, and whether a player.
fn first_met(world: &mut World, layer: Entity, by: Entity, at: [f32; 3], motion: [f32; 3], size: [f32; 2], passed: &[Entity]) -> Option<([f32; 3], Entity, bool)> {
    let speed = motion.iter().map(|c| c * c).sum::<f32>().sqrt();
    let looks = (speed / 0.25).ceil().max(1.0) as usize;
    let on_way = |look: usize| -> [f32; 3] { std::array::from_fn(|axis| at[axis] + motion[axis] * look as f32 / looks as f32) };
    (1..=looks).find_map(|look| {
        let point = on_way(look);
        met(world, layer, by, point, size, passed).map(|player| (point, player, true)).or_else(|| met_mob(world, layer, by, point, size, passed).map(|mob| (point, mob, false)))
    })
}

/// A tick of one flying shot: whether it is still there.
fn fly(world: &mut World, arrow: Entity, layer: Entity, id: u64, now: u64) -> Option<()> {
    let mut body = *world.get::<Body>(arrow)?;
    let (shot, born, by) = world.get::<Arrow>(arrow).map(|arrow| (arrow.shot, arrow.born, arrow.by))?;
    if shot.lasts.is_some_and(|lasts| now >= born + lasts) {
        mobs::remove(world, arrow);
        return None;
    }
    let level = &world.get::<Layer>(layer)?.world;
    let wet = super::holds(level, body.at.map(|c| c.floor() as i32), WATER);
    let drag = if wet { shot.wet_drag } else { shot.drag };
    let mut motion = [body.motion[0] * drag, body.motion[1] * drag - shot.weight, body.motion[2] * drag];
    let behind: [f32; 3] = std::array::from_fn(|axis| body.at[axis] - motion[axis]);
    let (passed, spent) = world.get::<Loosed>(arrow).map_or((Vec::new(), false), |loosed| (loosed.pierced.clone(), loosed.spent));
    let met = if spent { None } else { first_met(world, layer, by, body.at, motion, shot.size, &passed) };
    if let Some((at, struck, is_player)) = met {
        if shot.lands != Lands::Hurts {
            landing::land(world, arrow, layer, at, behind, Some((struck, is_player)));
            mobs::remove(world, arrow);
            return None;
        }
        match hits::hit(world, arrow, layer, struck, is_player, at, motion) {
            After::Gone => {
                mobs::remove(world, arrow);
                return None;
            }
            After::On => {}
            After::Back => {
                body.at = at;
                motion = [motion[0] * -0.01, motion[1] * -0.1, motion[2] * -0.01];
            }
        }
    }
    let before = body.at;
    body.motion = motion;
    body.shove(&world.get::<Layer>(layer)?.world, motion);
    let stopped = (0..3).any(|axis| (body.at[axis] - before[axis] - motion[axis]).abs() > 1e-4);
    if stopped && !shot.sticks {
        landing::land(world, arrow, layer, body.at, behind, None);
        mobs::remove(world, arrow);
        return None;
    }
    if stopped {
        body.motion = [0.0; 3];
        world.get_mut::<Arrow>(arrow)?.stuck = Some(now);
    }
    *world.get_mut::<Body>(arrow)? = body;
    let mut packets = world.get_mut::<Told>(arrow)?.update(id, body.at, body.motion, stopped);
    if stopped {
        keep(world, arrow, layer, &body);
        if shot.kind == super::arrows::ARROW.kind {
            packets.push(sapling_net::encode(&EntityEvent { runtime_entity_id: id, event_id: EntityEventEventId::ArrowShake, data: SHAKES, fire_at_position: None }));
        }
        packets.push(sound(shot.heard[1], body.at, ""));
    }
    entities::tell_viewers(world, arrow, layer, body.at.map(|c| c.floor() as i32), packets);
    Some(())
}

/// Has the world folder keep a stuck shot a player may pick up, with its column.
fn keep(world: &mut World, arrow: Entity, layer: Entity, body: &Body) -> Option<()> {
    let loosed = world.get::<Loosed>(arrow).filter(|loosed| loosed.pickup != Pickup::Never)?.clone();
    let kind = world.get::<Arrow>(arrow)?.shot.kind;
    let unique_id = entities::new_unique_id(world, arrow)?;
    let rotation = world.get::<Told>(arrow)?.facing();
    let trident = loosed.item.filter(|item| item.name != super::arrows::ARROW.kind);
    let record = ShotRecord { player: loosed.pickup == Pickup::Anyone, creative: loosed.pickup == Pickup::Creative, aux: loosed.tipped.map_or(0, |data| (data + 1) as i8), trident };
    let compound = sapling_world::new_shot_actor(unique_id, kind, body.at, rotation, &record, world.get::<Layer>(layer)?.world.registry());
    entities::keep(world, arrow, layer, unique_id, compound);
    Some(())
}

/// A shot a world folder kept lies stuck in `layer` again; false for one no player may pick up.
pub(crate) fn from_storage(world: &mut World, layer: Entity, kind: &str, at: [f32; 3], [yaw, pitch]: [f32; 2], unique_id: i64, compound: Vec<u8>, shot: Shot) -> bool {
    let Some(record) = sapling_world::shot_record(&compound) else { return false };
    let pickup = match (record.player, record.creative) {
        (true, _) => Pickup::Anyone,
        (false, true) => Pickup::Creative,
        _ => return false,
    };
    let tipped = (record.aux > 0).then(|| i16::from(record.aux) - 1);
    let item = record.trident.or_else(|| Some(ItemStack { name: kind.to_owned(), count: 1, damage: tipped.map_or(0, |data| data + 1), block: None, tag: None }));
    let Some(arrow) = entities::spawn(world, layer, kind, Body::new(at, [0.0; 3], shot.size), at, |id| {
        let options = AddEntityOptions { id, identifier: kind, feet: at, yaw, pitch, variant: 0, mark_variant: 0, color: 0, baby: false, flags: MetadataFlags1(0), health: None, name: "" };
        Some(vec![sapling_net::encode(&sapling_data::add_entity(&options))])
    }) else { return false };
    let now = world.resource::<Tick>().0;
    world.entity_mut(arrow).insert((Arrow { archer: String::new(), by: arrow, gives: None, shot, born: now, stuck: Some(now) }, Loosed { player: None, pickup, item, tipped, ..Loosed::new(arrow, pickup, None) }));
    entities::keep(world, arrow, layer, unique_id, compound);
    true
}

/// The stuck shots a player may pick up: each, its world, where it is, its id and what it carries.
pub(crate) fn lying(world: &mut World) -> Vec<(Entity, Entity, [f32; 3], u64, Loosed)> {
    let mut shots = world.query::<(Entity, &InLayer, &Body, &NetId, &Loosed, &Arrow)>();
    let lies = |loosed: &Loosed, arrow: &Arrow| loosed.pickup != Pickup::Never && !loosed.returning && arrow.stuck.is_some();
    shots.iter(world).filter(|(.., loosed, arrow)| lies(loosed, arrow)).map(|(arrow, layer, body, id, loosed, _)| (arrow, layer.0, body.at, id.0, loosed.clone())).collect()
}

/// The shot frees itself from where it was stuck: it flies again (a trident coming back).
pub(crate) fn unstick(world: &mut World, arrow: Entity) {
    if let Some(mut arrow) = world.get_mut::<Arrow>(arrow) {
        arrow.stuck = None;
    }
}

/// Moves every flying shot a tick; one that lay long enough goes, but a trident a player may pick up.
pub(super) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let arrows: Vec<(Entity, Entity, u64, Option<u64>, bool)> =
        world.query::<(Entity, &InLayer, &NetId, &Arrow)>().iter(world).map(|(arrow, layer, id, state)| (arrow, layer.0, id.0, state.stuck, state.shot.kind != super::arrows::ARROW.kind)).collect();
    for (arrow, layer, id, stuck, trident) in arrows {
        let (kept, returning) = world.get::<Loosed>(arrow).map_or((false, false), |loosed| (trident && loosed.pickup != Pickup::Never, loosed.returning));
        match stuck {
            _ if returning => {}
            Some(since) if now >= since + LIES && !kept => mobs::remove(world, arrow),
            Some(_) => {}
            None => drop(fly(world, arrow, layer, id, now)),
        }
    }
}

/// What a shot's hit reads of it: its shooter's name and entity, its effect and its kind.
pub(super) fn shooter(world: &World, arrow: Entity) -> Option<(String, Entity, Option<(&'static str, u32)>, Shot)> {
    world.get::<Arrow>(arrow).map(|arrow| (arrow.archer.clone(), arrow.by, arrow.gives, arrow.shot))
}
