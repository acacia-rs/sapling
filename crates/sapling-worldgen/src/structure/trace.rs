//! A debugging trace of one thread's structure stage (examples/structure_cells.rs): every block
//! written through a `Canvas` with the label of what wrote it. Off unless `capture` runs; the
//! hooks cost one thread-local read.

use std::cell::RefCell;
use std::rc::Rc;

use crate::block::Block;

/// Position, block, label.
pub type Write = ([i32; 3], Block, Rc<str>);

struct Trace {
    label: Rc<str>,
    writes: Vec<Write>,
}

thread_local! {
    static TRACE: RefCell<Option<Trace>> = const { RefCell::new(None) };
}

/// Runs `run` with the trace on and returns its writes in order.
pub fn capture<T>(run: impl FnOnce() -> T) -> (T, Vec<Write>) {
    TRACE.with(|t| *t.borrow_mut() = Some(Trace { label: Rc::from(""), writes: Vec::new() }));
    let result = run();
    (result, TRACE.with(|t| t.borrow_mut().take().map_or_else(Vec::new, |t| t.writes)))
}

/// Names the writes that follow.
pub fn label(text: impl FnOnce() -> String) {
    TRACE.with(|t| {
        if let Some(trace) = t.borrow_mut().as_mut() {
            trace.label = Rc::from(text());
        }
    });
}

pub fn write(at: [i32; 3], block: Block) {
    TRACE.with(|t| {
        if let Some(trace) = t.borrow_mut().as_mut() {
            trace.writes.push((at, block, trace.label.clone()));
        }
    });
}
