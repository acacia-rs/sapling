//! A mob under invisibility, as BDS showed it (research/captures/bds-bf1give.txt, bds-bf1potion.txt):
//! no `MobEffect`; at the tick-end sync its flags gain `INVISIBLE` and lose `CAN_SHOW_NAMETAG` (a
//! named mob's comes back), in one SetEntityData with its `VisibleMobEffects`; the tick after it is
//! heard `disappeared` (`reappeared` as it goes) at half its shown height, in the same sync order.
//! A raise while it lasts tells nothing. Viewers coming later are announced its flags (mobs/live.rs).
//! A hunter sees it nearer: [`sight`].

use acacia_proto::packets::{AddEntity, LevelSoundEvent};
use acacia_proto::types::{MetadataDictionaryItem, MetadataDictionaryItemValue as Value, MetadataFlags1 as Flags, Vec3f};
use acacia_proto::RawPacket;
use bevy_ecs::prelude::*;
use sapling_api::Effect;

use super::Active;
use crate::body::Body;
use crate::components::{NetId, Tick};
use crate::entities;

/// What a hunter's sight is multiplied by for invisible prey, before the share of armour it wears.
const UNSEEN: f32 = 0.7;
/// The least share of armour counted: a bare invisible player was taken 2.41 off a zombie that sees
/// 35, not 2.57 (bds-bf1near.txt); in iron, by the pieces worn (bds-bf1armour.txt).
const LEAST_WORN: f32 = 0.1;

/// An invisible mob, and whether its flags showed its name before.
#[derive(Component)]
pub(super) struct Unseen {
    nametag: bool,
}

/// Mobs whose invisibility came (true) or went in a tick, to be heard the next.
#[derive(Resource, Default)]
pub(super) struct Vanishing(Vec<(u64, Entity, bool)>);

pub(super) fn is_invisible(list: &[Active]) -> bool {
    list.iter().any(|active| active.effect == Effect::Invisibility)
}

/// What a hunter's sight comes to for prey under invisibility wearing `worn` of its four pieces.
pub(crate) fn sight(sight: f32, worn: usize) -> f32 {
    sight * UNSEEN * (worn as f32 / 4.0).max(LEAST_WORN)
}

fn announced_flags(world: &World, mob: Entity) -> Option<i64> {
    let first = crate::mobs::announced(world, mob)?.into_iter().next()?;
    let added = RawPacket::parse(first).ok()?.decode::<AddEntity>().ok()?;
    added.metadata.into_iter().find_map(|item| if let Value::Flags(flags) = item.value { Some(flags.0) } else { None })
}

fn flags_item(id: u64, word: i64) -> MetadataDictionaryItem {
    sapling_data::mob_flags_set(id, Flags(word)).metadata.remove(0)
}

/// The mob `id`'s invisibility came (`on`) or went: its flags word for viewers, and its sound for
/// the next tick.
pub(super) fn flip(world: &mut World, mob: Entity, id: u64, on: bool) -> Option<MetadataDictionaryItem> {
    let kind = world.get::<entities::Kind>(mob)?.0.clone();
    let stored = announced_flags(world, mob)?;
    let told = world.get::<crate::sync::Told>(mob).and_then(crate::sync::Told::mob_flags).map_or(stored, |flags| flags.0);
    let nametag = if on { told & Flags::CAN_SHOW_NAMETAG.0 != 0 } else { world.get::<Unseen>(mob).is_some_and(|unseen| unseen.nametag) };
    let word = if on {
        if let Some(mut told) = world.get_mut::<crate::sync::Told>(mob) {
            told.forget_announced(Flags::CAN_SHOW_NAMETAG);
        }
        entities::mob_flags(world, mob, &kind, Flags::CAN_SHOW_NAMETAG, false);
        world.entity_mut(mob).insert(Unseen { nametag });
        entities::mob_flags(world, mob, &kind, Flags::INVISIBLE, true)
    } else {
        world.entity_mut(mob).remove::<Unseen>();
        let shown = entities::mob_flags(world, mob, &kind, Flags::INVISIBLE, false);
        if nametag { entities::mob_flags(world, mob, &kind, Flags::CAN_SHOW_NAMETAG, true) } else { shown }
    };
    let synced = entities::sync_tick(world);
    world.get_resource_or_insert_with(Vanishing::default).0.push((synced, mob, on));
    Some(flags_item(id, word.0))
}

/// Plays the sounds of the mobs whose invisibility changed before this tick, held to the sync.
pub(super) fn heard(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let Some(mut vanishing) = world.get_resource_mut::<Vanishing>() else { return };
    let (due, waiting): (Vec<_>, Vec<_>) = std::mem::take(&mut vanishing.0).into_iter().partition(|(when, ..)| *when < now);
    vanishing.0 = waiting;
    for (_, mob, on) in due {
        let Some(sound) = sound(world, mob, if on { "disappeared" } else { "reappeared" }) else { continue };
        super::tell_at_sync(world, mob, vec![sound]);
    }
}

fn sound(world: &World, mob: Entity, name: &str) -> Option<bytes::Bytes> {
    let (kind, id, body) = (world.get::<entities::Kind>(mob)?.0.as_str(), world.get::<NetId>(mob)?.0, world.get::<Body>(mob)?);
    let baby = crate::creatures::shown::is_baby(world, mob);
    let [_, height] = super::told::shown_size(world, mob, kind, body);
    let [x, y, z] = body.at;
    let heard = LevelSoundEvent { sound_id: name.into(), position: Vec3f { x, y: y + height / 2.0, z }, extra_data: -1, entity_type: kind.into(), is_baby_mob: baby, is_global: false, entity_unique_id: id as i64, fire_at_position: None };
    Some(sapling_net::encode(&heard))
}
