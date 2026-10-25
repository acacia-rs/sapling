//! Running a function as BDS's FunctionManager does (bdsre `0x140fc4150`): its lines one by one as
//! the sender, a `function` line opening the named function in its place, until the world's
//! `functionCommandLimit` lines (each `function` line one of them) have run; what was left is
//! dropped. A line's own answer goes nowhere. A function whose lines do not all read as commands
//! is as unknown as one that does not exist, as BDS refuses to load it.

use std::collections::HashMap;
use std::sync::Arc;

use sapling_api::{Permission, RuleValue, Sender, ShardCtx};

use super::{Function, Functions};

/// BDS's cap when the world has no `functionCommandLimit`.
const MAX_LINES: u32 = 10_000;

/// A function being run and the index of its next line.
struct Frame {
    function: Arc<Function>,
    next: usize,
}

/// A shard's functions, which of them read as commands, and the lines still to run while one runs.
#[derive(Default)]
pub(crate) struct FunctionState {
    functions: Arc<Functions>,
    valid: HashMap<String, bool>,
    frames: Vec<Frame>,
    running: bool,
}

impl FunctionState {
    pub fn install(&mut self, functions: Arc<Functions>) {
        self.functions = functions;
    }

    pub fn tick_functions(&self) -> Vec<String> {
        self.functions.tick.clone()
    }

    fn next_line(&mut self) -> Option<String> {
        loop {
            let frame = self.frames.last_mut()?;
            match frame.function.lines.get(frame.next) {
                Some(line) => {
                    frame.next += 1;
                    return Some(line.clone());
                }
                None => drop(self.frames.pop()),
            }
        }
    }
}

/// What came of calling a function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Called {
    /// No such function, or one whose lines are not all commands.
    NotFound,
    /// Called from a running function: its lines run next, in the caller's count.
    Opened,
    /// Run through: how many lines ran.
    Ran(u32),
}

/// The function `name`, if it exists here and every line of it reads as a command.
pub(crate) fn find(ctx: &mut ShardCtx<'_>, name: &str) -> Option<Arc<Function>> {
    let function = ctx.state::<FunctionState>().functions.by_name.get(name)?.clone();
    if let Some(valid) = ctx.state::<FunctionState>().valid.get(name) {
        return valid.then_some(function);
    }
    let commands = ctx.commands();
    // BDS: "Commands in functions may not begin with '/'".
    let valid = function.lines.iter().all(|line| !line.starts_with('/') && commands.parses(line, Permission::Operator));
    if !valid {
        tracing::warn!(function = name, "not run: a line of it is not a command");
    }
    ctx.state::<FunctionState>().valid.insert(name.to_owned(), valid);
    valid.then_some(function)
}

fn line_limit(ctx: &ShardCtx<'_>) -> u32 {
    match ctx.game_rule("functionCommandLimit") {
        Some((_, RuleValue::Int(limit))) => limit.max(0) as u32,
        _ => MAX_LINES,
    }
}

/// Runs the function `name` as `sender`, or opens it in the function running now.
pub(crate) fn call(ctx: &mut ShardCtx<'_>, name: &str, sender: &Sender) -> Called {
    let Some(function) = find(ctx, name) else { return Called::NotFound };
    let state = ctx.state::<FunctionState>();
    state.frames.push(Frame { function, next: 0 });
    if state.running {
        return Called::Opened;
    }
    state.running = true;
    let (limit, commands) = (line_limit(ctx), ctx.commands());
    let mut ran = 0;
    while let Some(line) = ctx.state::<FunctionState>().next_line() {
        commands.run(ctx, sender, Permission::Operator, &line);
        ran += 1;
        if ran >= limit {
            break;
        }
    }
    let state = ctx.state::<FunctionState>();
    state.frames.clear();
    state.running = false;
    Called::Ran(ran)
}
