//! The `SculkBehaviour`s a cursor runs by the block it sits on (vtables 14a94bec0 default,
//! 14a94bf00 sculk, 14a94bf40 vein): slots decay delay, facings, changes on spread, spread veins,
//! use charge, on discharged.

use super::config::{SculkWorld, Sound, Spreader};
use super::cursor::{Cursor, FACINGS_EMPTY, FACINGS_UNSET};
use crate::block::Block;
pub use crate::feature::spreader::is_vein;
use crate::feature::spreader::{self, Cells, Config, SPREAD_ORDER, has_face, opposite, step};
use crate::random::Mt19937;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Behaviour {
    Default,
    Sculk,
    Vein,
}

/// `hasSubstrateAccess`: a face of the vein on a block that may turn to sculk.
pub fn substrate_access(view: &impl Cells, spreader: &Spreader, vein: Block, at: [i32; 3]) -> bool {
    SPREAD_ORDER.into_iter().any(|face| {
        let [x, y, z] = step(at, face);
        has_face(vein, face) && spreader.replaceable(view.block(x, y, z))
    })
}

fn set(view: &mut impl Cells, [x, y, z]: [i32; 3], block: Block) {
    view.set_block(x, y, z, block);
}

impl Behaviour {
    pub fn of(block: Block) -> Behaviour {
        if block == Block::Sculk {
            Behaviour::Sculk
        } else if is_vein(block) {
            Behaviour::Vein
        } else {
            Behaviour::Default
        }
    }

    pub fn decay_delay(self, delay: i32) -> i32 {
        if self == Behaviour::Default { delay.max(1) - 1 } else { 1 }
    }

    pub fn facings(self, facings: i32, state: Block) -> i32 {
        match self {
            Behaviour::Default => facings,
            Behaviour::Sculk => FACINGS_EMPTY,
            Behaviour::Vein => state.multiface().map_or(FACINGS_EMPTY, |(_, bits)| bits as i32),
        }
    }

    pub fn changes_on_spread(self) -> bool {
        self != Behaviour::Sculk
    }

    /// Default: 148b40dd0 (unset facings spread on the cell only, empty ones everywhere, else the
    /// vein regrows with them); sculk and vein: 148b41640.
    pub fn attempt_spread_vein(self, view: &mut impl Cells, at: [i32; 3], state: Block, facings: i32) -> bool {
        if self != Behaviour::Default || facings == FACINGS_EMPTY {
            return spreader::spread_all(Config::Vein, view, state, at, &SPREAD_ORDER) > 0;
        }
        if facings == FACINGS_UNSET {
            return spreader::spread_all(Config::VeinSameSpace, view, state, at, &SPREAD_ORDER) > 0;
        }
        if !spreader::vein_replaceable(state) || !(1..64).contains(&facings) {
            return false;
        }
        set(view, at, Block::with_faces(Block::SculkVein1, facings as u8));
        true
    }

    pub fn attempt_use_charge(self, view: &mut impl SculkWorld, spreader: &Spreader, cursor: &Cursor, origin: [i32; 3], random: &mut Mt19937, spread_veins: bool) -> i32 {
        match self {
            Behaviour::Default => if cursor.decay_delay > 0 { cursor.charge } else { 0 },
            Behaviour::Sculk => sculk_use_charge(view, spreader, cursor, origin, random),
            Behaviour::Vein => vein_use_charge(view, spreader, cursor, random, spread_veins),
        }
    }

    pub fn on_discharged(self, view: &mut impl Cells, at: [i32; 3]) {
        if self == Behaviour::Vein {
            discharge_vein(view, at);
        }
    }
}

