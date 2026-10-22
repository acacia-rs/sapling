//! The dragon's sitting goals and its last flight (research/dragon-ai-decompile.md 2): landing,
//! a path by the rings to the portal's top and a last leg onto it; scanning for a player near,
//! which flames one found and otherwise takes off after five seconds; flaming, a roar and a puke;
//! and the death's flight to the portal, where its health goes.
// TODO: the breath's cloud of harm is left out; the roar and the puke are told.

use acacia_proto::packets::{EntityEvent, EntityEventEventId};

use super::goals::{Ctx, take_off};
use super::{Flight, nodes};

const AIR: f32 = 0.6;
const TURNS: f32 = 0.7;
const LANDING_AIR: f32 = 1.5;
const PLAYER_REACH: f32 = 128.0;
const AHEAD: f32 = 40.0;
const AHEAD_Y: f32 = 105.0;
const SETTLED_SQ: f32 = 1.0;
const SCAN_REACH: f32 = 20.0;
const SCAN_FOR: f32 = 100.0;
const SCAN_SEES: f32 = 25.0;
const TAKEOFF_REACH: f32 = 150.0;
const SCAN_TURN: f32 = 100.0;
const ROAR: u32 = 40;
const PUKES: u32 = 10;
const BREATH_ENDS: u32 = 200;
const FLAMES: u32 = 4;
const DEATH_AIR: f32 = 3.0;
const DEATH_NEAR_SQ: f32 = 100.0;
const DEATH_FAR_SQ: f32 = 22_500.0;
const PARTICLES_EVERY: u32 = 10;

pub(super) fn start_landing(flight: &mut Flight) {
    (flight.motion.target, flight.path, flight.landing) = (None, None, false);
    (flight.motion.air_speed, flight.motion.turn_factor) = (AIR, TURNS);
}

pub(super) fn landing(cx: &mut Ctx<'_>, flight: &mut Flight) {
    if flight.landing || flight.keeps_target() {
        return;
    }
    let pod = cx.portal_y();
    if flight.path.is_none() || flight.path_done() {
        let start = nodes::closest(&flight.nodes, cx.crystals, flight.motion.at);
        let end = match cx.nearest_player([0.0, pod as f32, 0.0], PLAYER_REACH) {
            Some((_, [x, _, z])) => {
                let length = x.hypot(z);
                let n = if length < 1e-4 { [0.0; 2] } else { [x / length, z / length] };
                nodes::closest(&flight.nodes, cx.crystals, [(-n[0] * AHEAD).floor(), AHEAD_Y, (-n[1] * AHEAD).floor()])
            }
            None => nodes::closest(&flight.nodes, cx.crystals, [AHEAD, pod as f32, 0.0]),
        };
        flight.path = nodes::path(&flight.nodes, cx.crystals, start, end, Some([0, pod, 0])).map(|path| (path, 1));
    }
    // The last leg starts as the portal's node becomes the target, as Java's `isDone` after advancing.
    flight.next_node(cx.random);
    if flight.path_done() {
        flight.landing = true;
        flight.motion.target = Some([0.0, pod as f32, 0.0]);
        (flight.motion.air_speed, flight.motion.turn_factor) = (LANDING_AIR, 0.0);
    }
}

/// On its last leg, once on the portal's top it sits down and the goal ends.
pub(super) fn landing_goes_on(flight: &mut Flight) -> bool {
    let settled = flight.landing && flight.motion.target.is_some_and(|t| (0..3).map(|axis| (t[axis] - flight.motion.at[axis]).powi(2)).sum::<f32>() < SETTLED_SQ);
    if settled {
        flight.flames = 0;
        flight.sit(true);
    }
    !settled
}

pub(super) fn start_scanning(flight: &mut Flight) {
    (flight.timer, flight.motion.target, flight.prey) = (0.0, None, None);
    flight.sit(true);
}

pub(super) fn scanning(cx: &mut Ctx<'_>, flight: &mut Flight) {
    flight.timer += 1.0;
    let at = flight.motion.at.map(f32::floor);
    let Some((_, feet)) = cx.nearest_player(at, SCAN_REACH) else { return };
    let (dx, dz) = (feet[0] - at[0], feet[2] - at[2]);
    let (sin, cos) = flight.motion.yaw.to_radians().sin_cos();
    let length = dx.hypot(dz).max(1e-4);
    let angle = ((dx / length) * sin + (dz / length) * -cos).clamp(-1.0, 1.0).acos().to_degrees() + 0.5;
    if !(0.0..=10.0).contains(&angle) {
        let turn = super::wrap(180.0 - dx.atan2(dz).to_degrees() - flight.motion.yaw).clamp(-SCAN_TURN, SCAN_TURN);
        let s = dx.hypot(dz) + 1.0;
        flight.motion.yaw += turn * (1.0 / s.min(40.0) / s) + flight.motion.yaw_speed * 0.8;
    }
}

