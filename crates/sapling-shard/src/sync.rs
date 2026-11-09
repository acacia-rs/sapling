//! Keeping viewers' copies of an entity the server moves where it is, as BDS's generic actor sync
//! does: a MoveEntityDelta of the axes that changed once it moved more than 0.01, or more than
//! 0.001 when it has all but stopped, and a SetEntityMotion when its motion changed by more than
//! 0.2 since the last one, or stopped (docs/research/items.md 5; matched to an item, an orb and an
//! armour stand in research/captures/bds-water.txt).

use acacia_proto::packets::{MoveEntityDelta, SetEntityMotion};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::{Component, Entity};
use bytes::Bytes;

mod flags;
pub(crate) mod pace;

pub(crate) use flags::{flags_item, flags_packet};
use flags::Shown;
use pace::{Limits, Paced};

const MOVED: f32 = 0.01;
/// Degrees of turn, summed over yaw, head yaw and pitch, past which viewers are told
/// (research/bds-actor-move-sync.md).
const TURNED: f32 = 4.0;
const SETTLED: f32 = 0.001;
const STOPPING: f32 = 0.001;
const NEW_MOTION: f32 = 0.2;

pub(crate) use sapling_data::angle_byte;

fn length(v: [f32; 3]) -> f32 {
    v.iter().map(|axis| axis * axis).sum::<f32>().sqrt()
}

/// What viewers were last told of an entity.
#[derive(Component, Clone, Debug)]
pub(crate) struct Told {
    at: [f32; 3],
    motion: [f32; 3],
    flags: Option<Shown>,
    /// Its body's and its head's turn as viewers know them, and as they are now.
    turned: [u8; 2],
    turn: [u8; 2],
    /// Its head comes round to its body after the next update (a teleport's turn).
    head_follows: bool,
    /// Its pitch as viewers know it and as it is now; `None` for an entity whose pitch is not told.
    pitched: Option<[u8; 2]>,
    /// Its next move is a teleport, which BDS sends forced.
    jumped: bool,
    /// Where a piston left it this tick, and whether on the ground: BDS tells that move at once,
    /// with `force_move_local_entity` and paced as any, before the body's own move of the tick
    /// (research/captures/bds-u3carry.txt, bds-ffshove.txt).
    pistoned: Option<([f32; 3], bool)>,
    /// Viewers are told of every move, however small (an eye of ender's, bds-eye-throw.txt).
    every_move: bool,
    /// This tick's move until it is handed to viewers, each as often as pace.rs lets it.
    pending: Option<Pending>,
    paced: Paced,
    /// Whether the last move told stood on the ground; none was told yet.
    grounded: Option<bool>,
}

/// A move not yet handed to viewers, and what they knew before it.
#[derive(Clone, Debug)]
pub(crate) struct Pending {
    pub delta: MoveEntityDelta,
    pub bytes: Bytes,
    /// It jumps, lands or teleports: every viewer is told at once.
    at_once: bool,
    before: Before,
}

#[derive(Clone, Copy, Debug)]
struct Before {
    at: [f32; 3],
    turned: [u8; 2],
    pitched: Option<[u8; 2]>,
}

impl Told {
    /// An entity just announced at `at`; its announced motion does not count.
    pub fn new(at: [f32; 3]) -> Self {
        Self { at, motion: [0.0; 3], flags: None, turned: [0; 2], turn: [0; 2], head_follows: false, pitched: None, jumped: false, pistoned: None, every_move: false, pending: None, paced: Paced::default(), grounded: None }
    }

    /// This tick's move, not yet handed to viewers.
    pub fn pending(&self) -> Option<&Pending> {
        self.pending.as_ref()
    }

    /// Whether `viewer`, its eye at `eye`, is told of the pending move at `now`, and with how many
    /// `ticks`, by the entity's `limits`.
    pub fn told_to(&mut self, viewer: Entity, eye: [f32; 3], now: u64, limits: Option<Limits>) -> Option<u64> {
        let pending = self.pending.as_ref()?;
        let moving = [pending.delta.x, pending.delta.y, pending.delta.z].iter().any(Option::is_some);
        let dropped = if pending.at_once { 0 } else { pace::dropped(limits, eye, self.at, moving) };
        self.paced.due(viewer, now, dropped).then_some(dropped)
    }

    /// The pending move was handed out, to some viewer or `none`; told to none, BDS compares the
    /// next tick's with what they knew before it.
    pub fn settle(&mut self, none: bool) {
        let Some(pending) = self.pending.take() else { return };
        if none {
            (self.at, self.turned, self.pitched) = (pending.before.at, pending.before.turned, pending.before.pitched);
        } else {
            self.grounded = Some(pending.delta.on_ground);
        }
    }

