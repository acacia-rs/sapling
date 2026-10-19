//! The I/O threads' queue (loader.rs). Other jobs go first, in order. Loads go by area so that a
//! generating storage's workers each keep to their own part of the world:
//! - a worker takes the oldest load near the last column it loaded (its home);
//! - with none for `GRACE`, the oldest load near no other worker's home, which makes that its home;
//! - else the oldest load anywhere, helping a worker whose area has more than it can take.
//!
//! Workers far apart do not wait on each other's shared neighbours, and no more areas are generated
//! at once than there are workers, which is what `Generated` sizes its caches for. Loads in one area
//! keep the order they were asked in: a player's are nearest first.
//!
//! Urgent loads (a joining player's spawn columns) go before any area's, by area too: the oldest near
//! the worker's home, else the oldest. Taken in plain order, eight joining players' spawns were
//! generated interleaved over eight areas and every one of them spawned later.

use std::collections::VecDeque;
use std::sync::{Condvar, Mutex};
use std::time::{Duration, Instant};

use crate::ChunkPos;

/// Columns this close (in chunks, either axis) to a worker's home are its area: a radius-8 view's
/// width, and the neighbours generation reads around it.
const AREA_REACH: i32 = 20;
/// A shard asks for a player's next columns once a tick; an area that runs dry for less than this
/// keeps its worker. Leaving it half done would let its cached neighbours be evicted.
const GRACE: Duration = Duration::from_millis(100);

fn near(a: ChunkPos, b: ChunkPos) -> bool {
    (a.x - b.x).abs().max((a.z - b.z).abs()) <= AREA_REACH
}

pub(crate) enum Next<J> {
    Load(ChunkPos),
    Other(J),
}

pub(crate) struct JobQueue<J> {
    state: Mutex<State<J>>,
    queued: Condvar,
}

struct State<J> {
    other: VecDeque<J>,
    urgent: VecDeque<ChunkPos>,
    loads: VecDeque<ChunkPos>,
    homes: Vec<Option<ChunkPos>>,
    closed: bool,
}

impl<J> State<J> {
    fn take_at(&mut self, worker: usize, index: Option<usize>) -> Option<ChunkPos> {
        let pos = self.loads.remove(index?)?;
        self.homes[worker] = Some(pos);
        Some(pos)
    }

    /// The oldest urgent load near the worker's home, else the oldest; either becomes its home.
    fn take_urgent(&mut self, worker: usize) -> Option<ChunkPos> {
        let home = self.homes[worker];
        let index = self.urgent.iter().position(|pos| home.is_some_and(|home| near(*pos, home))).or((!self.urgent.is_empty()).then_some(0))?;
        let pos = self.urgent.remove(index)?;
        self.homes[worker] = Some(pos);
        Some(pos)
    }

    fn take_home(&mut self, worker: usize) -> Option<ChunkPos> {
        let home = self.homes[worker]?;
        let index = self.loads.iter().position(|pos| near(*pos, home));
        self.take_at(worker, index)
    }

    fn take_elsewhere(&mut self, worker: usize) -> Option<ChunkPos> {
        let homes = &self.homes;
        let free = |pos: &ChunkPos| homes.iter().enumerate().all(|(w, home)| w == worker || !home.is_some_and(|home| near(*pos, home)));
        let index = self.loads.iter().position(free).or((!self.loads.is_empty()).then_some(0));
        self.take_at(worker, index)
    }
}

impl<J> JobQueue<J> {
    pub fn new(workers: usize) -> Self {
        let state = State { other: VecDeque::new(), urgent: VecDeque::new(), loads: VecDeque::new(), homes: vec![None; workers], closed: false };
        Self { state: Mutex::new(state), queued: Condvar::new() }
    }

    pub fn push_load(&self, pos: ChunkPos, urgent: bool) {
        let mut state = self.state.lock().expect("loader queue lock");
        if urgent { state.urgent.push_back(pos) } else { state.loads.push_back(pos) }
        drop(state);
        self.queued.notify_all();
    }

    /// Makes a queued load urgent; one already taken or urgent stays as it is.
    pub fn promote(&self, pos: ChunkPos) {
        let mut state = self.state.lock().expect("loader queue lock");
        if let Some(index) = state.loads.iter().position(|&queued| queued == pos) {
            state.loads.remove(index);
            state.urgent.push_back(pos);
            drop(state);
            self.queued.notify_all();
        }
    }

    /// Takes the queued loads, urgent or not, that `keep` rejects out of the queue and returns
    /// them; a load a worker already took is not affected.
    pub fn cancel_loads(&self, keep: impl Fn(ChunkPos) -> bool) -> Vec<ChunkPos> {
        let mut state = self.state.lock().expect("loader queue lock");
        let mut cancelled = Vec::new();
        let mut sweep = |pos: &ChunkPos| {
            let kept = keep(*pos);
            if !kept {
                cancelled.push(*pos);
            }
            kept
        };
        state.urgent.retain(&mut sweep);
        state.loads.retain(&mut sweep);
        cancelled
    }

    pub fn push_other(&self, job: J) {
        self.state.lock().expect("loader queue lock").other.push_back(job);
        self.queued.notify_one();
    }

