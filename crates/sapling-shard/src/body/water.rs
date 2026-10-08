//! Where a body is in water, as BDS works it out at the start of each tick
//! (docs/research/water-movement.md); what water then does is the body's [`Water`] rule.

use sapling_world::World as Level;

use super::Body;

/// What water does to a kind of body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Water {
    /// Nothing: falling blocks fall through it as through air.
    Ignores,
    /// Lifted while its breathing point is under the surface, unless the water falls (items).
    Floats,
    /// Slowed and pulled down gently while its box is in it (mobs without AI).
    Sinks,
    /// Held up just under the surface, bobbing, before its own tick (orbs, by their
    /// `minecraft:buoyant`; BDS's `BuoyancySystem`).
    Buoyant,
}

/// What a buoyant body keeps of its fall speed, and the most it gains, a tick.
const BUOY_KEEP: f32 = 0.7;
const BUOY_LIFT: f32 = 0.05;
/// How the lift it rises to is worked out of how far into the top cell it is and the wave.
const BUOY_DEPTH: f32 = 0.9;
const BUOY_FLOOR: f32 = 0.1;
const BUOY_SCALE: f32 = 0.15;
const WAVE_HEIGHT: f32 = 0.035;
/// The wave moves on this much a tick, faster the faster the body drifts across.
const WAVE_STEP: f32 = 0.05;
const WAVE_DRIFT: f32 = 30.0;

/// Every body but an item is tested on its box with this much left off: a hair across, and
/// almost half a block at the top and at the bottom.
const SHRINK: [f32; 3] = [0.001, 0.401, 0.001];
/// An item breathes this far above its feet: 0.25 over the middle of its box.
const ITEM_BREATH: f32 = 0.375;

/// The `liquid_depth` of the water in `cell`, on either layer: 0 for a source, 8 and up for
/// falling water.
fn water_depth(level: &Level, [x, y, z]: [i32; 3]) -> Option<u8> {
    if !level.in_height(y) || !level.is_loaded(sapling_world::ChunkPos::of_block(x, z)) {
        return None;
    }
    [level.liquid(x, y, z), level.block(x, y, z)].into_iter().find_map(|id| {
        let state = level.registry().get(id).filter(|state| matches!(state.name, "minecraft:water" | "minecraft:flowing_water"))?;
        Some(state.property("liquid_depth").and_then(|depth| depth.parse().ok()).unwrap_or(0))
    })
}

/// The height of the surface of water `depth` deep in the cell at `y`: a source, and falling water,
/// fill it.
fn surface(y: i32, depth: u8) -> f32 {
    y as f32 + 1.0 - if depth < 8 { f32::from(depth) / 9.0 } else { 0.0 }
}

/// Whether any cell the body's box reaches, shrunk for all but items, holds water; no surface
/// height counts here.
pub(crate) fn in_water(level: &Level, body: &Body, water: Water) -> bool {
    let (mut low, mut high) = body.bounds();
    if water != Water::Floats {
        for axis in 0..3 {
            (low[axis], high[axis]) = (low[axis] + SHRINK[axis], high[axis] - SHRINK[axis]);
            if low[axis] > high[axis] {
                let middle = (body.bounds().0[axis] + body.bounds().1[axis]) / 2.0;
                (low[axis], high[axis]) = (middle, middle);
            }
        }
    }
    let cells = |axis: usize| low[axis].floor() as i32..=high[axis].floor() as i32;
    cells(0).any(|x| cells(1).any(|y| cells(2).any(|z| water_depth(level, [x, y, z]).is_some())))
}

/// Whether an item in water has its breathing point under the surface, a source's being the top of
/// its cell, and the water there is not falling.
fn lifts_item(level: &Level, body: &Body) -> bool {
    let [x, feet, z] = body.at;
    let breath = feet + ITEM_BREATH;
    let cell = [x.floor() as i32, breath.floor() as i32, z.floor() as i32];
    let under = water_depth(level, cell).is_some_and(|depth| surface(cell[1], depth) > breath);
    let middle = [x.floor() as i32, (feet + body.size[1] / 2.0).floor() as i32, z.floor() as i32];
    let falls = [middle, [middle[0], middle[1] - 1, middle[2]]].into_iter().any(|cell| water_depth(level, cell).is_some_and(|depth| depth > 7));
    in_water(level, body, Water::Floats) && under && !falls
}

/// Whether water acts on the body this tick, by its rule.
pub(super) fn acts_on(level: &Level, body: &Body, water: Water) -> bool {
    match water {
        Water::Ignores => false,
        Water::Floats => lifts_item(level, body),
        Water::Sinks => in_water(level, body, water),
        Water::Buoyant => buoyed(level, body).is_some(),
    }
}

