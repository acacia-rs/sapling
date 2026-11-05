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
/// bds-names.txt, bds-n13names.txt).
// TODO: those that were tried; BDS knows more.
const OLD_NAMES: [(&str, &str); 39] = [
    ("double_plant", "sunflower"),
    ("coral_fan_dead", "dead_tube_coral_fan"),
    ("skull", "skeleton_skull"),
    ("wood", "oak_wood"),
    ("invisiblebedrock", "invisible_bedrock"),
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

/// The block a command names; an old name in any case (BDS set `invisibleBedrock`, bds-n13names.txt).
pub(super) fn block(name: &str, states: Option<&States>) -> Block {
    let lower = name.to_ascii_lowercase();
    let plain = lower.strip_prefix("minecraft:").unwrap_or(&lower);
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
        if !ctx.ticks_chunk([pos[0] >> 4, pos[2] >> 4]) {
            return out.translate("commands.setblock.outOfWorld", Vec::new(), false);
        }
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

/// The `tile.` key BDS names a block by; one its table lists only by an old name by that name's
/// (`%tile.invisibleBedrock.name`, bds-n13names.txt).
fn tile_key(name: &str) -> String {
    let plain = name.rsplit(':').next().unwrap_or(name);
    let old = OLD_NAMES.iter().find(|(_, new)| *new == plain).map(|(old, _)| format!("minecraft:{old}"));
    let key = sapling_data::block_tile_key(name).or_else(|| old.as_deref().and_then(sapling_data::block_tile_key));
    key.map_or_else(|| format!("%tile.{plain}.name"), str::to_owned)
}

impl Command for TestForBlock {
    /// BDS names the place in each answer, and both blocks where the kind is another
    /// (research/captures/bds-defaults.txt), each by its `tile.` key (`sapling_data::block_tile_key`).
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let pos = self.position.resolve(ctx, sender);
        // research/captures/bds-tickedge.txt: past the tick region, as `/setblock` and `/fill` there.
        if !ctx.ticks_chunk([pos[0] >> 4, pos[2] >> 4]) {
            return out.translate("commands.testforblock.outOfWorld", Vec::new(), false);
        }
        let place = || pos.map(|c| c.to_string()).to_vec();
        let wanted = block(&self.block.0, self.states.as_ref());
        let there = ctx.block(pos).unwrap_or_else(|| Block::new(Block::AIR));
        if there.name != wanted.name {
            return out.translate("commands.testforblock.failed.tile", [place(), vec![tile_key(&there.name), tile_key(&wanted.name)]].concat(), false);
        }
        match self.states.as_ref().is_none_or(|states| has_states(&there, &states.0)) {
            true => out.translate("commands.testforblock.success", place(), true),
            false => out.translate("commands.testforblock.failed.data", place(), false),
        }
    }
}
