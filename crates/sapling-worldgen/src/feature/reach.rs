//! Which cells BDS's thread-local generator could have left different from ours: a taint analysis of
//! one source chunk's multiface features and sculk patches, for the oracle test's floor. Off unless
//! the generator tracks it. See docs/research/worldgen-bds-residue.md, "Reach".

use std::cell::{Cell, RefCell};

use super::data::FEATURES;
use super::spec::Feature;
use super::view::{REACH, View};
use crate::terrain::{ChunkShape, HEIGHT};

/// A sculk cursor acts within sqrt(224) of its patch's origin; a conversion and its veins lie up to
/// 1 + sqrt(2) further.
const PATCH_REACH: i32 = 18;
/// A multiface block lands in the origin or beside it, and spreads one cell around an edge.
const MULTIFACE_REACH: i32 = 2;

/// A bit per cell of a chunk, by `ChunkShape::index`.
#[derive(Clone)]
pub struct Cells(Box<[u64]>);

impl Default for Cells {
    fn default() -> Self {
        Self(vec![0; (256 * HEIGHT).div_ceil(64)].into())
    }
}

impl Cells {
    pub fn has(&self, index: usize) -> bool {
        self.0[index / 64] >> (index % 64) & 1 != 0
    }

    pub fn set(&mut self, index: usize) {
        self.0[index / 64] |= 1 << (index % 64);
    }

    pub fn merge(&mut self, other: &Cells) {
        self.0.iter_mut().zip(other.0.iter()).for_each(|(a, b)| *a |= b);
    }

    /// The cell of a chunk at local x and z.
    pub fn contains(&self, x: usize, y: i32, z: usize) -> bool {
        self.has(ChunkShape::index(x, y, z))
    }
}

/// How far a try of the feature may write when its walk differs; None for features not followed.
pub fn radius(feature: &Feature) -> Option<i32> {
    match feature {
        Feature::SculkPatch(_) => Some(PATCH_REACH),
        Feature::Multiface(_) => Some(MULTIFACE_REACH),
        Feature::SurfaceThreshold(threshold) => radius(&FEATURES[threshold.feature]),
        _ => None,
    }
}

/// Multiface and sculk patch tries followed and, of those, the ones a thread-local draw could have changed.
#[derive(Debug, Clone, Copy, Default)]
pub struct Tries {
    pub followed: u32,
    pub tainted: u32,
}

/// What earlier phases left in doubt in a chunk.
pub type Before<'a> = &'a dyn Fn(i32, i32) -> Option<Cells>;

pub struct Reach<'a> {
    before: Before<'a>,
    /// Per view slot, `before`'s answer once asked.
    inherited: RefCell<Vec<Option<Option<Cells>>>>,
    /// Per view slot, the cells this source leaves in doubt.
    own: Vec<Option<Cells>>,
    /// A followed try is running.
    following: bool,
    /// The running try read a cell in doubt, or its generator's stream is off.
    tainted: Cell<bool>,
    /// An earlier try of the rule may have drawn another number of values.
    stream: bool,
    writes: Vec<(usize, usize)>,
    pub tries: Tries,
}

impl<'a> Reach<'a> {
    pub fn new(before: Before<'a>) -> Self {
        let slots = ((2 * REACH + 1) * (2 * REACH + 1)) as usize;
        Self { before, inherited: RefCell::new(vec![None; slots]), own: vec![None; slots], following: false, tainted: Cell::new(false), stream: false, writes: Vec::new(), tries: Tries::default() }
    }

    /// `chunk` is the slot's chunk.
    pub fn read(&self, slot: usize, index: usize, chunk: (i32, i32)) {
        if !self.following || self.tainted.get() {
            return;
        }
        let own = self.own[slot].as_ref().is_some_and(|cells| cells.has(index));
        let inherited = || self.inherited.borrow_mut()[slot].get_or_insert_with(|| (self.before)(chunk.0, chunk.1)).as_ref().is_some_and(|cells| cells.has(index));
        if own || inherited() {
            self.tainted.set(true);
        }
    }

    pub fn wrote(&mut self, slot: usize, index: usize) {
        if self.following {
            self.writes.push((slot, index));
        }
    }

    /// `fresh`: the try draws from a generator no earlier try used.
    fn begin(&mut self, fresh: bool) {
        self.stream &= !fresh;
        self.following = true;
        self.tainted.set(self.stream);
        self.writes.clear();
    }

    fn doubt(&mut self, slot: usize, index: usize) {
        self.own[slot].get_or_insert_with(Cells::default).set(index);
    }

    /// Ends the try; true when it was tainted: its writes are in doubt then, the rule's later tries
    /// too, and the caller adds the cells the try could have reached.
    fn end(&mut self, counted: bool) -> bool {
        self.following = false;
        self.tries.followed += u32::from(counted);
        if !self.tainted.get() {
            return false;
        }
        self.tries.tainted += u32::from(counted);
        self.stream = true;
        for (slot, index) in std::mem::take(&mut self.writes) {
            self.doubt(slot, index);
        }
        true
    }

    pub fn into_parts(self) -> (Vec<Option<Cells>>, Tries) {
        (self.own, self.tries)
    }
}

impl View<'_> {
    pub fn reach_on(&self) -> bool {
        self.reach.is_some()
    }

    /// Starts a followed try (a rule's try, a liquid tick).
    pub fn reach_begin(&mut self, fresh: bool) {
        if let Some(reach) = &mut self.reach {
            reach.begin(fresh);
        }
    }

    /// The running try's outcome hangs on a thread-local draw.
    pub fn reach_taint(&mut self) {
        if let Some(reach) = &mut self.reach {
            reach.tainted.set(true);
        }
    }

    /// BDS's thread-local draws decide what this cell holds.
    pub fn reach_doubt(&mut self, [x, y, z]: [i32; 3]) {
        if let Some((slot, index)) = self.locate(x, y, z)
            && let Some(reach) = &mut self.reach
        {
            reach.doubt(slot, index);
        }
    }

    /// Ends the try started at `at`; a tainted one leaves the ball of `radius` around it in doubt.
    /// `Tries` counts the rules' tries, not liquid ticks (radius 1).
    pub fn reach_end(&mut self, at: [i32; 3], radius: i32) {
        if !self.reach.as_mut().is_some_and(|reach| reach.end(radius > 1)) {
            return;
        }
        for dx in -radius..=radius {
            for dy in -radius..=radius {
                for dz in (-radius..=radius).filter(|dz| dx * dx + dy * dy + dz * dz <= radius * radius) {
                    self.reach_doubt([at[0] + dx, at[1] + dy, at[2] + dz]);
                }
            }
        }
    }
}
