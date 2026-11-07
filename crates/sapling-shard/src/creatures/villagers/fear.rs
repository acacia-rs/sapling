//! What a villager fears (its pack): `behavior.avoid_mob_type` walks it to a spot away from a
//! zombie, zombie villager, illager, vex or zoglin in its box grown by 3; `behavior.hide` sends it to its bed for 30 s
//! once a bell rings near it. BDS's bell ring (0x141f1c400) marks every actor with
//! `minecraft:hide` in the bell's box grown 32 across and 12 up, then rests 300 ticks; a bell a
//! villager rings in a raid (`Village::tick`) marks none.

use std::collections::HashMap;

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::super::{Creature, Dying};
use super::Trader;
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{InLayer, Layer, Tick};
use crate::entities::Kind;

const FEARED: [&str; 5] = ["zombie", "zombie_villager", "illager", "vex", "zoglin"];
/// Each feared kind's `max_dist` 8; the goal looks only in its box grown by its own `max_dist`,
/// the default 3 (findNearest 14190d490): BDS's villagers fled a still zombie 3 off within half a
/// second and none 4 off (bds-an1fear.txt).
const FEARS_WITHIN: f32 = 8.0;
const LOOKS_ACROSS: f32 = 3.0;
/// `max_flee`'s default: fleeing ends with the mob this far off.
const STOPS_AT: f32 = 10.0;
const ARRIVED: f32 = 1.0;
const SPOT_TRIES: usize = 10;
const SPOT_ACROSS: i32 = 16;
const SPOT_UP: i32 = 7;
/// `minecraft:movement` 0.5 times each goal's `speed_multiplier`: avoid 0.6, hide 0.8.
const FLEES: f32 = 0.3;
const HIDES: f32 = 0.4;
const HIDES_FOR: u64 = 600;
const HEARD_ACROSS: f32 = 32.0;
const HEARD_UP: f32 = 12.0;
const BELL_RESTS: u64 = 300;
/// How near its bed a hiding villager stops.
const HIDDEN: f32 = 1.0;

/// The tick a villager hides until.
#[derive(Component)]
pub(super) struct Hiding(u64);

/// Each bell's tick it alarms again from, by world and cell.
#[derive(Resource, Default)]
struct Rested(HashMap<(Entity, BlockPos), u64>);

/// A bell at `pos` of `layer` rang: the villagers near it hide, unless it rang lately.
pub(crate) fn alarm(world: &mut World, layer: Entity, pos: BlockPos) {
    let now = world.resource::<Tick>().0;
    let mut rested = world.remove_resource::<Rested>().unwrap_or_default();
    rested.0.retain(|_, until| *until > now);
    let rang = rested.0.insert((layer, pos), now + BELL_RESTS).is_some();
    world.insert_resource(rested);
    if rang {
        return;
    }
    let heard = |at: [f32; 3]| {
        let [x, y, z] = pos.map(|c| c as f32);
        (x - HEARD_ACROSS..=x + 1.0 + HEARD_ACROSS).contains(&at[0]) && (y - HEARD_UP..=y + 1.0 + HEARD_UP).contains(&at[1]) && (z - HEARD_ACROSS..=z + 1.0 + HEARD_ACROSS).contains(&at[2])
    };
    let hiding: Vec<Entity> = world
        .query_filtered::<(Entity, &InLayer, &Body, &Trader), Without<Dying>>()
        .iter(world)
        .filter(|(_, on, body, trader)| on.0 == layer && trader.leaves_at.is_none() && heard(body.at))
        .map(|(villager, ..)| villager)
        .collect();
    for villager in hiding {
        world.entity_mut(villager).insert(Hiding(now + HIDES_FOR));
    }
}

/// A hiding villager goes to its bed: false once it is done hiding, or has none.
pub(super) fn hides(world: &mut World, entity: Entity, layer: Entity, body: &Body) -> bool {
    let Some(until) = world.get::<Hiding>(entity).map(|hiding| hiding.0) else { return false };
    if world.resource::<Tick>().0 >= until {
        world.entity_mut(entity).remove::<Hiding>();
        return false;
    }
    super::beds::go_to(world, entity, layer, body, HIDES, HIDDEN)
}

/// A spot a villager flees to, and what from.
#[derive(Component)]
pub(super) struct Fleeing {
    to: [f32; 3],
    from: Entity,
}

