//! `/clone` and `/testforblocks`, answered as BDS answers (research/captures/bds-worldcmds.txt).
// TODO: BDS marks a cloned chest `forceunpair`; here a clone holds what its source held, as is.

use sapling_api::command::{Args, BlockCoords, CommandSpec, CommandSyntax, Overload, Parameter, ParameterSpec, ParseError, States};
use sapling_api::{Block, BlockEntityData, BlockPos, Command, CommandEnum, Output, Permission, Sender, ShardCtx};

use super::blocks::{block, has_states};
use super::regions::{MOST_BLOCKS, Region, block_spec};

/// Which blocks of the source `/clone` copies.
#[derive(CommandEnum, Clone, Copy, PartialEq, Debug)]
#[command(crate = "crate")]
pub(crate) enum MaskMode {
    /// All of them, air too.
    Replace,
    /// All but air.
    Masked,
}

#[derive(CommandEnum, Clone, Copy, PartialEq, Debug)]
#[command(crate = "crate")]
pub(crate) enum CloneMode {
    Normal,
    /// Copies into a box that overlaps the source.
    Force,
    /// Leaves air where the copied blocks were.
    Move,
}

#[derive(Debug, PartialEq)]
enum Mask {
    All,
    NotAir,
    Only(Block),
}

/// `/clone <begin> <end> <destination> [replace|masked] [mode]`, or `... filtered <mode> <block>
/// [states]` to copy only that block.
pub(crate) struct CloneCommand {
    begin: BlockCoords,
    end: BlockCoords,
    destination: BlockCoords,
    mask: Mask,
    mode: CloneMode,
}

impl CommandSyntax for CloneCommand {
    fn spec() -> CommandSpec {
        let corners = || ["begin", "end", "destination"].map(ParameterSpec::of::<BlockCoords>).to_vec();
        let filtered = [ParameterSpec::literal("clone", "filtered"), ParameterSpec::of::<CloneMode>("cloneMode"), block_spec("tileName"), ParameterSpec::of::<Option<States>>("blockStates")];
        CommandSpec {
            name: "clone".into(),
            aliases: Vec::new(),
            description: "Clones blocks from one region to another".into(),
            permission: Permission::Operator,
            overloads: vec![
                Overload { parameters: [corners(), vec![ParameterSpec::of::<Option<MaskMode>>("maskMode"), ParameterSpec::of::<Option<CloneMode>>("cloneMode")]].concat() },
                Overload { parameters: [corners(), filtered.into()].concat() },
            ],
        }
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        let (begin, end, destination) = (BlockCoords::parse(args)?, BlockCoords::parse(args)?, BlockCoords::parse(args)?);
        let (mask, mode) = match args.peek() {
            Some(word) if word.eq_ignore_ascii_case("filtered") => {
                args.word()?;
                let mode = CloneMode::parse(args)?;
                let name = super::names::BlockName::parse(args)?.0;
                (Mask::Only(block(&name, Option::<States>::parse(args)?.as_ref())), mode)
            }
            _ => {
                let mask = match Option::<MaskMode>::parse(args)? {
                    Some(MaskMode::Masked) => Mask::NotAir,
                    _ => Mask::All,
                };
                (mask, Option::<CloneMode>::parse(args)?.unwrap_or(CloneMode::Normal))
            }
        };
        args.finish()?;
        Ok(Self { begin, end, destination, mask, mode })
    }
}

/// The answer for a source or destination too big or not all in the world.
fn refused(ctx: &ShardCtx<'_>, regions: [Region; 2], out: &mut Output, too_many: &str) -> bool {
    let count = regions[0].count();
    if count > MOST_BLOCKS {
        out.translate(too_many, vec![count.to_string(), MOST_BLOCKS.to_string()], false);
    } else if !regions.iter().all(|region| region.is_loaded(ctx)) {
        out.translate("commands.generic.outOfWorld", Vec::new(), false);
    } else {
        return false;
    }
    true
}

