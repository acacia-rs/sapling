//! The structure-block drills of bds_command_probe (`structblocks`, `structanim`): templates saved
//! and loaded by a structure block (`!structblock`, the packet its editor sends), previewed
//! (`!structdata`), and by `/structure`, animated. Offsets count from the actor's feet.

use acacia_bot::Bot;
use acacia_bot::proto::packets::{StructureBlockUpdate, StructureTemplateDataExportRequest, StructureTemplateDataExportRequestRequestType as RequestType};
use acacia_bot::proto::types::{StructureBlockSettings, StructureBlockSettingsAnimationMode, StructureBlockSettingsMirror, StructureBlockSettingsRotation, Vec3f};

use crate::cmdblocks::{COMMAND_BLOCK_SHOWN, coords, feet, floor, lines};

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    let lines = match name {
        "structblocks" => structure_blocks(),
        "structanim" => animations(),
        _ => return None,
    };
    Some((lines, COMMAND_BLOCK_SHOWN))
}

/// Structure blocks: put by `/setblock` in each mode, a save block storing a 3x3x3 build by its
/// packet (memory, then disk), `/structure` loading that save, a load block placing it turned and
/// animated, the client's preview requests, a corner and a data block, a save block by redstone.
fn structure_blocks() -> Vec<String> {
    let mut out = floor();
    out.extend(lines([
        "fill ~4 ~ ~-1 ~6 ~ ~1 stone",
        "setblock ~5 ~1 ~ chest [\"minecraft:cardinal_direction\"=\"east\"]",
        "setblock ~4 ~1 ~-1 oak_stairs [\"weirdo_direction\"=0]",
        "setblock ~6 ~2 ~1 glass",
        "setblock ~3 ~ ~ structure_block [\"structure_block_type\"=\"save\"]",
        "",
        "!structblock 3 0 0 1 sb1 1 0 -1 3 3 3 0 0 0 0 0 1",
        "",
        "structure load sb1 ~4 ~ ~4",
        "",
        "!structdata 3 0 0 sb1 2",
        "",
        "!structdata 3 0 0 sb1 0",
        "",
        "!structblock 3 0 0 1 sb2 1 0 -1 3 3 3 1 0 0 0 0 1",
        "",
        "setblock ~3 ~ ~5 structure_block [\"structure_block_type\"=\"load\"]",
        "",
        "!structblock 3 0 5 2 sb1 1 0 -1 3 3 3 0 1 0 0 0 0",
        "",
        "!structdata 3 0 5 sb1 1",
        "",
        "!structblock 3 0 5 2 sb1 1 0 -1 3 3 3 0 1 0 0 0 1",
        "",
        "",
        "!structblock 3 0 5 2 sb1 1 0 2 3 3 3 0 0 2 1 2 1",
        "",
        "",
        "",
        "setblock ~3 ~ ~-4 structure_block [\"structure_block_type\"=\"corner\"]",
        "!structblock 3 0 -4 3 sb3 0 0 0 0 0 0 0 0 0 0 0 0",
        "",
        "setblock ~2 ~ ~-4 structure_block [\"structure_block_type\"=\"data\"]",
        "",
        "structure delete sb1",
        "structure delete sb2",
        "",
        // Saved by power.
        "!structblock 3 0 0 1 sb4 1 0 -1 3 3 3 0 0 0 0 0 0",
        "setblock ~2 ~ ~ redstone_block",
        "",
        "!structdata 3 0 0 sb4 2",
        "",
        "setblock ~2 ~ ~ air",
        "!structblock 3 0 0 1 mystructure:sb5 1 0 -1 3 3 3 0 0 0 0 0 1",
        "",
        "structure load sb5 ~4 ~ ~-6",
        "",
        "!structdata 3 0 0 nosuch 2",
        "",
        "structure delete sb5",
        "",
    ]));
    out
}