pub(super) fn scanning_goes_on(cx: &mut Ctx<'_>, flight: &mut Flight) -> bool {
    let at = flight.motion.at.map(f32::floor);
    if let Some((player, _)) = cx.nearest_player(at, SCAN_REACH) {
        if flight.timer <= SCAN_SEES {
            return true;
        }
        flight.prey = Some(player);
        return false;
    }
    if flight.timer < SCAN_FOR {
        return true;
    }
    flight.prey = cx.nearest_player(at, TAKEOFF_REACH).filter(|(_, feet)| cx.sees(flight, *feet)).map(|(player, _)| player);
    take_off(flight);
    false
}

pub(super) fn start_flaming(flight: &mut Flight) {
    (flight.roar, flight.breath, flight.motion.target) = (0, 0, None);
    flight.sit(true);
    flight.flames += 1;
}

pub(super) fn stop_flaming(flight: &mut Flight) {
    (flight.roar, flight.breath, flight.motion.target, flight.prey) = (0, 0, None, None);
}

/// Its roar, then its puke every other tick, until the breath comes at the tenth.
pub(super) fn flaming(cx: &mut Ctx<'_>, flight: &mut Flight) {
    let t = flight.roar;
    flight.roar += 1;
    if t < ROAR {
        cx.growl_at_head();
        return;
    }
    flight.breath += 1;
    if flight.breath < PUKES && flight.breath % 2 == 0 {
        cx.tell_dragon(|id| EntityEvent { runtime_entity_id: id, event_id: EntityEventEventId::DragonPuke, data: 0, fire_at_position: None });
    }
    if flight.breath == PUKES {
        breathe(cx, flight);
    }
}

/// The breath cloud, half its radius out from the head, away from the body, on the ground under it.
fn breathe(cx: &mut Ctx<'_>, flight: &Flight) {
    let head = crate::dragon::parts::head_middle(flight).map(f32::floor);
    let at = flight.motion.at;
    let (dx, dz) = (head[0] - at[0], head[2] - at[2]);
    let length = dx.hypot(dz);
    let n = if length < 1e-4 { [0.0; 2] } else { [dx / length, dz / length] };
    let half = crate::dragon::cloud::FLAMES.radius * 0.5;
    let (x, z) = (head[0] + n[0] * half, head[2] + n[1] * half);
    let Some(level) = cx.world.get::<crate::components::Layer>(cx.layer).map(|layer| &layer.world) else { return };
    let column = [x.floor() as i32, z.floor() as i32];
    let mut y = head[1] as i32;
    while level.in_height(y - 1) && level.registry().get(level.block(column[0], y, column[1])).is_some_and(|state| state.is_air()) {
        y -= 1;
    }
    crate::dragon::cloud::spawn(cx.world, cx.layer, [x, (y + 1) as f32, z], 0.0, crate::dragon::cloud::FLAMES);
}

pub(super) fn flaming_goes_on(flight: &mut Flight) -> bool {
    if flight.breath < BREATH_ENDS {
        return true;
    }
    if flight.flames >= FLAMES {
        take_off(flight);
    }
    false
}

pub(super) fn start_death(flight: &mut Flight) {
    (flight.motion.target, flight.timer) = (None, 0.0);
    (flight.motion.air_speed, flight.motion.turn_factor) = (DEATH_AIR, TURNS);
}

/// To the portal's top; there its health goes (`dead`).
pub(super) fn death(cx: &mut Ctx<'_>, flight: &mut Flight) {
    if flight.timer as u32 % PARTICLES_EVERY == 0 {
        // BDS draws three for particles no server sends.
        (0..3).for_each(|_| {
            cx.random.next_float();
        });
    }
    flight.timer += 1.0;
    let target = *flight.motion.target.get_or_insert([0.0, cx.portal_y() as f32, 0.0]);
    let far = (0..3).map(|axis| (target[axis] - flight.motion.at[axis]).powi(2)).sum::<f32>();
    if !(DEATH_NEAR_SQ..=DEATH_FAR_SQ).contains(&far) {
        cx.dead = true;
    }
}
