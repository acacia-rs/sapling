//! Vanilla's commands, for operators, built on the plugin API alone: `/setblock`, `/fill`, `/tp`,
//! `/give`, `/clear`, `/gamemode`, `/summon`, `/kill`, `/damage`, `/difficulty`, `/effect`, `/gamerule`, `/replaceitem`. Opt in with
//! [`crate::ServerBuilder::vanilla_commands`]. Positions take `~`; targets take names and every
//! Bedrock selector with its filters ([`sapling_api::command::Selector`]). Blocks are written as `/setblock` writes them: their neighbours react.
// TODO: the other vanilla commands.

mod blocks;
mod chat;
mod clock;
mod containers;
mod effects;
mod execute;
mod players;
mod rules;

use blocks::{Fill, SetBlock, TestForBlock};
use containers::ReplaceItem;
use players::{Clear, Damage, DifficultyCommand, GameModeCommand, Give, Kill, Summon, Teleport};
use rules::GameRule;

use sapling_api::command::{Selected, Selector};
use sapling_api::{EntityInfo, Output, Player, PlayerId, RuleValue, Sender, ShardCtx};

use crate::ServerBuilder;

impl ServerBuilder {
    /// Adds vanilla's commands for operators (this module): `/setblock`, `/fill`, `/tp`, `/give`,
    /// `/clear`, `/gamemode`, `/summon`, `/kill`, `/gamerule`, `/time`, `/xp`, `/replaceitem block`,
    /// `/execute in <dimension> run tp`, `/say` and `/tell` (for anyone). A command
    /// added later under the same name replaces one of these.
    pub fn vanilla_commands(self) -> Self {
        let blocks = self.command::<SetBlock>().command::<Fill>().command::<TestForBlock>().command::<ReplaceItem>();
        blocks.command::<Teleport>().command::<Give>().command::<Clear>().command::<GameModeCommand>().command::<Summon>().command::<Kill>().command::<Damage>().command::<DifficultyCommand>().command::<effects::EffectCommand>().command::<GameRule>().command::<clock::Time>().command::<clock::Xp>().command::<clock::WeatherCommand>().command::<clock::ToggleDownfall>().command::<execute::Execute>().command::<chat::Say>().command::<chat::Tell>()
    }
}

/// Old names of items that BDS's `/give` still takes, and what they name now: each gave the
/// item its new name gives (research/captures/bds-item-names.txt).
// TODO: those that were tried; BDS knows more.
const OLD_ITEM_NAMES: [(&str, &str); 31] = [
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

/// An entity type or item as BDS names it in a command's answer: `minecraft:armor_stand` is
/// `Armor Stand`.
fn display_name(id: &str) -> String {
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
    let (mut players, mut entities) = (Vec::new(), Vec::new());
    for found in ctx.select(sender, selector) {
        match found {
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