impl Command for CloneCommand {
    /// BDS counts every block it copies, changed or not.
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let here = ctx.origin(sender);
        let source = Region::between(self.begin.from(here), self.end.from(here));
        let target = source.moved_to(self.destination.from(here));
        if refused(ctx, [source, target], out, "commands.clone.tooManyBlocks") {
            return;
        }
        if self.mode != CloneMode::Force && source.overlaps(&target) {
            return out.translate("commands.clone.noOverlap", Vec::new(), false);
        }
        let copied: Vec<(BlockPos, Block, Option<BlockEntityData>)> = source
            .cells()
            .filter_map(|pos| Some((pos, ctx.block(pos)?, ctx.block_entity(pos))))
            .filter(|(_, there, _)| match &self.mask {
                Mask::All => true,
                Mask::NotAir => !there.is_air(),
                Mask::Only(only) => there.name == only.name && has_states(there, &only.properties),
            })
            .collect();
        if copied.is_empty() {
            return out.translate("commands.clone.failed", Vec::new(), false);
        }
        ctx.batch_blocks(|ctx| {
            if self.mode == CloneMode::Move {
                for (pos, ..) in &copied {
                    ctx.update_block(*pos, &Block::air());
                }
            }
            for (pos, there, entity) in &copied {
                let to = std::array::from_fn(|axis| pos[axis] - source.low[axis] + target.low[axis]);
                ctx.update_block(to, there);
                if let Some(entity) = entity {
                    ctx.set_block_entity(to, entity);
                }
            }
        });
        out.translate("commands.clone.success", vec![copied.len().to_string()], true);
    }
}

/// `/testforblocks <begin> <end> <destination> [all|masked]`: whether the box at the destination
/// holds the blocks of the first; `masked` leaves out the first's air.
#[derive(Command)]
#[command(crate = "crate", name = "testforblocks", description = "Tests whether the blocks in two regions match", permission = "operator")]
pub(crate) struct TestForBlocks {
    begin: BlockCoords,
    end: BlockCoords,
    destination: BlockCoords,
    mode: Option<CompareMode>,
}

#[derive(CommandEnum, Clone, Copy, PartialEq, Debug)]
#[command(crate = "crate")]
pub(crate) enum CompareMode {
    All,
    Masked,
}

impl Command for TestForBlocks {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let here = ctx.origin(sender);
        let source = Region::between(self.begin.from(here), self.end.from(here));
        let target = source.moved_to(self.destination.from(here));
        if refused(ctx, [source, target], out, "commands.compare.tooManyBlocks") {
            return;
        }
        let mut compared = 0;
        for pos in source.cells() {
            let there = ctx.block(pos).unwrap_or_else(Block::air);
            if self.mode == Some(CompareMode::Masked) && there.is_air() {
                continue;
            }
            if ctx.block(std::array::from_fn(|axis| pos[axis] - source.low[axis] + target.low[axis])) != Some(there) {
                return out.translate("commands.compare.failed", Vec::new(), false);
            }
            compared += 1;
        }
        out.translate("commands.compare.success", vec![compared.to_string()], true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(line: &str) -> Result<(Mask, CloneMode), ParseError> {
        let sender = Sender::Console;
        CloneCommand::parse(&mut Args::new(line, 0, &sender, &[])).map(|clone| (clone.mask, clone.mode))
    }

    #[test]
    fn clone_takes_a_mask_and_a_mode_or_a_filter() {
        assert_eq!(parsed("0 0 0 1 1 1 5 5 5").unwrap(), (Mask::All, CloneMode::Normal));
        assert_eq!(parsed("0 0 0 1 1 1 5 5 5 masked move").unwrap(), (Mask::NotAir, CloneMode::Move));
        assert_eq!(parsed("0 0 0 1 1 1 5 5 5 replace force").unwrap(), (Mask::All, CloneMode::Force));
        let wool = Block::new("minecraft:red_wool").with("color=red");
        assert_eq!(parsed(r#"0 0 0 1 1 1 5 5 5 filtered normal red_wool ["color"="red"]"#).unwrap(), (Mask::Only(wool), CloneMode::Normal));
        assert!(parsed("0 0 0 1 1 1 5 5 5 filtered normal").is_err() && parsed("0 0 0 1 1 1 5 5 5 bogus").is_err());
    }
}
