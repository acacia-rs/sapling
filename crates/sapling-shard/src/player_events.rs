//! A player's own inputs and state changes offered to plugins (sapling_api's player_event.rs). The
//! systems that read inputs cannot reach plugins, so they queue them here for [`dispatch`]; a
//! cancel puts the player back as it was and tells its client so.

use acacia_proto::packets::{PlayerHotbar, UpdatePlayerGameType};
use acacia_proto::types::{InputData, WindowID};
use bevy_ecs::prelude::*;
use sapling_api::{GameMode, GameModeChangeEvent, HeldSlotChangeEvent, JumpEvent, PunchAirEvent, TeleportEvent, ToggleSneakEvent, ToggleSprintEvent};

use crate::components::{Mode, NetId, Outgoing, Position, Tick, in_world_identity};
use crate::inventory::Inventory;
use crate::movement::{self, Motion, Pose};
use crate::{plugins, worlds};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Input {
    Sprint(bool),
    Sneak(bool),
    Jump,
    PunchAir,
    HeldSlot { from: u8, to: u8 },
}

/// This tick's inputs, in the order they came.
#[derive(Resource, Default)]
pub(crate) struct Inputs(Vec<(Entity, Input)>);

impl Inputs {
    /// What one PlayerAuthInput says: the pose it changed from `before` to `after`, its jumps and
    /// its swings at nothing.
    pub fn record(&mut self, entity: Entity, before: Pose, after: Pose, flags: &[InputData]) {
        if before.sprinting != after.sprinting {
            self.0.push((entity, Input::Sprint(after.sprinting)));
        }
        if before.sneaking != after.sneaking {
            self.0.push((entity, Input::Sneak(after.sneaking)));
        }
        for flag in flags {
            match flag {
                InputData::StartJumping => self.0.push((entity, Input::Jump)),
                InputData::MissedSwing => self.0.push((entity, Input::PunchAir)),
                _ => {}
            }
        }
    }

    pub fn held(&mut self, entity: Entity, from: u8, to: u8) {
        self.0.push((entity, Input::HeldSlot { from, to }));
    }
}

/// Offers the queued inputs; a cancelled toggle or slot change is undone.
pub(crate) fn dispatch(world: &mut World) {
    for (entity, input) in std::mem::take(&mut world.resource_mut::<Inputs>().0) {
        let Some(player) = in_world_identity(world, entity) else { continue };
        worlds::focus_on(world, entity);
        let cancelled = match input {
            Input::Sprint(on) => plugins::offer(world, ToggleSprintEvent::new(player, on)).is_cancelled(),
            Input::Sneak(on) => plugins::offer(world, ToggleSneakEvent::new(player, on)).is_cancelled(),
            Input::Jump => {
                plugins::offer(world, JumpEvent::new(player));
                false
            }
            Input::PunchAir => {
                plugins::offer(world, PunchAirEvent::new(player));
                false
            }
            Input::HeldSlot { from, to } => plugins::offer(world, HeldSlotChangeEvent::new(player, from, to)).is_cancelled(),
        };
        match input {
            Input::Sprint(on) if cancelled => repose(world, entity, |pose| pose.sprinting = !on),
            Input::Sneak(on) if cancelled => repose(world, entity, |pose| pose.sneaking = !on),
            Input::HeldSlot { from, .. } if cancelled => hold_again(world, entity, from),
            _ => {}
        }
    }
}

/// Sets the pose back and tells the player; viewers follow from `movement::announce`.
fn repose(world: &mut World, entity: Entity, undo: impl FnOnce(&mut Pose)) {
    let (Some(id), tick) = (world.get::<NetId>(entity).map(|id| id.0), world.resource::<Tick>().0) else { return };
    let Some(mut motion) = world.get_mut::<Motion>(entity) else { return };
    undo(&mut motion.pose);
    let shown = movement::pose_packet(id, motion.pose, tick);
    world.get_mut::<Outgoing>(entity).expect("players have an outbox").push(&shown);
}

fn hold_again(world: &mut World, entity: Entity, slot: u8) {
    if let Some(mut inventory) = world.get_mut::<Inventory>(entity) {
        (inventory.selected, inventory.unsaved) = (slot, true);
    }
    if let Some(mut out) = world.get_mut::<Outgoing>(entity) {
        out.push(&PlayerHotbar { selected_slot: u32::from(slot), window_id: WindowID::Inventory, select_slot: true });
    }
}

/// Whether plugins let the game move `entity` to `to` ([`TeleportEvent`]).
pub(crate) fn may_teleport(world: &mut World, entity: Entity, to: [f32; 3]) -> bool {
    let (Some(player), Some(from)) = (in_world_identity(world, entity), world.get::<Position>(entity).map(|p| p.feet)) else { return true };
    !plugins::offer(world, TeleportEvent::new(player, from, to)).is_cancelled()
}

/// Whether plugins let `entity` go into `mode` ([`GameModeChangeEvent`]); a refused one tells the
/// client its mode again, since its own menu may have switched it already.
pub(crate) fn may_change_mode(world: &mut World, entity: Entity, mode: GameMode) -> bool {
    let (Some(player), Some(from)) = (in_world_identity(world, entity), world.get::<Mode>(entity).map(|mode| mode.0)) else { return true };
    if from == mode || !plugins::offer(world, GameModeChangeEvent::new(player, from, mode)).is_cancelled() {
        return true;
    }
    let (Some(id), tick) = (world.get::<NetId>(entity).map(|id| id.0), world.resource::<Tick>().0) else { return false };
    if let Some(mut out) = world.get_mut::<Outgoing>(entity) {
        out.push(&UpdatePlayerGameType { gamemode: crate::join::wire_game_mode(from), player_unique_id: id as i64, tick });
    }
    false
}
