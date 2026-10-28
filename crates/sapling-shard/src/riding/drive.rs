//! What riding players ask for. Their inputs are kept in [`Reins`] (movement.rs leaves their place
//! to their seat): a driver whose client moves its vehicle itself (a boat, a tamed and saddled
//! horse: `ClientPredictedVehicle`) reports where the vehicle is, and the vehicle is put there, as
//! BDS takes it (acacia's docs/research/riding-fishing-elytra.md). A rider leaves on an
//! `Interact LeaveVehicle`, which a client sends for sneak too, where its client put it. A
//! horse rears through its jumps (rearing.rs).
// TODO: BDS also simulates a predicted vehicle and corrects its driver; drivers are trusted here.

use acacia_proto::packets::PlayerAuthInput;
use acacia_proto::types::InputData;
use bevy_ecs::prelude::*;

use super::{driven_by, predicted, unseat, wire_lift};
use crate::body::Body;
use crate::components::{InLayer, NetId, Tick};
use crate::entities::Kind;
use crate::sync::Told;

/// The vehicles a driver's client always moves; a horse only tamed and saddled (`predicted`).
const BOATS: [&str; 2] = ["minecraft:boat", "minecraft:chest_boat"];

/// A riding player's last input.
#[derive(Component, Default, Clone, Copy, Debug)]
pub(crate) struct Reins {
    /// Where the client put the vehicle it drives (the vehicle's wire position) and its yaw.
    pub predicted: Option<([f32; 3], f32)>,
    /// The keys: across (left +) and forward.
    pub keys: [f32; 2],
    pub jumping: bool,
    /// The rider's own yaw and pitch.
    pub look: [f32; 2],
}

impl Reins {
    /// Takes in a riding player's input.
    pub fn take(&mut self, input: &PlayerAuthInput) {
        let has = |flag: InputData| input.input_data.contains(&flag);
        let predicted = input.vehicle_rotation.as_ref().filter(|_| has(InputData::ClientPredictedVehicle));
        self.predicted = predicted.map(|rotation| ([input.position.x, input.position.y, input.position.z], rotation.z));
        self.keys = [input.move_vector.x, input.move_vector.z];
        self.jumping = has(InputData::Jumping);
        self.look = [input.yaw, input.pitch];
    }
}

/// The riders who asked to leave this tick (inventory/equipment.rs takes their `Interact`s in).
#[derive(Resource, Default)]
pub(crate) struct Leaving(pub Vec<Entity>);

/// A vehicle its driver's client moved this tick: its own physics skip the tick.
#[derive(Component)]
pub(crate) struct Driven(u64);

/// Whether a driver's client moved the vehicle this tick.
pub(crate) fn is_driven(world: &World, vehicle: Entity) -> bool {
    world.get::<Driven>(vehicle).is_some_and(|driven| driven.0 == world.resource::<Tick>().0)
}

/// Lets off the riders who asked and moves the vehicles drivers move.
pub(crate) fn drive(world: &mut World) {
    let leave = std::mem::take(&mut world.get_resource_or_insert_with(Leaving::default).0);
    for rider in leave {
        unseat(world, rider, false);
    }
    let drivers: Vec<(Entity, Reins)> = world.query::<(Entity, &Reins)>().iter(world).map(|(rider, reins)| (rider, *reins)).collect();
    for (driver, reins) in drivers {
        if let (Some(vehicle), Some((wire, yaw))) = (driven_by(world, driver), reins.predicted) {
            follow(world, driver, vehicle, wire, yaw);
        }
    }
}

/// Puts `vehicle` where its driver's client says it is, telling everyone else who sees it.
fn follow(world: &mut World, driver: Entity, vehicle: Entity, [x, y, z]: [f32; 3], yaw: f32) {
    if [x, y, z, yaw].iter().any(|c| !c.is_finite()) {
        return;
    }
    let (Some(id), Some(layer), Some(kind)) = (world.get::<NetId>(vehicle).map(|id| id.0), world.get::<InLayer>(vehicle).map(|layer| layer.0), world.get::<Kind>(vehicle).map(|kind| kind.0.clone())) else { return };
    if !BOATS.contains(&kind.as_str()) && !predicted(world, vehicle, &kind) {
        return;
    }
    let feet = [x, y - wire_lift(&kind), z];
    let Some(mut body) = world.get_mut::<Body>(vehicle) else { return };
    let before = body.at;
    (body.motion, body.at) = (std::array::from_fn(|axis| feet[axis] - before[axis]), feet);
    let (motion, on_ground) = (body.motion, body.on_ground);
    let tick = world.resource::<Tick>().0;
    world.entity_mut(vehicle).insert(Driven(tick));
    if let Some(reared) = (!BOATS.contains(&kind.as_str())).then(|| super::rearing::rear(world, vehicle, id, &kind, motion[1])).flatten() {
        crate::blocks::to_viewers(world, layer, feet.map(|c| c.floor() as i32), &[reared]);
    }
    let Some(mut told) = world.get_mut::<Told>(vehicle) else { return };
    told.face(yaw, yaw);
    let packets = told.update(id, [x, y, z], motion, on_ground);
    if !packets.is_empty() {
        crate::view::to_viewers_but(world, layer, feet.map(|c| c.floor() as i32), Some(driver), &packets);
    }
}
