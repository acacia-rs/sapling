//! A bubble column lifting or dragging what is in it: BDS's `EntityInsideSystem` over
//! `InsideBubbleColumnBlockComponent` (1446abb90, 14469a100), which changes the speed up alone and
//! takes no account of what the body is. A body in the column's top cell, air over it, is "above"
//! it and thrown harder. Players move themselves.

use sapling_world::World as Level;

use super::Body;

const COLUMN: &str = "minecraft:bubble_column";
/// What a tick adds up (or down) and the speed it stops at: inside a rising column, above one,
/// inside a dragging one, above one.
const INSIDE_UP: (f32, f32) = (0.06, 0.7);
const ABOVE_UP: (f32, f32) = (0.1, 1.8);
const INSIDE_DOWN: (f32, f32) = (0.03, 0.3);
const ABOVE_DOWN: (f32, f32) = (0.03, 0.9);

/// The column the body's box reaches into, as (drags down, above it): the first found from its feet.
fn column(level: &Level, body: &Body) -> Option<(bool, bool)> {
    const EDGE: f32 = 1e-4;
    let (low, high) = body.bounds();
    let cells = |axis: usize| low[axis].floor() as i32..=(high[axis] - EDGE).floor() as i32;
    let registry = level.registry();
    for y in cells(1) {
        for x in cells(0) {
            for z in cells(2) {
                if !level.in_height(y) {
                    continue;
                }
                let Some(state) = registry.get(level.block(x, y, z)).filter(|state| state.name == COLUMN) else { continue };
                let over_air = level.in_height(y + 1) && registry.get(level.block(x, y + 1, z)).is_some_and(|over| over.is_air());
                return Some((state.properties.contains("drag_down=1"), over_air));
            }
        }
    }
    None
}

/// Changes the body's speed up by the column it is in, if one.
pub(super) fn push(level: &Level, body: &mut Body) {
    let Some((drags_down, above)) = column(level, body) else { return };
    let vy = &mut body.motion[1];
    *vy = match (drags_down, above) {
        (false, false) => (*vy + INSIDE_UP.0).min(INSIDE_UP.1),
        (false, true) => (*vy + ABOVE_UP.0).min(ABOVE_UP.1),
        (true, false) => (*vy - INSIDE_DOWN.0).max(-INSIDE_DOWN.1),
        (true, true) => (*vy - ABOVE_DOWN.0).max(-ABOVE_DOWN.1),
    };
}

#[cfg(test)]
mod tests {
    use super::super::Physics;
    use super::*;

    fn tube(drags_down: bool) -> Level {
        let mut level = Level::void();
        let stone = level.block_id("minecraft:stone").unwrap();
        let column = level.registry().find(COLUMN, if drags_down { "drag_down=1" } else { "drag_down=0" }).unwrap();
        level.fill([-1, 59, -1], [1, 59, 1], stone);
        level.fill([0, 60, 0], [0, 69, 0], column);
        level
    }

    fn after(drags_down: bool, ticks: usize) -> Body {
        let level = tube(drags_down);
        let mut body = Body::new([0.5, 64.0, 0.5], [0.0; 3], [0.25, 0.25]);
        (0..ticks).for_each(|_| _ = body.step(&level, &Physics::ITEM, &mut || 0.5));
        body
    }

    #[test]
    fn a_rising_column_lifts_an_item_out_of_its_top_and_a_dragging_one_pulls_it_down() {
        let lifted = after(false, 40);
        assert!(lifted.at[1] > 69.0, "{lifted:?}");
        let dragged = after(true, 20);
        assert!(dragged.at[1] < 61.0 && dragged.motion[1] >= -INSIDE_DOWN.1 - 0.05, "{dragged:?}");
    }
}
