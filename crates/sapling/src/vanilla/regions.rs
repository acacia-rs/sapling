//! `/fill` and its modes, and the box of blocks it shares with `/clone` and `/testforblocks`
//! (copying.rs), answered as BDS answers (research/captures/bds-worldcmds.txt).

use sapling_api::command::{Args, BlockCoords, CommandSpec, CommandSyntax, Overload, ParamKind, Parameter, ParameterSpec, ParseError, States};
use sapling_api::{Block, BlockPos, Command, CommandEnum, Output, Permission, Sender, ShardCtx};

use super::blocks::{block, changed, has_states};

/// The most blocks one command fills, copies or compares, as vanilla.
pub(super) const MOST_BLOCKS: i64 = 32_768;

/// A box of blocks between two corners, both in it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Region {
    pub low: BlockPos,
    pub high: BlockPos,
}

impl Region {
    pub fn between(a: BlockPos, b: BlockPos) -> Self {
        Self { low: std::array::from_fn(|axis| a[axis].min(b[axis])), high: std::array::from_fn(|axis| a[axis].max(b[axis])) }
    }

    pub fn count(&self) -> i64 {
        (0..3).map(|axis| i64::from(self.high[axis]) - i64::from(self.low[axis]) + 1).product()
    }

    pub fn cells(self) -> impl Iterator<Item = BlockPos> {
        let [low, high] = [self.low, self.high];
        (low[0]..=high[0]).flat_map(move |x| (low[1]..=high[1]).flat_map(move |y| (low[2]..=high[2]).map(move |z| [x, y, z])))
    }

    /// The same box with its low corner at `low`.
    pub fn moved_to(&self, low: BlockPos) -> Self {
        Self { low, high: std::array::from_fn(|axis| low[axis] + self.high[axis] - self.low[axis]) }
    }

    pub fn overlaps(&self, other: &Self) -> bool {
        (0..3).all(|axis| self.low[axis] <= other.high[axis] && other.low[axis] <= self.high[axis])
    }

    fn is_edge(&self, pos: BlockPos) -> bool {
        (0..3).any(|axis| pos[axis] == self.low[axis] || pos[axis] == self.high[axis])
    }

    /// Whether every block of it is in the world and loaded.
    pub fn is_loaded(self, ctx: &ShardCtx<'_>) -> bool {
        self.cells().all(|pos| ctx.block(pos).is_some())
    }
}

/// What `/fill` does with the blocks that are there.
#[derive(CommandEnum, Clone, Copy, PartialEq, Debug)]
#[command(crate = "crate")]
pub(crate) enum FillMode {
    /// All of them; with a block named after it, only those.
    Replace,
    /// Breaks each first, leaving what it drops.
    Destroy,
    /// Only fills air.
    Keep,
    /// The box's faces, air inside.
    Hollow,
    /// The box's faces, the inside left alone.
    Outline,
}

/// `/fill <from> <to> <block> [states] [mode]`, or `... replace [block] [states]` to fill only
/// where that block is.
pub(crate) struct Fill {
    from: BlockCoords,
    to: BlockCoords,
    block: String,
    states: Option<States>,
    mode: FillMode,
    only: Option<(String, Option<States>)>,
}

impl CommandSyntax for Fill {
    fn spec() -> CommandSpec {
        // BDS's overloads less those with data values: states, when given, come before the mode.
        let box_and_block = |states: bool| {
            let block = vec![ParameterSpec::of::<BlockCoords>("from"), ParameterSpec::of::<BlockCoords>("to"), block_spec("tileName")];
            [block, states.then(|| ParameterSpec::of::<States>("blockStates")).into_iter().collect()].concat()
        };
        let mode = || vec![ParameterSpec::of::<Option<FillMode>>("oldBlockHandling")];
        let replace = || vec![ParameterSpec::literal("fill", "replace"), ParameterSpec { optional: true, ..block_spec("replaceTileName") }, ParameterSpec::of::<Option<States>>("replaceBlockStates")];
        let overloads = [(true, mode()), (true, replace()), (false, mode()), (false, replace())];
        CommandSpec {
            name: "fill".into(),
            aliases: Vec::new(),
            description: "Fills all or parts of a region with a block".into(),
            permission: Permission::Operator,
            overloads: overloads.into_iter().map(|(states, rest)| Overload { parameters: [box_and_block(states), rest].concat() }).collect(),
        }
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        let (from, to, block) = (BlockCoords::parse(args)?, BlockCoords::parse(args)?, args.word()?.to_owned());
        let states = Option::<States>::parse(args)?;
        let mode = Option::<FillMode>::parse(args)?.unwrap_or(FillMode::Replace);
        let only = match mode == FillMode::Replace && !args.is_empty() {
            true => Some((args.word()?.to_owned(), Option::<States>::parse(args)?)),
            false => None,
        };
        args.finish()?;
        Ok(Self { from, to, block, states, mode, only })
    }
}

