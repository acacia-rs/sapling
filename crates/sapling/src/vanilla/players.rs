//! The commands aimed at players and entities: `/gamemode`, `/summon`, `/kill` and `/difficulty`
//! (`/tp`: teleport.rs, `/damage`: damage.rs, `/give` and `/clear`: items.rs).

use sapling_api::command::{Args, CommandSpec, CommandSyntax, Coords, Overload, Parameter, ParameterSpec, ParseError, Selector};
use sapling_api::{Command, Difficulty, GameMode, Output, Permission, Sender, ShardCtx};

use super::{display_name, no_target, players, targets, tell_target};

fn namespaced(name: &str) -> String {
    if name.contains(':') { name.to_owned() } else { format!("minecraft:{name}") }
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
    yaw: Option<f32>,
    pitch: Option<f32>,
    /// A spawn event of the kind's: `minecraft:panda_lazy`.
    event: Option<String>,
}

impl Command for Summon {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let at = self.position.map_or_else(|| ctx.origin(sender), |position| position.resolve(ctx, sender));
        // BDS summons its newer villager and zombie villager under the older ones' names, and knows no others.
        let kind = match namespaced(&self.kind) {
            kind if kind == "minecraft:zombie_villager" || kind == "minecraft:villager" => format!("{kind}_v2"),
            kind => kind,
        };
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
        let (people, named) = targets(ctx, sender, self.target.as_ref());
        let mut killed: Vec<String> = people.into_iter().filter(|player| ctx.kill_player(player.id)).map(|player| player.name).collect();
        killed.extend(named.iter().filter(|entity| ctx.kill_entity(entity.id)).map(|entity| display_name(&entity)));
        match killed.is_empty() {
            true => no_target(out),
            false => {
                // BDS counts each one killed (research/captures/bds-execute-if.txt `kill @e[type=cow]`).
                out.done_to(killed.len() as u32);
                out.translate("commands.kill.successful", vec![killed.join(", ")], true);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn difficulty_takes_names_letters_and_numbers() {
        let sender = Sender::Console;
        let parsed = |line: &str| DifficultyCommand::parse(&mut Args::new(line, 0, &sender, &[])).map(|command| command.difficulty);
        assert_eq!(["p", "e", "N", "hard", "0", "3"].map(|line| parsed(line).unwrap()), [Ok(Difficulty::Peaceful), Ok(Difficulty::Easy), Ok(Difficulty::Normal), Ok(Difficulty::Hard), Ok(Difficulty::Peaceful), Ok(Difficulty::Hard)]);
        assert_eq!(parsed("4").unwrap(), Err(4));
        assert!(parsed("x").is_err());
    }
}
