//! The commands aimed at players and entities: `/tp`, `/give`, `/clear`, `/gamemode`, `/summon`
//! `/kill` and `/damage`.

use sapling_api::command::{Args, CommandSpec, CommandSyntax, Coords, Overload, Parameter, ParameterSpec, ParseError, Selector};
use sapling_api::{Command, DamageCause, GameMode, Item, Output, Permission, Sender, ShardCtx};

use super::{display_name, entities, no_target, origin, players};

fn namespaced(name: &str) -> String {
    if name.contains(':') { name.to_owned() } else { format!("minecraft:{name}") }
}

/// The players a missing target means: the sender.
fn or_sender(ctx: &ShardCtx<'_>, sender: &Sender, target: Option<&Selector>) -> Vec<sapling_api::Player> {
    players(ctx, sender, target.unwrap_or(&Selector::Sender))
}

/// `/tp [victim] <destination>`: the first word is a target unless it is a coordinate.
pub(crate) struct Teleport {
    target: Option<Selector>,
    to: Coords,
}

impl CommandSyntax for Teleport {
    fn spec() -> CommandSpec {
        let destination = ParameterSpec::of::<Coords>("destination");
        CommandSpec {
            name: "tp".into(),
            aliases: vec!["teleport".into()],
            description: "Teleports entities".into(),
            permission: Permission::Operator,
            overloads: vec![Overload { parameters: vec![destination.clone()] }, Overload { parameters: vec![ParameterSpec::of::<Selector>("victim"), destination] }],
        }
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        let is_coordinate = |word: &str| word.starts_with('~') || word.parse::<f32>().is_ok();
        let target = match args.peek() {
            Some(word) if !is_coordinate(word) => Some(Selector::parse(args)?),
            _ => None,
        };
        Ok(Self { target, to: Coords::parse(args)? })
    }
}

impl Command for Teleport {
    /// Each player moved is told where to, as BDS tells them.
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let to = self.to.from(origin(ctx, sender));
        let (moved_players, moved) = (or_sender(ctx, sender, self.target.as_ref()), self.target.as_ref().map_or_else(Vec::new, |target| entities(ctx, target)));
        let [x, y, z] = to.map(|c| format!("{c:.2}"));
        for player in &moved_players {
            ctx.teleport(player.id, to);
            ctx.send_translation(player.id, "commands.tp.successVictim", &[format!("{x}, {y}, {z}")]);
        }
        for entity in &moved {
            ctx.move_entity(entity.id, to);
        }
        let names: Vec<String> = moved_players.iter().map(|player| player.name.clone()).chain(moved.iter().map(|entity| display_name(&entity.kind))).collect();
        match names.is_empty() {
            true => no_target(out),
            false => out.translate("commands.tp.success.coordinates", vec![names.join(", "), x, y, z], true),
        }
    }
}

#[derive(Command)]
#[command(crate = "crate", name = "give", description = "Gives an item to a player", permission = "operator")]
pub(crate) struct Give {
    player: Selector,
    item: String,
    amount: Option<u8>,
}

impl Command for Give {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let item = Item::new(namespaced(&self.item), self.amount.unwrap_or(1).max(1));
        let (shown, count) = (display_name(&item.name), item.count.to_string());
        let given = players(ctx, sender, &self.player);
        for player in &given {
            ctx.give(player.id, &item);
            ctx.send_translation(player.id, "commands.give.successRecipient", &[shown.clone(), count.clone()]);
        }
        match given.is_empty() {
            true => no_target(out),
            false => out.translate("commands.give.success", vec![shown, count, names(&given)], true),
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
            match sender.player().is_some_and(|me| me.id == player.id) {
                true => out.translate("commands.gamemode.success.self", vec![mode.clone()], true),
                false => out.translate("commands.gamemode.success.other", vec![mode.clone(), player.name.clone()], true),
            }
        }
    }
}

#[derive(Command)]
#[command(crate = "crate", name = "summon", description = "Summons an entity", permission = "operator")]
pub(crate) struct Summon {
    kind: String,
    position: Option<Coords>,
}

impl Command for Summon {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let here = origin(ctx, sender);
        let at = self.position.map_or(here, |position| position.from(here));
        let summoned = ctx.spawn_entity(&namespaced(&self.kind), at).is_some();
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
        let named = self.target.as_ref().map_or_else(Vec::new, |target| entities(ctx, target));
        let mut killed: Vec<String> = or_sender(ctx, sender, self.target.as_ref()).into_iter().filter(|player| ctx.kill_player(player.id)).map(|player| player.name).collect();
        killed.extend(named.iter().filter(|entity| ctx.kill_entity(entity.id)).map(|entity| display_name(&entity.kind)));
        match killed.is_empty() {
            true => no_target(out),
            false => out.translate("commands.kill.successful", vec![killed.join(", ")], true),
        }
    }
}

/// `/damage <target> <amount> [cause]`, for players (research/captures/bds-hurt.txt).
// TODO: BDS's refusals (an unknown cause, a target it cannot hurt) were not recorded; entities
// other than players are not hurt.
#[derive(Command)]
#[command(crate = "crate", name = "damage", description = "Apply damage to the specified entities", permission = "operator")]
pub(crate) struct Damage {
    target: Selector,
    amount: i32,
    cause: Option<String>,
}

impl Command for Damage {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let cause = match self.cause.as_deref() {
            None | Some("override" | "none") => DamageCause::Generic,
            Some("entity_attack") => DamageCause::Attack,
            Some("suffocation") => DamageCause::Suffocation,
            Some("fall") => DamageCause::Fall,
            Some("fire") => DamageCause::Fire,
            Some("fire_tick") => DamageCause::FireTick,
            Some("lava") => DamageCause::Lava,
            Some("drowning") => DamageCause::Drowning,
            Some(_) => return out.translate("commands.damage.failed", Vec::new(), false),
        };
        let hurt: Vec<String> = players(ctx, sender, &self.target).into_iter().filter(|player| ctx.hurt_player(player.id, self.amount as f32, cause)).map(|player| player.name).collect();
        match hurt.is_empty() {
            true => out.translate("commands.damage.failed", Vec::new(), false),
            false => out.translate("commands.damage.success", vec![hurt.join(", ")], true),
        }
    }
}
