//! Where a body is in water or lava, as BDS works it out at the start of each tick
//! (docs/research/water-movement.md), and what the liquid then does by the body's [`InLiquid`]
//! rule. Matched to BDS's captures: research/captures/bds-water.txt and bds-lava.txt.

use sapling_world::World as Level;

use super::Body;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Liquid {
    Water,
    Lava,
}

impl Liquid {
    pub(crate) fn of(name: &str) -> Option<Liquid> {
        match name {
            "minecraft:water" | "minecraft:flowing_water" => Some(Liquid::Water),
            "minecraft:lava" | "minecraft:flowing_lava" => Some(Liquid::Lava),
            _ => None,
        }
    }
}

/// What water and lava do to a kind of body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InLiquid {
    /// Nothing: falling blocks fall through both as through air.
    Ignores,
    /// Lifted while its breathing point is under water's surface or its box touches lava, unless
    /// the liquid falls (items).
    Floats,
    /// Slowed and pulled down gently while its box is in either (mobs without AI).
    Sinks,
    /// Held up just under water's surface, bobbing, before its own tick (orbs, by their
    /// `minecraft:buoyant`; BDS's `BuoyancySystem`); thrown up out of lava.
    Buoyant,
}

/// An item lifted rises toward this speed, closing this much of the gap a tick
/// (BDS `ItemActor::postNormalTick`), and keeps this much of its speed across before the usual drag.
const RISE: f32 = 0.06;
const RISE_EASE: f32 = 0.01;
const RISE_NUDGE: f32 = 0.0005;
const FLOAT_DRAG_WATER: f32 = 0.99;
const FLOAT_DRAG_LAVA: f32 = 0.95;
/// What a mob keeps of its speed each tick, and its pull down (BDS `WaterDrag::apply`,
/// `MobMovementDrag::Lava`, `MobWaterGravity`, `MobLavaGravity`).
const SINK_DRAG_WATER: f32 = 0.8;
const SINK_DRAG_LAVA: f32 = 0.5;
const SINK_GRAVITY: f32 = 0.02;
/// An orb in lava is thrown up this fast, and up to this much either way across (BDS
/// `ExperienceOrb::normalTick`).
const THROW_UP: f32 = 0.2;
const THROW_SPREAD: f32 = 0.2;

/// What a buoyant body keeps of its fall speed, and the most it gains, a tick.
const BUOY_KEEP: f32 = 0.7;
const BUOY_LIFT: f32 = 0.05;
/// How the lift it rises to is worked out of how far into the top cell it is and the wave.
const BUOY_DEPTH: f32 = 0.9;
const BUOY_FLOOR: f32 = 0.1;
const BUOY_SCALE: f32 = 0.15;
const WAVE_HEIGHT: f32 = 0.035;
/// The wave moves on this much a tick, faster the faster the body drifts across, and by chance
/// ten times as far (BDS's `big_wave_probability`, `big_wave_speed`).
const WAVE_STEP: f32 = 0.05;
const WAVE_DRIFT: f32 = 30.0;
const BIG_WAVE_CHANCE: f32 = 0.03;
const BIG_WAVE: f32 = 10.0;

/// Every body but an item is tested on its box with this much left off: a hair across, and
/// almost half a block at the top and at the bottom.
const SHRINK: [f32; 3] = [0.001, 0.401, 0.001];
/// An item breathes this far above its feet: 0.25 over the middle of its box.
const ITEM_BREATH: f32 = 0.375;

/// The `liquid_depth` of `liquid` in `cell`, on either layer: 0 for a source, 8 and up for a
/// falling one.
fn depth(level: &Level, [x, y, z]: [i32; 3], liquid: Liquid) -> Option<u8> {
    if !level.in_height(y) || !level.is_loaded(sapling_world::ChunkPos::of_block(x, z)) {
        return None;
    }
    let registry = level.registry();
    [level.liquid(x, y, z), level.block(x, y, z)].into_iter().find_map(|id| registry.get(id).filter(|state| Liquid::of(state.name) == Some(liquid)).map(|state| state.liquid_depth))
}

