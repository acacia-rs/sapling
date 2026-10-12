//! One side of a fence, pane or bar redone toward a neighbour that changed: what the neighbour-update component
//! (updater 1439cdbd0) does when a `setBlock` with flag 1 or 0x20 lands beside it (feature writes use 3).
//! For callers outside the structure stage (feature/view.rs); stairs are left alone there.

use super::connect::{SIDES, Shape, shape, with_states};
use crate::block::Block;

/// Whether a fence-like block of `kind` joins `neighbour` at `(dx, dz)`: a full face toward it by the registry's
/// support classes (a pointed dripstone has none, though connect.rs' full-cube test takes it), or its own kind.
fn joins(kind: Shape, neighbour: Block, (dx, dz): (i32, i32)) -> bool {
    // FacingID of the neighbour's face that looks back at the fence.
    let face = match (dx, dz) {
        (1, _) => 4,
        (-1, _) => 5,
        (_, 1) => 2,
        _ => 3,
    };
    let other = shape(neighbour);
    neighbour.supports_multiface_plain(face)
        || match kind {
            Shape::WoodenFence | Shape::NetherFence => other == Some(kind) || neighbour.name().ends_with("fence_gate"),
            _ => other == Some(Shape::Pane) || neighbour.states().contains("wall_post_bit"),
        }
}

/// `block` with its side toward `(dx, dz)` set from `neighbour`, or None when it has no such side or keeps it.
pub(crate) fn side_toward(block: Block, neighbour: Block, (dx, dz): (i32, i32)) -> Option<Block> {
    let kind = shape(block).filter(|&kind| kind != Shape::Stairs)?;
    let (side, ..) = SIDES.iter().find(|&&(_, sx, sz)| (sx, sz) == (dx, dz))?;
    let new = with_states(block, &[(format!("minecraft:connection_{side}"), u8::from(joins(kind, neighbour, (dx, dz))).to_string())]);
    (new != block).then_some(new)
}
