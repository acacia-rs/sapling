//! Whether one candidate passes a selector's filters, all but `c`.

use super::filters::{Bounds, Filters};
use super::items::Inventory;
use super::Selected;
use crate::GameMode;

/// A player or entity as the filters see it. What no filter asks for may be left empty.
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    pub target: Selected,
    /// Namespaced: `minecraft:player` for a player.
    pub kind: String,
    /// A player's name; an entity's name tag, empty without one.
    pub name: String,
    pub feet: [f32; 3],
    /// Yaw and pitch, in degrees.
    pub rotation: [f32; 2],
    pub families: Vec<String>,
    pub tags: Vec<String>,
    /// By objective.
    pub scores: Vec<(String, i32)>,
    /// `None` for anything but a player, as is `level`.
    pub mode: Option<GameMode>,
    pub level: Option<i32>,
    /// `None` for what carries nothing sapling can look into.
    pub inventory: Option<Inventory>,
}

impl Candidate {
    pub fn is_player(&self) -> bool {
        matches!(self.target, Selected::Player(_))
    }

    pub(super) fn distance(&self, from: [f32; 3]) -> f32 {
        (0..3).map(|axis| (self.feet[axis] - from[axis]).powi(2)).sum::<f32>().sqrt()
    }
}

/// Whether `candidate` passes `filters`, distances and volumes counted from `origin`.
pub(super) fn passes(filters: &Filters, candidate: &Candidate, origin: [f32; 3]) -> bool {
    let all = |tests: &[super::Not<String>], holds: &dyn Fn(&str) -> bool| tests.iter().all(|test| test.passes(holds(&test.value)));
    let tagged = |tag: &str| if tag.is_empty() { candidate.tags.is_empty() } else { candidate.tags.iter().any(|held| held == tag) };
    let player_only = |asked: bool, holds: &dyn Fn() -> bool| !asked || (candidate.is_player() && holds());
    // `default` stands for the world's game mode, survival until sapling keeps another.
    let mode = || filters.modes.iter().all(|test| test.passes(candidate.mode == Some(test.value.unwrap_or(GameMode::Survival))));
    filters.distance.holds(candidate.distance(origin))
        && in_volume(filters.volume, origin, candidate.feet)
        && all(&filters.names, &|name| candidate.name.eq_ignore_ascii_case(name))
        && all(&filters.types, &|kind| candidate.kind.eq_ignore_ascii_case(kind))
        && all(&filters.families, &|family| candidate.families.iter().any(|held| held == family))
        && all(&filters.tags, &tagged)
        && player_only(!filters.modes.is_empty(), &mode)
        && player_only(!filters.level.is_open(), &|| candidate.level.is_some_and(|level| filters.level.holds(level)))
        // TODO: no `/inputpermission` yet, so every input is enabled.
        && player_only(!filters.permissions.is_empty(), &|| filters.permissions.iter().all(|(_, enabled)| *enabled))
        && filters.pitch.holds(candidate.rotation[1])
        && turned(filters.yaw, candidate.rotation[0])
        // Without a score a test fails, `!` or not (research/captures/bds-scoreboard.txt).
        && filters.scores.iter().all(|(objective, test)| candidate.scores.iter().any(|(held, score)| held == objective && test.passes(test.value.holds(*score))))
        && filters.items.iter().all(|test| test.holds(candidate.inventory.as_ref()))
}

/// Whether `feet` is in the box `volume` spans from `origin`, whole blocks of it (none: no box).
fn in_volume(volume: [Option<f32>; 3], origin: [f32; 3], feet: [f32; 3]) -> bool {
    if volume.iter().all(Option::is_none) {
        return true;
    }
    (0..3).all(|axis| {
        let far = origin[axis] + volume[axis].unwrap_or(0.0);
        let (low, high) = (origin[axis].min(far).floor(), origin[axis].max(far).floor() + 1.0);
        (low..high).contains(&feet[axis])
    })
}

/// Whether `yaw` is within `bounds`, which go the short way round past 180 when `min` is more
/// than `max` (`rym=170,ry=-170`).
fn turned(bounds: Bounds<f32>, yaw: f32) -> bool {
    let yaw = (yaw + 180.0).rem_euclid(360.0) - 180.0;
    match (bounds.min, bounds.max) {
        (Some(min), Some(max)) if min > max => yaw >= min || yaw <= max,
        _ => bounds.holds(yaw),
    }
}
