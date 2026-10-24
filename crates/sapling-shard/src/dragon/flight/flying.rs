//! The dragon's flying goals (research/dragon-ai-decompile.md 2): the holding pattern round the
//! node rings, from which it lands or strafes a player; the takeoff from the portal; strafing,
//! which shoots a fireball once its aim has held; and the charge at a player it saw while sitting.

use super::goals::{Ctx, land};
use super::{Flight, nodes};

const LANDING_ODDS: u32 = 3;
const STRAFE_REACH: f32 = 64.0;
const STRAFE_SCALE: f32 = 0.001_953_125;
const TURN_ODDS: u32 = 8;
const TURN_AHEAD: i32 = 6;
const RING: i32 = 12;
const PORTAL_REACH_SQ: f32 = 100.0;
const AHEAD: f32 = 40.0;
const AHEAD_Y: f32 = 105.0;
const AIM_TICKS: u32 = 5;
const AIM_ANGLE: f32 = 10.0;
const CHARGE_SPEED: f32 = 3.0;
const CHARGE_REACH: f32 = 150.0;
const CHARGE_HOLD: u32 = 10;
const AIR: f32 = 0.6;
const TURNS: f32 = 0.7;

/// Where the next lap ends, by the fight's crystals: the outer ring's node, else the inner's.
fn lap_end(crystals: u32, j: i32) -> usize {
    (if crystals >= 1 { j.rem_euclid(RING) } else { ((j & 7) ^ 4) + RING }) as usize
}

/// A new lap of the rings from the node nearest the dragon, turning back one time in eight.
fn new_lap(cx: &mut Ctx<'_>, flight: &mut Flight, skip_first: bool) {
    let i = nodes::closest(&flight.nodes, cx.crystals, flight.motion.at);
    let mut j = i as i32;
    if cx.random.next_int(TURN_ODDS) == 0 {
        flight.clockwise = !flight.clockwise;
        j += TURN_AHEAD;
    }
    j += if flight.clockwise { 1 } else { -1 };
    flight.path = nodes::path(&flight.nodes, cx.crystals, i, lap_end(cx.crystals, j), None).map(|path| (path, usize::from(skip_first)));
}

pub(super) fn holding(cx: &mut Ctx<'_>, flight: &mut Flight) {
    if flight.keeps_target() {
        return;
    }
    if flight.path_done() {
        let pod = cx.portal_y();
        if cx.random.next_int(cx.crystals + LANDING_ODDS) == 0 {
            return land(flight);
        }
        let player = cx.nearest_player([0.0, pod as f32, 0.0], STRAFE_REACH);
        let far = player.map_or(64, |(_, [x, y, z])| (((x - 0.5).powi(2) + (y - pod as f32 - 0.5).powi(2) + (z - 0.5).powi(2)) * STRAFE_SCALE) as i32);
        let near = cx.random.next_int(far.unsigned_abs() + 2) == 0;
        if let Some((player, _)) = player.filter(|_| near || cx.random.next_int(cx.crystals + 2) == 0).filter(|(_, feet)| cx.sees(flight, *feet)) {
            flight.prey = Some(player);
            return;
        }
    }
    if flight.path.is_none() || flight.path_done() {
        new_lap(cx, flight, true);
    }
    flight.next_node(cx.random);
}

pub(super) fn start_takeoff(flight: &mut Flight) {
    (flight.motion.target, flight.path, flight.first) = (None, None, true);
    flight.sit(false);
}

/// On its first tick, a path from the portal ahead of it, its first node the target.
pub(super) fn takeoff(cx: &mut Ctx<'_>, flight: &mut Flight) {
    if !std::mem::take(&mut flight.first) {
        return;
    }
    let i = nodes::closest(&flight.nodes, cx.crystals, flight.motion.at);
    let (sin, cos) = flight.motion.yaw.to_radians().sin_cos();
    let j = nodes::closest(&flight.nodes, cx.crystals, [(sin * AHEAD).floor(), AHEAD_Y, (-cos * AHEAD).floor()]);
    flight.path = nodes::path(&flight.nodes, cx.crystals, i, lap_end(cx.crystals, j as i32), None).map(|path| (path, 0));
    flight.next_node(cx.random);
}

pub(super) fn takeoff_goes_on(cx: &Ctx<'_>, flight: &Flight) -> bool {
    let pod = cx.portal_y() as f32 + 0.5;
    let at = flight.motion.at;
    (at[0] - 0.5).powi(2) + (at[1] - pod).powi(2) + (at[2] - 0.5).powi(2) <= PORTAL_REACH_SQ
}

