//! Behavior pack functions, run as BDS runs them (research/captures/bds-functions.txt): `/function`
//! (run.rs), `/schedule delay` (schedule.rs) and the functions `tick.json` lists, every tick in
//! every world. Opt in with [`crate::ServerBuilder::behavior_packs`].

mod commands;
mod reload;
mod run;
mod schedule;

use std::collections::HashMap;
use std::sync::Arc;

use sapling_api::{Plugin, PluginBuilder};

pub(crate) use reload::Library;
use run::FunctionState;

/// A function: its command lines, comments and blank lines left out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Function {
    pub name: String,
    pub lines: Vec<String>,
}

impl Function {
    /// `None` for a file with no command in it, which BDS does not load.
    pub fn parse(name: &str, text: &str) -> Option<Self> {
        let lines: Vec<String> = text.lines().map(str::trim_start).filter(|line| !line.is_empty() && !line.starts_with('#')).map(str::to_owned).collect();
        (!lines.is_empty()).then(|| Self { name: name.to_owned(), lines })
    }
}

/// The functions of a server's behavior packs, by name (`folder/name`, case kept), and those
/// `tick.json` lists, in its order.
#[derive(Debug, Clone, Default)]
pub(crate) struct Functions {
    pub by_name: HashMap<String, Arc<Function>>,
    pub tick: Vec<String>,
}

/// Hands each shard the functions, and runs the scheduled and `tick.json` ones every tick.
pub(crate) struct FunctionsPlugin(pub Library);

impl Plugin for FunctionsPlugin {
    fn build(&self, plugin: &mut PluginBuilder<'_>) {
        let library = self.0.clone();
        plugin.command::<commands::FunctionCommand>().command::<schedule::Schedule>().command::<reload::Reload>();
        plugin.later(0, move |ctx| ctx.state::<FunctionState>().install(library));
        plugin.every(1, schedule::tick);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_function_keeps_its_commands_in_order() {
        let function = Function::parse("probe/hello", "# a comment\n\n   say hello  \r\nscoreboard players add @s fn 1\n").unwrap();
        assert_eq!(function.lines, ["say hello  ", "scoreboard players add @s fn 1"]);
        assert_eq!(Function::parse("probe/empty", "\n# only a comment\n"), None);
    }
}
