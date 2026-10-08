//! Wall connections and posts from the final neighbours (Java's `WallBlock.updateShape`, sides
//! `short`/`tall` as Bedrock saves them).

use super::connect::{SIDES, Shape, full_cube, shape, state, sturdy};
use crate::block::Block;
use crate::structure::canvas::Canvas;

pub fn is_wall(block: Block) -> bool {
    block.states().contains("wall_post_bit")
}

fn side_key(side: &str) -> String {
    format!("wall_connection_type_{side}")
}

/// A fence gate turned across `(dx, dz)`, so a wall beside it joins it.
fn gate_across(block: Block, (dx, _): (i32, i32)) -> bool {
    block.name().ends_with("fence_gate")
        && matches!(state(block, "minecraft:cardinal_direction"), Some("north" | "south")) == (dx != 0)
}

fn joins(neighbour: Block, side: (i32, i32)) -> bool {
    is_wall(neighbour) || shape(neighbour) == Some(Shape::Pane) || gate_across(neighbour, side) || sturdy(neighbour, side)
}

/// Whether the block above covers a wall side (`side` named) or the post (`None`) from above.
fn covered(above: Block, side: Option<&str>) -> bool {
    if full_cube(above) || shape(above) == Some(Shape::Stairs) && state(above, "upside_down_bit") == Some("0") {
        return true;
    }
    if state(above, "minecraft:vertical_half") == Some("bottom") {
        return true;
    }
    is_wall(above)
        && match side {
            Some(side) => state(above, &side_key(side)).is_some_and(|v| v != "none"),
            None => state(above, "wall_post_bit") == Some("1"),
        }
}

/// The wall at (x, y, z)'s side connections and post.
pub fn changes(canvas: &Canvas, (x, y, z): (i32, i32, i32)) -> Vec<(String, String)> {
    let above = canvas.get(x, y + 1, z);
    let sides: Vec<&str> = SIDES
        .iter()
        .map(|&(side, dx, dz)| match joins(canvas.get(x + dx, y, z + dz), (dx, dz)) {
            false => "none",
            true if covered(above, Some(side)) => "tall",
            true => "short",
        })
        .collect();
    let [east, north, south, west] = [sides[0], sides[1], sides[2], sides[3]];
    let none = |s: &str| s == "none";
    let post = if is_wall(above) && state(above, "wall_post_bit") == Some("1") {
        true
    } else if sides.iter().all(|s| none(s)) || none(north) != none(south) || none(east) != none(west) {
        true
    } else if north == "tall" && south == "tall" || east == "tall" && west == "tall" {
        false
    } else {
        covered(above, None) || above.name().contains("torch") || above.name().contains("sign") || above.name().contains("banner")
    };
    let mut changes: Vec<(String, String)> = SIDES.iter().zip(&sides).map(|((side, ..), v)| (side_key(side), v.to_string())).collect();
    changes.push(("wall_post_bit".into(), u8::from(post).to_string()));
    changes
}
