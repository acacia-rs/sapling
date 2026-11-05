//! `BlockTickingQueue` as decoration uses it: a pending heap and an active heap of liquid ticks
//! (`crate::tick_heap`), whose order among equal keys decides which cell flows first
//! (141603c30 add, 1416042f0 tickPendingTicks, 141607780).

use crate::tick_heap::{Keyed, TickHeap};

/// One scheduled tick; `lava` stands for the block it was scheduled with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tick {
    pub at: [i32; 3],
    pub lava: bool,
    pub time: u64,
}

impl Keyed for Tick {
    /// Decoration's liquid ticks all have priority 0.
    fn key(&self) -> (u64, i32) {
        (self.time, 0)
    }
}

type Heap = TickHeap<Tick>;

/// The queue decoration hands its block source; its clock starts at 1 (the queue's constructor).
pub struct TickQueue {
    now: u64,
    pending: Heap,
    /// `BlockSource::isInstaticking`: true while instatick generations run.
    pub instaticking: bool,
    /// Ticks whose cells lie too near the region's edge for this queue (`View::schedule`): BDS
    /// queues them on their chunk, to run in game. (cell, lava, delay), in the order they came.
    pub in_game: Vec<([i32; 3], bool, u64)>,
    /// The in-game queue: every tick joins it.
    pub game: bool,
}

impl Default for TickQueue {
    fn default() -> Self {
        Self { now: 1, pending: Heap::default(), instaticking: false, in_game: Vec::new(), game: false }
    }
}

impl TickQueue {
    /// Times wrap: once ticking runs with the clock at `u64::MAX`, new ticks land at `delay - 1`.
    pub fn add(&mut self, at: [i32; 3], lava: bool, delay: u64) {
        self.pending.push(Tick { at, lava, time: self.now.wrapping_add(delay) });
    }

    /// `LiquidBlock::neighborChanged`'s test: a tick for this cell and liquid already due at `now + delay`.
    pub fn has(&self, at: [i32; 3], lava: bool, delay: u64) -> bool {
        let time = self.now.wrapping_add(delay);
        self.pending.iter().any(|t| t.at == at && t.lava == lava && t.time == time)
    }

    /// `acquireAllTicks` (141607780): `other`'s pending ticks popped in its heap order into this queue.
    pub fn acquire(&mut self, mut other: TickQueue) {
        while let Some(tick) = other.pending.pop() {
            self.pending.push(tick);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    /// The ticks of the next `tickPendingTicks(MAX, INT_MAX, instatick)` call, in the order it
    /// runs them: every pending tick moved to an active heap. Ticks added meanwhile wait for the next.
    pub fn take_generation(&mut self) -> Vec<Tick> {
        self.take_due(u64::MAX)
    }

    /// `tickPendingTicks(until, ..)`: the clock set to `until`, the ticks due by then in run order.
    pub fn take_due(&mut self, until: u64) -> Vec<Tick> {
        self.now = until;
        let mut active = Heap::default();
        while self.pending.peek().is_some_and(|t| t.time <= until) {
            active.push(self.pending.pop().expect("non-empty"));
        }
        std::iter::from_fn(|| active.pop()).collect()
    }

    /// The pending ticks, in heap order, emptying the queue.
    pub fn drain(&mut self) -> Vec<Tick> {
        std::iter::from_fn(|| self.pending.pop()).collect()
    }

    /// A tick at an absolute time.
    pub fn add_at(&mut self, at: [i32; 3], lava: bool, time: u64) {
        self.pending.push(Tick { at, lava, time });
    }

    /// A chunk's own queue while the world generates: its clock stands at 0, so an aquifer tick queued "in 1"
    /// is due at 1, before the border scan's (the block source's queue starts at 1: due at 2). Dumped at the
    /// first `tickPendingTicks` of a decoration: 469 entries at time 1, the 69 border ticks at 2.
    pub fn chunk() -> Self {
        Self { now: 0, ..Self::default() }
    }

    /// An in-game queue whose clock starts at 0.
    pub fn game() -> Self {
        Self { now: 0, game: true, ..Self::default() }
    }
}

impl super::Features {
    /// The ticks placed blocks queued on `chunk` for the game (`View::keep_tick`), in decoration order.
    pub fn kept_ticks(&self, chunk: (i32, i32), inputs: super::Inputs) -> Vec<super::view::Kept> {
        let sources = self.sources_before(chunk, super::Rank::MAX);
        sources.into_iter().flat_map(|source| self.source_writes(source, inputs).kept_in(chunk).collect::<Vec<_>>()).collect()
    }
}
