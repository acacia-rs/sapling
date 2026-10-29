//! A villager's day, as its pack's `minecraft:scheduler` groups set it by `hourly_clock_time` (the
//! time of day): `work_schedule` (and the farmer's, fisher's and librarian's, the same hours) for
//! one with a profession, `basic_schedule` for an unskilled one, `jobless_schedule` for a nitwit,
//! `child_schedule` for a baby. Work is jobs.rs; gathering walks to the village's middle
//! (`behavior.mingle`), home to its bed (`move_towards_dwelling_restriction`), bed time sleeps on
//! it (beds.rs); wandering and play are strolling.
// Every tick: on BDS a villager by its bed lay down the tick after 12000 and all three sleepers
// rose the tick 24000 came, whatever the pack's `max_delay_secs` (research/captures/bds-village.txt).
// TODO: mingling villagers do not pair up.

use bevy_ecs::prelude::*;

use super::jobs::{NITWIT, UNSKILLED};
use super::{Trader, beds};
use crate::body::Body;
use crate::components::Layer;

const DAY: i64 = 24_000;
/// `minecraft:movement` 0.5 times each goal's `speed_multiplier`: mingle 0.5, dwelling 0.6.
const MINGLES: f32 = 0.25;
const GOES_HOME: f32 = 0.3;
/// The middle is reached this near (`mingle_distance` 2), home this near its bed.
const MINGLE_DISTANCE: f32 = 2.0;
const AT_HOME: f32 = 2.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Activity {
    Work,
    Gather,
    Wander,
    Home,
    Bed,
    Play,
}

/// What a villager of `profession`, a baby if `baby`, does at `time`.
pub(super) fn activity(profession: i32, baby: bool, time: i64) -> Activity {
    use Activity::*;
    let hour = time.rem_euclid(DAY);
    let table: &[(i64, Activity)] = match (baby, profession) {
        (true, _) => &[(11_000, Play), (12_000, Home), (DAY, Bed)],
        (false, NITWIT) => &[(2_000, Bed), (13_000, Wander), (14_000, Home), (DAY, Bed)],
        (false, UNSKILLED) => &[(8_000, Wander), (10_000, Gather), (11_000, Wander), (12_000, Home), (DAY, Bed)],
        _ => &[(8_000, Work), (10_000, Gather), (11_000, Work), (12_000, Home), (DAY, Bed)],
    };
    table.iter().find(|(until, _)| hour < *until).map_or(Bed, |(_, activity)| *activity)
}

/// What the villager does now; nothing for a wandering trader.
pub(super) fn now(world: &World, entity: Entity, layer: Entity) -> Option<Activity> {
    let trader = world.get::<Trader>(entity).filter(|trader| trader.leaves_at.is_none())?;
    let time = world.get::<Layer>(layer)?.world.level().time;
    Some(activity(trader.profession, super::super::shown::is_baby(world, entity), time))
}

/// Before its move, for what the schedule has it do but work: false where it strolls.
pub(super) fn steer(world: &mut World, entity: Entity, layer: Entity, activity: Activity) -> bool {
    let Some(body) = world.get::<Body>(entity).copied() else { return false };
    match activity {
        Activity::Gather => {
            let Some(middle) = super::super::raids::village_centre(world, layer, body.at) else { return false };
            if (middle[0] - body.at[0]).hypot(middle[2] - body.at[2]) <= MINGLE_DISTANCE {
                super::super::legs::rest(world, entity);
            } else {
                super::super::hunting::approach(world, entity, layer, &body, middle, MINGLES, MINGLE_DISTANCE);
            }
            true
        }
        Activity::Home => beds::go_to(world, entity, layer, &body, GOES_HOME, AT_HOME),
        Activity::Bed => beds::sleep(world, entity, layer, &body),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_packs_hours() {
        let at = |profession, baby| [0, 7_999, 8_000, 10_500, 11_500, 12_000, 13_500, 23_999].map(|time| activity(profession, baby, time));
        use Activity::*;
        assert_eq!(at(5, false), [Work, Work, Gather, Work, Home, Bed, Bed, Bed]);
        assert_eq!(at(UNSKILLED, false), [Wander, Wander, Gather, Wander, Home, Bed, Bed, Bed]);
        assert_eq!(at(NITWIT, false), [Bed, Wander, Wander, Wander, Wander, Wander, Home, Bed]);
        assert_eq!(at(5, true), [Play, Play, Play, Play, Home, Bed, Bed, Bed]);
    }
}
