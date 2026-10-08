//! Probe hooks for draw desyncs (examples/mineshaft_draws.rs): the chunk generator's draw count
//! before each piece and at marked steps, and a restart of it at a chosen draw before one piece.

use std::sync::Mutex;

use crate::random::Mt19937;

/// Restart the generator at `draws` before `piece` of the start in `start`, in `chunk`.
#[derive(Debug, Clone, Copy)]
pub struct Shift {
    pub start: (i32, i32),
    pub chunk: (i32, i32),
    pub piece: usize,
    pub draws: u32,
}

/// (start, chunk, piece, step, draws before it).
pub type Event = ((i32, i32), (i32, i32), usize, &'static str, u32);

pub static SHIFT: Mutex<Option<Shift>> = Mutex::new(None);
/// Events while Some.
pub static TRACE: Mutex<Option<Vec<Event>>> = Mutex::new(None);
static CURRENT: Mutex<((i32, i32), (i32, i32), usize)> = Mutex::new(((0, 0), (0, 0), 0));

pub(super) fn before_piece(start: (i32, i32), chunk: (i32, i32), piece: usize, random: &mut Mt19937) {
    *CURRENT.lock().unwrap() = (start, chunk, piece);
    mark("piece", random);
    if let Some(shift) = *SHIFT.lock().unwrap()
        && (shift.start, shift.chunk, shift.piece) == (start, chunk, piece)
    {
        *random = Mt19937::new(random.seed());
        random.consume_count(shift.draws);
    }
}

/// Records `step` of the current piece while tracing.
pub(super) fn mark(step: &'static str, random: &Mt19937) {
    if let Some(trace) = TRACE.lock().unwrap().as_mut() {
        let (start, chunk, piece) = *CURRENT.lock().unwrap();
        trace.push((start, chunk, piece, step, random.draws()));
    }
}