/// The height of the surface of a liquid `depth` deep in the cell at `y`: a source, and a falling
/// liquid, fill it.
fn surface(y: i32, depth: u8) -> f32 {
    y as f32 + 1.0 - if depth < 8 { f32::from(depth) / 9.0 } else { 0.0 }
}

/// Whether any cell the body's box reaches, shrunk for all but items, holds `liquid`; no surface
/// height counts here.
pub(crate) fn in_liquid(level: &Level, body: &Body, rule: InLiquid, liquid: Liquid) -> bool {
    let (mut low, mut high) = body.bounds();
    if rule != InLiquid::Floats {
        for axis in 0..3 {
            (low[axis], high[axis]) = (low[axis] + SHRINK[axis], high[axis] - SHRINK[axis]);
            if low[axis] > high[axis] {
                let middle = (body.bounds().0[axis] + body.bounds().1[axis]) / 2.0;
                (low[axis], high[axis]) = (middle, middle);
            }
        }
    }
    let cells = |axis: usize| low[axis].floor() as i32..=high[axis].floor() as i32;
    cells(0).any(|x| cells(1).any(|y| cells(2).any(|z| depth(level, [x, y, z], liquid).is_some())))
}

/// Whether `liquid` falls where the middle of the body is, or just under it.
fn falls(level: &Level, body: &Body, liquid: Liquid) -> bool {
    let [x, feet, z] = body.at;
    let middle = [x.floor() as i32, (feet + body.size[1] / 2.0).floor() as i32, z.floor() as i32];
    [middle, [middle[0], middle[1] - 1, middle[2]]].into_iter().any(|cell| depth(level, cell, liquid).is_some_and(|depth| depth > 7))
}

/// What lifts an item: water its breathing point is under, a source's surface being the top of its
/// cell, or else lava its box touches; neither where it falls.
fn lifting_item(level: &Level, body: &Body) -> Option<Liquid> {
    let [x, feet, z] = body.at;
    let breath = feet + ITEM_BREATH;
    let cell = [x.floor() as i32, breath.floor() as i32, z.floor() as i32];
    let under = depth(level, cell, Liquid::Water).is_some_and(|depth| surface(cell[1], depth) > breath);
    let liquid = if in_liquid(level, body, InLiquid::Floats, Liquid::Water) && under {
        Liquid::Water
    } else if in_liquid(level, body, InLiquid::Floats, Liquid::Lava) {
        Liquid::Lava
    } else {
        return None;
    };
    (!falls(level, body, liquid)).then_some(liquid)
}

/// The liquid that acts on the body this tick, by its rule.
pub(super) fn acting(level: &Level, body: &Body, rule: InLiquid) -> Option<Liquid> {
    let cell = body.at.map(|c| c.floor() as i32);
    match rule {
        InLiquid::Ignores => None,
        InLiquid::Floats => lifting_item(level, body),
        InLiquid::Sinks => [Liquid::Water, Liquid::Lava].into_iter().find(|liquid| in_liquid(level, body, rule, *liquid)),
        InLiquid::Buoyant if buoyed(level, body).is_some() => Some(Liquid::Water),
        InLiquid::Buoyant => depth(level, cell, Liquid::Lava).map(|_| Liquid::Lava),
    }
}

/// Lifts an item in `liquid` instead of its gravity, and slows it across.
pub(super) fn lift(body: &mut Body, liquid: Liquid) {
    let nudged = body.motion[1] + RISE_NUDGE;
    body.motion[1] = nudged + (RISE - nudged) * RISE_EASE;
    let drag = if liquid == Liquid::Water { FLOAT_DRAG_WATER } else { FLOAT_DRAG_LAVA };
    (body.motion[0], body.motion[2]) = (body.motion[0] * drag, body.motion[2] * drag);
}

/// Slows a mob in `liquid` and pulls it down, in place of the air's drag and gravity.
pub(super) fn sink(body: &mut Body, liquid: Liquid) {
    let drag = if liquid == Liquid::Water { SINK_DRAG_WATER } else { SINK_DRAG_LAVA };
    body.motion = body.motion.map(|speed| speed * drag);
    body.motion[1] -= SINK_GRAVITY;
}

