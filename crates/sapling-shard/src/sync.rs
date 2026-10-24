//! Keeping viewers' copies of an entity the server moves where it is, as BDS's generic actor sync
//! does: a MoveEntityDelta of the axes that changed once it moved more than 0.01, or more than
//! 0.001 when it has all but stopped, and a SetEntityMotion when its motion changed by more than
//! 0.2 since the last one, or stopped (docs/research/items.md 5; matched to an item, an orb and an
//! armour stand in research/captures/bds-water.txt).

use acacia_proto::packets::{MoveEntityDelta, SetEntityData, SetEntityMotion};
use acacia_proto::types::{EntityProperties, MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataFlags1 as Flags, Vec3f};
use bevy_ecs::prelude::Component;
use bytes::Bytes;

const MOVED: f32 = 0.01;
const SETTLED: f32 = 0.001;
const STOPPING: f32 = 0.001;
const NEW_MOTION: f32 = 0.2;
/// A tick's move, squared, past which the next tick shows `MOVING` (docs/research/moving-flag.md).
const MOVING: f32 = 0.001;

fn length(v: [f32; 3]) -> f32 {
    v.iter().map(|axis| axis * axis).sum::<f32>().sqrt()
}

/// What viewers were last told of an entity.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct Told {
    at: [f32; 3],
    motion: [f32; 3],
    flags: Option<Shown>,
    /// Its body's and its head's turn as viewers know them, and as they are now.
    turned: [u8; 2],
    turn: [u8; 2],
    /// Its pitch as viewers know it and as it is now; `None` for an entity whose pitch is not told.
    pitched: Option<[u8; 2]>,
    /// Its next move is a teleport, which BDS sends forced.
    jumped: bool,
    /// Viewers are told of every move, however small (an eye of ender's, bds-eye-throw.txt).
    every_move: bool,
}

/// The flags viewers were told the entity has, and whether the last tick moved far enough to show `MOVING`.
#[derive(Clone, Copy, Debug)]
struct Shown {
    moving: bool,
    lifted: bool,
    burning: bool,
    moved_far: Option<bool>,
    /// A mob's: the flags it was announced with, and those set since (on fire, a fuse, a charge).
    mob: Option<[i64; 2]>,
    /// The whole word viewers were last told.
    told: i64,
}

impl Shown {
    fn word(&self) -> i64 {
        let bit = |on: bool, flag: Flags| if on { flag.0 } else { 0 };
        match self.mob {
            Some([announced, set]) => announced | set | bit(self.moving, Flags::MOVING),
            None => Flags::HAS_COLLISION.0 | bit(!self.lifted, Flags::AFFECTED_BY_GRAVITY) | bit(self.moving, Flags::MOVING) | bit(self.burning, Flags::ONFIRE),
        }
    }
}

impl Told {
    /// An entity just announced at `at`; its announced motion does not count.
    pub fn new(at: [f32; 3]) -> Self {
        Self { at, motion: [0.0; 3], flags: None, turned: [0; 2], turn: [0; 2], pitched: None, jumped: false, every_move: false }
    }

    /// The entity now pitches `pitch` degrees, told with its next [`Told::update`] (a projectile's);
    /// the first call is the pitch it was announced with.
    pub fn pitch(&mut self, pitch: f32) {
        let byte = (pitch.rem_euclid(360.0) * 256.0 / 360.0) as u8;
        self.pitched = Some([self.pitched.map_or(byte, |[told, _]| told), byte]);
    }

    /// Viewers are told of each of the entity's moves, not only of those past 0.01.
    pub fn tell_every_move(&mut self) {
        self.every_move = true;
    }

    /// The entity teleported: viewers are told of its next move as forced
    /// (research/captures/bds-ender-findings.md).
    pub fn jump(&mut self) {
        self.jumped = true;
    }

    /// Tells viewers when the entity is moving, lifted and burning, as BDS tells them of items and
    /// falling blocks; `ticked` when its body already made a move, far enough to show, that viewers
    /// were not told of.
    pub fn flag_moving(&mut self, ticked: bool) {
        let shown = Shown { moving: false, lifted: false, burning: false, moved_far: ticked.then_some(true), mob: None, told: 0 };
        self.flags = Some(Shown { told: shown.word(), ..shown });
    }

