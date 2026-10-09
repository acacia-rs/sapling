//! `/setblock` and `/fill`.

use sapling_api::command::{BlockCoords, States};
use sapling_api::{Block, BlockPos, Command, CommandEnum, Output, Sender, ShardCtx};

use super::origin;

/// The most blocks one `/fill` changes, as vanilla.
const MAX_FILL: i64 = 32_768;

/// Writes the block as `/setblock` does; whether the cell changed (vanilla counts no other).
fn changed(ctx: &mut ShardCtx<'_>, pos: BlockPos, block: &Block) -> bool {
    let before = ctx.block(pos);
    ctx.update_block(pos, block) && ctx.block(pos) != before
}

fn block(name: &str, states: Option<&States>) -> Block {
    let block = Block::new(name);
    match states {
        Some(states) => block.with(states.0.clone()),
        None => block,
    }
}

/// What `/setblock` does to the block that is there.
#[derive(CommandEnum, Clone, Copy, PartialEq)]
#[command(crate = "crate")]
pub(crate) enum SetMode {
    Replace,
    /// Breaks it first, leaving what it drops.
    Destroy,
    /// Only sets air.
    Keep,
}

#[derive(Command)]
#[command(
    crate = "crate",
    name = "setblock",
    description = "Changes a block to another block",
    permission = "operator"
)]
pub(crate) struct SetBlock {
    position: BlockCoords,
    block: String,
    states: Option<States>,
    mode: Option<SetMode>,
}

impl Command for SetBlock {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let pos = self.position.from(origin(ctx, sender));
        let mode = self.mode.unwrap_or(SetMode::Replace);
        let kept = mode == SetMode::Keep && ctx.block(pos).is_some_and(|there| !there.is_air());
        let broke = mode == SetMode::Destroy && ctx.break_block(pos);
        match !kept && (changed(ctx, pos, &block(&self.block, self.states.as_ref())) || broke) {
            true => out.translate("commands.setblock.success", Vec::new(), true),
            false => out.translate("commands.setblock.noChange", Vec::new(), false),
        }
    }
}

#[derive(Command)]
#[command(
    crate = "crate",
    name = "fill",
    description = "Fills all or parts of a region with a block",
    permission = "operator"
)]
pub(crate) struct Fill {
    from: BlockCoords,
    to: BlockCoords,
    block: String,
    states: Option<States>,
}

impl Command for Fill {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let here = origin(ctx, sender);
        let (from, to) = (self.from.from(here), self.to.from(here));
        let (low, high): (BlockPos, BlockPos) = (
            std::array::from_fn(|axis| from[axis].min(to[axis])),
            std::array::from_fn(|axis| from[axis].max(to[axis])),
        );
        let count: i64 = (0..3)
            .map(|axis| i64::from(high[axis] - low[axis]) + 1)
            .product();
        if count > MAX_FILL {
            return out.translate(
                "commands.fill.tooManyBlocks",
                vec![count.to_string(), MAX_FILL.to_string()],
                false,
            );
        }
        let block = block(&self.block, self.states.as_ref());
        let mut filled = 0;
        for x in low[0]..=high[0] {
            for y in low[1]..=high[1] {
                for z in low[2]..=high[2] {
                    filled += u32::from(changed(ctx, [x, y, z], &block));
                }
            }
        }
        match filled {
            // BDS's line for this has no key in its own strings.
            0 => out.error("No blocks filled"),
            _ => out.translate("commands.fill.success", vec![filled.to_string()], true),
        }
    }
}