/// How hard water holds a buoyant body up, when it does: fully in a cell with water over it; in
/// the top cell, under its surface, less the higher it is and the higher the wave.
fn buoyed(level: &Level, body: &Body) -> Option<f32> {
    let [x, y, z] = body.at;
    let cell = [x.floor() as i32, y.floor() as i32, z.floor() as i32];
    let depth = water_depth(level, cell)?;
    if water_depth(level, [cell[0], cell[1] + 1, cell[2]]).is_some() {
        return Some(1.0);
    }
    let lift = ((1.0 - (y - y.floor())) * BUOY_DEPTH + BUOY_FLOOR).clamp(0.0, 1.0);
    (y < surface(cell[1], depth)).then(|| lift - WAVE_HEIGHT * (body.wave.sin() + 1.0))
}

/// Holds a buoyant body up, before its own tick: its fall slows, and it rises toward a speed set
/// by [`buoyed`]; the wave moves on.
// TODO: BDS makes one wave in 33 ten times as long, at random.
pub(super) fn buoy(level: &Level, body: &mut Body) {
    let Some(lift) = buoyed(level, body) else { return };
    let rise = (lift - BUOY_FLOOR) * BUOY_SCALE;
    body.motion[1] = (body.motion[1] * BUOY_KEEP + BUOY_LIFT).min(rise);
    body.wave += WAVE_STEP * (1.0 + WAVE_DRIFT * body.motion[0].hypot(body.motion[2]));
}

#[cfg(test)]
mod tests {
    use super::super::Physics;
    use super::*;

    /// A void world with a pool of still water from y 60 to 64 over stone, and a column of falling
    /// water at x 3.
    fn pool() -> Level {
        let mut level = Level::void();
        let (stone, water) = (level.block_id("minecraft:stone").unwrap(), level.block_id("minecraft:water").unwrap());
        level.fill([-4, 59, -4], [4, 59, 4], stone);
        level.fill([-4, 60, -4], [4, 64, 4], water);
        let falling = level.registry().find("minecraft:water", "liquid_depth=8").unwrap();
        level.fill([3, 60, 0], [3, 64, 0], falling);
        level
    }

    fn after(mut body: Body, physics: &Physics, ticks: usize) -> Body {
        let level = pool();
        (0..ticks).for_each(|_| _ = body.step(&level, physics));
        body
    }

    #[test]
    fn an_item_rises_to_float_with_its_breathing_point_at_the_surface() {
        let item = after(Body::new([0.5, 61.0, 0.5], [0.0; 3], [0.25, 0.25]), &Physics::ITEM, 400);
        // A source fills its cell: the surface is at 65, which the item bobs about with its
        // breathing point, 0.375 over its feet.
        assert!((item.at[1] + ITEM_BREATH - 65.0).abs() < 0.3, "{item:?}");
        let rising = after(Body::new([0.5, 61.0, 0.5], [0.0; 3], [0.25, 0.25]), &Physics::ITEM, 20);
        assert!(rising.motion[1] > 0.0 && rising.at[1] > 61.0, "{rising:?}");
    }

    #[test]
    fn falling_water_lets_an_item_fall_and_water_lets_falling_blocks_fall() {
        let item = after(Body::new([3.5, 63.0, 0.5], [0.0; 3], [0.25, 0.25]), &Physics::ITEM, 40);
        assert_eq!(item.at[1], 60.0, "down falling water to the floor");
        let block = after(Body::new([0.5, 63.0, 0.5], [0.0; 3], [0.98, 0.98]), &Physics::FALLING_BLOCK, 40);
        assert_eq!(block.at[1], 60.0);
    }

    #[test]
    fn an_orb_slows_as_bds_slowed_one_and_bobs_just_under_the_surface() {
        // BDS's orb went into still water at 0.3162 a tick and was next moving at 0.2069
        // (research/captures/bds-water.txt).
        let mut orb = Body::new([0.5, 64.5, 0.5], [0.0, -0.3162 * 0.98, 0.0], [0.25, 0.25]);
        let level = pool();
        let moved = orb.step(&level, &Physics::ORB);
        assert!((moved[1] + 0.2069).abs() < 1e-3, "{moved:?}");
        (0..300).for_each(|_| _ = orb.step(&level, &Physics::ORB));
        assert!(orb.at[1] > 65.0 - 0.4 && orb.at[1] < 65.0 - 0.25, "{orb:?}");
    }

    #[test]
    fn a_mob_sinks_slowly_and_is_slowed_across() {
        let mut mob = Body::new([0.5, 63.0, 0.5], [0.3, 0.0, 0.0], [0.9, 0.9]);
        let level = pool();
        (0..25).for_each(|_| _ = mob.step(&level, &Physics::MOB));
        // Drag 0.8 and a pull of 0.02 settle at a tenth of a block a tick.
        assert!((mob.motion[1] + 0.1).abs() < 1e-3 && mob.motion[0].abs() < 0.01, "{mob:?}");
        assert!(mob.at[1] < 63.0 && mob.at[1] > 60.0, "{mob:?}");
    }
}
