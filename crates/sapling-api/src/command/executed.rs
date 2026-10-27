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
}

// Positions are never NaN: they come from parsed finite numbers and the world.
impl Eq for Executed {}

impl Executed {
    pub fn new(by: Sender, executor: Option<Selected>, position: [f32; 3], rotation: [f32; 2], dimension: i32) -> Self {
        Self { by, executor, position, rotation, anchor: 0.0, dimension }
    }
}
