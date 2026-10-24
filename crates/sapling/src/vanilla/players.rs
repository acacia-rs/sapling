//! The commands aimed at players and entities: `/tp`, `/give`, `/clear`, `/gamemode`, `/summon`
//! `/kill` and `/damage`.

use sapling_api::command::{Args, CommandSpec, CommandSyntax, Coords, Overload, Parameter, ParameterSpec, ParseError, Selector};
use sapling_api::{Command, Difficulty, GameMode, Item, Output, Permission, Sender, ShardCtx};

use super::{display_name, entities, item_display_name, no_target, origin, players, tell_target};

fn namespaced(name: &str) -> String {
    if name.contains(':') { name.to_owned() } else { format!("minecraft:{name}") }
}

/// The players a missing target means: the sender.
fn or_sender(ctx: &ShardCtx<'_>, sender: &Sender, target: Option<&Selector>) -> Vec<sapling_api::Player> {
    players(ctx, sender, target.unwrap_or(&Selector::Sender))
}

/// `/tp [victim] <destination>`: the first word is a target unless it is a coordinate, and the
/// destination a place or whoever a target names (research/captures/bds-golem.txt). A place may
/// be followed by where to look: yaw, then pitch.
pub(crate) struct Teleport {
    target: Option<Selector>,
    to: Destination,
    looks: Option<[f32; 2]>,
}

enum Destination {
    Place(Coords),
    At(Selector),
}

impl CommandSyntax for Teleport {
    fn spec() -> CommandSpec {
        let destination = ParameterSpec::of::<Coords>("destination");
        CommandSpec {
            name: "tp".into(),
            aliases: vec!["teleport".into()],
            description: "Teleports entities".into(),
            permission: Permission::Operator,
            overloads: vec![
                Overload { parameters: vec![destination.clone()] },
                Overload { parameters: vec![ParameterSpec::of::<Selector>("victim"), destination] },
                Overload { parameters: vec![ParameterSpec::of::<Selector>("destination")] },
                Overload { parameters: vec![ParameterSpec::of::<Selector>("victim"), ParameterSpec::of::<Selector>("destination")] },
            ],
        }
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        let is_coordinate = |word: &str| word.starts_with('~') || word.parse::<f32>().is_ok();
        let first = match args.peek() {
            Some(word) if !is_coordinate(word) => Some(Selector::parse(args)?),
            _ => None,
        };
        Ok(match (first, args.peek().map(is_coordinate)) {
            (Some(to), None) => Self { target: None, to: Destination::At(to), looks: None },
            (target, Some(false)) => Self { target, to: Destination::At(Selector::parse(args)?), looks: None },
            (target, _) => {
                let to = Destination::Place(Coords::parse(args)?);
                let angle = |args: &mut Args<'_>| -> Result<Option<f32>, ParseError> {
                    let mark = args.mark();
                    if args.is_empty() { Ok(None) } else { args.word()?.parse().map(Some).map_err(|_| args.error_at(mark)) }
                };
                let looks = match (angle(args)?, angle(args)?) {
                    (Some(yaw), pitch) => Some([yaw, pitch.unwrap_or(0.0)]),
                    _ => None,
                };
                Self { target, to, looks }
            }
        })
    }
}

impl Command for Teleport {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        self.run_in(ctx, sender, out, None);
    }
}

