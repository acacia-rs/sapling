//! A shot's flight, a mob's (arrows.rs) or a player's (shots/): it flies by its own drag and
//! weight, looked at a quarter of a block at a time for the first player or mob in its way, which
//! it hits (hits.rs). Where it meets a block an arrow or a trident sticks, for a minute, or for
//! the world folder to keep if a player loosed it; anything else lands (landing.rs).

use acacia_proto::packets::{EntityEvent, EntityEventEventId, LevelSoundEvent};
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValueDefault as Plain, MetadataFlags1, Vec3f};
use bevy_ecs::prelude::*;
use sapling_data::AddEntityOptions;
use sapling_world::ItemStack;

mod in_way;

use super::arrows::{Lands, Shot};
use super::hits::{self, After};
use super::landing;
use crate::body::Body;
use crate::components::{InLayer, Layer, NetId, Tick};
use crate::dragon::cloud::item;
use crate::sync::Told;
use crate::{entities, mobs};
use in_way::first_met;

/// Ticks one lies stuck before it goes, unless a player may pick it up.
const LIES: u64 = 1200;
const SHAKES: i32 = 7;
const WATER: &[&str] = &["minecraft:water", "minecraft:flowing_water"];
/// The wire's own key 18 carries a tipped arrow's item data, its potion plus one (an `arrow` of data
/// 26 showed Byte 26; research/captures/bds-t2epack.txt).
const TIP: Key = Key::MinecartHasDisplay;

#[derive(Component)]
pub(crate) struct Arrow {
    /// Who shot it, named if it kills ("player:" and a name for a player), and the shooter itself,
    /// which it passes through.
    pub(super) archer: String,
    pub(super) by: Entity,
    /// The effect it gives the player or mob it hits, and for how many ticks (a stray's, a bogged's).
    pub(super) gives: Option<(&'static str, u32)>,
    pub(super) shot: Shot,
    pub(super) born: u64,
    pub(super) stuck: Option<u64>,
    /// By `/summon`: it stays put the tick it came.
    pub(super) summoned: bool,
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
    let arrow = entities::spawn_mob(world, layer, shot.kind, Body::new(from, motion, shot.size), from, announce)?;
    world.entity_mut(arrow).insert(Arrow { archer, by, gives, shot, born, stuck: None, summoned: false });
    if let Some(loosed) = loosed {
        world.entity_mut(arrow).insert(loosed);
    }
    Some(arrow)
}

/// A shot `/summon` made at `feet`, still and no one's: it starts to fall the tick after
/// (research/captures/bds-s7shots.txt).
pub(crate) fn summon(world: &mut World, layer: Entity, shot: Shot, feet: [f32; 3]) -> Option<Entity> {
    let arrow = launch(world, Launch { layer, by: Entity::PLACEHOLDER, archer: String::new(), from: feet, motion: [0.0; 3], shot, gives: None }, None)?;
    let mut state = world.get_mut::<Arrow>(arrow)?;
    (state.by, state.summoned) = (arrow, true);
    Some(arrow)
}

/// A tick of one flying shot: whether it is still there.
fn fly(world: &mut World, arrow: Entity, layer: Entity, id: u64, now: u64) -> Option<()> {
    let mut body = *world.get::<Body>(arrow)?;
    let start = body.at;
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
            After::Deflected => {
                body.at = at;
                motion = motion.map(|c| c * -0.1);
            }
        }
    }
    let before = body.at;
    body.motion = motion;
    body.shove(&world.get::<Layer>(layer)?.world, motion);
    let short: [bool; 3] = std::array::from_fn(|axis| (body.at[axis] - before[axis] - motion[axis]).abs() > 1e-4);
    let stopped = short.contains(&true);
    if stopped {
        let by_mob = world.get::<super::Creature>(by).is_some();
        crate::blocks::shot_stopped(world, layer, shot.kind, body.at, motion, shot.size, short, shot.sticks, shot.burns > 0, by_mob);
        let by_player = world.get::<crate::components::Identity>(by).is_some();
        crate::blocks::vibrate(world, layer, body.at, crate::blocks::GameEvent::ProjectileLand, by_player);
    }
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
    let gone: [f32; 3] = std::array::from_fn(|axis| body.at[axis] - start[axis]);
    let first = born + 1 == now;
    let flags = flags(world, arrow, id, gone, first);
    let level = &world.get::<Layer>(layer)?.world;
    let drag = if super::holds(level, body.at.map(|c| c.floor() as i32), WATER) { shot.wet_drag } else { shot.drag };
    // BDS keeps a shot's motion as its next move's, drag and weight taken after the move
    // (research/captures/bds-s7shots.txt, bds-shoot.txt): viewers are told that one.
    let next = if stopped { [0.0; 3] } else { [body.motion[0] * drag, body.motion[1] * drag - shot.weight, body.motion[2] * drag] };
    let summoned = world.get::<Arrow>(arrow).is_some_and(|arrow| arrow.summoned);
    let mut told = world.get_mut::<Told>(arrow)?;
    if summoned && first {
        // TODO: BDS turns a loosed shot as it flies too (bds-shoot.txt: pitch bytes 2, 1, 0, 255, 252
        // from its 4th move); that rule is not fitted.
        told.pitch(0.0);
        told.pitch(facing(body.motion)[1]);
    }
    let mut packets = told.update(id, body.at, next, stopped);
    packets.splice(0..0, flags);
    if stopped {
        super::stuck::keep(world, arrow, layer, &body);
        if shot.kind == super::arrows::ARROW.kind {
            packets.push(sapling_net::encode(&EntityEvent { runtime_entity_id: id, event_id: EntityEventEventId::ArrowShake, data: SHAKES, fire_at_position: None }));
        }
        packets.push(sound(shot.heard[1], body.at, ""));
    }
    entities::tell_viewers(world, arrow, layer, body.at.map(|c| c.floor() as i32), packets);
    Some(())
}