    /// Tells viewers when the mob, announced with `announced`, is moving; its other flags go
    /// through [`Told::set_flag`] so that each word told has them all.
    pub fn flag_mob(&mut self, announced: Flags) {
        self.flags = Some(Shown { moving: false, lifted: false, burning: false, moved_far: None, mob: Some([announced.0, 0]), told: announced.0 });
    }

    /// Sets or clears `flag` of a mob's: the whole word to tell viewers now, which is then what
    /// they know. `None` for an entity that is no mob [`Told::flag_mob`] was called for.
    pub fn set_flag(&mut self, flag: Flags, on: bool) -> Option<Flags> {
        let shown = self.flags.as_mut()?;
        let [_, set] = shown.mob.as_mut()?;
        *set = if on { *set | flag.0 } else { *set & !flag.0 };
        shown.told = shown.word();
        Some(Flags(shown.told))
    }

    /// The flags of the entity `id` when they changed, after a tick that moved it by `gone`, a
    /// liquid holding it up in place of gravity (`lifted`) and lava `burning` it. On BDS `MOVING`
    /// shows whether the tick before moved it far (docs/research/moving-flag.md); the rest go with
    /// the tick's move (research/captures/bds-water-flags.txt, bds-lava.txt).
    pub fn flags(&mut self, id: u64, gone: [f32; 3], lifted: bool, burning: bool) -> Option<Bytes> {
        let shown = self.flags.as_mut()?;
        let moved_far = gone.iter().map(|axis| axis * axis).sum::<f32>() > MOVING;
        shown.moving = std::mem::replace(&mut shown.moved_far, Some(moved_far)).unwrap_or(shown.moving);
        (shown.lifted, shown.burning) = (lifted, burning);
        if shown.word() == shown.told {
            return None;
        }
        shown.told = shown.word();
        let item = MetadataDictionaryItem { key: Key::Flags, r#type: Type::Long, legacy_type: 7, value: Value::Flags(Flags(shown.told)) };
        Some(sapling_net::encode(&SetEntityData { runtime_entity_id: id, metadata: vec![item], properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 }))
    }

    /// The entity's body and head now face `yaw` and `head`, in degrees; viewers are told with
    /// its next [`Told::update`].
    pub fn face(&mut self, yaw: f32, head: f32) {
        self.turn = [yaw, head].map(|degrees| (degrees.rem_euclid(360.0) * 256.0 / 360.0) as u8);
    }

    /// The packets that bring viewers of the entity `id` to `at` (where the wire puts it) and
    /// `motion`. BDS sends 0 in their tick fields.
    pub fn update(&mut self, id: u64, at: [f32; 3], motion: [f32; 3], on_ground: bool) -> Vec<Bytes> {
        let mut packets = Vec::new();
        let gone = length(std::array::from_fn(|axis| at[axis] - self.at[axis]));
        let jumped = std::mem::take(&mut self.jumped);
        let moved = jumped || gone > MOVED || (length(motion) < STOPPING && gone > SETTLED) || (self.every_move && at != self.at);
        let pitched = self.pitched.filter(|[told, now]| told != now).map(|[_, now]| now);
        if moved || self.turn != self.turned || pitched.is_some() {
            let [x, y, z] = std::array::from_fn(|axis| (moved && at[axis] != self.at[axis]).then_some(at[axis]));
            let [rot_y, rot_z] = std::array::from_fn(|side| (self.turn[side] != self.turned[side]).then_some(self.turn[side]));
            self.turned = self.turn;
            if let (Some(now), Some(pitch)) = (pitched, self.pitched.as_mut()) {
                pitch[0] = now;
            }
            if moved {
                self.at = at;
            }
            let delta = MoveEntityDelta {
                runtime_entity_id: id,
                x,
                y,
                z,
                rot_x: pitched,
                rot_y,
                rot_z,
                on_ground,
                force_move: jumped,
                force_move_local_entity: false,
                force_completion: false,
                ticks: 0,
            };
            packets.push(sapling_net::encode(&delta));
        }
        let change = length(std::array::from_fn(|axis| motion[axis] - self.motion[axis]));
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

    fn axes(packet: &Bytes) -> [bool; 3] {
        let delta: MoveEntityDelta = RawPacket::parse(packet.clone()).unwrap().decode().unwrap();
        [delta.x.is_some(), delta.y.is_some(), delta.z.is_some()]
    }

    #[test]
    fn a_move_counts_all_axes_together_and_sends_each_that_changed() {
        // BDS's orb: no axis past 0.01, together past it, all three sent.
        let mut told = Told::new([6.0, -19.0, 8.0]);
        let sent = told.update(1, [5.99276, -19.00726, 7.99742], [-0.007, -0.007, -0.002], false);
        assert_eq!(axes(&sent[0]), [true, true, true]);
        assert_eq!(axes(&told.update(1, [5.99276, -19.02, 7.99742], [0.0, -0.01, 0.0], false)[0]), [false, true, false]);
    }

    #[test]
    fn an_all_but_stopped_body_is_sent_its_last_small_move() {
        // An item at the bottom of its bob on BDS: 0.007 sent as it turned.
        let mut told = Told::new([0.5, 60.0, 0.5]);
        assert!(told.update(1, [0.5, 59.993, 0.5], [0.0, -0.002, 0.0], false).is_empty());
        assert_eq!(kinds(&told.update(1, [0.5, 59.993, 0.5], [0.0, 0.0005, 0.0], false)), [MoveEntityDelta::ID]);
        assert!(told.update(1, [0.5, 59.9935, 0.5], [0.0, 0.0005, 0.0], false).is_empty());
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
    fn moving_is_flagged_a_tick_after_a_move_of_more_than_0_0316_as_on_bds() {
        let mut told = Told::new([0.0; 3]);
        assert!(told.flags(1, [0.0; 3], false, false).is_none(), "only for kinds that asked");
        told.flag_moving(false);
        // BDS's item 18 rising in water: 0.000991 squared stays off, 0.001001 shows the next tick.
        let moves = [[0.0, 0.2, 0.0], [0.0107, 0.0011, 0.0], [0.00155, 0.03148, 0.0], [0.00155, 0.0316, 0.0], [0.0, 0.0316, 0.0]];
        let sent: Vec<bool> = moves.into_iter().map(|gone| told.flags(1, gone, false, false).is_some()).collect();
        assert_eq!(sent, [false, true, true, false, true]);
        let mut fallen = Told::new([0.0; 3]);
        fallen.flag_moving(true);
        assert!(fallen.flags(1, [0.0; 3], false, false).is_some(), "a falling block is flagged as it is shown");
    }

    fn sent_flags(packet: &Bytes) -> i64 {
        let data: SetEntityData = RawPacket::parse(packet.clone()).unwrap().decode().unwrap();
        match data.metadata[0].value {
            Value::Flags(flags) => flags.0,
            ref other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_mob_shows_moving_a_tick_after_it_walks_and_keeps_the_flags_set_meanwhile() {
        let announced = Flags(Flags::HAS_COLLISION.0 | Flags::WALKER.0);
        let mut told = Told::new([0.0; 3]);
        assert_eq!(told.set_flag(Flags::ONFIRE, true), None, "no mob yet");
        told.flag_mob(announced);
        assert!(told.flags(1, [0.1, 0.0, 0.0], false, false).is_none(), "the tick it walks");
        assert_eq!(sent_flags(&told.flags(1, [0.1, 0.0, 0.0], false, false).unwrap()), announced.0 | Flags::MOVING.0);
        assert_eq!(told.set_flag(Flags::ONFIRE, true), Some(Flags(announced.0 | Flags::MOVING.0 | Flags::ONFIRE.0)));
        assert!(told.flags(1, [0.0; 3], false, false).is_none(), "still moving: it moved last tick");
        assert_eq!(sent_flags(&told.flags(1, [0.0; 3], false, false).unwrap()), announced.0 | Flags::ONFIRE.0);
    }

    #[test]
    fn a_lifted_item_loses_its_gravity_and_one_in_lava_burns_as_on_bds() {
        let mut told = Told::new([0.0; 3]);
        told.flag_moving(true);
        // BDS's item into water, then lava: 281492156579840 and 281492156579841.
        assert_eq!(sent_flags(&told.flags(1, [0.0, -0.1, 0.0], true, false).unwrap()), 281_492_156_579_840);
        assert_eq!(sent_flags(&told.flags(1, [0.0, -0.1, 0.0], true, true).unwrap()), 281_492_156_579_841);
        assert!(told.flags(1, [0.0, -0.1, 0.0], true, true).is_none());
    }
}