/// Throws an orb up out of lava, aside by the difference of two `random` fractions each way.
pub(super) fn throw_up(body: &mut Body, random: &mut impl FnMut() -> f32) {
    let mut aside = || (random() - random()) * THROW_SPREAD;
    let x = aside();
    body.motion = [x, THROW_UP, aside()];
}

/// How hard water holds a buoyant body up, when it does: fully in a cell with water over it; in
/// the top cell, under its surface, less the higher it is and the higher the wave.
fn buoyed(level: &Level, body: &Body) -> Option<f32> {
    let [x, y, z] = body.at;
    let cell = [x.floor() as i32, y.floor() as i32, z.floor() as i32];
    let depth_here = depth(level, cell, Liquid::Water)?;
    if depth(level, [cell[0], cell[1] + 1, cell[2]], Liquid::Water).is_some() {
        return Some(1.0);
    }
    let lift = ((1.0 - (y - y.floor())) * BUOY_DEPTH + BUOY_FLOOR).clamp(0.0, 1.0);
    (y < surface(cell[1], depth_here)).then(|| lift - WAVE_HEIGHT * (body.wave.sin() + 1.0))
}

/// Holds a buoyant body up, before its own tick: its fall slows, and it rises toward a speed set
/// by [`buoyed`]; the wave moves on, drawing on `random` for a big one.
pub(super) fn buoy(level: &Level, body: &mut Body, random: &mut impl FnMut() -> f32) {
    let Some(lift) = buoyed(level, body) else { return };
    let rise = (lift - BUOY_FLOOR) * BUOY_SCALE;
    body.motion[1] = (body.motion[1] * BUOY_KEEP + BUOY_LIFT).min(rise);
    let big = if random() < BIG_WAVE_CHANCE { BIG_WAVE } else { 1.0 };
    body.wave += WAVE_STEP * (1.0 + WAVE_DRIFT * body.motion[0].hypot(body.motion[2])) * big;
}

#[cfg(test)]
mod tests {
    use super::super::Physics;
    use super::*;

    /// A void world with a pool of still `liquid` from y 60 to 64 over stone, and a column of it
    /// falling at x 3.
    fn pool(liquid: &str) -> Level {
        let mut level = Level::void();
        let (stone, still) = (level.block_id("minecraft:stone").unwrap(), level.block_id(liquid).unwrap());
        level.fill([-4, 59, -4], [4, 59, 4], stone);
        level.fill([-4, 60, -4], [4, 64, 4], still);
        let falling = level.registry().find(liquid, "liquid_depth=8").unwrap();
        level.fill([3, 60, 0], [3, 64, 0], falling);
        level
    }

    fn step(body: &mut Body, level: &Level, physics: &Physics) -> [f32; 3] {
        body.step(level, physics, &mut || 0.5)
    }

    fn after(mut body: Body, liquid: &str, physics: &Physics, ticks: usize) -> Body {
        let level = pool(liquid);
        (0..ticks).for_each(|_| _ = step(&mut body, &level, physics));
        body
    }

    #[test]
    fn an_item_rises_to_float_with_its_breathing_point_at_the_surface() {
        let item = after(Body::new([0.5, 61.0, 0.5], [0.0; 3], [0.25, 0.25]), "minecraft:water", &Physics::ITEM, 400);
        // A source fills its cell: the surface is at 65, which the item bobs about with its
        // breathing point, 0.375 over its feet.
        assert!((item.at[1] + ITEM_BREATH - 65.0).abs() < 0.3, "{item:?}");
        let rising = after(Body::new([0.5, 61.0, 0.5], [0.0; 3], [0.25, 0.25]), "minecraft:water", &Physics::ITEM, 20);
        assert!(rising.motion[1] > 0.0 && rising.at[1] > 61.0, "{rising:?}");
    }

