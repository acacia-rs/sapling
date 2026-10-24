//! `/effect <target> <effect> [seconds] [amplifier] [hideParticles]` and `/effect <target> clear`,
//! answering as BDS does (research/captures/bds-effects.txt).
// TODO: only players, and only the effects the shard knows (`sapling_api::Effect`).

use sapling_api::command::{Args, ParamKind, Parameter, ParseError, Selector, parse_choice};
use sapling_api::{Command, Effect, Output, Sender, ShardCtx};

use super::{no_target, players};

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
        let found = players(ctx, sender, Some(&self.target));
        if found.is_empty() {
            return no_target(out);
        }
        for player in found {
            let name = player.name.clone();
            match self.effect {
                Named::Clear => match ctx.clear_effects(player.id) {
                    true => out.translate("commands.effect.success.removed.all", vec![name], true),
                    false => out.translate("commands.effect.failure.notActive.all", vec![name], false),
                },
                // BDS says so of one that was not on, too.
                Named::Effect(effect) if seconds == 0 => {
                    ctx.remove_effect(player.id, effect);
                    out.translate("commands.effect.success.removed", vec![format!("%{}", effect.key()), name], true);
                }
                Named::Effect(effect) => {
                    ctx.give_effect(player.id, effect, seconds as u32 * TICKS, amplifier.max(0) as u8, self.hide_particles != Some(true));
                    let shown = if effect.is_instant() { 0 } else { seconds };
                    out.translate("commands.effect.success", vec![format!("%{}", effect.key()), amplifier.to_string(), name, shown.to_string()], true);
                }
            }
        }
    }
}
