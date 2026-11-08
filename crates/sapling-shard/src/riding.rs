//! Riders and what they ride: boats, minecarts and the mobs that take a rider (docs/research/
//! vehicles.md). A vehicle keeps its riders by seat ([`Seats`], the driver first) and each rider
//! the vehicle it is on ([`Riding`]). Seating and unseating tell the riders' seats and the link to
//! those who see the vehicle as BDS does (research/captures/bds-ride.txt); every tick the drivers'
//! inputs move what they drive (drive.rs) and the riders are carried to their seats (carry.rs).

mod announced;
mod backend;
mod body;
mod breaking;
mod carry;
mod drive;
mod evented;
mod exit;
mod grazing;
mod harness;
mod inventory;
mod mounts;
mod mouth;
mod packets;
mod packs;
mod rearing;
mod replaced;
mod seats;
mod stilled;
mod strength;
mod told;

use bevy_ecs::prelude::*;
use bytes::Bytes;

use crate::body::Body;
use crate::components::{Identity, InLayer, NetId};
use crate::entities::{Kind, Stored};
use crate::mobs;
use crate::sync::Told as Shown;
use packets::LinkKind;

pub(crate) use breaking::tick as break_in;
pub(crate) use carry::carry;
pub(crate) use inventory::{arrive as arrive_ungrouped, dress as dress_inventory, tamed_items as tamed_inventory, upgrade as upgrade_zombie_horses};
pub(crate) use evented::{is_bare as saddled_bare, looted_saddle, saddle_by_event};
pub(crate) use strength::{dress as dress_strength, give as give_strength, kept as kept_strength, of as strength_of, roll as roll_strength};
pub(crate) use drive::{Leaving, Reins, drive, is_driven};
pub(crate) use mounts::{Rein, boost, reaches, saddle_flags, steers, used_on};
pub(crate) use harness::shown_flags as ghast_flags;
use mounts::predicted;
pub(crate) use mouth::{fall as mouth_at_death, open_mouths};
pub(crate) use packets::sound as sound_packet;
pub(crate) use body::{put as put_body, worn as body_worn};
pub(crate) use packs::{load as load_pack, looted_armor, open as open_pack, refusal as pack_refusal, slots as pack_slots, spilled};
pub(crate) use seats::{layout, offset as seat_offset};
pub(crate) use replaced::{is_mount, set as set_mount_item};
use seats::PLAYER;

/// A vehicle's riders, the driver first.
#[derive(Component, Default, Clone)]
pub(crate) struct Seats(pub Vec<Entity>);

/// What a rider rides, and its id.
#[derive(Component, Clone, Copy)]
pub(crate) struct Riding {
    pub vehicle: Entity,
    pub id: u64,
}

/// What riding takes an entity or a player for: a player is `minecraft:player`.
pub(crate) fn kind_of(world: &World, entity: Entity) -> Option<String> {
    if world.get::<Identity>(entity).is_some() {
        return Some(PLAYER.to_owned());
    }
    world.get::<Kind>(entity).map(|kind| kind.0.clone())
}

/// Where `rider` sits on `seat` of a vehicle facing `yaw`, against the vehicle's wire: a baby at
/// the seat itself (bds-j3ride.txt: a baby zombie 0.48 over its chicken, a grown one 0.5 under its seat).
fn rider_offset(world: &World, seat: &seats::Seat, rider: Entity, yaw: f32) -> Option<[f32; 3]> {
    let mut offset = seats::offset(seat, &kind_of(world, rider)?, yaw);
    if crate::creatures::is_baby(world, rider) {
        offset[1] = seat.position[1];
    }
    Some(offset)
}

/// How far up from its box's bottom the wire puts a vehicle of `kind`.
pub(crate) fn wire_lift(kind: &str) -> f32 {
    match kind {
        _ if kind.ends_with("boat") => 0.375,
        _ if kind.ends_with("minecart") => 0.35,
        _ => 0.0,
    }
}