    #[test]
    fn falling_water_lets_an_item_fall_and_water_lets_falling_blocks_fall() {
        let item = after(Body::new([3.5, 63.0, 0.5], [0.0; 3], [0.25, 0.25]), "minecraft:water", &Physics::ITEM, 40);
        assert_eq!(item.at[1], 60.0, "down falling water to the floor");
        let block = after(Body::new([0.5, 63.0, 0.5], [0.0; 3], [0.98, 0.98]), "minecraft:water", &Physics::FALLING_BLOCK, 40);
        assert_eq!(block.at[1], 60.0);
    }

    #[test]
    fn an_item_in_lava_is_lifted_and_slowed_across_as_on_bds() {
        // BDS's item went into lava moving -0.43659 a tick, then -0.42249 (research/captures/bds-lava.txt).
        let mut item = Body::new([0.5, 64.8, 0.5], [0.1, -0.43659 * 0.98, 0.0], [0.25, 0.25]);
        let moved = step(&mut item, &pool("minecraft:lava"), &Physics::ITEM);
        assert!((moved[1] + 0.42249).abs() < 1e-4 && (moved[0] - 0.095).abs() < 1e-6, "{moved:?}");
        assert_eq!(item.liquid(), Some(Liquid::Lava));
    }

    #[test]
    fn an_orb_slows_as_bds_slowed_one_and_bobs_just_under_the_surface() {
        // BDS's orb went into still water at 0.3162 a tick and was next moving at 0.2069
        // (research/captures/bds-water.txt).
        let mut orb = Body::new([0.5, 64.5, 0.5], [0.0, -0.3162 * 0.98, 0.0], [0.25, 0.25]);
        let level = pool("minecraft:water");
        let moved = step(&mut orb, &level, &Physics::ORB);
        assert!((moved[1] + 0.2069).abs() < 1e-3, "{moved:?}");
        (0..300).for_each(|_| _ = step(&mut orb, &level, &Physics::ORB));
        assert!(orb.at[1] > 65.0 - 0.4 && orb.at[1] < 65.0 - 0.25, "{orb:?}");
    }

    #[test]
    fn an_orb_in_lava_is_thrown_up_and_aside_by_chance() {
        let mut orb = Body::new([0.5, 64.9, 0.5], [0.1, -0.3, 0.1], [0.25, 0.25]);
        let mut draws = [0.9, 0.1, 0.3, 0.8].into_iter();
        let moved = orb.step(&pool("minecraft:lava"), &Physics::ORB, &mut || draws.next().unwrap());
        assert!((moved[0] - 0.16).abs() < 1e-6 && moved[1] == THROW_UP && (moved[2] + 0.1).abs() < 1e-6, "{moved:?}");
    }

    #[test]
    fn a_mob_sinks_slowly_and_is_slowed_across() {
        let mob = after(Body::new([0.5, 63.0, 0.5], [0.3, 0.0, 0.0], [0.9, 0.9]), "minecraft:water", &Physics::MOB, 25);
        // Drag 0.8 and a pull of 0.02 settle at a tenth of a block a tick.
        assert!((mob.motion[1] + 0.1).abs() < 1e-3 && mob.motion[0].abs() < 0.01, "{mob:?}");
        assert!(mob.at[1] < 63.0 && mob.at[1] > 60.0, "{mob:?}");
    }

    #[test]
    fn a_mob_sinks_through_lava_as_bds_sank_an_armour_stand() {
        // Lava halves its speed and pulls 0.02: BDS's stand went from -0.51696 a tick to -0.27848,
        // and on to -0.04.
        // The first tick goes by the air's rule, a tick late, to -0.51696.
        let mob = after(Body::new([0.5, 64.0, 0.5], [0.0, -0.44751, 0.0], [0.5, 1.975]), "minecraft:lava", &Physics::MOB, 2);
        assert!((mob.motion[1] + 0.27848).abs() < 1e-4, "{mob:?}");
        let sunk = after(Body::new([0.5, 64.5, 0.5], [0.0; 3], [0.5, 1.975]), "minecraft:lava", &Physics::MOB, 30);
        assert!((sunk.motion[1] + 0.04).abs() < 1e-3, "{sunk:?}");
    }
}
