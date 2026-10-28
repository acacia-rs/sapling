//! The server's console ([`crate::ServerBuilder::console`]): each line read from stdin (input.rs)
//! runs as a command of the server (`Sender::Console`, operator commands) on the entry shard at its
//! next tick, and the answer is logged in English as BDS's console prints it
//! (research/captures/bds-console-cmds.txt): INFO when it succeeded, ERROR when not, the answer's
//! lines together in one entry. BDS does not echo the line itself.

mod input;

use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};

use sapling_api::{Handler, Output, Permission, Sender, ShardCtx};

/// Command names for completion, filled at the first tick.
type Names = Arc<Mutex<Vec<String>>>;

pub(crate) struct Console {
    lines: Receiver<String>,
    names: Names,
}

/// Starts reading stdin; the handler that runs what is read.
pub(crate) fn start() -> Console {
    let (sender, lines) = channel();
    let names = Names::default();
    input::spawn(sender, names.clone());
    Console { lines, names }
}

impl Handler for Console {
    fn on_tick(&mut self, ctx: &mut ShardCtx<'_>) {
        let commands = ctx.commands();
        {
            let mut names = self.names.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            if names.is_empty() {
                *names = commands.visible(Permission::Operator).flat_map(|spec| std::iter::once(spec.name.to_string()).chain(spec.aliases.iter().map(ToString::to_string))).collect();
                names.sort();
            }
        }
        while let Ok(line) = self.lines.try_recv() {
            let line = line.trim();
            if !line.is_empty() {
                log(&commands.run(ctx, &Sender::Console, Permission::Operator, line));
            }
        }
    }
}

/// What BDS's console prints of an answer.
fn log(output: &Output) {
    let messages = output.messages();
    if messages.is_empty() {
        return;
    }
    let text: Vec<String> = messages.iter().map(|message| sapling_data::command_text(&message.text, &message.parameters)).collect();
    let text = text.join("\n");
    if messages.iter().all(|message| message.success) {
        tracing::info!("{text}");
    } else {
        tracing::error!("{text}");
    }
}
