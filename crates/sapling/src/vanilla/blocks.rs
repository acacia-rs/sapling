//! `/setblock`, `/fill` and `/testforblock`.

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

/// Old names of blocks that BDS's commands still take, and what they name now
/// (research/captures/bds-loot2.txt).
// TODO: the eight that were tried; BDS knows many more.
const OLD_NAMES: [(&str, &str); 8] = [
    ("stonebrick", "stone_bricks"),
    ("grass", "grass_block"),
    ("planks", "oak_planks"),
    ("log", "oak_log"),
    ("wool", "white_wool"),
    ("stone_slab", "smooth_stone_slab"),
    ("concrete", "white_concrete"),
    ("leaves", "oak_leaves"),
];

fn block(name: &str, states: Option<&States>) -> Block {
    let plain = name.strip_prefix("minecraft:").unwrap_or(name);
    let block = Block::new(OLD_NAMES.iter().find(|(old, _)| *old == plain).map_or(name, |(_, new)| new));
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
#[command(crate = "crate", name = "setblock", description = "Changes a block to another block", permission = "operator")]
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
            return out.translate("commands.fill.tooManyBlocks", vec![count.to_string(), MAX_FILL.to_string()], false);
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

#[derive(Command)]
#[command(crate = "crate", name = "testforblock", description = "Tests whether a certain block is in a specific location", permission = "operator")]
pub(crate) struct TestForBlock {
    position: BlockCoords,
    block: String,
    states: Option<States>,
}

impl Command for TestForBlock {
    /// BDS names the place in each answer, and both blocks where the kind is another
    /// (research/captures/bds-defaults.txt).
    // TODO: BDS names a block by its `tile.` key, which for some is not its name (stone is
    // `tile.stone.stone.name`); here the key is made of the name.
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let pos = self.position.from(origin(ctx, sender));
        let place = || pos.map(|c| c.to_string()).to_vec();
        let wanted = block(&self.block, self.states.as_ref());
        let there = ctx.block(pos).unwrap_or_else(|| Block::new(Block::AIR));
        let key = |name: &str| format!("%tile.{}.name", name.rsplit(':').next().unwrap_or(name));
        let has = |state: &str| there.properties.split(',').any(|held| held == state);
        if there.name != wanted.name {
            return out.translate("commands.testforblock.failed.tile", [place(), vec![key(&there.name), key(&wanted.name)]].concat(), false);
        }
        match self.states.as_ref().is_none_or(|states| states.0.split(',').filter(|state| !state.is_empty()).all(has)) {
            true => out.translate("commands.testforblock.success", place(), true),
            false => out.translate("commands.testforblock.failed.data", place(), false),
        }
    }
}
