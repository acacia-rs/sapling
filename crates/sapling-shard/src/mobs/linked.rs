//! What a mob's announcement says of its seat or its riders, as BDS announced them
//! (research/captures/bds-h3herds.txt): a rider's `AddEntity` carries its seat and is followed by its
//! link, a mount's carries its riders' links, and a driver shown with its mount comes just before it.

use std::collections::HashSet;

use acacia_proto::RawPacket;
use acacia_proto::packets::AddEntity;
use acacia_proto::types::{Link, MetadataDictionaryItem};
use bevy_ecs::prelude::*;
use bytes::Bytes;

/// Set by riding (riding/announced.rs) on mobs that ride or are ridden.
#[derive(Component, Default)]
pub(crate) struct Linked {
    /// A mount's riders' links.
    pub links: Vec<Link>,
    /// A rider's seat items.
    pub seat: Vec<MetadataDictionaryItem>,
    /// A rider's mount, whether it drives it, and the link told after the rider's announcement.
    pub rides: Option<(Entity, bool, Bytes)>,
}

/// `announcement` with `linked`'s links and seat in its `AddEntity`, and a rider's link after it.
pub(crate) fn dressed(announcement: &[Bytes], linked: Option<&Linked>) -> Vec<Bytes> {
    let mut shown = announcement.to_vec();
    let Some(linked) = linked else { return shown };
    let decoded = shown.first().and_then(|first| RawPacket::parse(first.clone()).ok()?.decode::<AddEntity>().ok());
    if let Some(mut added) = decoded.filter(|_| !linked.links.is_empty() || !linked.seat.is_empty()) {
        added.links = linked.links.clone();
        for item in &linked.seat {
            match added.metadata.iter_mut().find(|old| old.key == item.key) {
                Some(old) => *old = item.clone(),
                None => added.metadata.push(item.clone()),
            }
        }
        shown[0] = sapling_net::encode(&added);
    }
    shown.extend(linked.rides.as_ref().map(|(_, _, link)| link.clone()));
    shown
}

/// The order `new` (by id) is announced in: a driver whose mount is among them moves to just before it.
pub(crate) fn order(new: &[Entity], rides: impl Fn(Entity) -> Option<(Entity, bool)>) -> Vec<Entity> {
    let shown: HashSet<Entity> = new.iter().copied().collect();
    let mut done = HashSet::new();
    let mut order = Vec::with_capacity(new.len());
    for &mob in new {
        if done.contains(&mob) || rides(mob).is_some_and(|(mount, drives)| drives && shown.contains(&mount) && !done.contains(&mount)) {
            continue;
        }
        let drivers: Vec<Entity> = new.iter().copied().filter(|driver| rides(*driver) == Some((mob, true)) && !done.contains(driver)).collect();
        for driver in drivers {
            done.insert(driver);
            order.push(driver);
        }
        done.insert(mob);
        order.push(mob);
    }
    order
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_driver_comes_just_before_its_mount_and_a_passenger_keeps_its_place() {
        let mut world = World::new();
        let [mount, driver, passenger, other] = [(); 4].map(|_| world.spawn_empty().id());
        let rides = |mob: Entity| match mob {
            m if m == driver => Some((mount, true)),
            m if m == passenger => Some((mount, false)),
            _ => None,
        };
        assert_eq!(order(&[other, mount, driver, passenger], rides), [other, driver, mount, passenger]);
        assert_eq!(order(&[driver], rides), [driver], "a mount already shown");
    }
}
