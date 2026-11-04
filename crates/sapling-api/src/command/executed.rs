use super::{Selected, Sender};

/// A command `/execute` runs: for whom (`as`), from where (`at`, `positioned`, `align`), turned
/// how (`rotated`, `facing`), anchored where and in which dimension (`in`).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Executed {
    /// Who sent the `/execute`.
    pub by: Sender,
    /// Whom `@s` names; `None` where nothing is (the console's own `/execute`).
    pub executor: Option<Selected>,
    /// Where `~` and a selector's distances count from.
    pub position: [f32; 3],
    /// Yaw and pitch in degrees: where `^` points.
    pub rotation: [f32; 2],
    /// How far above `position` `^` counts from: the executor's eye height after `anchored eyes`.
    pub anchor: f32,
    /// 0 overworld, 1 nether, 2 end.
    pub dimension: i32,
    /// Whom `@initiator` names: the player who pressed the NPC dialogue button running it.
    pub initiator: Option<crate::Player>,
    /// What a command calls its sender where no executor names one: a command block's `!§r` or
    /// custom name, as BDS's `/say` from a block said `[!§r] hi`.
    pub name: Option<String>,
}

// Positions are never NaN: they come from parsed finite numbers and the world.
impl Eq for Executed {}

impl Executed {
    pub fn new(by: Sender, executor: Option<Selected>, position: [f32; 3], rotation: [f32; 2], dimension: i32) -> Self {
        Self { by, executor, position, rotation, anchor: 0.0, dimension, initiator: None, name: None }
    }

    pub fn with_name(self, name: impl Into<String>) -> Self {
        Self { name: Some(name.into()), ..self }
    }

    pub fn with_initiator(self, initiator: crate::Player) -> Self {
        Self { initiator: Some(initiator), ..self }
    }
}

impl Sender {
    /// The player `@initiator` names, from the innermost `/execute` back to the dialogue button.
    pub fn initiator(&self) -> Option<&crate::Player> {
        match self {
            Self::Executed(executed) => executed.initiator.as_ref().or_else(|| executed.by.initiator()),
            _ => None,
        }
    }
}
