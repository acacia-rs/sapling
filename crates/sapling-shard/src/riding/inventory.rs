//! What a mount shows of the component groups its pack gives it: a tamed horse's or zombie horse's
//! `minecraft:inventory` (`ContainerType` 12, the horse container, of 2), a tamed donkey's, mule's
//! or llama's (of 16), a grown camel's (of 5, the component's default size). A zombie or skeleton
//! horse holds its `collision_box` in its age groups alone, so one summoned by an event that adds
//! neither is shown with the box every actor is made with, and such a zombie horse is tamed the
//! tick after by its pack's `upgrade_to_1_21_130` (its environment sensor, seeing no `rideable`).
//! A camel's inventory is in `camel_adult`, which `camel_saddled` alone does not add
//! (research/captures/bds-firstdata.txt, bds-saddleevents.txt, bds-s7mounts.txt).

use acacia_proto::packets::AddEntity;
use acacia_proto::types::{MetadataDictionaryItem as Item, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain};
use bevy_ecs::prelude::*;

use crate::body::Body;
use crate::components::Tick;
use crate::creatures::Raised;
use crate::entities::Kind;

const HORSE_CONTAINER: i8 = 12;
const TAMED_SIZES: [(&str, i32); 6] = [("minecraft:horse", 2), ("minecraft:zombie_horse", 2), ("minecraft:donkey", 16), ("minecraft:mule", 16), ("minecraft:llama", 16), ("minecraft:trader_llama", 16)];
const CAMELS: [&str; 2] = ["minecraft:camel", "minecraft:camel_husk"];
const CAMEL_SIZE: i32 = 5;
const ZOMBIE_HORSE: &str = "minecraft:zombie_horse";
/// Kinds whose box or inventory is only in an age group, and the spawn events adding one.
const AGED: [(&str, &[&str]); 4] = [
    (ZOMBIE_HORSE, &["spawn_adult", "spawn_tame_adult", "entity_born", "spawn_adult_with_rider"]),
    ("minecraft:skeleton_horse", &["entity_born", "spring_trap"]),
    ("minecraft:camel", &["spawn_adult", "entity_born"]),
    ("minecraft:camel_husk", &["spawn_adult", "entity_born"]),
];
/// Of those, the kinds whose box is in them.
const BOXED_BY_AGE: [&str; 2] = [ZOMBIE_HORSE, "minecraft:skeleton_horse"];
/// The box of an actor whose pack gave it none.
const MADE: [f32; 2] = [0.6, 1.8];

/// A zombie horse its spawn event left without an age group, to be tamed after the tick it came in.
#[derive(Component)]
pub(super) struct Upgrading(u64);

/// The container type and size a mob of `kind`, `tamed` and `baby` or not, shows; `None` for a
/// kind that is no mount.
fn container(kind: &str, tamed: bool, baby: bool, ungrouped: bool) -> Option<(i8, i32)> {
    if CAMELS.contains(&kind) {
        return Some(if baby || ungrouped { (0, 0) } else { (HORSE_CONTAINER, CAMEL_SIZE) });
    }
    let size = TAMED_SIZES.iter().find(|(of, _)| *of == kind)?.1;
    Some(if tamed { (HORSE_CONTAINER, size) } else { (0, 0) })
}

fn byte(key: Key, value: i8) -> Item {
    Item { key, r#type: Type::Byte, legacy_type: 0, value: Value::Default(Plain::Byte(value)) }
}

fn int(key: Key, value: i32) -> Item {
    Item { key, r#type: Type::Int, legacy_type: 2, value: Value::Default(Plain::Int(value)) }
}

/// The items that show a mount of `kind` tamed now; none for a kind that is no mount.
pub(crate) fn tamed_items(kind: &str) -> Vec<Item> {
    container(kind, true, false, false).map_or_else(Vec::new, |(of, size)| vec![byte(Key::ContainerType, of), int(Key::ContainerBaseSize, size)])
}

/// Shows a mount's inventory and box in its announcement, over the recorded kind's.
pub(crate) fn dress(added: &mut AddEntity, kind: &str, tamed: bool, baby: bool, ungrouped: bool) {
    let container = container(kind, tamed, baby, ungrouped);
    let unboxed = ungrouped && BOXED_BY_AGE.contains(&kind);
    for item in &mut added.metadata {
        match (item.key, container) {
            (Key::ContainerType, Some((of, _))) => *item = byte(Key::ContainerType, of),
            (Key::ContainerBaseSize, Some((_, size))) => *item = int(Key::ContainerBaseSize, size),
            (Key::BoundingboxWidth, _) if unboxed => item.value = Value::Default(Plain::Float(MADE[0])),
            (Key::BoundingboxHeight, _) if unboxed => item.value = Value::Default(Plain::Float(MADE[1])),
            _ => {}
        }
    }
}

/// A mob of `kind` summoned with `event`: one of a kind in [`AGED`] that the event added no age
/// group to is ungrouped, with the made box where its box is in them.
pub(crate) fn arrive(world: &mut World, mob: Entity, kind: &str, event: &str) {
    let event = event.trim_start_matches("minecraft:");
    if AGED.iter().find(|(of, _)| *of == kind).is_none_or(|(_, events)| events.contains(&event)) {
        return;
    }
    if let Some(mut raised) = world.get_mut::<Raised>(mob) {
        raised.ungrouped = true;
    }
    if BOXED_BY_AGE.contains(&kind)
        && let Some(mut body) = world.get_mut::<Body>(mob)
    {
        body.size = MADE;
    }
    if kind == ZOMBIE_HORSE {
        let now = world.resource::<Tick>().0;
        world.entity_mut(mob).insert(Upgrading(now));
    }
}

/// Whether `mob` is a camel without its age group, whose `rideable` is in `camel_adult` too: BDS
/// seated no one on a `camel_saddled` one (research/captures/bds-ai1ride.txt).
pub(super) fn unrideable(world: &World, mob: Entity, kind: &str) -> bool {
    CAMELS.contains(&kind) && world.get::<Raised>(mob).is_some_and(|raised| raised.ungrouped)
}

/// Tames each grown zombie horse [`Upgrading`] since an earlier tick.
pub(crate) fn upgrade(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let due: Vec<Entity> = world.query::<(Entity, &Upgrading)>().iter(world).filter(|(_, since)| since.0 < now).map(|(mob, _)| mob).collect();
    for mob in due {
        world.entity_mut(mob).remove::<Upgrading>();
        let grown_wild = world.get::<Raised>(mob).is_some_and(|raised| !raised.is_baby() && !raised.is_tamed());
        if let Some(kind) = world.get::<Kind>(mob).map(|kind| kind.0.clone()).filter(|_| grown_wild) {
            crate::creatures::tame_by_upgrade(world, mob, &kind);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mounts_show_the_inventory_their_groups_give() {
        assert_eq!([container("minecraft:horse", true, false, false), container("minecraft:horse", false, false, false)], [Some((12, 2)), Some((0, 0))]);
        assert_eq!([container("minecraft:llama", true, false, false), container("minecraft:donkey", true, false, false)], [Some((12, 16)), Some((12, 16))]);
        assert_eq!([container("minecraft:camel", false, false, false), container("minecraft:camel", false, true, false), container("minecraft:camel", false, false, true)], [Some((12, 5)), Some((0, 0)), Some((0, 0))]);
        assert_eq!([container("minecraft:skeleton_horse", true, false, false), container("minecraft:cow", true, false, false)], [None, None]);
    }
}
