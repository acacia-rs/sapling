//! The 3x3 chunks around one, each fetched when first read.

use std::cell::OnceCell;
use std::sync::Arc;

pub struct Around<'a, T> {
    centre: (i32, i32),
    cells: [OnceCell<Arc<T>>; 9],
    load: &'a dyn Fn((i32, i32)) -> Arc<T>,
}

impl<'a, T> Around<'a, T> {
    pub fn new(centre: (i32, i32), load: &'a dyn Fn((i32, i32)) -> Arc<T>) -> Self {
        Self { centre, cells: Default::default(), load }
    }

    pub fn holds(&self, chunk: (i32, i32)) -> bool {
        (chunk.0 - self.centre.0).abs() <= 1 && (chunk.1 - self.centre.1).abs() <= 1
    }

    /// Panics for a chunk outside the 3x3.
    pub fn get(&self, chunk: (i32, i32)) -> &Arc<T> {
        assert!(self.holds(chunk), "{chunk:?} is not around {:?}", self.centre);
        let slot = (chunk.0 - self.centre.0 + 1) * 3 + chunk.1 - self.centre.1 + 1;
        self.cells[slot as usize].get_or_init(|| (self.load)(chunk))
    }
}
