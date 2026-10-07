//! The commands aimed at players and entities: `/tp`, `/give`, `/clear`, `/gamemode`, `/summon`
//! and `/kill`.

use sapling_api::command::{Args, CommandSpec, CommandSyntax, Coords, Overload, Parameter, ParameterSpec, ParseError, Selector};
use sapling_api::{Command, GameMode, Item, Output, Permission, Sender, ShardCtx};

use super::{entities, origin, players};

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
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let to = self.to.from(origin(ctx, sender));
        let (moved_players, moved) = (or_sender(ctx, sender, self.target.as_ref()), self.target.as_ref().map_or_else(Vec::new, |target| entities(ctx, target)));
        moved_players.iter().for_each(|player| ctx.teleport(player.id, to));
        for entity in &moved {
            ctx.move_entity(entity.id, to);
        }
        match moved_players.len() + moved.len() {
            0 => out.error("No targets matched selector"),
            count => out.print(format!("Teleported {count} to {:.2}, {:.2}, {:.2}", to[0], to[1], to[2])),
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
        let given = players(ctx, sender, &self.player);
        for player in &given {
            ctx.give(player.id, &item);
        }
        match given.is_empty() {
            true => out.error("No targets matched selector"),
            false => out.print(format!("Gave {} * {} to {} players", item.name, item.count, given.len())),
        }
    }
}

#[derive(Command)]
#[command(crate = "crate", name = "clear", description = "Clears items from a player's inventory", permission = "operator")]
pub(crate) struct Clear {
    player: Option<Selector>,
}

impl Command for Clear {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let cleared = or_sender(ctx, sender, self.player.as_ref());
        cleared.iter().for_each(|player| ctx.clear_inventory(player.id));
        match cleared.is_empty() {
            true => out.error("No targets matched selector"),
            false => out.print(format!("Cleared the inventory of {} players", cleared.len())),
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
        changed.iter().for_each(|player| ctx.set_game_mode(player.id, self.mode));
        match changed.as_slice() {
            [] => out.error("No targets matched selector"),
            [player] => out.print(format!("{} is now in {:?} mode", player.name, self.mode)),
            many => out.print(format!("{} players are now in {:?} mode", many.len(), self.mode)),
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
        match ctx.spawn_entity(&namespaced(&self.kind), at) {
            Some(_) => out.print("Object successfully summoned"),
            None => out.error(format!("Unable to summon {}", self.kind)),
        }
    }
}

#[derive(Command)]
#[command(crate = "crate", name = "kill", description = "Kills entities", permission = "operator")]
pub(crate) struct Kill {
    target: Option<Selector>,
}

impl Command for Kill {
    /// Players are not hurt: sapling has no damage. Every other entity named is removed.
    fn run(self, ctx: &mut ShardCtx<'_>, _: &Sender, out: &mut Output) {
        let gone = self.target.as_ref().map_or_else(Vec::new, |target| entities(ctx, target));
        let killed = gone.iter().filter(|entity| ctx.remove_entity(entity.id)).count();
        match killed {
            0 => out.error("No targets matched selector"),
            count => out.print(format!("Killed {count} entities")),
        }
    }
}
