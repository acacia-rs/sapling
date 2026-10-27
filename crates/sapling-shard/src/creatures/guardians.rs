//! Guardians and elder guardians, as BDS's in the `guardians` drill
//! (research/captures/bds-guardians.txt). One takes a player it sees within 16 (`TargetEid`),
//! stops, and 9 ticks on shows its laser (`GuardianAttack`); 80 ticks after that, 60 for an
//! elder, the laser lands: the guardian's hit of 5 and magic of 1 (bds-guardianhits.txt), and it lets its prey go, to
//! take it again at its next look. Every 100 ticks an elder curses the survival and adventure
//! players within 50 that do not already have mining fatigue at its third level for a minute or
//! more: six minutes of it (`ElderGuardianCurse`), as `GuardianPreAIStepSystem` does (bdsre
//! 1459a1130).
// TODO: BDS flashes the guarded magic too (`HurtAnimation` 14 after 2); guardians take no squid or
// axolotl, spike no attacker, and swim at their prey as at nothing.

use acacia_proto::packets::{EntityEvent, EntityEventEventId};
use bevy_ecs::prelude::*;
use sapling_api::Effect;

use super::{Cause, hunting};
use crate::body::Body;
use crate::components::{InLayer, NetId, Position, Tick, in_world};
use crate::entities::{self, Kind};
use crate::vitals;

const ELDER: &str = "minecraft:elder_guardian";
/// Ticks from taking its prey to showing its laser, and from that to its landing.
const AIMS: u64 = 9;
const CHARGES: u64 = 80;
const ELDER_CHARGES: u64 = 60;
/// The laser's magic, which the hit before it guards against (bds-guardianhits.txt: 20 to 15 on
/// normal, from a guardian and from an elder alike).
const MAGIC: f32 = 1.0;
/// The laser throws its prey this fast away and up.
const THROW: f32 = 0.4;
/// An elder's curse: how often, how far, and the mining fatigue it gives.
const CURSES: u64 = 100;
const CURSE_REACH: f32 = 50.0;
const CURSE_TICKS: u32 = 6000;
const CURSE_LEVEL: u8 = 2;
/// A player with mining fatigue at least this high for at least this long is spared.
const SPARED_FOR: u32 = 1200;

fn tell(world: &mut World, mob: Entity, layer: Entity, at: [f32; 3], event: EntityEvent) {
    entities::tell_viewers(world, mob, layer, at.map(|c| c.floor() as i32), vec![sapling_net::encode(&event)]);
}

/// Before its move: with prey it holds still and fires; without, it swims about.
pub(super) fn steer(world: &mut World, mob: Entity, layer: Entity, id: u64, kind: &str, body: &Body, prey: Option<[f32; 3]>) -> bool {
    let Some((victim, since)) = prey.and(hunting::quarry(world, mob)) else { return super::swimming::steer(world, mob, layer, kind) };
    let now = world.resource::<Tick>().0;
    let charges = if kind == ELDER { ELDER_CHARGES } else { CHARGES };
    match now - since {
        AIMS => tell(world, mob, layer, body.at, EntityEvent { runtime_entity_id: id, event_id: EntityEventEventId::GuardianAttack, data: 0, fire_at_position: None }),
        into if into == AIMS + charges => {
            land(world, mob, kind, body, victim);
            hunting::give_up(world, mob, layer, id, kind, body);
        }
        _ => {}
    }
    true
}

/// The laser lands on `victim`: the guardian's hit, then the magic.
fn land(world: &mut World, mob: Entity, kind: &str, body: &Body, victim: Entity) {
    let Some(feet) = world.get::<Position>(victim).map(|at| at.feet) else { return };
    let Some(harm) = sapling_data::mob_traits(kind).hunts.and_then(|hunts| hunting::on_difficulty(world, hunts.attack)) else { return };
    let (dx, dz) = (feet[0] - body.at[0], feet[2] - body.at[2]);
    let far = dx.hypot(dz).max(1e-4);
    if vitals::bitten(world, victim, harm, kind, [dx / far * THROW, THROW, dz / far * THROW]) {
        crate::enchants::thorns(world, victim, mob);
    }
    vitals::hurt(world, victim, MAGIC, Cause::Magic);
}

/// Each elder curses the players near it on its own beat.
pub(super) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let elders: Vec<(Entity, [f32; 3])> = world
        .query::<(&InLayer, &NetId, &Kind, &Body)>()
        .iter(world)
        .filter(|(_, id, kind, _)| kind.0 == ELDER && (now + id.0).is_multiple_of(CURSES))
        .map(|(layer, _, _, body)| (layer.0, body.at))
        .collect();
    for (layer, at) in elders {
        for player in in_world(world) {
            let near = world.get::<InLayer>(player).is_some_and(|on| on.0 == layer) && world.get::<Position>(player).is_some_and(|position| (0..3).map(|axis| (position.feet[axis] - at[axis]).powi(2)).sum::<f32>() < CURSE_REACH * CURSE_REACH);
            let spared = vitals::effects::on(world, player, Effect::MiningFatigue).is_some_and(|(level, left)| level >= CURSE_LEVEL && left >= SPARED_FOR);
            if !near || spared || !vitals::is_mortal(world, player) {
                continue;
            }
            vitals::give_effect(world, player, Effect::MiningFatigue, CURSE_TICKS, CURSE_LEVEL, true);
            let Some((id, feet)) = world.get::<NetId>(player).map(|id| id.0).zip(world.get::<Position>(player).map(|at| at.feet)) else { continue };
            tell(world, player, layer, feet, EntityEvent { runtime_entity_id: id, event_id: EntityEventEventId::ElderGuardianCurse, data: 0, fire_at_position: None });
        }
    }
}
