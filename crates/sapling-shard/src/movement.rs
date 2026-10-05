//! Where players are and what their bodies do, as their clients report it, and what viewers are
//! told about it: packets and field values as BDS sends them (`sapling` example `bds_probe`).
// TODO: physics validation (acacia-physics); only teleport-sized steps are refused for now.

use std::collections::HashSet;

use acacia_proto::packets::{Animate, AnimateActionId, MovePlayer, MovePlayerMode, MovePlayerTeleport, MovePlayerTeleportCause, PlayerAuthInput, SetEntityData};
use acacia_proto::types::{
    EntityProperties, InputData, LegacyEntityType, MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType,
    MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault, MetadataFlags1, Vec3f,
};
use bevy_ecs::prelude::*;
use sapling_data::EYE_HEIGHT;

use crate::blocks::BlockInputs;
use crate::components::{NetId, Outgoing, PacketQueues, Position, Stage, Tick};
use crate::visibility::ToViewers;

/// Blocks an input may be from the one before it; further is a teleport the server never made.
const MAX_STEP: f32 = 10.0;
/// Ticks a refused player has to obey the correction before it is sent again.
const SETBACK_TICKS: u64 = 20;
/// `source_entity_type` of BDS's own teleports.
const TELEPORT_SOURCE: i64 = 1;
const STANDING_HEIGHT: f32 = 1.8;
const SNEAKING_HEIGHT: f32 = 1.49;

#[derive(Clone, Copy, PartialEq, Default)]
pub(crate) struct Pose {
    pub sneaking: bool,
    pub sprinting: bool,
    pub swimming: bool,
}

impl Pose {
    /// The entity flags viewers get on top of the recorded ones.
    pub fn flags(self) -> MetadataFlags1 {
        let mut flags = MetadataFlags1::BREATHING;
        for (on, flag) in [(self.sneaking, MetadataFlags1::SNEAKING), (self.sprinting, MetadataFlags1::SPRINTING), (self.swimming, MetadataFlags1::SWIMMING)] {
            if on {
                flags.insert(flag);
            }
        }
        flags
    }

    fn apply(&mut self, input: &[InputData]) {
        for flag in input {
            match flag {
                InputData::StartSneaking => self.sneaking = true,
                InputData::StopSneaking => self.sneaking = false,
                InputData::StartSprinting => self.sprinting = true,
                InputData::StopSprinting => self.sprinting = false,
                InputData::StartSwimming => self.swimming = true,
                InputData::StopSwimming => self.swimming = false,
                _ => {}
            }
        }
    }
}

#[derive(Component, Default)]
pub(crate) struct Motion {
    pub pose: Pose,
    /// The client's own tick of its last input; BDS stamps what it tells viewers with it.
    input_tick: u64,
    /// The shard tick until which far inputs are dropped without another correction.
    setback_until: u64,
    /// Set by a server teleport until the client's input says it handled one: inputs before
    /// that are from the old place. The tick is when the teleport is sent again.
    unconfirmed_teleport: Option<u64>,
    told: Option<(Position, Pose)>,
}

impl Motion {
    /// Whether the client has yet to follow a teleport: what it sends is from the old place.
    pub fn awaits_teleport(&self) -> bool {
        self.unconfirmed_teleport.is_some()
    }
}

fn eyes(position: &Position) -> Vec3f {
    let [x, y, z] = position.feet;
    Vec3f { x, y: y + EYE_HEIGHT, z }
}

/// A walk, or with `teleport` a jump the client must follow.
fn move_player(id: u64, position: &Position, teleport: Option<MovePlayerTeleportCause>, tick: u64) -> MovePlayer {
    MovePlayer {
        runtime_id: id as u32,
        position: eyes(position),
        pitch: position.pitch,
        yaw: position.yaw,
        head_yaw: position.head_yaw,
        mode: if teleport.is_some() { MovePlayerMode::Teleport } else { MovePlayerMode::Normal },
        on_ground: position.on_ground,
        ridden_runtime_id: 0,
        teleport: teleport.map(|cause| MovePlayerTeleport { cause, source_entity_type: LegacyEntityType::from_raw(TELEPORT_SOURCE) }),
        tick,
    }
}

/// Puts a player's feet at `feet` and tells its client, as BDS's `/tp` does. Viewers learn of it
/// from [`announce`].
pub(crate) fn teleport(world: &mut World, entity: Entity, feet: [f32; 3]) {
    let tick = world.resource::<Tick>().0;
    let Ok((id, mut position, mut motion, mut out)) = world.query::<(&NetId, &mut Position, &mut Motion, &mut Outgoing)>().get_mut(world, entity) else { return };
    (position.feet, position.on_ground) = (feet, true);
    motion.unconfirmed_teleport = Some(tick + SETBACK_TICKS);
    out.push(&move_player(id.0, &position, Some(MovePlayerTeleportCause::Command), motion.input_tick));
}

