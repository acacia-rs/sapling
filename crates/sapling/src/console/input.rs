//! Reading the console's lines: on a terminal through a line editor that completes command names
//! on Tab; from a pipe or file line by line as they come. Reading stops at the end of input.

use std::io::{BufRead, IsTerminal};
use std::sync::mpsc::Sender;

use rustyline::completion::Completer;
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::history::DefaultHistory;
use rustyline::validate::Validator;
use rustyline::{CompletionType, Config, Context, Editor, Helper};

use super::Names;

/// Reads stdin on a thread of its own, sending each line to `lines`.
pub(super) fn spawn(lines: Sender<String>, names: Names) {
    let reader = move || {
        if std::io::stdin().is_terminal() {
            edit(&lines, names);
        } else {
            for line in std::io::stdin().lock().lines().map_while(Result::ok) {
                if lines.send(line).is_err() {
                    return;
                }
            }
        }
    };
    if let Err(error) = std::thread::Builder::new().name("console".into()).spawn(reader) {
        tracing::warn!(%error, "no console: its thread did not start");
    }
}

fn edit(lines: &Sender<String>, names: Names) {
    let config = Config::builder().completion_type(CompletionType::List).auto_add_history(true).build();
    let mut editor = match Editor::<Completion, DefaultHistory>::with_config(config) {
        Ok(editor) => editor,
        Err(error) => {
            tracing::warn!(%error, "no console line editor");
            return;
        }
    };
    editor.set_helper(Some(Completion(names)));
    loop {
        let sent = match editor.readline("") {
            Ok(line) => lines.send(line).is_ok(),
            Err(ReadlineError::Interrupted) => true,
            Err(_) => false,
        };
        if !sent {
            return;
        }
    }
}

/// Completes the command name being typed, a leading `/` or not.
struct Completion(Names);

impl Completer for Completion {
    type Candidate = String;

    fn complete(&self, line: &str, pos: usize, _: &Context<'_>) -> rustyline::Result<(usize, Vec<String>)> {
        let typed = &line[..pos];
        let start = usize::from(typed.starts_with('/'));
        if typed[start..].contains(char::is_whitespace) {
            return Ok((pos, Vec::new()));
        }
        let names = self.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let prefix = typed[start..].to_ascii_lowercase();
        Ok((start, names.iter().filter(|name| name.to_ascii_lowercase().starts_with(&prefix)).cloned().collect()))
    }
}

impl Hinter for Completion {
    type Hint = String;
}

impl Highlighter for Completion {}

impl Validator for Completion {}

impl Helper for Completion {}