    /// Workers finish what is queued, then end.
    pub fn close(&self) {
        self.state.lock().expect("loader queue lock").closed = true;
        self.queued.notify_all();
    }

    /// Waits for `worker`'s next job; `None` once the queue is closed and empty.
    pub fn next(&self, worker: usize) -> Option<Next<J>> {
        let deadline = Instant::now() + GRACE;
        let mut state = self.state.lock().expect("loader queue lock");
        loop {
            if let Some(job) = state.other.pop_front() {
                return Some(Next::Other(job));
            }
            if let Some(pos) = state.take_urgent(worker) {
                return Some(Next::Load(pos));
            }
            if let Some(pos) = state.take_home(worker) {
                return Some(Next::Load(pos));
            }
            let patient = state.homes[worker].is_some() && !state.closed && Instant::now() < deadline;
            if let Some(pos) = (!patient).then(|| state.take_elsewhere(worker)).flatten() {
                return Some(Next::Load(pos));
            }
            if state.closed && state.loads.is_empty() && state.urgent.is_empty() {
                return None;
            }
            state = match (patient, state.loads.is_empty()) {
                (true, false) => self.queued.wait_timeout(state, deadline.saturating_duration_since(Instant::now())).expect("loader queue lock").0,
                _ => self.queued.wait(state).expect("loader queue lock"),
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urgent_loads_go_first_area_by_area() {
        let queue = JobQueue::new(1);
        queue.push_load(ChunkPos::new(0, 0), false);
        assert_eq!(load(&queue, 0), ChunkPos::new(0, 0));
        queue.push_load(ChunkPos::new(1, 0), false);
        queue.push_load(ChunkPos::new(100, 0), false);
        queue.promote(ChunkPos::new(100, 0));
        queue.push_load(ChunkPos::new(200, 0), true);
        queue.push_load(ChunkPos::new(101, 0), true);
        let urgent = [load(&queue, 0), load(&queue, 0), load(&queue, 0)];
        assert_eq!(urgent, [ChunkPos::new(100, 0), ChunkPos::new(101, 0), ChunkPos::new(200, 0)]);
        assert_eq!(load(&queue, 0), ChunkPos::new(1, 0), "then the plain load, after the grace");
    }

    fn load(queue: &JobQueue<()>, worker: usize) -> ChunkPos {
        match queue.next(worker) {
            Some(Next::Load(pos)) => pos,
            _ => panic!("a load"),
        }
    }

    #[test]
    fn workers_keep_to_their_own_areas_in_asked_order() {
        let queue = JobQueue::new(2);
        let (a, b) = (ChunkPos::new(0, 0), ChunkPos::new(100, 0));
        for i in 0..3 {
            queue.push_load(ChunkPos::new(a.x + i, 0), false);
            queue.push_load(ChunkPos::new(b.x + i, 0), false);
        }
        assert_eq!(load(&queue, 0), a);
        assert_eq!(load(&queue, 1), b, "the oldest load outside worker 0's area");
        assert_eq!([load(&queue, 0), load(&queue, 0)], [ChunkPos::new(1, 0), ChunkPos::new(2, 0)]);
        // Worker 0's area stays dry: after the grace it helps in worker 1's, in order.
        let start = Instant::now();
        assert_eq!(load(&queue, 0), ChunkPos::new(101, 0));
        assert!(start.elapsed() >= GRACE);
        assert_eq!(load(&queue, 1), ChunkPos::new(102, 0));
    }

    #[test]
    fn a_worker_waits_for_its_own_area_before_leaving_it() {
        let queue = JobQueue::new(1);
        queue.push_load(ChunkPos::new(0, 0), false);
        assert_eq!(load(&queue, 0), ChunkPos::new(0, 0));
        queue.push_load(ChunkPos::new(100, 0), false);
        std::thread::scope(|scope| {
            scope.spawn(|| {
                std::thread::sleep(GRACE / 4);
                queue.push_load(ChunkPos::new(1, 0), false);
            });
            assert_eq!(load(&queue, 0), ChunkPos::new(1, 0), "the next tick's column, not the far one");
        });
        assert_eq!(load(&queue, 0), ChunkPos::new(100, 0));
    }

    #[test]
    fn cancelled_loads_are_never_handed_out() {
        let queue = JobQueue::new(1);
        for x in 0..4 {
            queue.push_load(ChunkPos::new(x, 0), x == 3);
        }
        let mut cancelled = queue.cancel_loads(|pos| pos.x % 2 == 0);
        cancelled.sort();
        assert_eq!(cancelled, [ChunkPos::new(1, 0), ChunkPos::new(3, 0)]);
        queue.close();
        assert_eq!([load(&queue, 0), load(&queue, 0)], [ChunkPos::new(0, 0), ChunkPos::new(2, 0)]);
        assert!(queue.next(0).is_none());
    }

    #[test]
    fn other_jobs_go_first_and_closing_drains_the_queue() {
        let queue = JobQueue::new(1);
        queue.push_load(ChunkPos::new(0, 0), true);
        queue.push_other(());
        queue.close();
        assert!(matches!(queue.next(0), Some(Next::Other(()))));
        assert!(matches!(queue.next(0), Some(Next::Load(_))));
        assert!(queue.next(0).is_none());
    }
}
