//! Keeping viewers' copies of an entity the server moves where it is, as BDS's generic actor sync
//! does: a MoveEntityDelta of the axes it moved more than 0.01 along, and a SetEntityMotion when
//! its motion changed by more than 0.2 since the last one, or stopped (docs/research/items.md 5;
//! a falling block on BDS was sent exactly that, research/captures/bds-mechanisms-findings.md).

use acacia_proto::packets::{MoveEntityDelta, SetEntityData, SetEntityMotion};
use acacia_proto::types::{EntityProperties, MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataFlags1 as Flags, Vec3f};
use bevy_ecs::prelude::Component;
use bytes::Bytes;

const MOVED: f32 = 0.01;
const NEW_MOTION: f32 = 0.2;

/// What viewers were last told of an entity.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct Told {
    at: [f32; 3],
    motion: [f32; 3],
    moving: Option<Moving>,
}

/// Whether viewers were told the entity is `MOVING`, and whether the last tick started on the ground.
#[derive(Clone, Copy, Debug)]
struct Moving {
    set: bool,
    started_on_ground: Option<bool>,
}

impl Told {
    /// An entity just announced at `at`; its announced motion does not count.
    pub fn new(at: [f32; 3]) -> Self {
        Self { at, motion: [0.0; 3], moving: None }
    }

    /// Tells viewers when the entity is moving, as BDS tells them of items and falling blocks;
    /// `ticked` when its body already ran a tick, from the air, that viewers were not told of.
    pub fn flag_moving(&mut self, ticked: bool) {
        self.moving = Some(Moving { set: false, started_on_ground: ticked.then_some(false) });
    }

    /// The flags of the entity `id` when its `MOVING` changed, the tick starting `on_ground`. On
    /// BDS it follows whether the tick before started in the air: an item thrown at tick s was
    /// flagged at s+1 and, landing at L, cleared at L+2 (research/captures/bds-mechanisms.txt B:1269-1312).
    pub fn moving(&mut self, id: u64, on_ground: bool) -> Option<Bytes> {
        let moving = self.moving.as_mut()?;
        let now = !std::mem::replace(&mut moving.started_on_ground, Some(on_ground))?;
        if now == moving.set {
            return None;
        }
        moving.set = now;
        let flags = Flags(Flags::HAS_COLLISION.0 | Flags::AFFECTED_BY_GRAVITY.0 | if now { Flags::MOVING.0 } else { 0 });
        let item = MetadataDictionaryItem { key: Key::Flags, r#type: Type::Long, legacy_type: 7, value: Value::Flags(flags) };
        Some(sapling_net::encode(&SetEntityData { runtime_entity_id: id, metadata: vec![item], properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 }))
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

    #[test]
    fn moving_is_flagged_a_tick_after_a_tick_in_the_air_began_and_cleared_two_after_landing() {
        let mut told = Told::new([0.0; 3]);
        assert!(told.moving(1, false).is_none(), "only for kinds that asked");
        told.flag_moving(false);
        // Ticks start: thrown, in the air, landed during the third, on the ground, on the ground.
        let sent: Vec<bool> = [false, false, false, true, true].into_iter().map(|on_ground| told.moving(1, on_ground).is_some()).collect();
        assert_eq!(sent, [false, true, false, false, true]);
        let mut fallen = Told::new([0.0; 3]);
        fallen.flag_moving(true);
        assert!(fallen.moving(1, false).is_some(), "a falling block is flagged as it is shown");
    }
}