/// The shot's flag word where a tick that moved it by `gone` changed it: it is shown, and its
/// name may be, the tick after it came (`first`), and `MOVING` goes as for any entity
/// (research/captures/bds-s7shots.txt, bds-shoot.txt).
fn flags(world: &mut World, arrow: Entity, id: u64, gone: [f32; 3], first: bool) -> Option<bytes::Bytes> {
    let mut told = world.get_mut::<Told>(arrow)?;
    let moved = told.flags_word(gone, false, false);
    let seen = first.then(|| {
        told.forget_announced(MetadataFlags1::INVISIBLE);
        told.set_flag(MetadataFlags1::CAN_SHOW_NAMETAG, true)
    });
    seen.flatten().or(moved).map(|word| crate::sync::flags_packet(id, word))
}

/// Moves every flying shot a tick; one that lay long enough goes, but a trident a player may pick up.
pub(super) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let arrows: Vec<(Entity, Entity, u64, Option<u64>, bool, bool)> = world
        .query::<(Entity, &InLayer, &NetId, &Arrow)>()
        .iter(world)
        .map(|(arrow, layer, id, state)| (arrow, layer.0, id.0, state.stuck, state.shot.kind != super::arrows::ARROW.kind, state.summoned && state.born == now))
        .collect();
    for (arrow, layer, id, stuck, trident, still) in arrows {
        let (kept, returning) = world.get::<Loosed>(arrow).map_or((false, false), |loosed| (trident && loosed.pickup != Pickup::Never, loosed.returning));
        match stuck {
            _ if returning || still => {}
            Some(since) if now >= since + LIES && !kept => mobs::remove(world, arrow),
            Some(_) => {
                let at = world.get::<Body>(arrow).map_or([0; 3], |body| body.at.map(|c| c.floor() as i32));
                let flags = flags(world, arrow, id, [0.0; 3], false);
                entities::tell_viewers(world, arrow, layer, at, flags.into_iter().collect());
            }
            None => drop(fly(world, arrow, layer, id, now)),
        }
    }
}

/// What a shot's hit reads of it: its shooter's name and entity, its effect and its kind.
pub(super) fn shooter(world: &World, arrow: Entity) -> Option<(String, Entity, Option<(&'static str, u32)>, Shot)> {
    world.get::<Arrow>(arrow).map(|arrow| (arrow.archer.clone(), arrow.by, arrow.gives, arrow.shot))
}