fn pose_packet(id: u64, pose: Pose, tick: u64) -> SetEntityData {
    let height = MetadataDictionaryItem {
        key: Key::BoundingboxHeight,
        r#type: MetadataDictionaryItemType::Float,
        legacy_type: 3,
        value: Value::Default(MetadataDictionaryItemValueDefault::Float(if pose.sneaking { SNEAKING_HEIGHT } else { STANDING_HEIGHT })),
    };
    let flags = sapling_data::player_metadata("", pose.flags()).into_iter().filter(|item| item.key == Key::Flags);
    SetEntityData { runtime_entity_id: id, metadata: flags.chain([height]).collect(), properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick }
}

type Mover<'a> = (&'a NetId, &'a Stage, &'a mut Position, &'a mut Motion, &'a mut Outgoing);

/// Where each client says its player is. Trusted, except that non-finite values are dropped and
/// a step no player makes in a tick is answered with a teleport back.
pub(crate) fn player_input(mut packets: ResMut<PacketQueues>, tick: Res<Tick>, mut blocks: ResMut<BlockInputs>, mut players: Query<Mover>) {
    for (entity, mut input) in packets.take::<PlayerAuthInput>() {
        let Ok((id, stage, mut position, mut motion, mut out)) = players.get_mut(entity) else { continue };
        if input.block_action.is_some() || input.transaction.is_some() || input.item_stack_request.is_some() {
            tracing::debug!(tick = input.tick, actions = ?input.block_action, transaction = ?input.transaction, request = ?input.item_stack_request, "input with an action");
        }
        // A teleport may have crossed worlds: the blocks an older input names are not these.
        if !motion.awaits_teleport() || input.input_data.contains(&InputData::HandledTeleport) {
            blocks.take_from(entity, &mut input);
        }
        let reported = [input.position.x, input.position.y, input.position.z, input.yaw, input.pitch, input.head_yaw];
        if *stage < Stage::Loading || reported.iter().any(|c| !c.is_finite()) {
            continue;
        }
        if let Some(resend_at) = motion.unconfirmed_teleport {
            if input.input_data.contains(&InputData::HandledTeleport) {
                motion.unconfirmed_teleport = None;
            } else {
                if tick.0 >= resend_at {
                    out.push(&move_player(id.0, &position, Some(MovePlayerTeleportCause::Command), motion.input_tick));
                    motion.unconfirmed_teleport = Some(tick.0 + SETBACK_TICKS);
                }
                continue;
            }
        }
        let feet = [reported[0], reported[1] - EYE_HEIGHT, reported[2]];
        let step: f32 = feet.iter().zip(position.feet).map(|(to, from)| (to - from).powi(2)).sum();
        if step > MAX_STEP * MAX_STEP {
            if tick.0 >= motion.setback_until {
                tracing::debug!(player = id.0, from = ?position.feet, to = ?feet, "refusing a teleport-sized step");
                out.push(&move_player(id.0, &position, Some(MovePlayerTeleportCause::UnknownValue), 0));
                motion.setback_until = tick.0 + SETBACK_TICKS;
            }
            continue;
        }
        motion.pose.apply(&input.input_data);
        motion.input_tick = input.tick;
        // The client reports no on-ground flag; BDS derives one. Close enough for viewers' animation.
        let on_ground = input.input_data.contains(&InputData::VerticalCollision) && input.delta.y <= 0.0;
        *position = Position { feet, yaw: input.yaw, pitch: input.pitch, head_yaw: input.head_yaw, on_ground };
    }
}

/// Queues, once per player that moved or changed pose, what its viewers need to know.
pub(crate) fn announce(mut to_viewers: ResMut<ToViewers>, mut players: Query<(Entity, &NetId, &Stage, &Position, &mut Motion)>) {
    for (entity, id, stage, position, mut motion) in &mut players {
        if *stage != Stage::InWorld {
            continue;
        }
        let (now, tick) = ((*position, motion.pose), motion.input_tick);
        // A new viewer learns the current state from AddPlayer; only changes are announced.
        let Some(told) = motion.told.replace(now) else { continue };
        if told.0 != now.0 {
            to_viewers.push(entity, &move_player(id.0, position, None, tick));
        }
        if told.1 != now.1 {
            to_viewers.push(entity, &pose_packet(id.0, now.1, tick));
        }
    }
}

/// Shows an arm swing to the swinger's viewers, at most one a tick each.
pub(crate) fn relay_swings(mut packets: ResMut<PacketQueues>, mut to_viewers: ResMut<ToViewers>, players: Query<(&NetId, &Stage)>) {
    let mut swung = HashSet::new();
    for (entity, animate) in packets.take::<Animate>() {
        let Ok((id, stage)) = players.get(entity) else { continue };
        if animate.action_id == AnimateActionId::SwingArm && *stage == Stage::InWorld && swung.insert(entity) {
            // The id is the sender's whatever the packet says; BDS forwards no swing source.
            to_viewers.push(entity, &Animate { action_id: AnimateActionId::SwingArm, runtime_entity_id: id.0, data: 0.0, has_swing_source: false, swing_source: None });
        }
    }
}
