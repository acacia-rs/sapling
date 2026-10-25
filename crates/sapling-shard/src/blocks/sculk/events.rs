//! The game events sculk hears, and the frequency of each: BDS's table at 0x14a7445a8
//! (docs/research/sculk.md, S1).

/// Something that happened that sends out a vibration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GameEvent {
    /// BDS's `entity_move`: a step.
    Step,
    /// Something stepped on a sensor.
    SculkTouch,
    ProjectileLand,
    ProjectileShoot,
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
            GameEvent::ContainerClose | GameEvent::BlockClose => 9,
            GameEvent::ContainerOpen | GameEvent::BlockOpen | GameEvent::DispenseFail | GameEvent::NotePlay | GameEvent::PrimeFuse => 10,
            GameEvent::BlockDestroy | GameEvent::FluidPickup => 12,
            GameEvent::BlockPlace | GameEvent::FluidPlace => 13,
            GameEvent::LightningStrike => 14,
            GameEvent::Explode => 15,
            GameEvent::TendrilsClicking => 0,
        }
    }
}
