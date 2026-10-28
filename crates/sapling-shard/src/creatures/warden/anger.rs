//! A warden's anger at each of its suspects, by its pack's `minecraft:anger_level` and
//! `minecraft:heartbeat` as BDS told them (research/captures/bds-warden-findings.md, "Anger").

use bevy_ecs::prelude::Entity;

/// What a disturbance, a projectile's and a hit add, the most it holds, and where it is angry.
pub(super) const ANNOYS: u32 = 35;
pub(super) const PROJECTILE_ANNOYS: u32 = 10;
pub(super) const HIT: u32 = ANGRY + 20;
const MOST: u32 = 150;
pub(super) const ANGRY: u32 = 80;
/// Heard `listening_angry` and `agitated` from this much.
const AGITATED: u32 = 40;
/// Each suspect's anger falls by one every this many ticks.
pub(super) const COOLS: u64 = 20;

#[derive(Default)]
pub(super) struct Anger {
    suspects: Vec<(Entity, u32)>,
}

impl Anger {
    /// Adds `by` to its anger at `suspect`; the sound it is heard with.
    pub(super) fn raise(&mut self, suspect: Entity, by: u32) -> &'static str {
        let anger = match self.suspects.iter_mut().find(|(entity, _)| *entity == suspect) {
            Some((_, anger)) => anger,
            None => {
                self.suspects.push((suspect, 0));
                &mut self.suspects.last_mut().expect("pushed above").1
            }
        };
        *anger = (*anger + by).min(MOST);
        if *anger >= AGITATED { "listening_angry" } else { "listening" }
    }

    /// A second of calm: each anger falls by one, and the suspects `keep` refuses are forgotten.
    pub(super) fn cool(&mut self, keep: impl Fn(Entity) -> bool) {
        for (_, anger) in &mut self.suspects {
            *anger = anger.saturating_sub(1);
        }
        self.suspects.retain(|(suspect, anger)| *anger > 0 && keep(*suspect));
    }

    /// The suspect it is angriest at, and how angry.
    pub(super) fn top(&self) -> Option<(Entity, u32)> {
        self.suspects.iter().copied().max_by_key(|(_, anger)| *anger)
    }

    pub(super) fn at(&self, suspect: Entity) -> u32 {
        self.suspects.iter().find(|(entity, _)| *entity == suspect).map_or(0, |(_, anger)| *anger)
    }

    pub(super) fn level(&self) -> u32 {
        self.top().map_or(0, |(_, anger)| anger)
    }
}

/// `HeartbeatIntervalTicks` at `anger`: `2.0 - clamp(anger / 80 * 1.5, 0, 1.5)` seconds.
pub(super) fn heartbeat(anger: u32) -> i32 {
    ((2.0 - (anger as f32 / ANGRY as f32 * 1.5).clamp(0.0, 1.5)) * 20.0).round() as i32
}

/// `AmbientSoundEventName` at `anger`.
pub(super) fn ambient(anger: u32) -> &'static str {
    match anger {
        ANGRY.. => "angry",
        AGITATED.. => "agitated",
        _ => "ambient",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_disturbances_make_it_angry_as_bds_heard_them() {
        let suspect = Entity::from_raw(7);
        let mut anger = Anger::default();
        assert_eq!((anger.raise(suspect, ANNOYS), heartbeat(anger.level())), ("listening", 27));
        anger.cool(|_| true);
        assert_eq!((anger.raise(suspect, ANNOYS), heartbeat(anger.level()), ambient(anger.level())), ("listening_angry", 14, "agitated"));
        anger.cool(|_| true);
        anger.raise(suspect, ANNOYS);
        assert_eq!((anger.level(), heartbeat(anger.level()), ambient(anger.level())), (103, 10, "angry"));
        assert_eq!(heartbeat(0), 40);
    }

    #[test]
    fn anger_is_capped_and_cools_away() {
        let (one, two) = (Entity::from_raw(1), Entity::from_raw(2));
        let mut anger = Anger::default();
        for _ in 0..5 {
            anger.raise(one, HIT);
        }
        anger.raise(two, 1);
        assert_eq!(anger.top(), Some((one, MOST)));
        anger.cool(|_| true);
        assert_eq!((anger.at(one), anger.at(two)), (MOST - 1, 0));
        anger.cool(|suspect| suspect != one);
        assert_eq!(anger.top(), None);
    }
}
