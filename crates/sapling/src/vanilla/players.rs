//! The commands aimed at players and entities: `/tp`, `/gamemode`, `/summon`, `/kill` and
//! `/damage`, and `/difficulty`.

use sapling_api::command::{Args, CommandSpec, CommandSyntax, Coords, Overload, Parameter, ParameterSpec, ParseError, Selector};
use sapling_api::{Command, DamageCause, Difficulty, GameMode, Output, Permission, Sender, ShardCtx};

use super::{display_name, no_target, players, targets, tell_target};

fn namespaced(name: &str) -> String {
    if name.contains(':') { name.to_owned() } else { format!("minecraft:{name}") }
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
        let here = ctx.origin(sender);
        let (to, at) = match &self.to {
            Destination::Place(coords) => (coords.from(here), None),
            // The nearest of those it names.
            Destination::At(selector) => {
                let (players, entities) = targets(ctx, sender, Some(selector));
                let named = players.into_iter().filter_map(|player| Some((ctx.position(player.id)?.feet, player.name)));
                let all = named.chain(entities.into_iter().map(|entity| (entity.feet, display_name(&entity.kind))));
                let far = |feet: &[f32; 3]| (0..3).map(|axis| (feet[axis] - here[axis]).powi(2)).sum::<f32>();
                match all.min_by(|a, b| far(&a.0).total_cmp(&far(&b.0))) {
                    Some((feet, name)) => (feet, Some(name)),
                    None => return no_target(out),
                }
            }
        };
        let (moved_players, moved) = targets(ctx, sender, self.target.as_ref());
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
        let names: Vec<String> = moved_players.iter().map(|player| player.name.clone()).chain(moved.iter().map(|entity| display_name(&entity.kind))).collect();
        match (names.is_empty(), at) {
            (true, _) => no_target(out),
            (false, Some(at)) => out.translate("commands.tp.success", vec![names.join(", "), at], true),
            (false, None) => out.translate("commands.tp.success.coordinates", vec![names.join(", "), x, y, z], true),
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
        let changed = players(ctx, sender, self.player.as_ref());
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

/// `/difficulty <difficulty>`: a name, its first letter, or 0 to 3; another number is refused
/// when run (research/captures/bds-worldcmds.txt).
pub(crate) struct DifficultyCommand {
    difficulty: Result<Difficulty, i32>,
}

impl CommandSyntax for DifficultyCommand {
    fn spec() -> CommandSpec {
        let overload = |parameter: ParameterSpec| Overload { parameters: vec![parameter] };
        CommandSpec {
            name: "difficulty".into(),
            aliases: Vec::new(),
            description: "Sets the difficulty level".into(),
            permission: Permission::Operator,
            overloads: vec![overload(ParameterSpec::of::<Difficulty>("difficulty")), overload(ParameterSpec::of::<i32>("difficulty"))],
        }
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        let levels = [Difficulty::Peaceful, Difficulty::Easy, Difficulty::Normal, Difficulty::Hard];
        let difficulty = match args.peek().and_then(|word| word.parse::<i32>().ok()) {
            Some(number) => {
                args.word()?;
                usize::try_from(number).ok().and_then(|at| levels.get(at).copied()).ok_or(number)
            }
            None => Ok(Difficulty::parse(args)?),
        };
        args.finish()?;
        Ok(Self { difficulty })
    }
}

impl Command for DifficultyCommand {
    fn run(self, ctx: &mut ShardCtx<'_>, _: &Sender, out: &mut Output) {
        match self.difficulty {
            Ok(difficulty) => {
                ctx.set_difficulty(difficulty);
                out.translate("commands.difficulty.success", vec![format!("{difficulty:?}").to_uppercase()], true);
            }
            Err(number) => out.translate("commands.generic.parameter.invalid", vec![number.to_string()], false),
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
        let here = ctx.origin(sender);
        let at = self.position.map_or(here, |position| position.from(here));
        // BDS summons its newer zombie villager under the older one's name, and knows no other.
        let kind = Some(namespaced(&self.kind)).filter(|kind| kind != "minecraft:zombie_villager").unwrap_or_else(|| "minecraft:zombie_villager_v2".into());
        let summoned = ctx.spawn_entity(&kind, at).is_some();
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
        let (players, named) = targets(ctx, sender, self.target.as_ref());
        let mut killed: Vec<String> = players.into_iter().filter(|player| ctx.kill_player(player.id)).map(|player| player.name).collect();
        killed.extend(named.iter().filter(|entity| ctx.kill_entity(entity.id)).map(|entity| display_name(&entity.kind)));
        match killed.is_empty() {
            true => no_target(out),
            false => out.translate("commands.kill.successful", vec![killed.join(", ")], true),
        }
    }
}

/// `/damage <target> <amount> [cause]`, for players (research/captures/bds-hurt.txt).
// TODO: what BDS answers an unknown cause was not recorded; entities
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
        let found = players(ctx, sender, Some(&self.target));
        let names = || found.iter().map(|player| player.name.clone()).collect::<Vec<_>>().join(", ");
        // BDS fails `suffocation` for anyone (research/captures/bds-worn.txt).
        let Some(cause) = DamageCause::named(self.cause.as_deref().unwrap_or("none")).filter(|cause| *cause != DamageCause::Suffocation) else {
            return out.translate("commands.damage.failed", vec![names()], false);
        };
        let hurt: Vec<String> = found.iter().filter(|player| ctx.hurt_player(player.id, self.amount as f32, cause)).map(|player| player.name.clone()).collect();
        match hurt.is_empty() {
            true => out.translate("commands.damage.failed", vec![names()], false),
            false => out.translate("commands.damage.success", vec![hurt.join(", ")], true),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sapling_api::command::Base;

    #[test]
    fn tp_takes_a_place_or_a_target_to_go_to_with_or_without_a_victim() {
        let sender = Sender::Console;
        let parsed = |line: &str| Teleport::parse(&mut Args::new(line, 0, &sender, &[])).map(|tp| (tp.target, matches!(tp.to, Destination::At(Selector::Variable(Base::Entities, _)))));
        assert_eq!(parsed("~ ~8 ~").ok(), Some((None, false)));
        assert_eq!(parsed("@s 1 2 3").ok(), Some((Some(Selector::sender()), false)));
        assert_eq!(parsed("@s @e[type=iron_golem, c=1]").ok(), Some((Some(Selector::sender()), true)));
        assert_eq!(parsed("@e[type=iron_golem]").ok(), Some((None, true)));
        assert!(matches!(parsed("@s"), Ok((None, false))), "alone, a target is where to");
        let looks = |line: &str| Teleport::parse(&mut Args::new(line, 0, &sender, &[])).ok().and_then(|tp| tp.looks);
        assert_eq!([looks("~ ~ ~4 -90 10"), looks("@s ~ ~ ~4 45"), looks("~ ~ ~4")], [Some([-90.0, 10.0]), Some([45.0, 0.0]), None]);
    }

    #[test]
    fn difficulty_takes_names_letters_and_numbers() {
        let sender = Sender::Console;
        let parsed = |line: &str| DifficultyCommand::parse(&mut Args::new(line, 0, &sender, &[])).map(|command| command.difficulty);
        assert_eq!(["p", "e", "N", "hard", "0", "3"].map(|line| parsed(line).unwrap()), [Ok(Difficulty::Peaceful), Ok(Difficulty::Easy), Ok(Difficulty::Normal), Ok(Difficulty::Hard), Ok(Difficulty::Peaceful), Ok(Difficulty::Hard)]);
        assert_eq!(parsed("4").unwrap(), Err(4));
        assert!(parsed("x").is_err());
    }
}
