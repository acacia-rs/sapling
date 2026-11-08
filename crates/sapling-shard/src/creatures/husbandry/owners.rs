//! Taming and owners: a wild one fed its taming item is tamed by chance (`minecraft:tameable`); a
//! wild mount's temper rises with its foods and a ride tames it by chance (`minecraft:tamemount`).
//! A tamed one sits and stands at its owner's click, follows it (`behavior.follow_owner`) and
//! teleports to it when left far behind.
// TODO: the teleport distance and the spots it lands on are the Java game's; a ride's temper rise is
// the Java game's 5.

use acacia_proto::packets::EntityEventEventId;
use acacia_proto::types::MetadataFlags1 as Flags;
use bevy_ecs::prelude::*;

use super::{Raised, distance, keep_for_good, regroup, shown};
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{InLayer, Layer, Position};
use crate::creatures::Health;
use crate::sync::Told;
use crate::tags::Marks;

/// A tamed one left this far behind teleports to its owner.
const TELEPORTS: f32 = 12.0;
/// A failed ride makes a mount this much calmer.
const RIDE_TEMPER: i32 = 5;
/// Spawn events that tame a mount: they add its tamed group.
const TAMING_EVENTS: [&str; 2] = ["spawn_tame_adult", "on_tame"];
/// Its pack has neither event: BDS's summon with one came wild (bds-s7mounts.txt).
const TRADER_LLAMA: &str = "minecraft:trader_llama";
/// A cat tamed by feeding sits, told with its tamed flag (bds-ar2cat.txt); BDS's tamed parrot did not
/// (bds-parrots.txt). The wolf is undrilled.
const SITS_WHEN_TAMED: [&str; 1] = ["minecraft:cat"];

/// Whether `player` owns the mob.
pub(crate) fn is_owner(world: &World, entity: Entity, player: Entity) -> bool {
    let (Some(raised), Some(marks)) = (world.get::<Raised>(entity), world.get::<Marks>(player)) else { return false };
    raised.is_tamed() && raised.kept.owner == Some(marks.unique_id)
}

/// The mob of `kind` is tamed by the player whose `UniqueID` is `owner`: its tamed groups and,
/// where they give one, a tamed one's full health.
pub(super) fn tame_to(world: &mut World, entity: Entity, kind: &str, owner: i64) {
    tame(world, entity, kind, Some(owner));
}

/// A mob summoned with a `spawn_tame_adult` or `on_tame` event (horses, donkeys, mules, llamas)
/// comes tamed with no owner (research/captures/bds-tack.txt, bds-s7mounts.txt); false for any
/// other event.
pub(crate) fn tame_by_event(world: &mut World, entity: Entity, kind: &str, event: &str) -> bool {
    if !TAMING_EVENTS.contains(&event.trim_start_matches("minecraft:")) || kind == TRADER_LLAMA || sapling_data::husbandry(kind).is_none_or(|rules| rules.temper.is_none()) {
        return false;
    }
    tame(world, entity, kind, None);
    true
}

/// A zombie horse's pack tames it with no owner, as riding/inventory.rs says when; its viewers
/// are told the flag and the inventory alone.
pub(crate) fn tame_by_upgrade(world: &mut World, entity: Entity, kind: &str) {
    if let Some(mut raised) = world.get_mut::<Raised>(entity) {
        raised.kept.tamed = true;
    }
    shown::flag_with(world, entity, Flags::TAMED, true, crate::riding::tamed_inventory(kind));
}

fn tame(world: &mut World, entity: Entity, kind: &str, owner: Option<i64>) {
    settle_tamed(world, entity, kind, owner);
    shown::flag_with(world, entity, Flags::TAMED, true, crate::riding::tamed_inventory(kind));
    shown::links(world, entity);
}

/// The mob's tamed state, groups and health, nothing told.
fn settle_tamed(world: &mut World, entity: Entity, kind: &str, owner: Option<i64>) {
    if let Some(mut raised) = world.get_mut::<Raised>(entity) {
        (raised.kept.tamed, raised.kept.owner) = (true, owner);
    }
    keep_for_good(world, entity);
    let rules = sapling_data::husbandry(kind);
    if let (Some(max), Some(mut health)) = (rules.and_then(|rules| rules.tame_health), world.get_mut::<Health>(entity)) {
        *health = Health::new(max, None, health.burning());
    }
    if let Some(swap) = rules.map(|rules| &rules.on_tame) {
        regroup(world, entity, swap);
    }
}