/// Where the wire puts the vehicle, and the way it faces.
fn vehicle_wire(world: &World, vehicle: Entity) -> Option<([f32; 3], f32)> {
    if let Some(position) = world.get::<crate::components::Position>(vehicle).filter(|_| world.get::<Identity>(vehicle).is_some()) {
        let [x, y, z] = position.feet;
        return Some(([x, y + sapling_data::EYE_HEIGHT, z], position.yaw));
    }
    let (body, kind) = (world.get::<Body>(vehicle)?, world.get::<Kind>(vehicle)?);
    let [x, y, z] = body.at;
    let yaw = world.get::<Shown>(vehicle).map_or(0.0, |shown| shown.facing()[0]);
    Some(([x, y + wire_lift(&kind.0), z], yaw))
}

/// Why a rider is not seated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Refusal {
    /// It takes no riders, or none of this family, or wants a saddle first.
    NotRideable,
    Full,
    /// The rider is the vehicle, or the vehicle rides the rider.
    Loop,
}

/// Whether `rider` may take a seat on `vehicle` now.
pub(crate) fn can_seat(world: &World, rider: Entity, vehicle: Entity) -> Result<(), Refusal> {
    let (Some(kind), Some(rider_kind)) = (kind_of(world, vehicle), kind_of(world, rider)) else { return Err(Refusal::NotRideable) };
    let layout = layout(&kind).filter(|layout| layout.takes(&rider_kind, crate::creatures::is_baby(world, rider))).ok_or(Refusal::NotRideable)?;
    if layout.saddled && !mounts::saddled(world, vehicle) || layout.tamed_riders && !world.get::<crate::creatures::Raised>(rider).is_some_and(|raised| raised.is_tamed()) {
        return Err(Refusal::NotRideable);
    }
    let mut above = Some(vehicle);
    while let Some(on) = above {
        if on == rider {
            return Err(Refusal::Loop);
        }
        above = world.get::<Riding>(on).map(|riding| riding.vehicle);
    }
    let taken = world.get::<Seats>(vehicle).map_or(0, |seats| seats.0.len());
    if world.get::<Riding>(rider).is_some_and(|riding| riding.vehicle == vehicle) || taken >= layout.capacity() {
        return Err(Refusal::Full);
    }
    Ok(())
}

/// How many more riders `vehicle` takes.
pub(crate) fn seats_left(world: &World, vehicle: Entity) -> usize {
    let Some(kind) = kind_of(world, vehicle) else { return 0 };
    let Some(layout) = layout(&kind) else { return 0 };
    // An unharnessed happy ghast has no `rideable`: `/ride` found it not rideable (bds-as3ghast.txt).
    if kind == harness::KIND && !harness::harnessed(world, vehicle) {
        return 0;
    }
    layout.capacity().saturating_sub(world.get::<Seats>(vehicle).map_or(0, |seats| seats.0.len()))
}

