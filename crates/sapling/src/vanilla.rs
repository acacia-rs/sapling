//! Vanilla's commands, for operators, built on the plugin API alone: `/setblock`, `/fill`, `/tp`,
//! `/give`, `/clear`, `/gamemode`, `/summon`, `/kill`, `/damage`, `/difficulty`, `/effect`, `/gamerule`, `/replaceitem`. Opt in with
//! [`crate::ServerBuilder::vanilla_commands`]. Positions take `~`; targets take `@s`, `@p`, `@a`,
//! `@e[type=...]`, `@e[type=!...]` and names. Blocks are written as `/setblock` writes them: their neighbours react.
// TODO: the other vanilla commands; `@r` and selector arguments besides `type`.

mod blocks;
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

use sapling_api::command::Selector;
use sapling_api::{EntityInfo, Output, Player, Sender, ShardCtx};

use crate::ServerBuilder;

impl ServerBuilder {
    /// Adds vanilla's commands for operators (this module): `/setblock`, `/fill`, `/tp`, `/give`,
    /// `/clear`, `/gamemode`, `/summon`, `/kill`, `/gamerule`, `/time`, `/xp`, `/replaceitem block` and
    /// `/execute in <dimension> run tp`. A command
    /// added later under the same name replaces one of these.
    pub fn vanilla_commands(self) -> Self {
        let blocks = self.command::<SetBlock>().command::<Fill>().command::<TestForBlock>().command::<ReplaceItem>();
        blocks.command::<Teleport>().command::<Give>().command::<Clear>().command::<GameModeCommand>().command::<Summon>().command::<Kill>().command::<Damage>().command::<DifficultyCommand>().command::<effects::EffectCommand>().command::<GameRule>().command::<clock::Time>().command::<clock::Xp>().command::<clock::WeatherCommand>().command::<clock::ToggleDownfall>().command::<execute::Execute>()
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

/// Where the sender is, as `~` counts from: its feet, or the spawn for the console.
fn origin(ctx: &ShardCtx<'_>, sender: &Sender) -> [f32; 3] {
    sender.player().and_then(|player| ctx.position(player.id)).map_or_else(|| ctx.spawn(), |position| position.feet)
}

/// The players `selector` names, in the sender's world.
fn players(ctx: &ShardCtx<'_>, sender: &Sender, selector: &Selector) -> Vec<Player> {
    let here = origin(ctx, sender);
    let distance = |player: &Player| ctx.position(player.id).map_or(f32::MAX, |position| (0..3).map(|axis| (position.feet[axis] - here[axis]).powi(2)).sum());
    match selector {
        Selector::Sender => sender.player().cloned().into_iter().collect(),
        Selector::Nearest => ctx.players().min_by(|a, b| distance(a).total_cmp(&distance(b))).into_iter().collect(),
        Selector::Players => ctx.players().collect(),
        Selector::Named(name) => ctx.players().filter(|player| player.name.eq_ignore_ascii_case(name)).collect(),
        Selector::Entities { kind } => ctx.players().filter(|_| Selector::takes(kind.as_deref(), "minecraft:player")).collect(),
    }
}

/// The entities other than players that `selector` names, in the world calls act on.
fn entities(ctx: &ShardCtx<'_>, selector: &Selector) -> Vec<EntityInfo> {
    match selector {
        Selector::Entities { kind } => ctx.entities().into_iter().filter(|entity| Selector::takes(kind.as_deref(), &entity.kind)).collect(),
        _ => Vec::new(),
    }
}