/// A wild mob is fed `item` by `player`: if it is a taming item it is tamed by its kind's chance,
/// its viewers seeing hearts or smoke. False for an item that does not tame it.
pub(super) fn try_tame(world: &mut World, entity: Entity, kind: &str, item: &str, player: Entity) -> bool {
    let Some(rules) = sapling_data::husbandry(kind).filter(|rules| rules.tame.contains(&item)) else { return false };
    if world.get::<Raised>(entity).is_none_or(Raised::is_tamed) {
        return false;
    }
    let tamed = world.get_resource_or_insert_with(Dice::default).fraction() < rules.tame_chance;
    let owner = world.get::<Marks>(player).map(|marks| marks.unique_id);
    match (tamed, owner) {
        (true, Some(owner)) => {
            if SITS_WHEN_TAMED.contains(&kind) {
                sit_untold(world, entity);
            }
            tame_to(world, entity, kind, owner);
            shown::event(world, entity, EntityEventEventId::TameSuccess, 0);
        }
        _ => shown::event(world, entity, EntityEventEventId::TameFail, 0),
    }
    true
}

/// A wild mount is fed `item`: its temper rises by what the item gives, to its kind's most.
/// False for an item that is none of its foods, or a mount tamed already.
pub(super) fn calm(world: &mut World, entity: Entity, kind: &str, item: &str) -> bool {
    let Some(rules) = sapling_data::husbandry(kind) else { return false };
    let (Some((_, most)), Some(gives)) = (rules.temper, rules.temper_feed.iter().find(|(food, _)| *food == item).map(|(_, gives)| *gives)) else { return false };
    let Some(mut raised) = world.get_mut::<Raised>(entity).filter(|raised| !raised.is_tamed()) else { return false };
    raised.kept.temper = (raised.kept.temper + gives).min(most);
    true
}

/// A new mount's temper, rolled in its kind's range as BDS's `MountTamingComponent` starts it
/// (148f38bb0; two of three fresh horses were tamed by their first ride, bds-n3ride.txt); 0 for others.
pub(super) fn first_temper(world: &mut World, kind: &str) -> i32 {
    let Some((least, most)) = sapling_data::husbandry(kind).and_then(|rules| rules.temper) else { return 0 };
    least + world.get_resource_or_insert_with(Dice::default).roll((most - least).max(1) as u32) as i32
}

/// A rider tries to ride the wild mount `entity`: it is tamed if its kind's most temper, rolled,
/// is below its temper; else it grows calmer and its viewers see it refuse. True when it is tamed
/// now; for the riding code, which throws the rider off where it is not, and tells a tamed one's
/// inventory the tick after. Its viewers see the hearts, then the flag with its owner in one
/// packet (research/captures/bds-n3ride.txt).
pub(crate) fn tame_by_riding(world: &mut World, entity: Entity, rider: Entity) -> bool {
    let Some(kind) = world.get::<crate::entities::Kind>(entity).map(|kind| kind.0.clone()) else { return false };
    let Some((_, most)) = sapling_data::husbandry(&kind).and_then(|rules| rules.temper) else { return false };
    let Some(temper) = world.get::<Raised>(entity).filter(|raised| !raised.is_tamed()).map(|raised| raised.kept.temper) else { return false };
    let rolled = world.get_resource_or_insert_with(Dice::default).roll(most.max(1) as u32) as i32;
    let owner = world.get::<Marks>(rider).map(|marks| marks.unique_id);
    if let Some(owner) = owner.filter(|_| rolled < temper) {
        shown::event(world, entity, EntityEventEventId::TameSuccess, 0);
        settle_tamed(world, entity, &kind, Some(owner));
        let owned = shown::owner_item(world, entity);
        shown::flag_with(world, entity, Flags::TAMED, true, owned);
        return true;
    }
    if let Some(mut raised) = world.get_mut::<Raised>(entity) {
        raised.kept.temper = (raised.kept.temper + RIDE_TEMPER).min(most);
    }
    shown::event(world, entity, EntityEventEventId::TameFail, 0);
    false
}

