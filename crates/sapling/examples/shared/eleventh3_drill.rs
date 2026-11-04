//! The eleventh wave's block, worldgen and command drills (research/parity/ELEVENTH-BRIEF.md "Group 2"):
//! random-tick pace by lit ore and copper (`e3pace`), grass spread by light and a mushroom by a lamp
//! (`e3light`), armour stands in a piston's way (`e3pmob`), blocks set by command (`e3sets`), coral out
//! of water (`e3coral`), a frame broken in creative with mouse input (`e3mouseframe`), a command block
//! minecart's line (`e3cart`), a structure block's seed (`e3seed`) and turned states (`e3turn`).

#[path = "eleventh3_scenes.rs"]
mod scenes;

use acacia_bot::Bot;
use acacia_bot::proto::packets::StructureBlockUpdate;
use acacia_bot::proto::types::{StructureBlockSettings, StructureBlockSettingsAnimationMode, StructureBlockSettingsMirror, StructureBlockSettingsRotation, Vec3f};

const SHOWN: &[&str] = &[
    "UpdateBlock", "UpdateBlockSynced", "UpdateSubchunkBlocks", "LevelSoundEvent", "LevelEvent", "LevelEventGeneric", "BlockEvent", "BlockEntityData", "AddEntity", "RemoveEntity",
    "AddItemEntity", "InventorySlot", "Text", "CommandOutput", "SetEntityData",
];
const MOTION: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks", "AddEntity", "RemoveEntity", "MoveEntityDelta", "MoveEntityAbsolute", "SetEntityMotion", "CommandOutput"];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "e3pace" => (scenes::pace(), SHOWN),
        "e3light" => (scenes::light(), SHOWN),
        "e3pmob" => (scenes::piston_mob(), MOTION),
        "e3lamp" => (scenes::lamps(), SHOWN),
        "e3sets" => (scenes::sets(), SHOWN),
        "e3coral" => (scenes::coral(), SHOWN),
        "e3coralmany" => (scenes::coral_many(), SHOWN),
        "e3mouseframe" => (scenes::frame(), SHOWN),
        "e3cart" => (scenes::cart(), SHOWN),
        "e3seed" => (scenes::seed(), SHOWN),
        "e3turn" => (scenes::turn(), SHOWN),
        _ => return None,
    })
}

/// `!e3sb x y z name integrity seed trigger`: the structure block at that cell is set to load `name` a
/// cell east of it, 8x1x8, at that integrity (0 to 100) and seed, and with trigger 1 loads it, as its
/// editor's Load button does. False for any other line.
pub async fn act(actor: &mut Bot, line: &str) -> bool {
    let Some(rest) = line.strip_prefix("!e3sb ") else { return false };
    let words: Vec<&str> = rest.split(' ').collect();
    let number = |index: usize| words.get(index).and_then(|word| word.parse::<i64>().ok()).unwrap_or(0);
    let settings = StructureBlockSettings {
        palette_name: "default".into(),
        ignore_entities: false,
        ignore_blocks: false,
        non_ticking_players_and_ticking_areas: false,
        size: crate::cmdblocks::coords([8, 1, 8]),
        structure_offset: crate::cmdblocks::coords([1, 0, 0]),
        last_editing_player_unique_id: 0,
        rotation: StructureBlockSettingsRotation::from_raw(0),
        mirror: StructureBlockSettingsMirror::from_raw(0),
        animation_mode: StructureBlockSettingsAnimationMode::from_raw(0),
        animation_duration: 0.0,
        integrity: number(4) as f32,
        seed: number(5) as _,
        pivot: Vec3f { x: 0.0, y: 0.0, z: 0.0 },
    };
    let sent = actor.client().send(&StructureBlockUpdate {
        position: crate::cmdblocks::coords([number(0), number(1), number(2)].map(|c| c as i32)),
        structure_name: words.get(3).copied().unwrap_or_default().to_owned(),
        filtered_structure_name: String::new(),
        data_field: String::new(),
        include_players: false,
        show_bounding_box: true,
        structure_block_type: 2,
        settings,
        redstone_save_mode: 0,
        should_trigger: number(6) == 1,
        water_logged: false,
    });
    println!("## sent !e3sb: {sent:?}");
    true
}
