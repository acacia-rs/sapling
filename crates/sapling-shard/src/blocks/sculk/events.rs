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
    /// BDS's type number for it, which a listener's block actor keeps (sculk.md's table).
    pub(crate) fn type_id(self) -> i32 {
        match self {
            GameEvent::BlockClose => 4,
            GameEvent::BlockDestroy => 6,
            GameEvent::BlockOpen => 8,
            GameEvent::BlockPlace => 9,
            GameEvent::ContainerClose => 11,
            GameEvent::ContainerOpen => 12,
            GameEvent::DispenseFail => 13,
            GameEvent::Step => 22,
            GameEvent::Explode => 27,
            GameEvent::FluidPickup => 29,
            GameEvent::FluidPlace => 30,
            GameEvent::LightningStrike => 34,
            GameEvent::NotePlay => 36,
            GameEvent::PrimeFuse => 39,
            GameEvent::ProjectileLand => 40,
            GameEvent::ProjectileShoot => 41,
            GameEvent::SculkTouch => 57,
            GameEvent::TendrilsClicking => 58,
        }
    }

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