/// Seats `rider` on `vehicle`, off what it rode before; tells everyone who sees it.
pub(crate) fn seat(world: &mut World, rider: Entity, vehicle: Entity) -> Result<(), Refusal> {
    can_seat(world, rider, vehicle)?;
    if world.get::<Riding>(rider).is_some() {
        unseat(world, rider, false);
    }
    let (Some(_), Some(vehicle_id)) = (world.get::<NetId>(rider), world.get::<NetId>(vehicle).map(|id| id.0)) else { return Err(Refusal::NotRideable) };
    let mut seats = world.get::<Seats>(vehicle).cloned().unwrap_or_default();
    seats.0.push(rider);
    let index = seats.0.len() - 1;
    world.entity_mut(vehicle).insert(seats);
    world.entity_mut(rider).insert(Riding { vehicle, id: vehicle_id });
    if world.get::<Identity>(rider).is_some() {
        world.entity_mut(rider).insert(Reins::default());
    }
    let mut told = told::seat_packets(world, vehicle, told::Change::Newcomer(rider));
    let mount = vehicle_wire(world, vehicle).zip(kind_of(world, vehicle)).map(|((at, _), kind)| packets::sound("mount", vehicle_id, &kind, at));
    // Every rider's link again, as BDS told a boat's, a happy ghast's and a player's shoulders'
    // (research/captures/bds-ride.txt, bds-as3ghast.txt, bds-as3parrot.txt).
    let links = told::links(world, vehicle);
    let ghast = harness::riders_now(world, vehicle, index + 1, true);
    if world.get::<Identity>(vehicle).is_some() {
        told.extend(links);
        told.extend(mount);
    } else {
        told.extend(mount);
        told.extend(ghast.as_ref().map(|(sound, _)| sound.clone()));
        told.extend(links);
        told.extend(ghast.map(|(_, interval)| interval));
    }
    tell(world, vehicle, rider, told);
    retell(world, vehicle);
    carry::place(world, vehicle);
    Ok(())
}

/// Takes `rider` off what it rides; with `step_off` it is put where vanilla's riders step off to
/// (exit.rs). False for one that rides nothing.
pub(crate) fn unseat(world: &mut World, rider: Entity, step_off: bool) -> bool {
    unseat_told(world, rider, step_off, false, true)
}

/// [`unseat`], its link told `immediate` when the rider is going (BDS 1457bd1f0, vehicles.md);
/// with `relink` those left are linked again, as BDS did when one got off a happy ghast
/// (bds-as3ghast.txt), and not when all were let off (bds-ride.txt).
fn unseat_told(world: &mut World, rider: Entity, step_off: bool, immediate: bool, relink: bool) -> bool {
    let Some(Riding { vehicle, id: vehicle_id }) = world.get::<Riding>(rider).copied() else { return false };
    world.entity_mut(rider).remove::<(Riding, Reins)>();
    let rider_id = world.get::<NetId>(rider).map_or(0, |id| id.0);
    let mut index = 0;
    let left = world.get_mut::<Seats>(vehicle).map(|mut seats| {
        index = seats.0.iter().position(|seated| *seated == rider).unwrap_or(0);
        seats.0.retain(|seated| *seated != rider);
        seats.0.len()
    });
    if left == Some(0) {
        world.entity_mut(vehicle).remove::<Seats>();
    }
    let mut told = vec![packets::link(vehicle_id, rider_id, LinkKind::Off, immediate)];
    if relink {
        told.extend(told::links(world, vehicle));
    }
    told.extend(told::seat_packets(world, vehicle, told::Change::Left(index)));
    tell(world, vehicle, rider, told);
    if let Some(left) = left {
        harness::detach_next_tick(world, vehicle, left);
    }
    announced::unfile(world, rider);
    retell(world, vehicle);
    if step_off {
        exit::step_off(world, rider, vehicle);
    }
    true
}

/// Takes `rider` off where it is, those left not linked again, as when all are let off at once.
pub(crate) fn let_off(world: &mut World, rider: Entity) -> bool {
    unseat_told(world, rider, false, false, false)
}

/// Takes every rider off `vehicle`, each stepping off; how many there were.
pub(crate) fn evict(world: &mut World, vehicle: Entity) -> usize {
    let riders = world.get::<Seats>(vehicle).map(|seats| seats.0.clone()).unwrap_or_default();
    riders.iter().filter(|rider| unseat_told(world, **rider, true, false, false)).count()
}

/// Lets an entity or player that is going leave its seat and lets its riders off.
pub(crate) fn forget(world: &mut World, entity: Entity) {
    unseat_told(world, entity, false, true, true);
    evict(world, entity);
}

