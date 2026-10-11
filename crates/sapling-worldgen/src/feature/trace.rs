//! A debugging trace of one thread's decoration (examples/feature_trace.rs): each rule's tries and
//! each tree attempt with its draw range, to set beside BDS's cells or a breakpoint log. Off unless
//! `capture` runs; the hooks cost one thread-local read.

use std::cell::RefCell;

thread_local! {
    static LINES: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
}

/// Runs `run` with the trace on and returns its lines.
pub fn capture<T>(run: impl FnOnce() -> T) -> (T, Vec<String>) {
    LINES.with(|lines| *lines.borrow_mut() = Some(Vec::new()));
    let result = run();
    (result, LINES.with(|lines| lines.borrow_mut().take().unwrap_or_default()))
}

pub fn on() -> bool {
    LINES.with(|lines| lines.borrow().is_some())
}

pub fn line(text: impl FnOnce() -> String) {
    if !on() {
        return;
    }
    // `text` may itself run traced code.
    let text = text();
    LINES.with(|lines| lines.borrow_mut().as_mut().expect("on").push(text));
}
