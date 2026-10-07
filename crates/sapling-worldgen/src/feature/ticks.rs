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
    /// `BlockSource::isInstaticking`: true while `run_generation` runs.
    pub instaticking: bool,
}

impl Default for TickQueue {
    fn default() -> Self {
        Self { now: 1, pending: Heap::default(), instaticking: false }
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

    pub fn is_empty(&self) -> bool {
        self.pending.0.is_empty()
    }

    /// One `tickPendingTicks(MAX, INT_MAX, instatick)` call: every pending tick, in heap order,
    /// moved to an active heap; `run` ticks each and may add new ones for the next call.
    /// The ticks of the next `tickPendingTicks(MAX, INT_MAX, instatick)` call, in the order it
    /// runs them: every pending tick moved to an active heap. Ticks added meanwhile wait for the next.
    pub fn take_generation(&mut self) -> Vec<Tick> {
        self.now = u64::MAX;
        let mut active = Heap::default();
        while let Some(tick) = self.pending.pop() {
            active.push(tick);
        }
        std::iter::from_fn(|| active.pop()).collect()
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
