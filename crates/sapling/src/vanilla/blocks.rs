//! `/setblock` and `/testforblock`, and the block names and states the block commands share.

use sapling_api::command::{BlockCoords, States};
use sapling_api::{Block, BlockPos, Command, CommandEnum, Output, Sender, ShardCtx};

use super::names::BlockName;

/// Writes the block as `/setblock` does; whether the cell changed (vanilla counts no other).
pub(super) fn changed(ctx: &mut ShardCtx<'_>, pos: BlockPos, block: &Block) -> bool {
    let before = ctx.block(pos);
    ctx.update_block(pos, block) && ctx.block(pos) != before
}

/// Whether `there` has each of `states` (`a=1,b=2`, the form [`States`] reads).
pub(super) fn has_states(there: &Block, states: &str) -> bool {
    states.split(',').filter(|state| !state.is_empty()).all(|state| there.properties.split(',').any(|held| held == state))
}

/// Old names of blocks that BDS's commands still take, and what they name now: each was set by
/// its old name on BDS and found under its new one (research/captures/bds-loot2.txt,
/// bds-names.txt).
// TODO: those that were tried; `double_plant`, `coral_fan_dead`, `skull` and `wood` were taken
// too, but what they set was gone or wet before it was asked for.
const OLD_NAMES: [(&str, &str); 34] = [
    ("stonebrick", "stone_bricks"),
    ("grass", "grass_block"),
    ("planks", "oak_planks"),
    ("log", "oak_log"),
    ("wool", "white_wool"),
    ("stone_slab", "smooth_stone_slab"),
    ("concrete", "white_concrete"),
    ("leaves", "oak_leaves"),
    ("sapling", "oak_sapling"),
    ("fence", "oak_fence"),
    ("carpet", "white_carpet"),
    ("stained_glass", "white_stained_glass"),
    ("stained_glass_pane", "white_stained_glass_pane"),
    ("stained_hardened_clay", "white_terracotta"),
    ("concrete_powder", "white_concrete_powder"),
    ("concretepowder", "white_concrete_powder"),
    ("log2", "acacia_log"),
    ("leaves2", "acacia_leaves"),
    ("wooden_slab", "oak_slab"),
    ("stone_slab2", "red_sandstone_slab"),
    ("stone_slab3", "end_stone_brick_slab"),
    ("stone_slab4", "mossy_stone_brick_slab"),
    ("double_stone_slab", "smooth_stone_double_slab"),
    ("double_wooden_slab", "oak_double_slab"),
    ("red_flower", "poppy"),
    ("yellow_flower", "dandelion"),
    ("tallgrass", "short_grass"),
    ("coral", "tube_coral"),
    ("coral_block", "tube_coral_block"),
    ("coral_fan", "tube_coral_fan"),
    ("monster_egg", "infested_stone"),
    ("shulker_box", "white_shulker_box"),
    ("sealantern", "sea_lantern"),
    ("light_block", "light_block_0"),
];

pub(super) fn block(name: &str, states: Option<&States>) -> Block {
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
    block: BlockName,
    states: Option<States>,
    mode: Option<SetMode>,
}

impl Command for SetBlock {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let pos = self.position.resolve(ctx, sender);
        let mode = self.mode.unwrap_or(SetMode::Replace);
        let kept = mode == SetMode::Keep && ctx.block(pos).is_some_and(|there| !there.is_air());
        let broke = mode == SetMode::Destroy && ctx.break_block(pos);
        match !kept && (changed(ctx, pos, &block(&self.block.0, self.states.as_ref())) || broke) {
            true => out.translate("commands.setblock.success", Vec::new(), true),
            false => out.translate("commands.setblock.noChange", Vec::new(), false),
        }
    }
}

#[derive(Command)]
#[command(crate = "crate", name = "testforblock", description = "Tests whether a certain block is in a specific location", permission = "operator")]
pub(crate) struct TestForBlock {
    position: BlockCoords,
    block: BlockName,
    states: Option<States>,
}

impl Command for TestForBlock {
    /// BDS names the place in each answer, and both blocks where the kind is another
    /// (research/captures/bds-defaults.txt).
    // TODO: BDS names a block by its `tile.` key, which for some is not its name (stone is
    // `tile.stone.stone.name`); here the key is made of the name.
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let pos = self.position.resolve(ctx, sender);
        let place = || pos.map(|c| c.to_string()).to_vec();
        let wanted = block(&self.block.0, self.states.as_ref());
        let there = ctx.block(pos).unwrap_or_else(|| Block::new(Block::AIR));
        let key = |name: &str| format!("%tile.{}.name", name.rsplit(':').next().unwrap_or(name));
        if there.name != wanted.name {
            return out.translate("commands.testforblock.failed.tile", [place(), vec![key(&there.name), key(&wanted.name)]].concat(), false);
        }
        match self.states.as_ref().is_none_or(|states| has_states(&there, &states.0)) {
            true => out.translate("commands.testforblock.success", place(), true),
            false => out.translate("commands.testforblock.failed.data", place(), false),
        }
    }
}