/// Where a strafe aims over its prey: higher the farther the dragon is across, at most ten.
fn over(prey: [f32; 3], at: [f32; 3]) -> [f32; 3] {
    let across = (prey[0] - at[0]).hypot(prey[2] - at[2]);
    [prey[0], prey[1] + (across / 80.0 + 0.4 - 1.0).min(10.0), prey[2]]
}

pub(super) fn start_strafe(cx: &mut Ctx<'_>, flight: &mut Flight) {
    (flight.fired, flight.aim) = (false, 0);
    let Some(prey) = flight.prey.and_then(|prey| cx.feet_of(prey)) else { return };
    let i = nodes::closest(&flight.nodes, cx.crystals, flight.motion.at);
    let j = nodes::closest(&flight.nodes, cx.crystals, prey);
    let [x, y, z] = over(prey, flight.motion.at);
    let then = [x.floor() as i32, y as i32, z.floor() as i32];
    flight.path = nodes::path(&flight.nodes, cx.crystals, i, j, Some(then)).map(|path| (path, 1));
    flight.next_node(cx.random);
    flight.sit(false);
    (flight.motion.air_speed, flight.motion.turn_factor) = (AIR, TURNS);
}

pub(super) fn stop_strafe(flight: &mut Flight) {
    (flight.prey, flight.motion.target) = (None, None);
    (flight.motion.air_speed, flight.motion.turn_factor) = (AIR, TURNS);
}

pub(super) fn strafe_goes_on(cx: &Ctx<'_>, flight: &Flight) -> bool {
    !flight.fired && flight.prey.is_some_and(|prey| cx.feet_of(prey).is_some())
}

pub(super) fn strafe(cx: &mut Ctx<'_>, flight: &mut Flight) {
    let Some(prey) = flight.prey.and_then(|prey| cx.feet_of(prey)) else { return };
    if flight.path_done() {
        flight.motion.target = Some(over(prey, flight.motion.at));
    }
    if !flight.keeps_target() {
        new_lap(cx, flight, true);
        flight.next_node(cx.random);
    }
    let at = flight.motion.at;
    let far = (0..3).map(|axis| (prey[axis] - at[axis]).powi(2)).sum::<f32>();
    if far >= STRAFE_REACH * STRAFE_REACH || !cx.sees(flight, prey) {
        flight.aim = flight.aim.saturating_sub(1);
        return;
    }
    flight.aim += 1;
    let (sin, cos) = flight.motion.yaw.to_radians().sin_cos();
    let (dx, dz) = (prey[0] - at[0], prey[2] - at[2]);
    let length = dx.hypot(dz).max(1e-4);
    let angle = ((dx / length) * sin + (dz / length) * -cos).clamp(-1.0, 1.0).acos().to_degrees() + 0.5;
    if flight.aim >= AIM_TICKS && (0.0..AIM_ANGLE).contains(&angle) && !flight.fired {
        flight.aim = 0;
        flight.fired = true;
        crate::dragon::fireball::shoot(cx.world, cx.layer, cx.dragon, cx.from, prey, cx.random);
        if let Some((path, index)) = &mut flight.path {
            *index = path.len();
        }
    }
}

pub(super) fn start_charge(cx: &mut Ctx<'_>, flight: &mut Flight) {
    flight.count = 0;
    flight.sit(false);
    (flight.motion.air_speed, flight.motion.turn_factor) = (CHARGE_SPEED, TURNS);
    flight.motion.target = cx.nearest_player(flight.motion.at, CHARGE_REACH).map(|(_, feet)| feet);
}

pub(super) fn stop_charge(flight: &mut Flight) {
    (flight.motion.target, flight.prey, flight.motion.air_speed) = (None, None, AIR);
}

pub(super) fn charge_goes_on(flight: &mut Flight) -> bool {
    flight.motion.target.is_some() && (flight.count < 1 || {
        flight.count += 1;
        flight.count - 1 < CHARGE_HOLD
    })
}

pub(super) fn charge(_cx: &mut Ctx<'_>, flight: &mut Flight) {
    let Some(target) = flight.motion.target else { return };
    let far = (0..3).map(|axis| (target[axis] - flight.motion.at[axis]).powi(2)).sum::<f32>();
    if !(100.0..=22500.0).contains(&far) {
        flight.count += 1;
    }
}
