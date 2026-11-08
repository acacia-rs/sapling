//! A hunter that shoots: when its shots go and what they are (`arrows`, `bullets`).

use bevy_ecs::prelude::*;
use sapling_data::Hunts;

use super::hunting::Hunter;
use super::{arrows, bullets, splashes};
use crate::body::Body;
use crate::components::{NetId, Tick};
use crate::{entities, late_sounds};
use acacia_proto::types::MetadataFlags1;

/// The ticks after a shot at which a crossbow's loading is heard, and what of it: from where the
/// archer is by then (bds-k2shot.txt: a walking pillager's three each at its place of that tick).
const LOADS: [(u64, &str); 3] = [(3, "start"), (13, "middle"), (23, "end")];

/// Crossbow loadings due: when, the archer, its world and the stage.
#[derive(Resource, Default)]
struct Loads(Vec<(u64, Entity, Entity, &'static str)>);

/// Plays the loadings whose time has come from their archers' middles; a gone archer's are dropped.
pub(crate) fn play_loads(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let Some(mut loads) = world.get_resource_mut::<Loads>() else { return };
    let (due, waiting): (Vec<_>, Vec<_>) = std::mem::take(&mut loads.0).into_iter().partition(|(when, ..)| *when <= now);
    loads.0 = waiting;
    for (_, archer, layer, stage) in due {
        let Some(body) = world.get::<Body>(archer).copied() else { continue };
        let middle = [body.at[0], body.at[1] + MIDDLE * body.size[1], body.at[2]];
        late_sounds::play_later(world, 0, layer, body.at.map(|c| c.floor() as i32), arrows::sound(&format!("crossbow.loading.{stage}"), middle, ""));
    }
}
/// Its crossbow is heard this much of its height up.
const MIDDLE: f32 = 0.5;
const SPIT: &str = "llama_spit";
/// A llama charges this many ticks before its spit and stops this many before it, heard `warn`
/// from this high and this far toward its prey (research/captures/bds-provoke2.txt).
const CHARGES: [u64; 2] = [21, 2];
const WARNS: [f32; 2] = [0.85, 0.64];

/// A llama `due` ticks from its spit shows its charge (`CHARGED`, `ChargeAmount`) or its end.
fn charge(world: &mut World, entity: Entity, layer: Entity, kind: &str, body: &Body, prey: [f32; 3], due: u64) {
    let Some(on) = [true, false].into_iter().find(|on| due == CHARGES[usize::from(!on)]) else { return };
    let Some(id) = world.get::<NetId>(entity).map(|id| id.0) else { return };
    let flags = entities::mob_flags(world, entity, kind, MetadataFlags1::CHARGED, on);
    let mut told = Vec::new();
    if on {
        let (dx, dz) = (prey[0] - body.at[0], prey[2] - body.at[2]);
        let across = dx.hypot(dz).max(1e-4);
        let mut ahead = *body;
        ahead.at = [body.at[0] + dx / across * WARNS[1], body.at[1], body.at[2] + dz / across * WARNS[1]];
        told.push(super::hunting::sound("warn", id, kind, &ahead, WARNS[0]));
    }
    told.push(sapling_net::encode(&sapling_data::mob_charge(id, flags, on)));
    entities::tell_viewers(world, entity, layer, body.at.map(|c| c.floor() as i32), told);
}

/// After its move: a hunter that shoots, `every` ticks, at its `prey` looses what is due.
pub(super) fn shoot(world: &mut World, entity: Entity, layer: Entity, kind: &str, body: &Body, hunts: Hunts, every: u32, prey: Entity) -> Option<()> {
    let now = world.resource::<Tick>().0;
    let feet = super::hunting::feet_of(world, prey)?;
    let ready = world.get::<Hunter>(entity)?.ready;
    if hunts.shot == Some(SPIT) && ready > now {
        charge(world, entity, layer, kind, body, feet, ready - now);
    }
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
    // Further off its crossbow stays loaded and goes once the prey comes near (bds-pillrange.txt).
    if hunts.fires.is_some_and(|fires| super::hunting::distance(feet, body.at) > fires) && ready != 0 && now >= ready {
        return Some(());
    }
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
        for (after, stage) in LOADS.into_iter().filter(|_| hunts.closes) {
            world.get_resource_or_insert_with(Loads::default).0.push((now + after, entity, layer, stage));
        }
    }
    Some(())
}
