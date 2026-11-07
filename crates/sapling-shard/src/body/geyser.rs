//! An erupting geyser lifting what is over it: BDS's `GeyserBoostSystem` (Linux 0x753a640, docs/research/
//! world-ticks.md "Potent sulfur"). Down from the cell at the body's feet, the first cell with a collision box
//! within 24 must be a potent sulfur erupting open to the top; under the plume's top its speed up gains 0.2
//! a tick while below the plume's speed.

use sapling_world::World as Level;

use super::Body;

/// Cells looked at under the feet, the feet's own after the first.
const DOWN: i32 = 24;
const LIFT: f32 = 0.2;

/// Speeds the body up if it is over an erupting geyser.
pub(super) fn lift(level: &Level, body: &mut Body) {
    body.motion[1] += boost(level, body, body.motion[1]);
}

/// What an erupting geyser under the body adds to its speed up `rising`: BDS's system runs after a
/// mob's move and drag, so a mob moves by it a tick later (body.rs `mob_step`; bds-ag2boost.txt: the
/// pig's first rise in the eruption's tick is unboosted, its told speed 0.2856 already boosted).
pub(super) fn boost(level: &Level, body: &Body, rising: f32) -> f32 {
    let (low, high) = body.bounds();
    let [x, z] = [0, 2].map(|axis| ((high[axis] - low[axis]) * 0.5 + low[axis]).floor() as i32);
    let feet = low[1].floor() as i32;
    let registry = level.registry();
    let solid = |y: i32| level.in_height(y) && registry.get(level.block(x, y, z)).is_some_and(|state| !state.boxes.is_empty());
    let Some(y) = (0..=DOWN).map(|down| feet - down).find(|&y| solid(y)) else { return 0.0 };
    let Some((speed, reach)) = crate::blocks::geyser_plume(level, [x, y, z]) else { return 0.0 };
    if low[1] < (y + reach) as f32 && rising < speed { LIFT } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::super::Physics;
    use super::*;

    /// A potent sulfur at y 60 in `state` under one water source, open air over it.
    fn geyser(state: &str) -> Level {
        let mut level = Level::void();
        let sulfur = level.registry().find("minecraft:potent_sulfur", &format!("potent_sulfur_state={state}")).unwrap();
        let water = level.registry().find("minecraft:water", "liquid_depth=0").unwrap();
        level.fill([0, 60, 0], [0, 60, 0], sulfur);
        level.fill([0, 61, 0], [0, 61, 0], water);
        level
    }

    fn highest(state: &str) -> f32 {
        let level = geyser(state);
        let mut body = Body::new([0.5, 61.0, 0.5], [0.0; 3], [0.25, 0.25]);
        (0..40).map(|_| {
            _ = body.step(&level, &Physics::ITEM, &mut || 0.5);
            body.at[1]
        }).fold(f32::MIN, f32::max)
    }

    /// One water over it: lifted until the feet pass y 67, then coasting at most 0.6 up against item gravity.
    #[test]
    fn an_erupting_geyser_lifts_a_body_to_its_plume_and_a_dormant_one_does_not() {
        let lifted = highest("erupting");
        assert!((67.0..72.0).contains(&lifted), "{lifted}");
        assert!(highest("dormant") < 62.5);
    }
}