/// `/structure load` animated: a 3x3x3 build loaded layer by layer and block by block, over 2
/// seconds and over 0, watched for 4 seconds each.
fn animations() -> Vec<String> {
    let mut out = floor();
    out.extend(lines([
        "fill ~4 ~ ~-1 ~6 ~2 ~1 stone",
        "setblock ~5 ~1 ~ chest",
        "structure save anim ~4 ~ ~-1 ~6 ~2 ~1",
        "fill ~4 ~ ~-1 ~6 ~2 ~1 air",
        "",
        "structure load anim ~4 ~ ~-1 0_degrees none layer_by_layer 2",
        "",
        "",
        "",
        "fill ~4 ~ ~-1 ~6 ~2 ~1 air",
        "",
        "structure load anim ~4 ~ ~-1 0_degrees none block_by_block 2",
        "",
        "",
        "",
        "fill ~4 ~ ~-1 ~6 ~2 ~1 air",
        "",
        "structure load anim ~4 ~ ~-1 0_degrees none block_by_block 0",
        "",
        "fill ~4 ~ ~-1 ~6 ~2 ~1 air",
        "",
        "structure load anim ~4 ~ ~-1 90_degrees none layer_by_layer 0.5",
        "",
        "",
        "fill ~4 ~ ~-1 ~6 ~2 ~1 air",
        "structure delete anim",
        "",
    ]));
    out
}

/// `!structblock dx dy dz type name ox oy oz sx sy sz disk rotation mirror animation seconds
/// trigger` (type 0 data, 1 save, 2 load, 3 corner), `!structdata dx dy dz name request` (0 export
/// from save, 1 export from load, 2 query). False for any other line.
pub async fn act(actor: &mut Bot, line: &str) -> bool {
    let (word, rest) = line.split_once(' ').unwrap_or((line, ""));
    if !["!structblock", "!structdata"].contains(&word) {
        return false;
    }
    let words: Vec<&str> = rest.split(' ').collect();
    let number = |index: usize| words.get(index).and_then(|word| word.parse::<i32>().ok()).unwrap_or(0);
    let Some([fx, fy, fz]) = feet(actor) else { return true };
    let at = coords([fx + number(0), fy + number(1), fz + number(2)]);
    let name = words.get(3).copied().unwrap_or_default().to_owned();
    let sent = if word == "!structblock" {
        let settings = settings([number(5), number(6), number(7)], [number(8), number(9), number(10)], number(12), number(13), number(14), number(15) as f32);
        actor.client().send(&StructureBlockUpdate {
            position: at,
            structure_name: words.get(4).copied().unwrap_or_default().to_owned(),
            filtered_structure_name: String::new(),
            data_field: String::new(),
            include_players: false,
            show_bounding_box: true,
            structure_block_type: number(3),
            settings,
            redstone_save_mode: number(11) as u8,
            should_trigger: number(16) == 1,
            water_logged: false,
        })
    } else {
        let request_type = match number(4) {
            1 => RequestType::ExportFromLoad,
            2 => RequestType::QuerySavedStructure,
            _ => RequestType::ExportFromSave,
        };
        actor.client().send(&StructureTemplateDataExportRequest { name, position: at, settings: settings([1, 0, -1], [3, 3, 3], 0, 0, 0, 0.0), request_type })
    };
    println!("## sent {word}: {sent:?}");
    true
}

fn settings(offset: [i32; 3], size: [i32; 3], rotation: i32, mirror: i32, animation: i32, seconds: f32) -> StructureBlockSettings {
    StructureBlockSettings {
        palette_name: "default".into(),
        ignore_entities: false,
        ignore_blocks: false,
        non_ticking_players_and_ticking_areas: false,
        size: coords(size),
        structure_offset: coords(offset),
        last_editing_player_unique_id: 0,
        rotation: StructureBlockSettingsRotation::from_raw(rotation.into()),
        mirror: StructureBlockSettingsMirror::from_raw(mirror.into()),
        animation_mode: StructureBlockSettingsAnimationMode::from_raw(animation.into()),
        animation_duration: seconds,
        integrity: 100.0,
        seed: 0,
        pivot: Vec3f { x: 1.0, y: 1.0, z: 1.0 },
    }
}
