//! What the dragon's viewers are told of its flight, as BDS told it (research/captures/
//! bds-dragon-watch.txt): a MoveEntityDelta whenever it moved or turned, with its pitch, its yaw
//! and its head's yaw (the same); a SetEntityMotion once its motion changed by more than 0.2 or
//! stopped; its flags when `MOVING` (a tick after a move) or `SITTING` change.

use acacia_proto::packets::{MoveEntityDelta, SetEntityMotion};
use acacia_proto::types::{MetadataFlags1, Vec3f};
use bytes::Bytes;

use super::flight::Flight;

const NEW_MOTION: f32 = 0.2;
/// A tick's move, squared, past which the next tick shows `MOVING` (docs/research/moving-flag.md).
const MOVING: f32 = 0.001;

/// What viewers were last told of the dragon.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Shown {
    at: Option<[f32; 3]>,
    turn: [u8; 3],
    motion: [f32; 3],
    flags: Option<(bool, bool)>,
    moved_far: bool,
    target: i64,
}

/// Its target's id, told whenever it changes (a strafe or a scan picks one, its end lets it go).
pub(super) fn tell_target(shown: &mut Shown, id: u64, target: i64) -> Option<Bytes> {
    if shown.target == target {
        return None;
    }
    shown.target = target;
    let item = super::cloud::item(acacia_proto::types::MetadataDictionaryItemKey::TargetEid, acacia_proto::types::MetadataDictionaryItemValueDefault::Long(target));
    Some(sapling_net::encode(&acacia_proto::packets::SetEntityData { runtime_entity_id: id, metadata: vec![item], properties: acacia_proto::types::EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 }))
}

fn byte(degrees: f32) -> u8 {
    ((degrees.rem_euclid(360.0) / 360.0 * 256.0).round() as u32 % 256) as u8
}

/// The packets that bring viewers to where the dragon `id` is now.
pub(super) fn tell(shown: &mut Shown, id: u64, flight: &Flight) -> Vec<Bytes> {
    let mut packets = Vec::new();
    let at = flight.motion.at;
    let last = *shown.at.get_or_insert(at);
    let gone: f32 = (0..3).map(|axis| (at[axis] - last[axis]).powi(2)).sum();
    let moving = std::mem::replace(&mut shown.moved_far, gone > MOVING);
    let flags = (moving, flight.sitting);
    if shown.flags != Some(flags) {
        shown.flags = Some(flags);
        let bit = |on: bool, flag: MetadataFlags1| if on { flag } else { MetadataFlags1(0) };
        let extra = MetadataFlags1(bit(moving, MetadataFlags1::MOVING).0 | bit(flight.sitting, MetadataFlags1::SITTING).0);
        packets.push(sapling_net::encode(&sapling_data::mob_flags_set(id, super::DRAGON, extra)));
    }
    let turn = [byte(flight.pitch), byte(flight.motion.yaw), byte(flight.motion.yaw)];
    if at != last || turn != shown.turn {
        let axis = |i: usize| (at[i] != last[i]).then_some(at[i]);
        let rot = |i: usize| (turn[i] != shown.turn[i]).then_some(turn[i]);
        let delta = MoveEntityDelta { runtime_entity_id: id, x: axis(0), y: axis(1), z: axis(2), rot_x: rot(0), rot_y: rot(1), rot_z: rot(2), on_ground: false, force_move: false, force_move_local_entity: false, force_completion: false, ticks: 0 };
        packets.push(sapling_net::encode(&delta));
        (shown.at, shown.turn) = (Some(at), turn);
    }
    let motion = flight.motion.delta;
    let change = (0..3).map(|axis| (motion[axis] - shown.motion[axis]).powi(2)).sum::<f32>().sqrt();
    if change > NEW_MOTION || motion == [0.0; 3] && shown.motion != [0.0; 3] {
        shown.motion = motion;
        let [x, y, z] = motion;
        packets.push(sapling_net::encode(&SetEntityMotion { runtime_entity_id: id, velocity: Vec3f { x, y, z }, tick: 0 }));
    }
    packets
}
