//! Riders and what they ride: boats, minecarts and the mobs that take a rider (docs/research/
//! vehicles.md). A vehicle keeps its riders by seat ([`Seats`], the driver first) and each rider
//! the vehicle it is on ([`Riding`]). Seating and unseating tell the riders' seats and the link to
//! those who see the vehicle as BDS does (research/captures/bds-ride.txt); every tick the drivers'
//! inputs move what they drive (drive.rs) and the riders are carried to their seats (carry.rs).

mod backend;
mod carry;
mod drive;
mod exit;
mod mounts;
mod packets;
mod packs;
mod rearing;
mod seats;
mod strength;

use bevy_ecs::prelude::*;
use bytes::Bytes;

use crate::body::Body;
use crate::components::{Identity, InLayer, NetId};
use crate::entities::{Kind, Stored};
use crate::mobs;
use crate::sync::Told as Shown;
use packets::LinkKind;

pub(crate) use carry::carry;
pub(crate) use strength::{give as give_strength, shown as strength_shown};
pub(crate) use drive::{Leaving, Reins, drive, is_driven};
pub(crate) use mounts::{Rein, boost, reaches, steers, used_on};
use mounts::predicted;
pub(crate) use packets::sound as sound_packet;
pub(crate) use packs::{load as load_pack, open as open_pack, refusal as pack_refusal, slots as pack_slots, spilled};
pub(crate) use seats::layout;
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
    let (Some(kind), Some(rider_kind)) = (world.get::<Kind>(vehicle), kind_of(world, rider)) else { return Err(Refusal::NotRideable) };
    let layout = layout(&kind.0).filter(|layout| layout.takes(&rider_kind)).ok_or(Refusal::NotRideable)?;
    if layout.saddled && !mounts::saddled(world, vehicle) {
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
    let Some(layout) = world.get::<Kind>(vehicle).and_then(|kind| layout(&kind.0)) else { return 0 };
    layout.capacity().saturating_sub(world.get::<Seats>(vehicle).map_or(0, |seats| seats.0.len()))
}

/// Seats `rider` on `vehicle`, off what it rode before; tells everyone who sees it.
pub(crate) fn seat(world: &mut World, rider: Entity, vehicle: Entity) -> Result<(), Refusal> {
    can_seat(world, rider, vehicle)?;
    if world.get::<Riding>(rider).is_some() {
        unseat(world, rider, false);
    }
    let (Some(rider_id), Some(vehicle_id)) = (world.get::<NetId>(rider).map(|id| id.0), world.get::<NetId>(vehicle).map(|id| id.0)) else { return Err(Refusal::NotRideable) };
    let mut seats = world.get::<Seats>(vehicle).cloned().unwrap_or_default();
    seats.0.push(rider);
    let index = seats.0.len() - 1;
    world.entity_mut(vehicle).insert(seats);
    world.entity_mut(rider).insert(Riding { vehicle, id: vehicle_id });
    if world.get::<Identity>(rider).is_some() {
        world.entity_mut(rider).insert(Reins::default());
    }
    let mut told = seat_packets(world, vehicle, Told::Newcomer(rider));
    if let (Some((at, _)), Some(kind)) = (vehicle_wire(world, vehicle), world.get::<Kind>(vehicle).map(|kind| kind.0.clone())) {
        told.push(packets::sound("mount", vehicle_id, &kind, at));
    }
    told.push(packets::link(vehicle_id, rider_id, if index == 0 { LinkKind::Driver } else { LinkKind::Passenger }, false));
    tell(world, vehicle, rider, told);
    retell(world, vehicle);
    carry::place(world, vehicle);
    Ok(())
}

