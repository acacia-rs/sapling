//! An item or a block as a vanilla command names it: a word BDS's commands take, else the syntax
//! error BDS answers (`sapling_data::command_takes_item`).

use sapling_api::command::{Args, ParamKind, Parameter, ParseError};

/// An item's name as typed: `diamond`, `minecraft:potion`, an old name BDS still takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ItemName(pub String);

/// A block's name as typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BlockName(pub String);

fn taken(args: &mut Args<'_>, takes: fn(&str) -> bool) -> Result<String, ParseError> {
    let mark = args.mark();
    let word = args.word()?;
    if takes(word) { Ok(word.to_owned()) } else { Err(args.error_at(mark)) }
}

impl Parameter for ItemName {
    fn kind() -> ParamKind {
        ParamKind::String
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        taken(args, sapling_data::command_takes_item).map(Self)
    }
}

impl Parameter for BlockName {
    fn kind() -> ParamKind {
        ParamKind::String
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        taken(args, sapling_data::command_takes_block).map(Self)
    }
}
