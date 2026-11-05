//! A BDS chunk's block tick queue (`LevelChunk` +0x130, `BlockTickingQueue`; docs/research/tick-queue.md):
//! one heap per column with BDS's order among ticks due together (`sapling_worldgen::tick_heap`), a
//! second tick for a cell kept and run, a removed tick only marked, and at most [`MAX_PER_TICK`] moved
//! out a game tick, which then run in the order a second heap gives them. A column's queue runs once
//! a game tick; a tick queued for now after it ran waits for the next.

use std::collections::BTreeMap;

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_world::ChunkPos;
use sapling_worldgen::tick_heap::{Keyed, TickHeap};

/// `LevelChunk::tickImpl` (1409f98a0) passes 100 to `tickPendingTicks`.
pub(super) const MAX_PER_TICK: usize = 100;

#[derive(Clone, Copy)]
struct Entry<T> {
    removed: bool,
    pos: BlockPos,
    due: u64,
    what: T,
}

impl<T: Copy> Keyed for Entry<T> {
    /// Every tick sapling queues has priority offset 0, as BDS's liquids do.
    fn key(&self) -> (u64, i32) {
        (self.due, 0)
    }
}

struct Queue<T> {
    next: TickHeap<Entry<T>>,
    ran: Option<u64>,
}

impl<T> Default for Queue<T> {
    fn default() -> Self {
        Self { next: TickHeap::default(), ran: None }
    }
}

/// Every column's queue, by world and column; `T` says what a tick is for.
pub(super) struct ChunkTicks<T>(BTreeMap<(Entity, i32, i32), Queue<T>>);

impl<T> Default for ChunkTicks<T> {
    fn default() -> Self {
        Self(BTreeMap::new())
    }
}

fn key(layer: Entity, [x, _, z]: BlockPos) -> (Entity, i32, i32) {
    let column = ChunkPos::of_block(x, z);
    (layer, column.x, column.z)
}

impl<T: Copy> ChunkTicks<T> {
    pub(super) fn add(&mut self, layer: Entity, pos: BlockPos, due: u64, what: T) {
        self.0.entry(key(layer, pos)).or_default().next.push(Entry { removed: false, pos, due, what });
    }

    fn live(&self, layer: Entity, pos: BlockPos) -> impl Iterator<Item = &Entry<T>> {
        self.0.get(&key(layer, pos)).into_iter().flat_map(|queue| queue.next.iter()).filter(move |entry| !entry.removed && entry.pos == pos)
    }

    /// Whether a tick for `pos` that `is` waits, due at `due` if given.
    pub(super) fn has(&self, layer: Entity, pos: BlockPos, due: Option<u64>, is: impl Fn(&T) -> bool) -> bool {
        self.live(layer, pos).any(|entry| due.is_none_or(|due| entry.due == due) && is(&entry.what))
    }

    /// `BlockTickingQueue::remove` (1415f5520): the ticks for `pos` that `is` are marked, left in the heap.
    pub(super) fn remove(&mut self, layer: Entity, pos: BlockPos, is: impl Fn(&T) -> bool) {
        if let Some(queue) = self.0.get_mut(&key(layer, pos)) {
            queue.next.for_each_mut(|entry| entry.removed |= entry.pos == pos && is(&entry.what));
        }
    }

    /// Marks every tick where `gone` says so, dropping queues left with none.
    pub(super) fn forget(&mut self, gone: &dyn Fn(Entity, BlockPos) -> bool) {
        for (&(layer, ..), queue) in self.0.iter_mut() {
            queue.next.for_each_mut(|entry| entry.removed |= gone(layer, entry.pos));
        }
        self.0.retain(|_, queue| queue.next.iter().any(|entry| !entry.removed));
    }

    /// What waits: each tick's world, cell, due time and purpose.
    pub(super) fn waiting(&self) -> Vec<(Entity, BlockPos, u64, T)> {
        self.0.iter().flat_map(|(&(layer, ..), queue)| queue.next.iter().filter(|entry| !entry.removed).map(move |entry| (layer, entry.pos, entry.due, entry.what))).collect()
    }

