//! The commands aimed at players and entities: `/give`, `/clear`, `/gamemode`, `/summon`,
//! `/kill` and `/difficulty` (`/tp`: teleport.rs).

use sapling_api::command::{Coords, Selector};
use sapling_api::{Command, Difficulty, GameMode, Item, Output, Sender, ShardCtx};

use super::{display_name, item_display_name, no_target, origin, players, targets, tell_target};

fn namespaced(name: &str) -> String {
    if name.contains(':') { name.to_owned() } else { format!("minecraft:{name}") }
}

/// The players a missing target means: the sender.
fn or_sender(ctx: &ShardCtx<'_>, sender: &Sender, target: Option<&Selector>) -> Vec<sapling_api::Player> {
    players(ctx, sender, target.unwrap_or(&Selector::sender()))
}


#[derive(Command)]
#[command(crate = "crate", name = "give", description = "Gives an item to a player", permission = "operator")]
pub(crate) struct Give {
    player: Selector,
    item: super::names::ItemName,
    amount: Option<u8>,
}

impl Command for Give {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let item = Item::new(super::item_named(&self.item.0), self.amount.unwrap_or(1).max(1));
        let (shown, count) = (item_display_name(&item.name, 0), item.count.to_string());
        let given = players(ctx, sender, &self.player);
        for player in &given {
            ctx.give(player.id, &item);
            tell_target(ctx, player.id, "commands.give.successRecipient", &[shown.clone(), count.clone()]);
        }
        match given.is_empty() {
            true => no_target(out),
            false => {
                out.translate("commands.give.success", vec![shown, count, names(&given)], true);
                out.done_to(given.len() as u32);
            }
        }
    }
}

fn names(players: &[sapling_api::Player]) -> String {
    players.iter().map(|player| player.name.as_str()).collect::<Vec<_>>().join(", ")
}

#[derive(Command)]
#[command(crate = "crate", name = "clear", description = "Clears items from a player's inventory", permission = "operator")]
pub(crate) struct Clear {
    player: Option<Selector>,
}

impl Command for Clear {
    /// One line per player, as BDS answers.
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let cleared = or_sender(ctx, sender, self.player.as_ref());
        if cleared.is_empty() {
            return no_target(out);
        }
        for player in &cleared {
            match ctx.clear_inventory(player.id) {
                0 => out.translate("commands.clear.failure.no.items", vec![player.name.clone()], false),
                removed => out.translate("commands.clear.success", vec![player.name.clone(), removed.to_string()], true),
            }
        }
    }
}

#[derive(Command)]
#[command(crate = "crate", name = "gamemode", description = "Sets a player's game mode", permission = "operator")]
pub(crate) struct GameModeCommand {
    mode: GameMode,
    player: Option<Selector>,
}

impl Command for GameModeCommand {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let changed = or_sender(ctx, sender, self.player.as_ref());
        // A parameter starting with `%` is a translation key too.
        let mode = format!("%createWorldScreen.gameMode.{}", format!("{:?}", self.mode).to_lowercase());
        if changed.is_empty() {
            return no_target(out);
        }
        for player in &changed {
            ctx.set_game_mode(player.id, self.mode);
            tell_target(ctx, player.id, "gameMode.changed", std::slice::from_ref(&mode));
            match sender.player().is_some_and(|me| me.id == player.id) {
                true => out.translate("commands.gamemode.success.self", vec![mode.clone()], true),
                false => out.translate("commands.gamemode.success.other", vec![mode.clone(), player.name.clone()], true),
            }
        }
    }
}

// TODO: the game also takes `p`, `e`, `n`, `h` and 0 to 3; not tried on BDS.
#[derive(Command)]
#[command(crate = "crate", name = "difficulty", description = "Sets the difficulty level", permission = "operator")]
pub(crate) struct DifficultyCommand {
    difficulty: Difficulty,
}

impl Command for DifficultyCommand {
    fn run(self, ctx: &mut ShardCtx<'_>, _: &Sender, out: &mut Output) {
        ctx.set_difficulty(self.difficulty);
        out.translate("commands.difficulty.success", vec![format!("{:?}", self.difficulty).to_uppercase()], true);
    }
}

#[derive(Command)]
#[command(crate = "crate", name = "summon", description = "Summons an entity", permission = "operator")]
pub(crate) struct Summon {
    kind: String,
    position: Option<Coords>,
    yaw: Option<f32>,
    pitch: Option<f32>,
    /// A spawn event of the kind's: `minecraft:panda_lazy`.
    event: Option<String>,
}

impl Command for Summon {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let here = origin(ctx, sender);
        let at = self.position.map_or(here, |position| position.from(here));
        // BDS summons its newer zombie villager under the older one's name, and knows no other.
        let kind = Some(namespaced(&self.kind)).filter(|kind| kind != "minecraft:zombie_villager").unwrap_or_else(|| "minecraft:zombie_villager_v2".into());
        // TODO: the pitch is taken and not kept.
        let _ = self.pitch;
        let summoned = ctx.summon_as(sender.player().map(|by| by.id), &kind, at, self.yaw.unwrap_or(0.0), self.event.as_deref()).is_some();
        out.translate(if summoned { "commands.summon.success" } else { "commands.summon.failed" }, Vec::new(), summoned);
    }
}

#[derive(Command)]
#[command(crate = "crate", name = "kill", description = "Kills entities", permission = "operator")]
pub(crate) struct Kill {
    target: Option<Selector>,
}

impl Command for Kill {
    /// The players named die (`kill_player`: not those in creative), the sender where none is
    /// named, and so does every other entity named (`kill_entity`).
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let (people, named) = targets(ctx, sender, self.target.as_ref().unwrap_or(&Selector::sender()));
        let mut killed: Vec<String> = people.into_iter().filter(|player| ctx.kill_player(player.id)).map(|player| player.name).collect();
        killed.extend(named.iter().filter(|entity| ctx.kill_entity(entity.id)).map(|entity| display_name(&entity)));
        match killed.is_empty() {
            true => no_target(out),
            false => out.translate("commands.kill.successful", vec![killed.join(", ")], true),
        }
    }
}
