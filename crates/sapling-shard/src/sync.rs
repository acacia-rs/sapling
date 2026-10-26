//! Keeping viewers' copies of an entity the server moves where it is, as BDS's generic actor sync
//! does: a MoveEntityDelta of the axes that changed once it moved more than 0.01, or more than
//! 0.001 when it has all but stopped, and a SetEntityMotion when its motion changed by more than
//! 0.2 since the last one, or stopped (docs/research/items.md 5; matched to an item, an orb and an
//! armour stand in research/captures/bds-water.txt).

use acacia_proto::packets::{MoveEntityDelta, SetEntityData, SetEntityMotion};
use acacia_proto::types::{EntityProperties, MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataFlags1 as Flags, Vec3f};
use bevy_ecs::prelude::{Component, Entity};
use bytes::Bytes;

pub(crate) mod pace;

use pace::{Limits, Paced};

const MOVED: f32 = 0.01;
/// Degrees of turn, summed over yaw, head yaw and pitch, past which viewers are told
/// (research/bds-actor-move-sync.md).
const TURNED: f32 = 4.0;
const SETTLED: f32 = 0.001;
const STOPPING: f32 = 0.001;
const NEW_MOTION: f32 = 0.2;
/// A tick's move, squared, past which the next tick shows `MOVING` (docs/research/moving-flag.md).
const MOVING: f32 = 0.001;

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
    /// Its pitch as viewers know it and as it is now; `None` for an entity whose pitch is not told.
    pitched: Option<[u8; 2]>,
    /// Its next move is a teleport, which BDS sends forced.
    jumped: bool,
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
        Self { at, motion: [0.0; 3], flags: None, turned: [0; 2], turn: [0; 2], pitched: None, jumped: false, every_move: false, pending: None, paced: Paced::default(), grounded: None }
    }

    /// This tick's move, not yet handed to viewers.
    pub fn pending(&self) -> Option<&Pending> {
        self.pending.as_ref()
    }

    /// Whether `viewer`, its eye at `eye`, is told of the pending move at `now`, and with how many
    /// `ticks`, by the entity's `limits`.
    pub fn told_to(&mut self, viewer: Entity, eye: [f32; 3], now: u64, limits: Option<Limits>) -> Option<u64> {
        let pending = self.pending.as_ref()?;
        let dropped = if pending.at_once { 0 } else { pace::dropped(limits, eye, self.at) };
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

    /// [`Told::set_flag`], `None` too where viewers already knew `flag` to be so.
    pub fn change_flag(&mut self, flag: Flags, on: bool) -> Option<Flags> {
        let told = self.flags?.told;
        if (told & flag.0 != 0) == on {
            return None;
        }
        self.set_flag(flag, on)
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
        self.pending = None;
        let gone = length(std::array::from_fn(|axis| at[axis] - self.at[axis]));
        let jumped = std::mem::take(&mut self.jumped);
        let moved = jumped || gone > MOVED || (length(motion) < STOPPING && gone > SETTLED) || (self.every_move && at != self.at);
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
                force_move_local_entity: false,
                force_completion: false,
                ticks: 0,
            };
            let bytes = sapling_net::encode(&delta);
            packets.push(bytes.clone());
            let at_once = jumped || self.grounded != Some(on_ground);
            self.pending = Some(Pending { delta, bytes, at_once, before });
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