pub(super) fn block_spec(name: &str) -> ParameterSpec {
    ParameterSpec { name: name.into(), kind: ParamKind::String, optional: false }
}

impl Command for Fill {
    /// BDS counts the blocks that changed: none for `air destroy`, whose blocks the breaking
    /// already took.
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let here = ctx.origin(sender);
        let region = Region::between(self.from.from(here), self.to.from(here));
        if region.count() > MOST_BLOCKS {
            return out.translate("commands.fill.tooManyBlocks", vec![region.count().to_string(), MOST_BLOCKS.to_string()], false);
        }
        let block = block(&self.block, self.states.as_ref());
        let only = self.only.as_ref().map(|(name, states)| super::blocks::block(name, states.as_ref()));
        let air = Block::air();
        let mut filled = 0;
        for pos in region.cells() {
            let there = || ctx.block(pos).unwrap_or_else(Block::air);
            let put = match self.mode {
                FillMode::Replace => only.as_ref().is_none_or(|only| there().name == only.name && has_states(&there(), &only.properties)).then_some(&block),
                FillMode::Keep => there().is_air().then_some(&block),
                FillMode::Hollow if !region.is_edge(pos) => Some(&air),
                FillMode::Outline if !region.is_edge(pos) => None,
                FillMode::Destroy => {
                    ctx.break_block(pos);
                    Some(&block)
                }
                FillMode::Hollow | FillMode::Outline => Some(&block),
            };
            filled += put.map_or(0, |put| u32::from(changed(ctx, pos, put)));
        }
        out.translate("commands.fill.success", vec![filled.to_string()], filled > 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(line: &str) -> Result<(FillMode, Option<(String, Option<States>)>), ParseError> {
        let sender = Sender::Console;
        Fill::parse(&mut Args::new(line, 0, &sender, &[])).map(|fill| (fill.mode, fill.only))
    }

    #[test]
    fn fill_takes_a_mode_or_a_block_to_replace() {
        assert_eq!(parsed("1 2 3 4 5 6 stone").unwrap(), (FillMode::Replace, None));
        assert_eq!(parsed("1 2 3 4 5 6 stone hollow").unwrap(), (FillMode::Hollow, None));
        assert_eq!(parsed(r#"1 2 3 4 5 6 stone [] replace oak_log ["pillar_axis"="x"]"#).unwrap(), (FillMode::Replace, Some(("oak_log".into(), Some(States("pillar_axis=x".into()))))));
        assert!(parsed("1 2 3 4 5 6 stone bogus").is_err() && parsed("1 2 3 4 5 6 stone keep dirt").is_err());
    }

    #[test]
    fn a_region_is_counted_walked_and_tested_for_overlap() {
        let region = Region::between([3, 0, 5], [1, 2, 4]);
        assert_eq!((region.low, region.high, region.count()), ([1, 0, 4], [3, 2, 5], 18));
        assert_eq!(region.cells().count(), 18);
        assert_eq!(region.cells().filter(|pos| !region.is_edge(*pos)).collect::<Vec<_>>(), Vec::<BlockPos>::new());
        assert_eq!(Region::between([0, 0, 0], [2, 2, 2]).cells().filter(|pos| !Region::between([0, 0, 0], [2, 2, 2]).is_edge(*pos)).collect::<Vec<_>>(), [[1, 1, 1]]);
        assert!(region.overlaps(&region.moved_to([3, 2, 5])) && !region.overlaps(&region.moved_to([4, 0, 4])));
    }
}