/// Takes `rider` off what it rides; with `step_off` it is put where vanilla's riders step off to
/// (exit.rs). False for one that rides nothing.
pub(crate) fn unseat(world: &mut World, rider: Entity, step_off: bool) -> bool {
    let Some(Riding { vehicle, id: vehicle_id }) = world.get::<Riding>(rider).copied() else { return false };
    world.entity_mut(rider).remove::<(Riding, Reins)>();
    let rider_id = world.get::<NetId>(rider).map_or(0, |id| id.0);
    let left = world.get_mut::<Seats>(vehicle).map(|mut seats| {
        seats.0.retain(|seated| *seated != rider);
        seats.0.is_empty()
    });
    if left == Some(true) {
        world.entity_mut(vehicle).remove::<Seats>();
    }
    let mut told = vec![packets::link(vehicle_id, rider_id, LinkKind::Off, false)];
    told.extend(seat_packets(world, vehicle, Told::Moved));
    tell(world, vehicle, rider, told);
    retell(world, vehicle);
    if step_off {
        exit::step_off(world, rider, vehicle);
    }
    true
}

/// Takes every rider off `vehicle`, each stepping off; how many there were.
pub(crate) fn evict(world: &mut World, vehicle: Entity) -> usize {
    let riders = world.get::<Seats>(vehicle).map(|seats| seats.0.clone()).unwrap_or_default();
    riders.iter().filter(|rider| unseat(world, **rider, true)).count()
}

/// Lets an entity or player that is going leave its seat and lets its riders off.
pub(crate) fn forget(world: &mut World, entity: Entity) {
    unseat(world, entity, false);
    evict(world, entity);
}

/// Who of `vehicle`'s riders is told its seat: everyone in it.
#[derive(Clone, Copy, PartialEq)]
enum Told {
    /// One just seated, with all of BDS's items; the others' moved seats as positions alone.
    Newcomer(Entity),
    /// Each one's seat moved: positions alone.
    Moved,
    /// Each one's whole seat, for a viewer who did not know of them.
    Whole,
}

/// The seats of `vehicle`'s riders (their wire positions in the vehicle's frame).
fn seat_packets(world: &World, vehicle: Entity, told: Told) -> Vec<Bytes> {
    let (Some(kind), Some(riders)) = (world.get::<Kind>(vehicle), world.get::<Seats>(vehicle)) else { return Vec::new() };
    let Some(layout) = layout(&kind.0) else { return Vec::new() };
    let count = riders.0.len();
    let mut packets = Vec::new();
    for (index, rider) in riders.0.iter().enumerate() {
        let (Some(seat), Some(id), Some(rider_kind)) = (layout.seat(count, index), world.get::<NetId>(*rider), kind_of(world, *rider)) else { continue };
        let whole = told == Told::Whole || told == Told::Newcomer(*rider);
        if whole || count > 1 || told == Told::Moved {
            packets.push(packets::seat(id.0, &seat, seats::offset(&seat, &rider_kind, 0.0), whole));
        }
    }
    packets
}

/// Sends `packets` to those who see `vehicle`.
/// A vehicle that appeared this tick is shown with its riders (`retell`); a rider that did waits
/// for its own announcement.
fn tell(world: &mut World, vehicle: Entity, rider: Entity, packets: Vec<Bytes>) {
    let (Some(layer), Some(body)) = (world.get::<InLayer>(vehicle).map(|layer| layer.0), world.get::<Body>(vehicle)) else { return };
    if crate::entities::is_new(world, vehicle) {
        return;
    }
    let cell = body.at.map(|c| c.floor() as i32);
    crate::entities::tell_viewers(world, rider, layer, cell, packets);
}

/// What players who come to see `vehicle` later are told after it: its tack, its riders' seats
/// and links; and its stored compound names its riders that a world folder keeps.
fn retell(world: &mut World, vehicle: Entity) {
    let riders = world.get::<Seats>(vehicle).map(|seats| seats.0.clone()).unwrap_or_default();
    let vehicle_id = world.get::<NetId>(vehicle).map_or(0, |id| id.0);
    let mut after = mounts::worn(world, vehicle);
    after.extend(seat_packets(world, vehicle, Told::Whole));
    for (index, rider) in riders.iter().enumerate() {
        let Some(id) = world.get::<NetId>(*rider) else { continue };
        after.push(packets::link(vehicle_id, id.0, if index == 0 { LinkKind::Driver } else { LinkKind::Passenger }, true));
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
