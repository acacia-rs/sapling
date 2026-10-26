//! `/effect <target> <effect> [seconds] [amplifier] [hideParticles]` and `/effect <target> clear`,
//! answering as BDS does (research/captures/bds-effects.txt).
// TODO: only the effects the shard knows (`sapling_api::Effect`).

use sapling_api::command::{Args, ParamKind, Parameter, ParseError, Selector, parse_choice};
use sapling_api::{Command, Effect, EntityId, Output, PlayerId, Sender, ShardCtx};

use super::{display_name, no_target, targets};

/// Seconds an effect lasts when none are given.
const SECONDS: i32 = 30;
const STRONGEST: i32 = 255;
const TICKS: u32 = 20;

#[derive(Clone, Copy)]
pub(crate) enum Named {
    Clear,
    Effect(Effect),
}

impl Parameter for Named {
    fn kind() -> ParamKind {
        ParamKind::Enum { name: "Effect".into(), values: std::iter::once("clear").chain(Effect::ALL.map(Effect::name)).map(str::to_owned).collect() }
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        let choices: Vec<(&str, Named)> = std::iter::once(("clear", Named::Clear)).chain(Effect::ALL.map(|effect| (effect.name(), Named::Effect(effect)))).collect();
        parse_choice(args, &choices)
    }
}

#[derive(Command)]
#[command(crate = "crate", name = "effect", description = "Add or remove status effects", permission = "operator")]
pub(crate) struct EffectCommand {
    target: Selector,
    effect: Named,
    seconds: Option<i32>,
    amplifier: Option<i32>,
    hide_particles: Option<bool>,
}

impl Command for EffectCommand {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let (seconds, amplifier) = (self.seconds.unwrap_or(SECONDS).max(0), self.amplifier.unwrap_or(0));
        if amplifier > STRONGEST {
            return out.translate("commands.generic.num.tooBig", vec![amplifier.to_string(), STRONGEST.to_string()], false);
        }
        let (named_players, named) = targets(ctx, sender, Some(&self.target));
        let named_players = named_players.into_iter().map(|player| (Target::Player(player.id), player.name));
        let found: Vec<(Target, String)> = named_players.chain(named.into_iter().map(|entity| (Target::Entity(entity.id), display_name(&entity)))).collect();
        if found.is_empty() {
            return no_target(out);
        }
        for (target, name) in found {
            match self.effect {
                Named::Clear => match target.clear(ctx) {
                    true => out.translate("commands.effect.success.removed.all", vec![name], true),
                    false => out.translate("commands.effect.failure.notActive.all", vec![name], false),
                },
                // BDS says so of one that was not on, too.
                Named::Effect(effect) if seconds == 0 => {
                    target.remove(ctx, effect);
                    out.translate("commands.effect.success.removed", vec![format!("%{}", effect.key()), name], true);
                }
                Named::Effect(effect) => {
                    target.give(ctx, effect, seconds as u32 * TICKS, amplifier.max(0) as u8, self.hide_particles != Some(true));
                    let shown = if effect.is_instant() { 0 } else { seconds };
                    out.translate("commands.effect.success", vec![format!("%{}", effect.key()), amplifier.to_string(), name, shown.to_string()], true);
                }
            }
        }
    }
}

/// Whom an `/effect` acts on: a player, or a mob (research/captures/bds-mob-effects.txt).
#[derive(Clone, Copy)]
enum Target {
    Player(PlayerId),
    Entity(EntityId),
}

impl Target {
    fn give(self, ctx: &mut ShardCtx<'_>, effect: Effect, ticks: u32, amplifier: u8, particles: bool) {
        match self {
            Target::Player(id) => drop(ctx.give_effect(id, effect, ticks, amplifier, particles)),
            Target::Entity(id) => drop(ctx.give_entity_effect(id, effect, ticks, amplifier)),
        }
    }

    fn remove(self, ctx: &mut ShardCtx<'_>, effect: Effect) {
        match self {
            Target::Player(id) => drop(ctx.remove_effect(id, effect)),
            Target::Entity(id) => drop(ctx.remove_entity_effect(id, effect)),
        }
    }

    fn clear(self, ctx: &mut ShardCtx<'_>) -> bool {
        match self {
            Target::Player(id) => ctx.clear_effects(id),
            Target::Entity(id) => ctx.clear_entity_effects(id),
        }
    }
}
