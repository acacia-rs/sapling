//! The game events sculk hears, and the frequency of each: BDS's table at 0x14a7445a8
//! (docs/research/sculk.md, S1), and the particle a vibration flies as.

use acacia_proto::nbt::{Nbt, Value};
use acacia_proto::packets::LevelEventGeneric;
use sapling_api::BlockPos;

/// Something that happened that sends out a vibration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GameEvent {
    /// BDS's `entity_move`: a step.
    Step,
    /// Something stepped on a sensor.
    SculkTouch,
    ProjectileLand,
    ProjectileShoot,
    /// A food eaten or a drink drunk (BDS's `eat` and `drink`, both 8), heard from the eyes as it was
    /// finished (research/captures/bds-v2tp.txt, bds-x2drink.txt).
    Eat,
    ContainerClose,
    BlockClose,
    ContainerOpen,
    BlockOpen,
    DispenseFail,
    NotePlay,
    PrimeFuse,
    BlockDestroy,
    FluidPickup,
    BlockPlace,
    FluidPlace,
    LightningStrike,
    /// An entity put elsewhere by `/tp`, heard where it arrives (research/captures/bds-v2tp.txt).
    Teleport,
    Explode,
    /// A sensor went active: what shriekers hear, and nothing else does.
    TendrilsClicking,
}

impl GameEvent {
    /// What a comparator reads off a sensor that heard it; 0 for what no sensor hears.
    pub(crate) fn frequency(self) -> u8 {
        match self {
            GameEvent::Step | GameEvent::SculkTouch => 1,
            GameEvent::ProjectileLand => 2,
            GameEvent::ProjectileShoot => 3,
            GameEvent::Eat => 8,
            GameEvent::ContainerClose | GameEvent::BlockClose => 9,
            GameEvent::ContainerOpen | GameEvent::BlockOpen | GameEvent::DispenseFail | GameEvent::NotePlay | GameEvent::PrimeFuse => 10,
            GameEvent::BlockDestroy | GameEvent::FluidPickup => 12,
            GameEvent::BlockPlace | GameEvent::FluidPlace => 13,
            GameEvent::LightningStrike | GameEvent::Teleport => 14,
            GameEvent::Explode => 15,
            GameEvent::TendrilsClicking => 0,
        }
    }
}

/// `ParticlesVibrationSignal` (2027, sent zigzagged), a vibration flying from the middle of `from`
/// to the listener at `to` in `ticks` ticks (S21; bds-z2bolt.txt).
pub(super) fn signal(from: BlockPos, to: BlockPos, ticks: u64) -> bytes::Bytes {
    let point = |pos: BlockPos| {
        let [x, y, z] = pos.map(|c| Value::Float(c as f32 + 0.5));
        Value::Compound(vec![("type".into(), Value::String("vec3".into())), ("x".into(), x), ("y".into(), y), ("z".into(), z)])
    };
    let field = |name: &str, value| Nbt { name: name.into(), value };
    let nbt = vec![field("origin", point(from)), field("speed", Value::Float(20.0)), field("target", point(to)), field("timeToLive", Value::Float(ticks as f32 / 20.0))];
    sapling_net::encode(&LevelEventGeneric { event_id: 2027 << 1, nbt })
}
