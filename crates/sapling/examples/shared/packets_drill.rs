//! The client-packets drill of bds_command_probe: what BDS answers a client's emote, emote list,
//! skin change, pick block and pick entity, game type, anvil damage and server settings request.

use acacia_bot::Bot;
use acacia_bot::proto::manual::Uuid;
use acacia_bot::proto::packets::{AnvilDamage, BlockPickRequest, Emote, EmoteFlags, EmoteList, EntityPickRequest, PlayerSkin, RiderJump, ServerSettingsRequest, SetPlayerGameType};
use acacia_bot::proto::types::{BlockCoordinates, GameMode, Skin, SkinArmSize, SkinImage};

pub const PACKETS_SHOWN: &[&str] = &[
    "Emote", "EmoteList", "PlayerSkin", "PlayerList", "InventorySlot", "InventoryContent", "PlayerHotbar", "MobEquipment", "SetPlayerGameType", "UpdatePlayerGameType", "UpdateAbilities",
    "UpdateAdventureSettings", "UpdateBlock", "LevelSoundEvent", "LevelEvent", "ServerSettingsResponse", "ModalFormRequest", "ItemStackResponse", "SetEntityData", "Animate", "SetEntityMotion",
];

/// An emote's id as a vanilla client sends one (the wave).
const EMOTE: &str = "4c8ae710-df2e-47cd-814d-cc7bf21a3d67";

fn owned<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

/// The actor in creative: emotes, sends its emote list and a new skin, picks a chest (with and
/// without its data), a log and a cow, sets its own game type, damages an anvil, asks for the
/// server's settings, and jumps on a horse it rides.
pub fn packets() -> Vec<String> {
    let mut lines = owned(["gamemode creative @s", "clear @s", "fill ~-2 ~-1 ~-2 ~6 ~-1 ~6 stone", "fill ~-2 ~ ~-2 ~6 ~3 ~6 air", "", "!emote", "", "!emotelist", ""]);
    lines.extend(owned(["setblock ~2 ~ ~ chest", "replaceitem block ~2 ~ ~ slot.container 0 diamond 3", "", "!pick 2 0 0 0", "", "!pick 2 0 0 1", "", "setblock ~2 ~ ~ oak_log", "", "!pick 2 0 0 0", ""]));
    lines.extend(owned(["replaceitem entity @s slot.hotbar 0 oak_log", "", "!pick 2 0 0 0", "", "summon cow ~3 ~ ~", "", "!pickentity cow", "", "kill @e[type=cow]", ""]));
    lines.extend(owned(["!gametype 0", "", "!gametype 1", "", "gamemode survival @s", "", "!gametype 1", "", "gamemode creative @s", ""]));
    lines.extend(owned(["setblock ~2 ~ ~ anvil", "", "!anvil 2 0 0", "", "!anvil 2 0 0", "", "!settings", "", ""]));
    lines.extend(owned(["summon horse ~3 ~ ~", "", "ride @s start_riding @e[type=horse,c=1] teleport_rider", "", "!riderjump 90", "", "", "ride @s stop_riding", "kill @e[type=horse]", "clear @s", ""]));
    // Last: BDS dropped the actor for a skin sent under a zero uuid.
    lines.extend(owned(["!skin", "", ""]));
    lines
}

/// The grey 64 px skin with the humanoid geometry, as sapling replaces a malformed one.
fn plain_skin() -> Skin {
    let empty = || SkinImage { width: 0, height: 0, data: Default::default() };
    Skin {
        skin_id: "drill-plain".into(),
        play_fab_id: String::new(),
        skin_resource_pack: r#"{"geometry":{"default":"geometry.humanoid.custom"}}"#.into(),
        skin_data: SkinImage { width: 64, height: 64, data: [0x9a, 0x9a, 0x9a, 0xff].repeat(64 * 64).into() },
        animations: Vec::new(),
        cape_data: empty(),
        geometry_data: String::new(),
        geometry_data_version: "0.0.0".into(),
        animation_data: String::new(),
        cape_id: String::new(),
        full_skin_id: "drill-plain".into(),
        arm_size: SkinArmSize::Wide,
        skin_color: 0,
        personal_pieces: Vec::new(),
        piece_tint_colors: Vec::new(),
        premium: false,
        persona: false,
        cape_on_classic: false,
        primary_user: true,
        overriding_player_appearance: false,
        trusted: "false".into(),
        profile_hash: String::new(),
    }
}

/// Does a packet drill's `!` line: `!emote`, `!emotelist`, `!skin`, `!pick x y z data` (a block
/// that far from the feet), `!pickentity kind` (the newest of that kind), `!gametype n`,
/// `!anvil x y z`, `!settings`, `!riderjump strength`. False for any other line.
pub async fn act(actor: &mut Bot, line: &str) -> bool {
    let (word, rest) = line.split_once(' ').unwrap_or((line, ""));
    let numbers: Vec<i32> = rest.split(' ').filter_map(|word| word.parse().ok()).collect();
    let feet = actor.movement().and_then(|movement| movement.position()).map(|at| at.map(|c| c.floor() as i32));
    let me = actor.client().runtime_entity_id();
    let sent = match word {
        "!emote" => actor.client().send(&Emote { entity_id: me, emote_id: EMOTE.into(), emote_length_ticks: 60, xuid: String::new(), platform_id: String::new(), flags: EmoteFlags::Unknown(0) }),
        "!emotelist" => actor.client().send(&EmoteList { player_id: me, emote_pieces: vec![EMOTE.parse::<Uuid>().expect("a uuid")] }),
        "!skin" => {
            let uuid = actor.state().player_list.by_name("CmdActor").map(|entry| entry.uuid).unwrap_or_default();
            actor.client().send(&PlayerSkin { uuid, skin: plain_skin(), skin_name: "drill-plain".into(), old_skin_name: String::new() })
        }
        "!pick" => {
            let (Some([fx, fy, fz]), &[x, y, z, data]) = (feet, &numbers[..]) else { return true };
            actor.client().send(&BlockPickRequest { x: fx + x, y: fy + y, z: fz + z, add_user_data: data == 1, selected_slot: 0 })
        }
        "!pickentity" => {
            let kind = format!("minecraft:{rest}");
            let Some(id) = actor.state().entities.iter().filter(|entity| entity.kind == kind).map(|entity| entity.runtime_id).max() else { return true };
            actor.client().send(&EntityPickRequest { runtime_entity_id: id, selected_slot: 0, with_data: false })
        }
        "!gametype" => actor.client().send(&SetPlayerGameType { gamemode: if numbers.first() == Some(&1) { GameMode::Creative } else { GameMode::Survival } }),
        "!anvil" => {
            let (Some([fx, fy, fz]), &[x, y, z]) = (feet, &numbers[..]) else { return true };
            actor.client().send(&AnvilDamage { position: BlockCoordinates { x: fx + x, y: fy + y, z: fz + z } })
        }
        "!settings" => actor.client().send(&ServerSettingsRequest {}),
        "!riderjump" => actor.client().send(&RiderJump { jump_strength: numbers.first().copied().unwrap_or(0) }),
        _ => return false,
    };
    println!("## sent {word}: {sent:?}");
    true
}
