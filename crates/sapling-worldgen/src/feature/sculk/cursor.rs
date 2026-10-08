//! `SculkSpreader::updateCursors` (148b48870) and its charge cursors, worldgen only: cursors are
//! never merged and one that strays more than sqrt(224) from the patch's origin loses its charge.

use super::behaviour::{Behaviour, is_vein};
use crate::feature::spreader::face_sturdy;
use crate::feature::view::View;
use crate::random::Mt19937;
use crate::block::Block;

/// The non-corner neighbour offsets (table 0x14a94bf78), shuffled per move.
const NEIGHBOURS: [[i32; 3]; 18] = [
    [-1, -1, 0], [0, -1, -1], [0, -1, 0], [0, -1, 1], [1, -1, 0], [-1, 0, -1], [-1, 0, 0], [-1, 0, 1], [0, 0, -1],
    [0, 0, 1], [1, 0, -1], [1, 0, 0], [1, 0, 1], [-1, 1, 0], [0, 1, -1], [0, 1, 0], [0, 1, 1], [1, 1, 0],
];

/// No facings yet; sculk's (empty) facings; else a vein's face bits.
pub const FACINGS_UNSET: i32 = -1;
pub const FACINGS_EMPTY: i32 = -2;

pub struct Cursor {
    pub pos: [i32; 3],
    pub charge: i32,
    update_delay: i32,
    pub decay_delay: i32,
    pub facings: i32,
}

impl Cursor {
    pub fn new(pos: [i32; 3], charge: i32) -> Self {
        Self { pos, charge, update_delay: 0, decay_delay: 1, facings: FACINGS_UNSET }
    }
}

pub fn update_cursors(view: &mut View, cursors: &mut Vec<Cursor>, origin: [i32; 3], random: &mut Mt19937, spread_veins: bool) {
    for cursor in cursors.iter_mut() {
        if cursor.charge > 0 {
            update(view, cursor, origin, random, spread_veins);
        }
    }
    cursors.retain(|cursor| cursor.charge > 0);
}

fn block_at(view: &View, [x, y, z]: [i32; 3]) -> Block {
    view.block(x, y, z)
}

fn update(view: &mut View, cursor: &mut Cursor, origin: [i32; 3], random: &mut Mt19937, spread_veins: bool) {
    if cursor.update_delay >= 1 {
        cursor.update_delay -= 1;
        return;
    }
    let mut state = block_at(view, cursor.pos);
    let mut behaviour = Behaviour::of(state);
    if spread_veins && behaviour.attempt_spread_vein(view, cursor.pos, state, cursor.facings) && behaviour.changes_on_spread() {
        state = block_at(view, cursor.pos);
        behaviour = Behaviour::of(state);
    }
    cursor.charge = behaviour.attempt_use_charge(view, cursor, origin, random, spread_veins);
    if cursor.charge < 1 {
        behaviour.on_discharged(view, cursor.pos);
        return;
    }
    if let Some(next) = movement_target(view, cursor.pos, random) {
        behaviour.on_discharged(view, cursor.pos);
        cursor.pos = next;
        let d = [next[0] - origin[0], next[1] - origin[1], next[2] - origin[2]];
        if d[0] * d[0] + d[1] * d[1] + d[2] * d[2] > 224 {
            cursor.charge = 0;
            return;
        }
        state = block_at(view, next);
    }
    if state == Block::Sculk || is_vein(state) {
        behaviour = Behaviour::of(state);
        cursor.facings = behaviour.facings(cursor.facings, state);
    }
    cursor.update_delay = 1;
    cursor.decay_delay = behaviour.decay_delay(cursor.decay_delay);
}

/// `getValidMovementPos`: the last sculk or vein neighbour in shuffled order that is not walled
/// off, or the first vein with sculk-able ground behind a face.
fn movement_target(view: &View, pos: [i32; 3], random: &mut Mt19937) -> Option<[i32; 3]> {
    let mut offsets = NEIGHBOURS;
    for i in 1..offsets.len() {
        let j = (random.next_u32() % (i as u32 + 1)) as usize;
        offsets.swap(i, j);
    }
    let mut found = None;
    for [dx, dy, dz] in offsets {
        let next = [pos[0] + dx, pos[1] + dy, pos[2] + dz];
        let block = block_at(view, next);
        if !(block == Block::Sculk || is_vein(block)) || !unobstructed(view, pos, [dx, dy, dz]) {
            continue;
        }
        found = Some(next);
        if is_vein(block) && super::behaviour::substrate_access(view, block, next) {
            break;
        }
    }
    found
}

/// A diagonal move needs one of the two cells it passes free of a sturdy face toward it.
fn unobstructed(view: &View, pos: [i32; 3], [dx, dy, dz]: [i32; 3]) -> bool {
    if dx.abs() + dy.abs() + dz.abs() == 1 {
        return true;
    }
    let along = |axis: usize, sign: i32| {
        let mut p = pos;
        p[axis] += sign;
        p
    };
    let (a, b) = if dx == 0 {
        (along(1, dy), along(2, dz))
    } else if dy == 0 {
        (along(0, dx), along(2, dz))
    } else {
        (along(0, dx), along(1, dy))
    };
    !(face_sturdy(block_at(view, a)) && face_sturdy(block_at(view, b)))
}
