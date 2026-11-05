//! Vanilla's commands, for operators, built on the plugin API alone: the block commands
//! (`/setblock`, `/fill`, `/clone`, `/testforblock`, `/testforblocks`), `/tp`, `/give`, `/clear`,
//! `/enchant`, `/gamemode`, `/summon`, `/kill`, `/damage`, `/difficulty`, `/effect`, `/gamerule`,
//! `/replaceitem`, the spawn commands (`/setworldspawn`, `/spawnpoint`, `/clearspawnpoint`),
//! `/testfor`, `/spreadplayers`, the server commands (`/defaultgamemode`, `/setmaxplayers`, `/seed`,
//! `/version`, `/transfer`, `/list`, `/stop`, `/save`), `/kick`, `/op`, `/deop`, the messaging commands (`/me`, `/tellraw`,
//! `/title`, `/titleraw`, `/playsound`, `/stopsound`, `/particle`), `/scoreboard`, `/tag`, `/scriptevent`,
//! `/execute`, and with access lists `/allowlist` and the ban commands. Opt in with
//! [`crate::ServerBuilder::vanilla_commands`].
//! Positions take `~` and `^`; targets take names and every Bedrock selector with its filters
//! ([`sapling_api::command::Selector`]). Blocks are written as `/setblock` writes them: their
//! neighbours react.
// TODO: the other vanilla commands (BDS's list against sapling's: docs/research/world-commands.md).

mod access;
mod allowlist;
mod bans;
mod blocks;
mod camera;
mod chat;
mod clock;
mod containers;
mod controls;
mod copying;
mod damage;
mod effects;
mod enchanting;
mod execute;
mod items;
mod lifecycle;
mod loot;
mod message;
mod messaging;
mod names;
mod operators;
mod permission;
mod players;
mod recipes;
mod regions;
mod riding;
mod rules;
mod saving;
mod scoreboard;
mod screen;
mod scriptevent;
mod selecting;
mod session;
mod sounds;
mod spawns;
mod structure;
mod tag;
mod teleport;
mod tickingarea;
mod titles;
mod worldgen;

use blocks::{SetBlock, TestForBlock};
use containers::ReplaceItem;
use damage::Damage;
use players::{DifficultyCommand, GameModeCommand, Kill, Summon};
use regions::Fill;
use teleport::Teleport;
use rules::GameRule;

use sapling_api::command::{Selected, Selector};
use sapling_api::{EntityInfo, Output, Player, PlayerId, RuleValue, Sender, ShardCtx};

use crate::ServerBuilder;

impl ServerBuilder {
    /// Adds vanilla's commands for operators (this module), `/time`, `/xp`, `/weather`,
    /// `/execute`, and `/say`, `/tell` and `/me` for anyone. A command added
    /// later under the same name replaces one of these. The ban and allowlist commands come with
    /// [`ServerBuilder::access`].
    pub fn vanilla_commands(self) -> Self {
        let limit = session::Limit(self.net.max_players.clone());
        let blocks = self.plugin(limit).command::<SetBlock>().command::<Fill>().command::<TestForBlock>().command::<ReplaceItem>();
        let blocks = blocks.command::<copying::CloneCommand>().command::<copying::TestForBlocks>();
        let blocks = blocks.command::<Teleport>().command::<items::Give>().command::<items::Clear>().command::<enchanting::Enchant>().command::<GameModeCommand>().command::<Summon>().command::<Kill>().command::<Damage>().command::<DifficultyCommand>().command::<effects::EffectCommand>().command::<GameRule>().command::<clock::Time>().command::<clock::Xp>().command::<clock::WeatherCommand>().command::<clock::ToggleDownfall>().command::<execute::Execute>().command::<chat::Say>().command::<chat::Tell>();
        let world = blocks.command::<spawns::SetWorldSpawn>().command::<spawns::SpawnPoint>().command::<spawns::ClearSpawnPoint>().command::<selecting::TestFor>().command::<selecting::SpreadPlayers>();
        let server = world.command::<session::DefaultGameMode>().command::<session::SetMaxPlayers>().command::<session::Seed>().command::<session::Version>().command::<session::Transfer>();
        let admin = server.command::<operators::Kick>().command::<operators::Op>().command::<operators::Deop>();
        let messaging = admin.command::<messaging::Me>().command::<messaging::TellRaw>().command::<titles::Title>().command::<titles::TitleRaw>();
        let sounds = messaging.command::<sounds::PlaySound>().command::<sounds::StopSound>().command::<sounds::Particle>();
        let world_tools = sounds.command::<structure::StructureCommand>().command::<tickingarea::TickingAreaCommand>().command::<worldgen::Place>().command::<worldgen::Locate>();
        let world_tools = world_tools.command::<scoreboard::Scoreboard>().command::<tag::Tag>().command::<scriptevent::ScriptEventCommand>().command::<riding::Ride>().command::<loot::Loot>();
        let screen = world_tools.command::<camera::Camera>().command::<camera::CameraShake>().command::<screen::Fog>().command::<screen::Hud>().command::<screen::PlayAnimation>().command::<screen::Music>();
        let controls = screen.command::<controls::InputPermission>().command::<controls::MobEvent>().command::<controls::DayLock>().command::<recipes::Recipe>();
        let stopping = lifecycle::Stopping(controls.stop.clone());
        controls.plugin(stopping).command::<lifecycle::List>().command::<lifecycle::Stop>().command::<saving::Save>()
    }
}

