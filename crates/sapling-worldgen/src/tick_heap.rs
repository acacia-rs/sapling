//! BDS's `MovePriorityQueue` of block ticks: a `std::vector` kept as a min-heap by MSVC's
//! `push_heap`/`pop_heap` with the comparator `greater` on (time, priority) only
//! (141603c30 add, 141617160 pop; docs/research/tick-queue.md). Equal keys come out in an order
//! set by the heap's whole history, not by insertion; libstdc++'s heap moves the same way.

/// What orders an entry: its due time, then its priority offset.
pub trait Keyed: Copy {
    fn key(&self) -> (u64, i32);
}

#[derive(Debug, Clone)]
pub struct TickHeap<T>(Vec<T>);

impl<T> Default for TickHeap<T> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

/// MSVC's predicate: `a` sorts after `b`.
fn after<T: Keyed>(a: &T, b: &T) -> bool {
    a.key() > b.key()
}

impl<T: Keyed> TickHeap<T> {
    pub fn push(&mut self, entry: T) {
        self.0.push(entry);
        let hole = self.0.len() - 1;
        self.sift_up(hole, entry);
    }

    fn sift_up(&mut self, mut hole: usize, entry: T) {
        while hole > 0 {
            let parent = (hole - 1) >> 1;
            if !after(&self.0[parent], &entry) {
                break;
            }
            self.0[hole] = self.0[parent];
            hole = parent;
        }
        self.0[hole] = entry;
    }

    pub fn pop(&mut self) -> Option<T> {
        if crate::platform::linux() { self.pop_libcxx() } else { self.pop_msvc() }
    }

    /// libc++'s `__pop_heap` (Floyd), the Linux BDS's: the hole goes down to a leaf of the whole heap, last
    /// entry included, taking the left child unless the right sorts first; the last entry fills it and is
    /// sifted up. Equal keys thus leave in another order than MSVC's.
    fn pop_libcxx(&mut self) -> Option<T> {
        let top = *self.0.first()?;
        let len = self.0.len();
        if len == 1 {
            return self.0.pop();
        }
        let mut hole = 0;
        loop {
            let mut child = 2 * hole + 1;
            if child + 1 < len && after(&self.0[child], &self.0[child + 1]) {
                child += 1;
            }
            self.0[hole] = self.0[child];
            hole = child;
            if child > (len - 2) / 2 {
                break;
            }
        }
        let last = self.0.pop().expect("non-empty");
        if hole != len - 1 {
            self.sift_up(hole, last);
        }
        Some(top)
    }

    /// `_Pop_heap_hole_by_index`: the hole goes down to the bottom, taking the right child unless
    /// the left sorts first, and the last entry is pushed back up from there.
    fn pop_msvc(&mut self) -> Option<T> {
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
            if after(&self.0[child], &self.0[child - 1]) {
                child -= 1;
            }
            self.0[hole] = self.0[child];
            hole = child;
        }
        if hole == max_non_leaf && count % 2 == 0 {
            self.0[hole] = self.0[count - 1];
            hole = count - 1;
        }
        self.sift_up(hole, last);
        Some(top)
    }

    pub fn peek(&self) -> Option<&T> {
        self.0.first()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The entries in heap (array) order.
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.0.iter()
    }

    /// Changes entries in place; `f` must not change their keys.
    pub fn for_each_mut(&mut self, f: impl FnMut(&mut T)) {
        self.0.iter_mut().for_each(f);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Entry(u64, usize);

    impl Keyed for Entry {
        fn key(&self) -> (u64, i32) {
            (self.0, 0)
        }
    }

    #[test]
    fn pops_in_time_order() {
        for pop in [TickHeap::pop_msvc, TickHeap::pop_libcxx] {
            let mut heap = TickHeap::default();
            for (i, time) in [5u64, 1, 4, 1, 3, 9, 0, 2, 2, 7].into_iter().enumerate() {
                heap.push(Entry(time, i));
            }
            let times: Vec<u64> = std::iter::from_fn(|| pop(&mut heap)).map(|e| e.0).collect();
            assert_eq!(times, [0, 1, 1, 2, 2, 3, 4, 5, 7, 9]);
        }
    }

    /// `tickPendingTicks`: every entry popped into an active heap, which is then popped in turn.
    fn through_both_heaps(count: usize) -> Vec<usize> {
        let mut next = TickHeap::default();
        (0..count).for_each(|i| next.push(Entry(7, i)));
        let mut active = TickHeap::default();
        std::iter::from_fn(|| next.pop()).for_each(|e| active.push(e));
        std::iter::from_fn(|| active.pop()).map(|e| e.1).collect()
    }

    #[test]
    fn equal_entries_come_out_as_bds_orders_them() {
        // tick-queue.md "Order": up to four keep insertion order, five give e0,e4,e3,e2,e1.
        for count in 1..=4 {
            assert_eq!(through_both_heaps(count), (0..count).collect::<Vec<_>>());
        }
        assert_eq!(through_both_heaps(5), [0, 4, 3, 2, 1]);
    }
}
