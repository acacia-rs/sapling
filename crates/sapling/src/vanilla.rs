//! Vanilla's commands, for operators, built on the plugin API alone: `/setblock`, `/fill`, `/tp`,
//! `/give`, `/clear`, `/gamemode`, `/summon`, `/kill`, `/gamerule`. Opt in with
//! [`crate::ServerBuilder::vanilla_commands`]. Positions take `~`; targets take `@s`, `@p`, `@a`,
//! `@e[type=...]` and names. Blocks are written as `/setblock` writes them: their neighbours react.
// TODO: the other vanilla commands; `@r` and selector arguments besides `type`.

mod blocks;
mod players;
mod rules;

pub(crate) use blocks::{Fill, SetBlock};
pub(crate) use players::{Clear, GameModeCommand, Give, Kill, Summon, Teleport};
pub(crate) use rules::GameRule;

use sapling_api::command::Selector;
use sapling_api::{EntityInfo, Output, Player, Sender, ShardCtx};

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
        Selector::Entities { kind } => ctx.players().filter(|_| kind.as_deref().is_none_or(|kind| kind == "minecraft:player")).collect(),
    }
}

/// The entities other than players that `selector` names, in the world calls act on.
fn entities(ctx: &ShardCtx<'_>, selector: &Selector) -> Vec<EntityInfo> {
    match selector {
        Selector::Entities { kind } => ctx.entities().into_iter().filter(|entity| kind.as_ref().is_none_or(|kind| *kind == entity.kind)).collect(),
        _ => Vec::new(),
    }
}
