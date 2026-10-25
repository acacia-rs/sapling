//! A minecart's roll along a rail, as BDS's `MinecartMoveAlongRailSystem` does
//! (docs/research/vehicles.md "Rail physics"): slopes pull, the motion is turned along the rail
//! and capped, a powered rail speeds a cart up and an unpowered one brakes it, the move is capped
//! a tick, and a cart gains speed going down.
// TODO: the cart's height above its rail is unverified (its box's bottom on the rail block's
// floor here), and two carts do not push each other.

use sapling_api::BlockPos;
use sapling_world::World as Level;

/// What slopes add to the motion, a tick.
const SLOPE: f32 = 0.0078125;
const TOP_SPEED: f32 = 2.0;
/// The most a cart moves along an axis in a tick (`rail_movement` `max_speed`), and the share of
/// its motion it moves with a rider.
pub(super) const MOVE_CAP: f32 = 0.4;
const RIDDEN_MOVE: f32 = 0.75;
/// An unpowered powered rail halves the speed, or stops a cart slower than this (squared).
const BRAKE_STOP: f32 = 0.0009;
/// A powered rail adds this along the motion of a cart faster than `BOOST_FROM`, and kicks one
/// at rest off a solid block at `KICK`.
const BOOST: f32 = 0.06;
const BOOST_FROM: f32 = 0.01;
const KICK: f32 = 0.02;
/// Speed gained a block dropped.
const DOWNHILL: f32 = 0.05;
/// A rider's forward key pushes a cart slower than this (squared) by `PUSH`.
const PUSH_BELOW: f32 = 0.01;
const PUSH: f32 = 0.1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RailKind {
    Plain,
    Powered,
    Detector,
    Activator,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Rail {
    pub pos: BlockPos,
    /// `rail_direction`: north-south, east-west, rising east, west, north, south, then the curves
    /// south-east, south-west, north-west, north-east.
    pub shape: u8,
    pub kind: RailKind,
    /// `rail_data_bit`: a powered or activator rail powered, a detector rail pressed.
    pub on: bool,
}

/// The two ends of each shape, as offsets in x and z.
const ENDS: [[[f32; 2]; 2]; 10] = [
    [[0.0, -1.0], [0.0, 1.0]],
    [[-1.0, 0.0], [1.0, 0.0]],
    [[-1.0, 0.0], [1.0, 0.0]],
    [[-1.0, 0.0], [1.0, 0.0]],
    [[0.0, -1.0], [0.0, 1.0]],
    [[0.0, -1.0], [0.0, 1.0]],
    [[0.0, 1.0], [1.0, 0.0]],
    [[0.0, 1.0], [-1.0, 0.0]],
    [[0.0, -1.0], [-1.0, 0.0]],
    [[0.0, -1.0], [1.0, 0.0]],
];

/// The rail in the cell `pos`.
pub(super) fn rail_at(level: &Level, [x, y, z]: BlockPos) -> Option<Rail> {
    let state = level.registry().get(level.block(x, y, z)).filter(|_| level.in_height(y))?;
    let kind = match state.name {
        "minecraft:rail" => RailKind::Plain,
        "minecraft:golden_rail" => RailKind::Powered,
        "minecraft:detector_rail" => RailKind::Detector,
        "minecraft:activator_rail" => RailKind::Activator,
        _ => return None,
    };
    let get = |key: &str| state.properties.split(',').filter_map(|pair| pair.split_once('=')).find(|(k, _)| *k == key).map(|(_, value)| value);
    let shape = get("rail_direction").and_then(|shape| shape.parse().ok()).filter(|shape| *shape < 10).unwrap_or(0);
    Some(Rail { pos: [x, y, z], shape, kind, on: get("rail_data_bit") == Some("1") })
}

/// The rail a cart with its box's bottom at `feet` is on: in its cell, or the one below.
pub(super) fn rail_under(level: &Level, feet: [f32; 3]) -> Option<Rail> {
    let [x, y, z] = feet.map(|c| c.floor() as i32);
    rail_at(level, [x, y, z]).or_else(|| rail_at(level, [x, y - 1, z]))
}

/// How far up from its cell's floor a rail of `shape` is at `[fx, fz]` within the cell.
fn rise(shape: u8, [fx, fz]: [f32; 2]) -> f32 {
    match shape {
        2 => fx,
        3 => 1.0 - fx,
        4 => 1.0 - fz,
        5 => fz,
        _ => 0.0,
    }
    .clamp(0.0, 1.0)
}

/// What a cart has that changes its roll.
pub(super) struct Roll {
    /// Its motion kept a tick on a rail: 0.997 ridden, 0.96 empty, more for a full one.
    pub drag: f32,
    pub ridden: bool,
    /// The way its rider faces, while the rider holds its forward key.
    pub pushed_to: Option<f32>,
    /// Whether a solid block is beside the rail's end, north, south, west and east, for the kick.
    pub walls: [bool; 4],
}

/// A tick of a cart with its box's bottom at `feet` moving at `motion` on `rail`: where its
/// box's bottom ends up and its motion.
pub(super) fn roll(rail: &Rail, feet: [f32; 3], motion: [f32; 3], how: &Roll) -> ([f32; 3], [f32; 3]) {
    let [bx, by, bz] = rail.pos.map(|c| c as f32);
    let (mut vx, mut vz) = (motion[0] * how.drag, motion[2] * how.drag);
    match rail.shape {
        2 => vx -= SLOPE,
        3 => vx += SLOPE,
        4 => vz += SLOPE,
        5 => vz -= SLOPE,
        _ => {}
    }
    let [start, end] = ENDS[usize::from(rail.shape)];
    let (mut dx, mut dz) = (end[0] - start[0], end[1] - start[1]);
    if vx * dx + vz * dz < 0.0 {
        (dx, dz) = (-dx, -dz);
    }
    let length = dx.hypot(dz);
    let speed = vx.hypot(vz).min(TOP_SPEED);
    (vx, vz) = (speed * dx / length, speed * dz / length);
    let mut brakes = rail.kind == RailKind::Powered && !rail.on;
    if let Some(yaw) = how.pushed_to.filter(|_| vx * vx + vz * vz < PUSH_BELOW) {
        let (sin, cos) = yaw.to_radians().sin_cos();
        (vx, vz, brakes) = (vx - sin * PUSH, vz + cos * PUSH, false);
    }
    if brakes {
        (vx, vz) = if vx * vx + vz * vz >= BRAKE_STOP { (vx * 0.5, vz * 0.5) } else { (0.0, 0.0) };
    }
    // The cart is put on the rail's line, then moved along it.
    let ([x0, z0], [x1, z1]) = ([bx + 0.5 + start[0] * 0.5, bz + 0.5 + start[1] * 0.5], [bx + 0.5 + end[0] * 0.5, bz + 0.5 + end[1] * 0.5]);
    let (lx, lz) = (x1 - x0, z1 - z0);
    let along = if lx == 0.0 {
        feet[2] - bz
    } else if lz == 0.0 {
        feet[0] - bx
    } else {
        ((feet[0] - x0) * lx + (feet[2] - z0) * lz) * 2.0
    };
    let share = if how.ridden { RIDDEN_MOVE } else { 1.0 };
    let x = x0 + lx * along + (vx * share).clamp(-MOVE_CAP, MOVE_CAP);
    let z = z0 + lz * along + (vz * share).clamp(-MOVE_CAP, MOVE_CAP);
    let y = by + rise(rail.shape, [x - bx, z - bz]);
    let dropped = feet[1] - y;
    if dropped != 0.0 && speed > 0.0 {
        let gained = (vx.hypot(vz) + dropped * DOWNHILL).max(0.0) / vx.hypot(vz).max(f32::EPSILON);
        (vx, vz) = (vx * gained, vz * gained);
    }
    if rail.kind == RailKind::Powered && rail.on {
        let now = vx.hypot(vz);
        if now > BOOST_FROM {
            (vx, vz) = (vx + vx / now * BOOST, vz + vz / now * BOOST);
        } else if rail.shape == 0 {
            vz = if how.walls[0] { KICK } else if how.walls[1] { -KICK } else { vz };
        } else if rail.shape == 1 {
            vx = if how.walls[2] { KICK } else if how.walls[3] { -KICK } else { vx };
        }
    }
    ([x, y, z], [vx, 0.0, vz])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat(kind: RailKind, on: bool) -> Rail {
        Rail { pos: [0, 64, 0], shape: 0, kind, on }
    }

    const EMPTY: Roll = Roll { drag: 0.96, ridden: false, pushed_to: None, walls: [false; 4] };

    #[test]
    fn a_cart_rolls_along_its_rail_capped_a_tick_and_brakes_on_an_unpowered_rail() {
        let (at, motion) = roll(&flat(RailKind::Plain, false), [0.7, 64.0, 0.5], [0.3, 0.0, 1.0], &EMPTY);
        assert_eq!(at[0], 0.5, "put on the rail's line");
        assert!((at[2] - 0.9).abs() < 1e-6, "moved at most {MOVE_CAP}: {at:?}");
        assert!((motion[2] - 0.3f32.hypot(1.0) * 0.96).abs() < 1e-5 && motion[0] == 0.0, "turned along the rail: {motion:?}");
        let (_, braked) = roll(&flat(RailKind::Powered, false), [0.5, 64.0, 0.5], [0.0, 0.0, 0.5], &EMPTY);
        assert!((braked[2] - 0.24).abs() < 1e-6);
        let (_, boosted) = roll(&flat(RailKind::Powered, true), [0.5, 64.0, 0.5], [0.0, 0.0, 0.5], &EMPTY);
        assert!((boosted[2] - 0.54).abs() < 1e-6);
        let (_, kicked) = roll(&flat(RailKind::Powered, true), [0.5, 64.0, 0.5], [0.0; 3], &Roll { walls: [true, false, false, false], ..EMPTY });
        assert_eq!(kicked[2], KICK);
    }

    #[test]
    fn a_cart_runs_down_a_slope_and_round_a_curve() {
        let slope = Rail { pos: [0, 64, 0], shape: 3, kind: RailKind::Plain, on: false };
        let (at, motion) = roll(&slope, [0.5, 64.5, 0.5], [0.0; 3], &EMPTY);
        assert!(motion[0] > 0.0 && at[1] < 64.5, "rolls down east, the low end: {at:?} {motion:?}");
        let curve = Rail { pos: [0, 64, 0], shape: 6, kind: RailKind::Plain, on: false };
        let (_, turned) = roll(&curve, [0.5, 64.0, 0.9], [0.0, 0.0, -0.2], &EMPTY);
        assert!(turned[0] > 0.0 && turned[2] < 0.0, "in from the south end, out towards the east: {turned:?}");
    }
}