/// Old names of items that BDS's `/give` still takes, and what they name now: each gave the
/// item its new name gives (research/captures/bds-item-names.txt, bds-n13names.txt).
// TODO: those that were tried; BDS knows more.
const OLD_ITEM_NAMES: [(&str, &str); 37] = [
    ("scute", "turtle_scute"),
    ("horsearmorgold", "golden_horse_armor"),
    ("horsearmordiamond", "diamond_horse_armor"),
    ("lodestonecompass", "lodestone_compass"),
    ("record_cat", "music_disc_cat"),
    ("map", "filled_map"),
    ("chain", "iron_chain"),
    ("sign", "oak_sign"),
    ("boat", "oak_boat"),
    ("skull", "skeleton_skull"),
    ("record_13", "music_disc_13"),
    ("fish", "cod"),
    ("cooked_fish", "cooked_cod"),
    ("clownfish", "tropical_fish"),
    ("muttonraw", "mutton"),
    ("muttoncooked", "cooked_mutton"),
    ("appleenchanted", "enchanted_golden_apple"),
    ("reeds", "sugar_cane"),
    ("melon", "melon_slice"),
    ("speckled_melon", "glistering_melon_slice"),
    ("netherstar", "nether_star"),
    ("fireball", "fire_charge"),
    ("fireworks", "firework_rocket"),
    ("fireworkscharge", "firework_star"),
    ("horsearmorleather", "leather_horse_armor"),
    ("horsearmoriron", "iron_horse_armor"),
    ("carrotonastick", "carrot_on_a_stick"),
    ("emptymap", "empty_map"),
    ("totem", "totem_of_undying"),
    ("turtle_shell_piece", "turtle_scute"),
    ("chorus_fruit_popped", "popped_chorus_fruit"),
    ("stone_slab", "smooth_stone_slab"),
    ("planks", "oak_planks"),
    ("log", "oak_log"),
    ("wool", "white_wool"),
    ("stonebrick", "stone_bricks"),
    ("dye", "ink_sac"),
];

/// The namespaced name of the item a command calls `name`, an old name too.
fn item_named(name: &str) -> String {
    let plain = name.strip_prefix("minecraft:").unwrap_or(name);
    match OLD_ITEM_NAMES.iter().find(|(old, _)| *old == plain) {
        Some((_, new)) => format!("minecraft:{new}"),
        None if name.contains(':') => name.to_owned(),
        None => format!("minecraft:{name}"),
    }
}

fn no_target(out: &mut Output) {
    out.translate("commands.generic.noTargetMatch", Vec::new(), false);
}

