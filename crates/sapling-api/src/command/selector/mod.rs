//! Target selectors, as Bedrock writes them: `@s`, `@p`, `@a`, `@r`, `@e`, `@n` and `@initiator`,
//! each with filters in brackets (`@e[type=!player,r=..8,c=-2]`), or a player's name.
//! [`ShardCtx::select`](crate::ShardCtx::select) resolves one; every command that takes a target
//! should go through it.

mod filters;
mod gather;
mod items;
mod lex;
mod matching;
mod resolve;
#[cfg(test)]
mod tests;

pub use filters::{Bounds, Filters, Input, Not};
pub use items::{Inventory, ItemTest, Location};
pub use resolve::Selected;

use super::{Args, ParamKind, Parameter, ParseError};
use lex::Span;

/// The `@` part of a selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Base {
    /// `@s`: the sender.
    Sender,
    /// `@p`: the nearest player.
    NearestPlayer,
    /// `@a`: every player.
    Players,
    /// `@r`: a player at random; with `type=`, any entity.
    RandomPlayer,
    /// `@e`: every entity, players too.
    Entities,
    /// `@n`: the nearest entity, players too.
    NearestEntity,
    /// `@initiator`: the player talking to an NPC; nobody, as sapling has no NPC dialogues.
    Initiator,
}

const BASES: [(&str, Base); 7] = [
    ("s", Base::Sender),
    ("p", Base::NearestPlayer),
    ("a", Base::Players),
    ("r", Base::RandomPlayer),
    ("e", Base::Entities),
    ("n", Base::NearestEntity),
    ("initiator", Base::Initiator),
];

/// Who or what a command is aimed at.
#[derive(Debug, Clone, PartialEq)]
pub enum Selector {
    /// A player by name.
    Named(String),
    Variable(Base, Filters),
}

impl Selector {
    /// `@s`, what a command's left-out target means.
    pub fn sender() -> Self {
        Self::Variable(Base::Sender, Filters::default())
    }

    /// Whether its `type=` rules players out, whoever it finds.
    pub fn excludes_players(&self) -> bool {
        let Self::Variable(_, filters) = self else { return false };
        filters.types.iter().any(|kind| !kind.passes(kind.value == "minecraft:player"))
    }

    /// The selector `word`, which starts at byte `at` of the line.
    fn read(word: &str, at: usize) -> Result<Self, Span> {
        let Some(body) = word.strip_prefix('@') else { return Ok(Self::Named(word.to_owned())) };
        let name_end = body.find('[').unwrap_or(body.len());
        let base = BASES.iter().find(|(name, _)| *name == &body[..name_end]).map(|(_, base)| *base).ok_or((at, at + 1 + name_end))?;
        let brackets = &body[name_end..];
        if brackets.is_empty() {
            return Ok(Self::Variable(base, Filters::default()));
        }
        let end = at + word.len();
        let inner = brackets.strip_prefix('[').and_then(|inner| inner.strip_suffix(']')).ok_or((end, end))?;
        Filters::read(inner, at + 2 + name_end).map(|filters| Self::Variable(base, filters))
    }
}

impl Parameter for Selector {
    fn kind() -> ParamKind {
        ParamKind::Target
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        if !args.peek().is_some_and(|word| word.starts_with('@')) {
            return args.word().map(|name| Self::Named(name.to_owned()));
        }
        let (at, word) = args.balanced()?;
        Self::read(word, at).map_err(|(start, end)| args.error_span(start, end))
    }
}