impl Teleport {
    /// Each player moved is told where to, as BDS tells them; into `dimension` of its world when
    /// one is named (`/execute in`), the way BDS changes a player's dimension.
    pub(crate) fn run_in(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output, dimension: Option<i32>) {
        let here = origin(ctx, sender);
        let (to, at) = match &self.to {
            Destination::Place(coords) => (coords.from(here), None),
            // The nearest of those it names.
            Destination::At(selector) => {
                let named = players(ctx, sender, selector).into_iter().filter_map(|player| Some((ctx.position(player.id)?.feet, player.name)));
                let all = named.chain(entities(ctx, selector).into_iter().map(|entity| (entity.feet, display_name(&entity))));
                let far = |feet: &[f32; 3]| (0..3).map(|axis| (feet[axis] - here[axis]).powi(2)).sum::<f32>();
                match all.min_by(|a, b| far(&a.0).total_cmp(&far(&b.0))) {
                    Some((feet, name)) => (feet, Some(name)),
                    None => return no_target(out),
                }
            }
        };
        let (moved_players, moved) = (or_sender(ctx, sender, self.target.as_ref()), self.target.as_ref().map_or_else(Vec::new, |target| entities(ctx, target)));
        let [x, y, z] = to.map(|c| format!("{c:.2}"));
        for player in &moved_players {
            match (dimension, self.looks) {
                (Some(dimension), _) => drop(ctx.travel(player.id, dimension, to)),
                (None, Some(looks)) => ctx.teleport_facing(player.id, to, looks),
                (None, None) => ctx.teleport(player.id, to),
            }
            tell_target(ctx, player.id, "commands.tp.successVictim", &[at.clone().unwrap_or_else(|| format!("{x}, {y}, {z}"))]);
        }
        for entity in &moved {
            ctx.move_entity(entity.id, to);
        }
        let names: Vec<String> = moved_players.iter().map(|player| player.name.clone()).chain(moved.iter().map(|entity| display_name(&entity))).collect();
        match (names.is_empty(), at) {
            (true, _) => no_target(out),
            (false, Some(at)) => out.translate("commands.tp.success", vec![names.join(", "), at], true),
            (false, None) => out.translate("commands.tp.success.coordinates", vec![names.join(", "), x, y, z], true),
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
        let item = Item::new(super::item_named(&self.item), self.amount.unwrap_or(1).max(1));
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
}

impl Command for Summon {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let here = origin(ctx, sender);
        let at = self.position.map_or(here, |position| position.from(here));
        // BDS summons its newer zombie villager under the older one's name, and knows no other.
        let kind = Some(namespaced(&self.kind)).filter(|kind| kind != "minecraft:zombie_villager").unwrap_or_else(|| "minecraft:zombie_villager_v2".into());
        let summoned = match sender.player() {
            Some(by) => ctx.summon(by.id, &kind, at),
            None => ctx.spawn_entity(&kind, at),
        }
        .is_some();
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
        killed.extend(named.iter().filter(|entity| ctx.kill_entity(entity.id)).map(|entity| display_name(&entity)));
        match killed.is_empty() {
            true => no_target(out),
            false => out.translate("commands.kill.successful", vec![killed.join(", ")], true),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tp_takes_a_place_or_a_target_to_go_to_with_or_without_a_victim() {
        let sender = Sender::Console;
        let parsed = |line: &str| Teleport::parse(&mut Args::new(line, 0, &sender, &[])).map(|tp| (tp.target, matches!(tp.to, Destination::At(Selector::Entities { .. }))));
        let golem = Some(Selector::Entities { kind: Some("minecraft:iron_golem".into()) });
        assert_eq!(parsed("~ ~8 ~").ok(), Some((None, false)));
        assert_eq!(parsed("@s 1 2 3").ok(), Some((Some(Selector::Sender), false)));
        assert_eq!(parsed("@s @e[type=iron_golem,c=1]").ok(), Some((Some(Selector::Sender), true)));
        assert_eq!(parsed("@e[type=iron_golem]").ok(), Some((None, true)));
        assert_eq!(golem.is_some() && parsed("@s").is_ok(), true, "alone, a target is where to");
        let looks = |line: &str| Teleport::parse(&mut Args::new(line, 0, &sender, &[])).ok().and_then(|tp| tp.looks);
        assert_eq!([looks("~ ~ ~4 -90 10"), looks("@s ~ ~ ~4 45"), looks("~ ~ ~4")], [Some([-90.0, 10.0]), Some([45.0, 0.0]), None]);
    }
}