/// Tells `player` what a command did to it, unless the world's `sendCommandFeedback` is off
/// (research/captures/bds-feedback.txt: the shard holds back the sender's answer too).
fn tell_target(ctx: &mut ShardCtx<'_>, player: PlayerId, key: &str, parameters: &[String]) {
    if !matches!(ctx.game_rule("sendCommandFeedback"), Some((_, RuleValue::Bool(false)))) {
        ctx.send_translation(player, key, parameters);
    }
}

/// An entity as BDS names it in a command's answer (sapling-data's `names.tsv`): `Evoker` for
/// `minecraft:evocation_illager`, a lying item by what it holds.
fn display_name(entity: &EntityInfo) -> String {
    if let Some(item) = &entity.item {
        return item_display_name(&item.name, item.damage);
    }
    let id = entity.kind.as_str();
    // names.tsv lists only `villager`; BDS names a `villager_v2` "Villager" too (bds-raid.txt).
    let listed = |id: &str| sapling_data::entity_display_name(id).or_else(|| id.strip_suffix("_v2").and_then(sapling_data::entity_display_name));
    listed(id).or_else(|| sapling_data::item_display_name(id, 0)).map_or_else(|| title_case(id), str::to_owned)
}

/// An item of `data` as BDS names it in a command's answer: `Raw Chicken`, `Potion of Slowness`.
fn item_display_name(id: &str, data: i16) -> String {
    sapling_data::item_display_name(id, u16::try_from(data).unwrap_or(0)).map_or_else(|| title_case(id), str::to_owned)
}

/// The name of a kind `names.tsv` lacks (or a build without it): the identifier title-cased,
/// `minecraft:armor_stand` as `Armor Stand`.
fn title_case(id: &str) -> String {
    let words = id.rsplit(':').next().unwrap_or(id).split('_');
    let capitalised = words.map(|word| {
        let mut chars = word.chars();
        chars.next().map_or_else(String::new, |first| first.to_uppercase().chain(chars).collect())
    });
    capitalised.collect::<Vec<_>>().join(" ")
}

/// The players and the other entities `selector` names ([`ShardCtx::select`]), each in its order;
/// the sender where there is no selector.
fn targets(ctx: &mut ShardCtx<'_>, sender: &Sender, selector: Option<&Selector>) -> (Vec<Player>, Vec<EntityInfo>) {
    let Some(selector) = selector else { return (sender.player().cloned().into_iter().collect(), Vec::new()) };
    split(ctx.select(sender, selector))
}

fn split(found: Vec<Selected>) -> (Vec<Player>, Vec<EntityInfo>) {
    let (mut players, mut entities) = (Vec::new(), Vec::new());
    for one in found {
        match one {
            Selected::Player(player) => players.push(player),
            Selected::Entity(entity) => entities.push(entity),
        }
    }
    (players, entities)
}

/// The players `selector` names; the sender where there is none.
fn players(ctx: &mut ShardCtx<'_>, sender: &Sender, selector: Option<&Selector>) -> Vec<Player> {
    targets(ctx, sender, selector).0
}

#[cfg(test)]
mod tests {
    use sapling_api::EntityId;

    use super::*;

    #[test]
    fn a_v2_kind_takes_its_listed_kind_s_name() {
        let named = |kind: &str| display_name(&EntityInfo::new(EntityId(1), kind.into(), [0.0; 3], [0.0; 3], true));
        assert_eq!([named("minecraft:villager_v2"), named("minecraft:zombie_villager_v2"), named("minecraft:evocation_illager")], ["Villager", "Zombie Villager", "Evoker"]);
    }

    #[test]
    fn old_names_give_what_bds_gave() {
        // bds-n13names.txt
        assert_eq!(["scute", "horsearmorgold", "record_cat", "map", "frame"].map(item_named), ["minecraft:turtle_scute", "minecraft:golden_horse_armor", "minecraft:music_disc_cat", "minecraft:filled_map", "minecraft:frame"]);
        let set = |name: &str| blocks::block(name, None).name;
        assert_eq!(["double_plant", "coral_fan_dead", "skull", "wood", "invisibleBedrock"].map(set), ["sunflower", "dead_tube_coral_fan", "skeleton_skull", "oak_wood", "invisible_bedrock"].map(|name| sapling_api::Block::new(name).name));
    }
}
