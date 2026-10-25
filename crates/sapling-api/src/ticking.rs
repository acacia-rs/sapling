//! Chunks kept loaded and ticking with no player near: a world's ticking areas, which it keeps on
//! disk as `/tickingarea` does, and chunks a plugin forces for as long as the server runs. Bedrock
//! keeps no spawn chunks loaded; only these are.

use crate::ShardCtx;

/// A ticking area of the world in scope: chunks `min` to `max` (chunk coordinates, both in), or
/// those of a circle about its centre chunk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TickingArea {
    pub name: String,
    pub min: [i32; 2],
    pub max: [i32; 2],
    pub circle: bool,
    /// Whether the world waits for its chunks to load before it ticks after starting.
    pub preload: bool,
}

impl TickingArea {
    /// Whether the area keeps the chunk (x, z).
    pub fn holds(&self, [x, z]: [i32; 2]) -> bool {
        if !(self.min[0]..=self.max[0]).contains(&x) || !(self.min[1]..=self.max[1]).contains(&z) {
            return false;
        }
        if !self.circle {
            return true;
        }
        let radius = (self.max[0] - self.min[0]) / 2;
        let (dx, dz) = (x - (self.min[0] + radius), z - (self.min[1] + radius));
        dx * dx + dz * dz <= radius * radius
    }

    /// How many chunks it keeps.
    pub fn chunk_count(&self) -> usize {
        (self.min[0]..=self.max[0]).flat_map(|x| (self.min[1]..=self.max[1]).map(move |z| [x, z])).filter(|chunk| self.holds(*chunk)).count()
    }
}

/// How many ticking areas a world holds at most, as BDS allows.
pub const MOST_TICKING_AREAS: usize = 10;

impl ShardCtx<'_> {
    /// The ticking areas of the world in scope, by name.
    pub fn ticking_areas(&self) -> Vec<TickingArea> {
        self.backend.ticking_areas()
    }

    /// Adds a ticking area to the world in scope; false where it already holds
    /// [`MOST_TICKING_AREAS`].
    pub fn add_ticking_area(&mut self, area: TickingArea) -> bool {
        self.backend.add_ticking_area(area)
    }

    /// Removes the ticking areas that `pick` picks, returning them.
    pub fn remove_ticking_areas(&mut self, pick: &dyn Fn(&TickingArea) -> bool) -> Vec<TickingArea> {
        self.backend.remove_ticking_areas(pick)
    }

    /// Sets `preload` on the areas that `pick` picks, returning how many it set.
    pub fn set_ticking_preload(&mut self, pick: &dyn Fn(&TickingArea) -> bool, preload: bool) -> usize {
        self.backend.set_ticking_preload(pick, preload)
    }

    /// Keeps the chunk (x, z) of the world in scope loaded and ticking, or lets it go; held
    /// in memory only.
    pub fn force_chunk(&mut self, chunk: [i32; 2], forced: bool) {
        self.backend.force_chunk(chunk, forced);
    }

    /// The chunks of the world in scope a plugin forces.
    pub fn forced_chunks(&self) -> Vec<[i32; 2]> {
        self.backend.forced_chunks()
    }
}

/// The ticking side of [`crate::Backend`].
pub trait TickingBackend {
    fn ticking_areas(&self) -> Vec<TickingArea>;
    fn add_ticking_area(&mut self, area: TickingArea) -> bool;
    fn remove_ticking_areas(&mut self, pick: &dyn Fn(&TickingArea) -> bool) -> Vec<TickingArea>;
    fn set_ticking_preload(&mut self, pick: &dyn Fn(&TickingArea) -> bool, preload: bool) -> usize;
    fn force_chunk(&mut self, chunk: [i32; 2], forced: bool);
    fn forced_chunks(&self) -> Vec<[i32; 2]>;
}

#[cfg(test)]
impl TickingBackend for crate::testing::Fake {
    fn ticking_areas(&self) -> Vec<TickingArea> {
        Vec::new()
    }
    fn add_ticking_area(&mut self, _: TickingArea) -> bool {
        false
    }
    fn remove_ticking_areas(&mut self, _: &dyn Fn(&TickingArea) -> bool) -> Vec<TickingArea> {
        Vec::new()
    }
    fn set_ticking_preload(&mut self, _: &dyn Fn(&TickingArea) -> bool, _: bool) -> usize {
        0
    }
    fn force_chunk(&mut self, _: [i32; 2], _: bool) {}
    fn forced_chunks(&self) -> Vec<[i32; 2]> {
        Vec::new()
    }
}