    /// The columns with a tick due by `now` whose queue has not run this game tick, in world and
    /// column order; empty queues that ran before now are dropped.
    pub(super) fn columns_due(&mut self, now: u64) -> Vec<(Entity, ChunkPos)> {
        self.0.retain(|_, queue| !queue.next.is_empty() || queue.ran == Some(now));
        let due = |queue: &Queue<T>| queue.ran != Some(now) && queue.next.peek().is_some_and(|entry| entry.due <= now);
        self.0.iter().filter(|(_, queue)| due(queue)).map(|(&(layer, x, z), _)| (layer, ChunkPos { x, z })).collect()
    }

    /// `tickPendingTicks(now, 100)` (1416042f0) for one column: its due ticks, in the order they run.
    /// Removed ticks are dropped on the way and do not count; the limit is taken off the heap's size.
    pub(super) fn take(&mut self, layer: Entity, column: ChunkPos, now: u64) -> Vec<(BlockPos, T)> {
        let Some(queue) = self.0.get_mut(&(layer, column.x, column.z)).filter(|queue| queue.ran != Some(now)) else { return Vec::new() };
        queue.ran = Some(now);
        let limit = MAX_PER_TICK.min(queue.next.len());
        let mut active = TickHeap::default();
        while active.len() < limit {
            let Some(top) = queue.next.peek().copied() else { break };
            if !top.removed && top.due > now {
                break;
            }
            queue.next.pop();
            if !top.removed {
                active.push(top);
            }
        }
        std::iter::from_fn(|| active.pop()).map(|entry: Entry<T>| (entry.pos, entry.what)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LAYER: Entity = Entity::PLACEHOLDER;

    #[test]
    fn ticks_due_together_run_in_bds_heap_order_and_duplicates_both_run() {
        let mut ticks = ChunkTicks::default();
        for i in 0..5 {
            ticks.add(LAYER, [i, 0, 0], 3, i);
        }
        ticks.add(LAYER, [0, 0, 0], 3, 0);
        ticks.add(LAYER, [9, 0, 0], 4, 9);
        let column = ChunkPos { x: 0, z: 0 };
        assert_eq!(ticks.columns_due(3), [(LAYER, column)]);
        let order: Vec<i32> = ticks.take(LAYER, column, 3).into_iter().map(|(_, what)| what).collect();
        assert_eq!(order.len(), 6);
        assert_eq!(order.iter().filter(|&&what| what == 0).count(), 2);
        assert!(ticks.take(LAYER, column, 3).is_empty(), "a column's queue runs once a game tick");
        assert_eq!(ticks.waiting().len(), 1);
    }

    #[test]
    fn five_ticks_due_together_come_out_as_bds_orders_them() {
        let mut ticks = ChunkTicks::default();
        (0..5).for_each(|i| ticks.add(LAYER, [i, 0, 0], 1, i));
        let order: Vec<i32> = ticks.take(LAYER, ChunkPos { x: 0, z: 0 }, 1).into_iter().map(|(_, what)| what).collect();
        assert_eq!(order, [0, 4, 3, 2, 1]);
    }

    #[test]
    fn removed_ticks_do_not_run_or_count_and_the_limit_is_per_column() {
        let mut ticks = ChunkTicks::default();
        for i in 0..150 {
            ticks.add(LAYER, [0, i, 0], 1, i);
            ticks.add(LAYER, [16, i, 0], 1, i);
        }
        ticks.remove(LAYER, [0, 0, 0], |_| true);
        assert!(!ticks.has(LAYER, [0, 0, 0], None, |_| true));
        assert_eq!(ticks.take(LAYER, ChunkPos { x: 0, z: 0 }, 1).len(), MAX_PER_TICK);
        assert_eq!(ticks.take(LAYER, ChunkPos { x: 1, z: 0 }, 1).len(), MAX_PER_TICK);
        assert_eq!(ticks.waiting().len(), 49 + 50);
    }
}
