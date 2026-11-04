//! The tenth wave's block drills (research/parity/TENTH-BRIEF.md "Group B"), on redstone_parts_drill.rs's
//! stage (stone at y -61, the actor at 10 -60 10): copper's pace (`t3copper`), coral, nether vines,
//! chorus and amethyst odds (`t3odds`), a sponge's reach (`t3sponge`), lecterns, lodestones, pots,
//! frames, dripleaf and anchors (`t3uses`), anchors and beds in the Nether (`t3nether`), chests
//! unpaired and cloned (`t3chests`), bells (`t3bells`), brushing (`t3brush`), bone meal (`t3meal`),
//! command blocks and their minecarts (`t3cmd`), structure turns, entities and liquids (`t3turn`),
//! gateways (`t3gateway`), trial spawners and vaults (`t3trial`).

#[path = "tenth3_scenes.rs"]
mod scenes;

use acacia_bot::Bot;
use acacia_bot::proto::packets::{CommandBlockUpdate, CommandBlockUpdateContent, CommandBlockUpdateContentFalse, PositionTrackingDbRequest, PositionTrackingDbRequestAction};

const SHOWN: &[&str] = &[
    "UpdateBlock", "UpdateBlockSynced", "UpdateSubchunkBlocks", "LevelSoundEvent", "LevelEvent", "LevelEventGeneric", "BlockEvent", "BlockEntityData", "AddEntity", "RemoveEntity",
    "AddItemEntity", "InventorySlot", "InventoryContent", "Text", "CommandOutput", "PositionTrackingDbBroadcast", "SetSpawnPosition", "StructureTemplateDataExportResponse", "MovePlayer",
    "ChangeDimension", "SetEntityMotion",
];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some((
        match name {
            "t3copper" => scenes::copper(),
            "t3odds" => scenes::odds(),
            "t3sponge" => scenes::sponge(),
            "t3uses" => scenes::uses(),
            "t3nether" => scenes::nether(),
            "t3chests" => scenes::chests(),
            "t3bells" => scenes::bells(),
            "t3brush" => scenes::brush(),
            "t3meal" => scenes::meal(),
            "t3water" => scenes::water_meal(),
            "t3frame" => scenes::frame(),
            "t3cmd" => scenes::command_blocks(),
            "t3turn" => scenes::turns(),
            "t3gateway" => scenes::gateway(),
            "t3trial" => scenes::trial(),
            "t3integ" => scenes::integrity(),
            _ => return None,
        },
        SHOWN,
    ))
}

/// The `trackingHandle` of the held item, read off its NBT as printed.
fn held_handle(actor: &Bot) -> Option<i32> {
    let nbt = format!("{:?}", actor.state().inventory.held().nbt);
    let after = &nbt[nbt.find("trackingHandle")? + "trackingHandle".len()..];
    let digits: String = after.chars().skip_while(|c| !c.is_ascii_digit() && *c != '-').take_while(|c| c.is_ascii_digit() || *c == '-').collect();
    digits.parse().ok()
}

/// `!punch dx dy dz`: starts breaking the block that far from the feet and lets go two ticks on;
/// `!track id|held`: asks for a lodestone target; `!cartcmd command...`: sets the newest command
/// block minecart's command. False for any other line.
pub async fn act(actor: &mut Bot, line: &str) -> bool {
    let (word, rest) = line.split_once(' ').unwrap_or((line, ""));
    match word {
        "!punch" => {
            let numbers: Vec<i32> = rest.split(' ').filter_map(|word| word.parse().ok()).collect();
            let (Some(feet), &[x, y, z]) = (super::cmdblocks::feet(actor), &numbers[..]) else { return true };
            let pos = [feet[0] + x, feet[1] + y, feet[2] + z];
            println!("## punched: {:?}", actor.start_mining(pos, acacia_bot::interact::Face::Up));
            drop(actor.wait_ticks(3).await);
            actor.stop_mining();
        }
        "!track" => {
            let id = if rest == "held" { held_handle(actor) } else { rest.parse().ok() };
            println!("## track {id:?}");
            if let Some(tracking_id) = id {
                println!("## sent !track: {:?}", actor.client().send(&PositionTrackingDbRequest { action: PositionTrackingDbRequestAction::Query, tracking_id }));
            }
        }
        "!cartcmd" => {
            let cart = actor.state().entities.iter().filter(|entity| entity.kind == "minecraft:command_block_minecart").map(|entity| entity.runtime_id).max();
            let Some(cart) = cart else {
                println!("## no command block minecart");
                return true;
            };
            let sent = actor.client().send(&CommandBlockUpdate {
                is_block: false,
                content: CommandBlockUpdateContent::False(CommandBlockUpdateContentFalse { minecart_entity_runtime_id: cart }),
                command: rest.to_owned(),
                last_output: String::new(),
                name: String::new(),
                filtered_name: String::new(),
                should_track_output: true,
                tick_delay: 0,
                execute_on_first_tick: false,
            });
            println!("## sent !cartcmd: {sent:?}");
        }
        _ => return false,
    }
    true
}