/// Sends `packets` to those who see `vehicle`.
/// A vehicle that appeared this tick is shown with its riders (`retell`); a rider that did waits
/// for its own announcement.
fn tell(world: &mut World, vehicle: Entity, rider: Entity, packets: Vec<Bytes>) {
    let at = world.get::<Body>(vehicle).map(|body| body.at).or_else(|| vehicle_wire(world, vehicle).map(|(at, _)| at));
    let (Some(layer), Some(at)) = (world.get::<InLayer>(vehicle).map(|layer| layer.0), at) else { return };
    if crate::entities::is_new(world, vehicle) {
        return;
    }
    let cell = at.map(|c| c.floor() as i32);
    crate::entities::tell_viewers(world, rider, layer, cell, packets);
}

/// What players who come to see `vehicle` later are told after it: its tack and its mob riders'
/// links (seats, and player riders' links, ride in the riders' announcements, announced.rs); and its
/// stored compound names its riders that a world folder keeps.
fn retell(world: &mut World, vehicle: Entity) {
    let riders = world.get::<Seats>(vehicle).map(|seats| seats.0.clone()).unwrap_or_default();
    let vehicle_id = world.get::<NetId>(vehicle).map_or(0, |id| id.0);
    let mut after = mounts::worn(world, vehicle);
    announced::file(world, vehicle);
    for (index, rider) in riders.iter().enumerate() {
        let Some(id) = world.get::<NetId>(*rider).filter(|_| world.get::<mobs::Mob>(*rider).is_some()) else { continue };
        // BDS sent no seating link `immediate` in any capture (118 of type 1, 6 of type 2).
        after.push(packets::link(vehicle_id, id.0, if index == 0 { LinkKind::Driver } else { LinkKind::Passenger }, false));
    }
    mobs::show_after(world, vehicle, after);
    let kept: Vec<i64> = riders.iter().filter_map(|rider| world.get::<crate::tags::UniqueId>(*rider).map(|id| id.0)).collect();
    if let Some(mut stored) = world.get_mut::<Stored>(vehicle)
        && let Ok(compound) = sapling_world::linked_actor(&stored.compound, &kept)
    {
        stored.compound = compound;
    }
}

/// The riders a vehicle a world folder kept names (`LinksTag`), until they are loaded too.
#[derive(Component)]
struct Awaits(Vec<i64>);

/// Seats the riders the just loaded `entity` was stored with, and puts it on a vehicle loaded
/// before it that awaits it; a mount wears the armour it kept.
// TODO: a player's `RideID` is neither kept nor read: a player loads standing.
pub(crate) fn adopt(world: &mut World, entity: Entity) {
    body::adopt(world, entity);
    packs::adopt(world, entity);
    let riders = world.get::<Stored>(entity).map(|stored| sapling_world::actor_links(&stored.compound)).unwrap_or_default();
    if !riders.is_empty() {
        world.entity_mut(entity).insert(Awaits(riders));
    }
    let waiting: Vec<Entity> = world.query_filtered::<Entity, With<Awaits>>().iter(world).collect();
    if waiting.is_empty() {
        return;
    }
    let loaded: std::collections::HashMap<i64, Entity> = world.query::<(Entity, &crate::tags::UniqueId)>().iter(world).map(|(entity, id)| (id.0, entity)).collect();
    for vehicle in waiting {
        let Some(Awaits(riders)) = world.entity_mut(vehicle).take::<Awaits>() else { continue };
        let (here, left): (Vec<i64>, Vec<i64>) = riders.into_iter().partition(|id| loaded.contains_key(id));
        for id in here {
            let _ = seat(world, loaded[&id], vehicle);
        }
        if !left.is_empty() {
            world.entity_mut(vehicle).insert(Awaits(left));
        }
    }
}

/// What `rider` drives: the vehicle it has the first seat of.
pub(crate) fn driven_by(world: &World, rider: Entity) -> Option<Entity> {
    let riding = world.get::<Riding>(rider)?;
    (world.get::<Seats>(riding.vehicle)?.0.first() == Some(&rider)).then_some(riding.vehicle)
}
