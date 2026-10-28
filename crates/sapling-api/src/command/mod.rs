//! Commands: a struct whose fields are the parameters, or an enum whose variants are
//! sub-commands. `#[derive(Command)]` writes the syntax ([`CommandSyntax`]); the plugin writes
//! [`Command::run`]. Registered commands are shown to each client that may use them, with their
//! syntax and completions, and run on the sender's shard.

mod args;
mod executed;
mod output;
mod params;
mod places;
mod rawtext;
mod registry;
mod selector;
mod soft_enum;

pub use args::{Args, ParseError};
pub use executed::Executed;
pub use output::{Output, OutputMessage};
pub use params::{Parameter, Target, Text, parse_choice};
pub use places::{Axis, BlockCoords, Coords, States};
pub use rawtext::{RawPart, RawText};
pub use selector::{Base, Bounds, Filters, Input, Inventory, ItemTest, Location, Not, Selected, Selector};
pub use registry::Commands;
pub use soft_enum::SoftEnumChange;

use crate::{Permission, Player, ShardCtx};

/// What a command does once its arguments are parsed.
pub trait Command: CommandSyntax + 'static {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output);
}

/// A command's name, parameters and parser; written by `#[derive(Command)]`.
pub trait CommandSyntax: Sized {
    fn spec() -> CommandSpec;
    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError>;
}

/// Who ran a command.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Sender {
    Player(Player),
    /// The server's console (`ServerBuilder::console`).
    Console,
    Executed(Box<Executed>),
}

impl Sender {
    /// The player who sent the command, or the one an `/execute` runs it as.
    pub fn player(&self) -> Option<&Player> {
        match self {
            Self::Player(player) => Some(player),
            Self::Console => None,
            Self::Executed(executed) => match &executed.executor {
                Some(Selected::Player(player)) => Some(player),
                _ => None,
            },
        }
    }

    pub fn name(&self) -> &str {
        self.player().map_or("Console", |p| p.name.as_str())
    }
}

/// A command as clients are told about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    pub name: String,
    pub aliases: Vec<String>,
    pub description: String,
    /// Players below this neither see nor run the command.
    pub permission: Permission,
    /// The ways to call it: one for a struct, one per sub-command for an enum.
    pub overloads: Vec<Overload>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Overload {
    pub parameters: Vec<ParameterSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterSpec {
    pub name: String,
    pub kind: ParamKind,
    pub optional: bool,
}

impl ParameterSpec {
    pub fn of<P: Parameter>(name: &str) -> Self {
        Self { name: name.to_owned(), kind: P::kind(), optional: P::OPTIONAL }
    }

    /// A word parameter (`String` or `Option<String>`) completed from the soft enum `enum_name`,
    /// as `#[command(soft_enum = "Arena")]` on a field declares it.
    pub fn soft_enum<P: Parameter>(name: &str, enum_name: &str) -> Self {
        Self { name: name.to_owned(), kind: ParamKind::SoftEnum { name: enum_name.to_owned(), values: Vec::new() }, optional: P::OPTIONAL }
    }

    /// The fixed word that picks sub-command `word` of `command`.
    pub fn literal(command: &str, word: &str) -> Self {
        Self { name: word.to_owned(), kind: ParamKind::Enum { name: format!("{command}_{word}"), values: vec![word.to_owned()] }, optional: false }
    }
}

/// What a parameter takes, as far as the client's command line needs to know.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParamKind {
    Int,
    Float,
    /// One word, or several in quotes.
    String,
    /// Everything up to the end of the line.
    Text,
    /// A player's name or a selector.
    Target,
    /// One of a fixed set of words, completed by the client.
    Enum { name: String, values: Vec<String> },
    /// One number that may be `~` and an offset: an axis of a place.
    Value,
    /// Three numbers, each perhaps `~` and an offset: a point.
    Position,
    /// The same, naming a block.
    BlockPosition,
    /// A block's states in brackets.
    BlockStates,
    /// A JSON object: rawtext.
    Json,
    /// A path of words between `/`: a function's name.
    FilePath,
    /// One word, completed from an enum the plugin changes while the server runs (arena names):
    /// these are its first values; [`ShardCtx::set_soft_enum`] and the like change them.
    SoftEnum { name: String, values: Vec<String> },
}
