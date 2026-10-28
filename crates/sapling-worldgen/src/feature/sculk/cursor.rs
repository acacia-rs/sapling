//! `SculkSpreader::updateCursors` (148b48870) and its charge cursors. In world generation cursors
//! are never merged and one that strays more than sqrt(224) from the origin loses its charge; a
//! catalyst's merge on one cell while their charge stays at most 1000.

use std::collections::HashMap;

use super::MAX_CHARGE;
use super::behaviour::{Behaviour, is_vein, substrate_access};
use super::config::{SculkWorld, Spreader};
use crate::block::Block;
use crate::feature::spreader::Cells;
use crate::random::Mt19937;

/// The non-corner neighbour offsets (table 0x14a94bf78), shuffled per move.
const NEIGHBOURS: [[i32; 3]; 18] = [
    [-1, -1, 0], [0, -1, -1], [0, -1, 0], [0, -1, 1], [1, -1, 0], [-1, 0, -1], [-1, 0, 0], [-1, 0, 1], [0, 0, -1],
    [0, 0, 1], [1, 0, -1], [1, 0, 0], [1, 0, 1], [-1, 1, 0], [0, 1, -1], [0, 1, 0], [0, 1, 1], [1, 1, 0],
];

/// No facings yet; sculk's (empty) facings; else a vein's face bits.
pub const FACINGS_UNSET: i32 = -1;
pub const FACINGS_EMPTY: i32 = -2;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cursor {
    pub pos: [i32; 3],
    pub charge: i32,
    pub update_delay: i32,
    pub decay_delay: i32,
    pub facings: i32,
}

impl Cursor {
    pub fn new(pos: [i32; 3], charge: i32) -> Self {
        Self { pos, charge, update_delay: 0, decay_delay: 1, facings: FACINGS_UNSET }
    }
}

/// One round of every cursor. A spent cursor pops (2038); the cursors left on each cell are told
/// with their summed charge (2037), in the order their cells were first reached.
pub fn update_cursors(view: &mut impl SculkWorld, spreader: &Spreader, cursors: &mut Vec<Cursor>, origin: [i32; 3], random: &mut Mt19937, spread_veins: bool) {
    let mut kept: Vec<Cursor> = Vec::with_capacity(cursors.len());
    let mut totals: Vec<([i32; 3], i32)> = Vec::new();
    // Per cell, the kept cursor whose facings the charge event shows.
    let mut shown: HashMap<[i32; 3], usize> = HashMap::new();
    for mut cursor in cursors.drain(..) {
        if cursor.charge > 0 && view.ticking(cursor.pos) {
            update(view, spreader, &mut cursor, origin, random, spread_veins);
        }
        if cursor.charge <= 0 {
            view.charge_pop(cursor.pos);
            continue;
        }
        match totals.iter_mut().find(|(at, _)| *at == cursor.pos) {
            Some((_, total)) => *total += cursor.charge,
            None => totals.push((cursor.pos, cursor.charge)),
        }
        let Some(&first) = shown.get(&cursor.pos) else {
            shown.insert(cursor.pos, kept.len());
            kept.push(cursor);
            continue;
        };
        let existing = &mut kept[first];
        if !spreader.worldgen && cursor.charge + existing.charge <= MAX_CHARGE {
            existing.charge += cursor.charge;
            existing.update_delay = existing.update_delay.min(cursor.update_delay);
            continue;
        }
        if cursor.charge < existing.charge {
            shown.insert(cursor.pos, kept.len());
        }
        kept.push(cursor);
    }
    for (at, total) in totals {
        view.charge(at, total, kept[shown[&at]].facings);
    }
    *cursors = kept;
}

fn block_at(view: &impl Cells, [x, y, z]: [i32; 3]) -> Block {
    view.block(x, y, z)
}

fn update(view: &mut impl SculkWorld, spreader: &Spreader, cursor: &mut Cursor, origin: [i32; 3], random: &mut Mt19937, spread_veins: bool) {
    if cursor.update_delay >= 1 {
        cursor.update_delay -= 1;
        return;
    }
    let mut state = block_at(view, cursor.pos);
    let mut behaviour = Behaviour::of(state);
    if spread_veins && behaviour.attempt_spread_vein(view, cursor.pos, state, cursor.facings) {
        if behaviour.changes_on_spread() {
            state = block_at(view, cursor.pos);
            behaviour = Behaviour::of(state);
        }
        view.sound(cursor.pos, super::config::Sound::Spread);
    }
    cursor.charge = behaviour.attempt_use_charge(view, spreader, cursor, origin, random, spread_veins);
    if cursor.charge < 1 {
        behaviour.on_discharged(view, cursor.pos);
        return;
    }
    if let Some(next) = movement_target(view, spreader, cursor.pos, random) {
        behaviour.on_discharged(view, cursor.pos);
        cursor.pos = next;
        let d = [next[0] - origin[0], next[1] - origin[1], next[2] - origin[2]];
        if spreader.worldgen && d[0] * d[0] + d[1] * d[1] + d[2] * d[2] > 224 {
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
fn movement_target(view: &impl Cells, spreader: &Spreader, pos: [i32; 3], random: &mut Mt19937) -> Option<[i32; 3]> {
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
        if is_vein(block) && substrate_access(view, spreader, block, next) {
            break;
        }
    }
    found
}

/// A diagonal move needs one of the two cells it passes free of a sturdy face toward it.
fn unobstructed(view: &impl Cells, pos: [i32; 3], [dx, dy, dz]: [i32; 3]) -> bool {
    if dx.abs() + dy.abs() + dz.abs() == 1 {
        return true;
    }
    // The cell one step along an axis holds its face toward the cursor (FacingIDs: y 0/1, z 2/3, x 4/5).
    let walled = |axis: usize, sign: i32| {
        let mut p = pos;
        p[axis] += sign;
        let toward = [4, 0, 2][axis] + usize::from(sign < 0);
        block_at(view, p).supports_multiface_plain(toward)
    };
    let (a, b) = if dx == 0 {
        (walled(1, dy), walled(2, dz))
    } else if dy == 0 {
        (walled(0, dx), walled(2, dz))
    } else {
        (walled(0, dx), walled(1, dy))
    };
    !(a && b)
}
