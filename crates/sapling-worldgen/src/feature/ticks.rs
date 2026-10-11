//! `BlockTickingQueue` as decoration uses it: a pending heap and an active heap of liquid ticks,
//! ordered by (time, priority) with MSVC's `push_heap`/`pop_heap`, whose order among equal keys
//! decides which cell flows first (141603c30 add, 1416042f0 tickPendingTicks, 141607780).

/// One scheduled tick; `lava` stands for the block it was scheduled with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tick {
    pub at: [i32; 3],
    pub lava: bool,
    pub time: u64,
}

impl Tick {
    /// MSVC's predicate for a min-heap on time: `self` sorts after `other`. Priorities are all 0.
    fn after(&self, other: &Tick) -> bool {
        self.time > other.time
    }
}

#[derive(Default)]
struct Heap(Vec<Tick>);

impl Heap {
    fn push(&mut self, tick: Tick) {
        self.0.push(tick);
        let mut hole = self.0.len() - 1;
        while hole > 0 {
            let parent = (hole - 1) >> 1;
            if !self.0[parent].after(&tick) {
                break;
            }
            self.0[hole] = self.0[parent];
            hole = parent;
        }
        self.0[hole] = tick;
    }

    fn pop(&mut self) -> Option<Tick> {
        let top = *self.0.first()?;
        let last = self.0.pop().expect("non-empty");
        let count = self.0.len();
        if count == 0 {
            return Some(top);
        }
        let mut hole = 0;
        let max_non_leaf = (count - 1) >> 1;
        while hole < max_non_leaf {
            let mut child = 2 * hole + 2;
            if self.0[child].after(&self.0[child - 1]) {
                child -= 1;
            }
            self.0[hole] = self.0[child];
            hole = child;
        }
        if hole == max_non_leaf && count % 2 == 0 {
            self.0[hole] = self.0[count - 1];
            hole = count - 1;
        }
        while hole > 0 {
            let parent = (hole - 1) >> 1;
            if !self.0[parent].after(&last) {
                break;
            }
            self.0[hole] = self.0[parent];
            hole = parent;
        }
        self.0[hole] = last;
        Some(top)
    }
}

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
        self.pending.0.iter().any(|t| t.at == at && t.lava == lava && t.time == time)
    }

    /// `acquireAllTicks` (141607780): `other`'s pending ticks popped in its heap order into this queue.
    pub fn acquire(&mut self, mut other: TickQueue) {
        while let Some(tick) = other.pending.pop() {
            self.pending.push(tick);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.pending.0.is_empty()
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
        while self.pending.0.first().is_some_and(|t| t.time <= until) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pops_in_time_order() {
        let mut heap = Heap::default();
        for (i, time) in [5u64, 1, 4, 1, 3, 9, 0].into_iter().enumerate() {
            heap.push(Tick { at: [i as i32, 0, 0], lava: false, time });
        }
        let times: Vec<u64> = std::iter::from_fn(|| heap.pop()).map(|t| t.time).collect();
        assert_eq!(times, [0, 1, 1, 3, 4, 5, 9]);
    }
}
