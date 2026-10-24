//! The batch drill of bds_command_probe: fills of growing size where both bots see them, each left
//! a second alone, to read when BDS sends `UpdateSubchunkBlocks` for a change in place of one
//! `UpdateBlock` a block, and how it groups the blocks.

pub const BATCH_SHOWN: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks", "UpdateBlockSynced"];

/// Fills of 1 to 512 blocks, in a 16-wide strip that crosses no chunk, set and then cleared.
pub fn batch() -> Vec<String> {
    let mut lines: Vec<String> = ["kill @e[type=!player]", "fill ~1 ~-1 ~-1 ~17 ~-1 ~17 stone", "fill ~1 ~ ~-1 ~17 ~9 ~17 air", "", ""].map(String::from).into();
    for count in [1u32, 2, 3, 4, 6, 8, 12, 16, 24, 32, 48, 64, 96, 128, 256, 512] {
        let (across, deep) = (count.min(16), count.div_ceil(16));
        let (rows, high) = (deep.min(16), deep.div_ceil(16));
        let area = format!("~2 ~ ~0 ~{} ~{} ~{}", 1 + across, high - 1, rows - 1);
        lines.extend([format!("fill {area} stone"), String::new(), format!("fill {area} air"), String::new()]);
    }
    lines
}
