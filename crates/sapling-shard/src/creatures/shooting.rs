//! A hunter that shoots: when its shots go and what they are (`arrows`, `bullets`).

use bevy_ecs::prelude::*;
use sapling_data::Hunts;

use super::hunting::Hunter;
use super::{arrows, bullets, splashes};
use crate::body::Body;
use crate::components::Tick;
use crate::late_sounds;

/// The ticks after a shot at which a crossbow's loading is heard, and what of it.
// TODO: heard where the archer stood at the shot, not where it has walked to since.
const LOADS: [(u64, &str); 3] = [(3, "start"), (13, "middle"), (23, "end")];
/// Its crossbow is heard this much of its height up.
const MIDDLE: f32 = 0.5;

/// After its move: a hunter that shoots, `every` ticks, at its `prey` looses what is due.
pub(super) fn shoot(world: &mut World, entity: Entity, layer: Entity, kind: &str, body: &Body, hunts: Hunts, every: u32, prey: Entity) -> Option<()> {
    let now = world.resource::<Tick>().0;
    let feet = super::hunting::feet_of(world, prey)?;
    let ready = world.get::<Hunter>(entity)?.ready;
    let fire = |world: &mut World| match hunts.shot {
        Some(bullets::NAME) => bullets::fire(world, layer, entity, kind, body, prey),
        _ => {
            let mut shot = arrows::shot_named(hunts.shot);
            if shot.is_potion() {
                shot = shot.of_potion(splashes::choose(world, prey, (feet[0] - body.at[0]).hypot(feet[2] - body.at[2])));
            }
            arrows::loose(world, layer, entity, kind, body, feet, hunts.gives, shot);
        }
    };
    let volley = world.get::<Hunter>(entity)?.volley;
    if let Some((left, _)) = volley.filter(|(_, due)| now >= *due) {
        fire(world);
        let gap = hunts.volley.map_or(0, |(_, gap)| u64::from(gap));
        world.get_mut::<Hunter>(entity)?.volley = (left > 1).then_some((left - 1, now + gap));
    }
    if ready != 0 && now >= ready {
        fire(world);
        let more = hunts.volley.filter(|(shots, _)| *shots > 1).map(|(shots, gap)| (shots - 1, now + u64::from(gap)));
        world.get_mut::<Hunter>(entity)?.volley = more;
    }
    if ready == 0 || now >= ready {
        // The first shot waits as long as the next, or as long as the kind aims.
        let wait = if ready == 0 { hunts.aims.unwrap_or(every) } else { every };
        world.get_mut::<Hunter>(entity)?.ready = now + u64::from(wait);
        // A crossbow is heard being loaded for the next (bds-shoot2.txt).
        let middle = [body.at[0], body.at[1] + MIDDLE * body.size[1], body.at[2]];
        for (after, stage) in LOADS.into_iter().filter(|_| hunts.closes) {
            late_sounds::play_later(world, after, layer, body.at.map(|c| c.floor() as i32), arrows::sound(&format!("crossbow.loading.{stage}"), middle, ""));
        }
    }
    Some(())
}
