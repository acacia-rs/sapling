//! Keeping viewers' copies of an entity the server moves where it is, as BDS's generic actor sync
//! does: a MoveEntityDelta of the axes it moved more than 0.01 along, and a SetEntityMotion when
//! its motion changed by more than 0.2 since the last one, or stopped (docs/research/items.md 5;
//! a falling block on BDS was sent exactly that, research/captures/bds-mechanisms-findings.md).

use acacia_proto::packets::{MoveEntityDelta, SetEntityMotion};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::Component;
use bytes::Bytes;

const MOVED: f32 = 0.01;
const NEW_MOTION: f32 = 0.2;

/// What viewers were last told of an entity.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct Told {
    at: [f32; 3],
    motion: [f32; 3],
}

impl Told {
    /// An entity just announced at `at`; its announced motion does not count.
    pub fn new(at: [f32; 3]) -> Self {
        Self { at, motion: [0.0; 3] }
    }

    /// The packets that bring viewers of the entity `id` to `at` (where the wire puts it) and
    /// `motion`. BDS sends 0 in their tick fields.
    pub fn update(&mut self, id: u64, at: [f32; 3], motion: [f32; 3], on_ground: bool) -> Vec<Bytes> {
        let mut packets = Vec::new();
        let moved: [Option<f32>; 3] = std::array::from_fn(|axis| ((at[axis] - self.at[axis]).abs() > MOVED).then_some(at[axis]));
        if moved.iter().any(Option::is_some) {
            moved.iter().enumerate().filter_map(|(axis, sent)| Some((axis, (*sent)?))).for_each(|(axis, sent)| self.at[axis] = sent);
            let [x, y, z] = moved;
            let delta = MoveEntityDelta {
                runtime_entity_id: id,
                x,
                y,
                z,
                rot_x: None,
                rot_y: None,
                rot_z: None,
                on_ground,
                force_move: false,
                force_move_local_entity: false,
                force_completion: false,
                ticks: 0,
            };
            packets.push(sapling_net::encode(&delta));
        }
        let change = (0..3).map(|axis| (motion[axis] - self.motion[axis]).powi(2)).sum::<f32>().sqrt();
        let stopped = motion == [0.0; 3] && self.motion != [0.0; 3];
        if change > NEW_MOTION || stopped {
            self.motion = motion;
            let [x, y, z] = motion;
            packets.push(sapling_net::encode(&SetEntityMotion { runtime_entity_id: id, velocity: Vec3f { x, y, z }, tick: 0 }));
        }
        packets
    }
}

#[cfg(test)]
mod tests {
    use acacia_proto::{Packet, RawPacket};

    use super::*;

    fn kinds(packets: &[Bytes]) -> Vec<u32> {
        packets.iter().map(|packet| RawPacket::parse(packet.clone()).unwrap().id).collect()
    }

    #[test]
    fn small_moves_are_kept_and_motion_is_sent_when_it_changed_a_lot_or_stopped() {
        let mut told = Told::new([0.5, 70.0, 0.5]);
        assert!(told.update(1, [0.5, 69.995, 0.5], [0.0, -0.1, 0.0], false).is_empty());
        assert_eq!(kinds(&told.update(1, [0.5, 69.9, 0.5], [0.0, -0.19, 0.0], false)), [MoveEntityDelta::ID]);
        assert_eq!(kinds(&told.update(1, [0.5, 69.6, 0.5], [0.0, -0.25, 0.0], false)), [MoveEntityDelta::ID, SetEntityMotion::ID]);
        assert_eq!(kinds(&told.update(1, [0.5, 69.0, 0.5], [0.0; 3], true)), [MoveEntityDelta::ID, SetEntityMotion::ID]);
    }
}