/// 148b41810: within the no-growth radius (`r² > d²`) a cursor only decays, and there by one.
fn sculk_use_charge(view: &mut impl SculkWorld, spreader: &Spreader, cursor: &Cursor, origin: [i32; 3], random: &mut Mt19937) -> i32 {
    let charge = cursor.charge;
    if random.next_int(spreader.charge_decay_rate) != 0 {
        return charge;
    }
    let d = [cursor.pos[0] - origin[0], cursor.pos[1] - origin[1], cursor.pos[2] - origin[2]];
    let dist_sq = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]) as f32;
    let radius_sq = (spreader.no_growth_radius * spreader.no_growth_radius) as f32;
    if radius_sq <= dist_sq && can_place_growth(view, cursor.pos) {
        let cost = spreader.growth_spawn_cost;
        if random.next_int(cost) < charge {
            let shrieker = random.next_int(11) == 0;
            // A catalyst's shrieker cannot summon (148b41ca1 passes `worldgen`): the live world's writer maps it.
            set(view, step(cursor.pos, 1), if shrieker { Block::SculkShrieker } else { Block::SculkSensor });
            view.sound(cursor.pos, if shrieker { Sound::ShriekerPlaced } else { Sound::SensorPlaced });
        }
        return (charge - cost).max(0);
    }
    if random.next_int(spreader.additional_decay_rate) != 0 {
        return charge;
    }
    let penalty = if radius_sq >= dist_sq {
        1
    } else {
        let f = ((dist_sq.sqrt() - 4.0).powi(2) / 400.0).min(1.0);
        ((charge as f32 * f * 0.5) as i32).max(1)
    };
    charge - penalty
}

/// Air or water above, and at most two sensors or shriekers in the 9x3x9 box around the cell
/// above the cursor (148b4194f: its centre +-4, +-1, +-4), which is Java's box.
fn can_place_growth(view: &impl Cells, at: [i32; 3]) -> bool {
    let above = view.block(at[0], at[1] + 1, at[2]);
    // 148b41810 compares block types: water of any depth passes.
    if above != Block::Air && !(above.is_fluid() && !above.is_lava()) {
        return false;
    }
    let mut growths = 0;
    for dx in -4..=4 {
        for dy in 0..=2 {
            for dz in -4..=4 {
                if view.block(at[0] + dx, at[1] + dy, at[2] + dz).is_sculk_growth() {
                    growths += 1;
                }
            }
        }
    }
    growths < 3
}

/// 148b42070: with veins spreading, turn a block behind a face (in shuffled order) to sculk,
/// clear the veins around it off its faces and spread veins from it.
fn vein_use_charge(view: &mut impl SculkWorld, spreader: &Spreader, cursor: &Cursor, random: &mut Mt19937, spread_veins: bool) -> i32 {
    if spread_veins {
        let state = view.block(cursor.pos[0], cursor.pos[1], cursor.pos[2]);
        let order = shuffled_faces(random);
        let target = order.into_iter().find(|&face| {
            let [x, y, z] = step(cursor.pos, face);
            has_face(state, face) && spreader.replaceable(view.block(x, y, z))
        });
        if let Some(face) = target {
            let sculk = step(cursor.pos, face);
            set(view, sculk, Block::Sculk);
            view.sound(cursor.pos, Sound::Spread);
            for d in SPREAD_ORDER.into_iter().filter(|&d| d != opposite(face)) {
                discharge_vein(view, step(sculk, d));
            }
            spreader::spread_all(Config::Vein, view, Block::Sculk, sculk, &SPREAD_ORDER);
            return cursor.charge - 1;
        }
    }
    if random.next_int(spreader.charge_decay_rate) == 0 { (cursor.charge as f32 * 0.5).floor() as i32 } else { cursor.charge }
}

/// 1403747f0: BDS's inside-out shuffle of the six faces.
fn shuffled_faces(random: &mut Mt19937) -> [usize; 6] {
    let mut faces = [1, 0, 2, 3, 4, 5];
    if random.next_u32() & 1 == 0 {
        faces[0] = 0;
        faces[1] = 1;
    }
    for i in 2..6 {
        let k = (random.next_u32() % (i as u32 + 1)) as usize;
        if k != i {
            faces[i] = faces[k];
            faces[k] = i;
        }
    }
    faces
}

/// 148b42aa0: a vein drops its faces on sculk, and goes when none are left.
fn discharge_vein(view: &mut impl Cells, at: [i32; 3]) {
    let state = view.block(at[0], at[1], at[2]);
    let Some((_, bits)) = state.multiface().filter(|_| is_vein(state)) else { return };
    let mut kept = bits;
    // One write per face, as BDS's `removeFace` (148b6adf0).
    for face in SPREAD_ORDER {
        let [x, y, z] = step(at, face);
        if has_face(state, face) && view.block(x, y, z) == Block::Sculk {
            kept &= !Block::FACE_BITS[face];
            set(view, at, if kept == 0 { Block::Air } else { Block::with_faces(Block::SculkVein1, kept) });
        }
    }
}