    /// The entity now pitches `pitch` degrees, told with its next [`Told::update`] (a projectile's);
    /// the first call is the pitch it was announced with.
    pub fn pitch(&mut self, pitch: f32) {
        let byte = angle_byte(pitch);
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

    /// A piston left the entity at `at` (its feet), `on_ground` or not: told by
    /// [`Told::update_pistoned`], or else as its next move.
    pub fn pistoned(&mut self, at: [f32; 3], on_ground: bool) {
        self.pistoned = Some((at, on_ground));
    }

    /// The packets that tell viewers of the entity `id` where a piston left it, its wire `lift`
    /// above its feet; its motion is left to its next [`Told::update`].
    pub fn update_pistoned(&mut self, id: u64, lift: f32) -> Vec<Bytes> {
        let Some(([x, y, z], on_ground)) = self.pistoned.take() else { return Vec::new() };
        self.tell(id, [x, y + lift, z], self.motion, on_ground, true)
    }

    /// The entity's body and head now face `yaw` and `head`, in degrees; viewers are told with its next [`Told::update`].
    pub fn face(&mut self, yaw: f32, head: f32) {
        self.turn = [yaw, head].map(angle_byte);
    }

    /// Its body alone now faces `yaw`, as a minecart turns along its rail (bds-u3yaw.txt).
    pub fn face_body(&mut self, yaw: f32) { self.turn[0] = angle_byte(yaw); }

    /// Viewers knew it turned `yaw` and `head` at its announcement.
    pub fn announced_facing(&mut self, yaw: f32, head: f32) {
        self.face(yaw, head);
        self.turned = self.turn;
    }

    /// Its body now faces `yaw`: viewers are told with its next [`Told::update`] and of its head
    /// following with the one after, as BDS told teleported armour stands (bds-g8rot.txt).
    pub fn turn_body(&mut self, yaw: f32) {
        self.turn[0] = angle_byte(yaw);
        self.head_follows = true;
    }

    /// Its body's yaw and its pitch as they are now, in degrees from -180 to 180, to the
    /// 1.4 degrees viewers are told them in.
    pub fn facing(&self) -> [f32; 2] {
        let degrees = |byte: u8| (f32::from(byte) * 360.0 / 256.0 + 180.0).rem_euclid(360.0) - 180.0;
        [degrees(self.turn[0]), self.pitched.map_or(0.0, |[_, now]| degrees(now))]
    }

    /// The packets that bring viewers of the entity `id` to `at` (where the wire puts it) and
    /// `motion`. BDS sends 0 in their tick fields.
    pub fn update(&mut self, id: u64, at: [f32; 3], motion: [f32; 3], on_ground: bool) -> Vec<Bytes> {
        let pistoned = self.pistoned.take().is_some();
        self.tell(id, at, motion, on_ground, pistoned)
    }

    /// [`Told::update`]; a piston's move is told however small (bds-ffshove.txt: a cow's 0.0024
    /// drift went with its lift).
    fn tell(&mut self, id: u64, at: [f32; 3], motion: [f32; 3], on_ground: bool, pistoned: bool) -> Vec<Bytes> {
        let mut packets = Vec::new();
        self.pending = None;
        let gone = length(std::array::from_fn(|axis| at[axis] - self.at[axis]));
        let jumped = std::mem::take(&mut self.jumped);
        let moved = jumped || pistoned || gone > MOVED || (length(motion) < STOPPING && gone > SETTLED) || (self.every_move && at != self.at);
        let pitched = self.pitched.filter(|[told, now]| told != now).map(|[_, now]| now);
        let degrees = |[told, now]: [u8; 2]| (f32::from(now) - f32::from(told)).abs() * 360.0 / 256.0;
        let turned = degrees([self.turned[0], self.turn[0]]) + degrees([self.turned[1], self.turn[1]]) + self.pitched.map_or(0.0, degrees);
        if moved || turned > TURNED {
            let before = Before { at: self.at, turned: self.turned, pitched: self.pitched };
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
                force_move_local_entity: pistoned,
                force_completion: false,
                ticks: 0,
            };
            let bytes = sapling_net::encode(&delta);
            packets.push(bytes.clone());
            let at_once = jumped || self.grounded != Some(on_ground);
            self.pending = Some(Pending { delta, bytes, at_once, before });
        }
        if std::mem::take(&mut self.head_follows) {
            self.turn[1] = self.turn[0];
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
mod tests;
