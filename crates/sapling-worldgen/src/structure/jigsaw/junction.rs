//! v2 ground level deltas and junctions (Java's `JigsawPlacement.Placer.tryPlacingChildren`),
//! which the terrain adjustments read (`structure::adjust`).

use super::super::adjust::Junction;
use super::Piece;
use super::element::JigsawInfo;

/// Sets `child`'s ground level delta and its junction back to `source`, attached through
/// `jigsaw` (on `source`) and `candidate` (template-relative, on `child`); returns `source`'s
/// junction to the child. `surface` is the first free height at `jigsaw`.
pub fn link(source: &Piece, jigsaw: &JigsawInfo, child: &mut Piece, candidate: &JigsawInfo, surface: i32) -> Junction {
    let relative = jigsaw.pos.1 - source.bb.y0;
    let step = jigsaw.front.offset();
    let k = relative - candidate.pos.1 + step.1;
    child.ground = if child.is_rigid() { source.ground - k } else { 1 };
    let y = if source.is_rigid() {
        jigsaw.pos.1
    } else if child.is_rigid() {
        child.bb.y0 + candidate.pos.1
    } else {
        surface + k / 2
    };
    child.junctions.push(Junction { x: jigsaw.pos.0, ground: y - candidate.pos.1 + child.ground, z: jigsaw.pos.2 });
    Junction { x: jigsaw.pos.0 + step.0, ground: y - relative + source.ground, z: jigsaw.pos.2 + step.2 }
}
