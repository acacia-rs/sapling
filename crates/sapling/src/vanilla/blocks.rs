//! `/setblock` and `/fill`.

use sapling_api::command::{BlockCoords, States};
use sapling_api::{Block, BlockPos, Command, Output, Sender, ShardCtx};

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

#[derive(Command)]
#[command(crate = "crate", name = "setblock", description = "Changes a block to another block", permission = "operator")]
pub(crate) struct SetBlock {
    position: BlockCoords,
    block: String,
    states: Option<States>,
}

impl Command for SetBlock {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let pos = self.position.from(origin(ctx, sender));
        match changed(ctx, pos, &block(&self.block, self.states.as_ref())) {
            true => out.print("Block placed"),
            false => out.error("The block couldn't be placed"),
        }
    }
}

#[derive(Command)]
#[command(crate = "crate", name = "fill", description = "Fills all or parts of a region with a block", permission = "operator")]
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
        let (low, high): (BlockPos, BlockPos) = (std::array::from_fn(|axis| from[axis].min(to[axis])), std::array::from_fn(|axis| from[axis].max(to[axis])));
        let count: i64 = (0..3).map(|axis| i64::from(high[axis] - low[axis]) + 1).product();
        if count > MAX_FILL {
            return out.error(format!("Too many blocks in the specified area ({count} > {MAX_FILL})"));
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
            0 => out.error("No blocks filled"),
            _ => out.print(format!("{filled} blocks filled")),
        }
    }
}