/// The nearest mob the villager fears, sees and whose box is within [`LOOKS_ACROSS`] of its own.
fn feared(world: &mut World, layer: Entity, body: &Body) -> Option<(Entity, [f32; 3])> {
    let (low, high) = body.bounds();
    let off = |at: [f32; 3]| (0..3).map(|axis| (at[axis] - body.at[axis]).powi(2)).sum::<f32>().sqrt();
    let near: Vec<(Entity, [f32; 3])> = world
        .query_filtered::<(Entity, &InLayer, &Kind, &Body), (With<Creature>, Without<Dying>)>()
        .iter(world)
        .filter(|(_, on, _, at)| {
            let (their_low, their_high) = at.bounds();
            on.0 == layer && off(at.at) < FEARS_WITHIN && (0..3).all(|axis| their_low[axis] < high[axis] + LOOKS_ACROSS && their_high[axis] > low[axis] - LOOKS_ACROSS)
        })
        .filter(|(_, _, kind, _)| sapling_data::mob_families(&kind.0).iter().any(|family| FEARED.contains(family)))
        .map(|(mob, _, _, at)| (mob, at.at))
        .collect();
    near.into_iter().filter(|(_, at)| super::super::hunting::in_sight(world, layer, body, *at)).min_by(|a, b| off(a.1).total_cmp(&off(b.1)))
}

/// A villager flees from a mob it fears to a spot found away from it, until it gets there or the
/// mob is [`STOPS_AT`] off: false with no such mob, or no spot found this tick (BDS's lateness).
/// A zombie caged in glass moved no BDS villager (research/captures/bds-village.txt).
pub(super) fn flees(world: &mut World, entity: Entity, layer: Entity, body: &Body) -> bool {
    let fleeing = world.get::<Fleeing>(entity).map(|fleeing| (fleeing.to, fleeing.from));
    if let Some((to, from)) = fleeing {
        let threat = world.get::<Body>(from).filter(|_| world.get::<Dying>(from).is_none()).map(|threat| threat.at);
        let arrived = (to[0] - body.at[0]).hypot(to[2] - body.at[2]) <= ARRIVED;
        if !arrived && threat.is_some_and(|at| (0..3).map(|axis| (at[axis] - body.at[axis]).powi(2)).sum::<f32>().sqrt() < STOPS_AT) {
            super::super::legs::walk(world, entity, layer, body, [to[0], to[2]], FLEES, 0.0);
            return true;
        }
        world.entity_mut(entity).remove::<Fleeing>();
    }
    let Some((from, threat)) = feared(world, layer, body) else { return false };
    let Some(to) = avoid_spot(world, layer, body.at, threat) else { return false };
    world.entity_mut(entity).insert(Fleeing { to, from });
    super::super::legs::walk(world, entity, layer, body, [to[0], to[2]], FLEES, 0.0);
    true
}

/// BDS's `RandomPos::getPosAvoid` (14597bd90, by 14597ab80) as the avoid goal asks it
/// (`avoid_target_xz` 16, `avoid_target_y` 7, 10 tries): the first standable cell drawn within a
/// half turn of straight away from `threat`, if it lies no nearer the threat than `feet` does.
fn avoid_spot(world: &mut World, layer: Entity, feet: [f32; 3], threat: [f32; 3]) -> Option<[f32; 3]> {
    let away = (feet[2] - threat[2]).atan2(feet[0] - threat[0]);
    let mut dice = world.remove_resource::<Dice>().unwrap_or_default();
    let spot = world.get::<Layer>(layer).and_then(|kept| {
        let [x, y, z] = feet.map(|c| c.floor() as i32);
        (0..SPOT_TRIES).find_map(|_| {
            let angle = away + (2.0 * dice.fraction() - 1.0) * std::f32::consts::FRAC_PI_2;
            let reach = dice.fraction().sqrt() * std::f32::consts::SQRT_2 * SPOT_ACROSS as f32;
            let (dx, dz) = ((reach * angle.cos()).round() as i32, (reach * angle.sin()).round() as i32);
            let dy = dice.roll(2 * SPOT_UP as u32 + 1) as i32 - SPOT_UP;
            if dx.abs() > SPOT_ACROSS || dz.abs() > SPOT_ACROSS {
                return None;
            }
            let base = y + dy;
            (0..=SPOT_UP).flat_map(|step| [base - step, base + step]).map(|y| [x + dx, y, z + dz]).find(|cell| super::super::paths::is_standable(&kept.world, *cell))
        })
    });
    world.insert_resource(dice);
    let to = spot.map(|[x, y, z]| [x as f32 + 0.5, y as f32, z as f32 + 0.5])?;
    let off = |at: [f32; 3]| (0..3).map(|axis| (at[axis] - threat[axis]).powi(2)).sum::<f32>();
    (off(to) >= off(feet)).then_some(to)
}