/// The mob sits, its viewers told with its next flags.
fn sit_untold(world: &mut World, entity: Entity) {
    if let Some(mut raised) = world.get_mut::<Raised>(entity) {
        raised.kept.sitting = true;
    }
    shown::flags_with(world, entity, Flags::SITTING, true);
}

/// Its owner's click sits a tamed mob of a kind that sits down, or stands it up. False for anyone
/// else's click or a kind that does not sit.
pub(super) fn toggle_sitting(world: &mut World, entity: Entity, kind: &str, player: Entity) -> bool {
    if !sapling_data::husbandry(kind).is_some_and(|rules| rules.sits) || !is_owner(world, entity, player) {
        return false;
    }
    let Some(mut raised) = world.get_mut::<Raised>(entity) else { return false };
    raised.kept.sitting = !raised.kept.sitting;
    let sitting = raised.kept.sitting;
    shown::flag(world, entity, Flags::SITTING, sitting);
    true
}

/// A sitting mob stays where it is.
pub(super) fn stays(world: &mut World, entity: Entity) -> bool {
    let sitting = world.get::<Raised>(entity).is_some_and(|raised| raised.kept.sitting);
    if sitting {
        crate::creatures::legs::rest(world, entity);
    }
    sitting
}

/// Its owner's feet, if it is in the mob's layer.
fn owner_feet(world: &World, entity: Entity, layer: Entity) -> Option<[f32; 3]> {
    let owner = shown::player_of(world, world.get::<Raised>(entity).filter(|raised| raised.is_tamed())?.kept.owner?)?;
    (world.get::<InLayer>(owner)?.0 == layer).then(|| world.get::<Position>(owner).map(|position| position.feet))?
}

/// Before the move of a tamed mob that follows its owner: it walks to it from its kind's start
/// distance until its stop distance, and teleports beside it from [`TELEPORTS`].
pub(super) fn follow(world: &mut World, entity: Entity, layer: Entity, kind: &str) -> bool {
    let Some(rules) = sapling_data::husbandry(kind) else { return false };
    let (Some((start, stop)), Some(feet), Some(body)) = (rules.follows, owner_feet(world, entity, layer), world.get::<Body>(entity).copied()) else { return false };
    let far = distance(body.at, feet);
    if far >= TELEPORTS {
        if let Some(to) = beside(world, layer, &body, feet) {
            teleport(world, entity, to);
        }
        return true;
    }
    let Some(mut raised) = world.get_mut::<Raised>(entity) else { return false };
    raised.following = if raised.following { far > stop } else { far > start };
    if !raised.following {
        return false;
    }
    crate::creatures::legs::walk(world, entity, layer, &body, [feet[0], feet[2]], rules.walks, stop);
    true
}

/// A free spot on a floor two or three blocks from `feet`, where `body` fits.
fn beside(world: &World, layer: Entity, body: &Body, feet: [f32; 3]) -> Option<[f32; 3]> {
    let level = &world.get::<Layer>(layer)?.world;
    let [x, y, z] = feet.map(|c| c.floor() as i32);
    let floor = |x: i32, y: i32, z: i32| level.in_height(y) && level.registry().get(level.block(x, y, z)).is_some_and(|state| !state.boxes.is_empty());
    let ring = (-3..=3).flat_map(|dx| (-3..=3).map(move |dz| (dx, dz))).filter(|&(dx, dz): &(i32, i32)| dx.abs().max(dz.abs()) >= 2);
    ring.flat_map(|(dx, dz)| [0, 1, -1].map(|dy| [x + dx, y + dy, z + dz])).find_map(|[x, y, z]| {
        let at = [x as f32 + 0.5, y as f32, z as f32 + 0.5];
        let moved = Body::new(at, [0.0; 3], body.size);
        (level.is_loaded(sapling_world::ChunkPos::of_block(x, z)) && floor(x, y - 1, z) && moved.is_free(level)).then_some(at)
    })
}

fn teleport(world: &mut World, entity: Entity, to: [f32; 3]) {
    if let Some(mut body) = world.get_mut::<Body>(entity) {
        (body.at, body.motion) = (to, [0.0; 3]);
    }
    if let Some(mut told) = world.get_mut::<Told>(entity) {
        told.jump();
    }
}
