//! `/damage <target> <amount> [cause] [entity <damager>]`, for players and mobs (research/captures/bds-hurt.txt,
//! bds-mob-effects.txt, bds-fixmisc.txt).

use sapling_api::command::{Args, CommandSpec, CommandSyntax, Overload, ParamKind, Parameter, ParameterSpec, ParseError, Selector};
use sapling_api::{Attacker, Command, DamageCause, Output, Permission, Sender, ShardCtx};

use super::{display_name, no_target, targets};

pub(crate) struct Damage {
    target: Selector,
    amount: i32,
    cause: DamageCause,
    by: Option<Selector>,
}

impl CommandSyntax for Damage {
    fn spec() -> CommandSpec {
        let head = || vec![ParameterSpec::of::<Selector>("target"), ParameterSpec::of::<i32>("amount")];
        let cause = |optional: bool| ParameterSpec { name: "cause".into(), kind: ParamKind::String, optional };
        let by = [cause(false), ParameterSpec::literal("damage", "entity"), ParameterSpec::of::<Selector>("damager")];
        CommandSpec {
            name: "damage".into(),
            aliases: Vec::new(),
            description: "Apply damage to the specified entities".into(),
            permission: Permission::Operator,
            overloads: vec![Overload { parameters: [head(), vec![cause(true)]].concat() }, Overload { parameters: [head(), by.into()].concat() }],
        }
    }

    /// An unknown cause is a syntax error at it, as is `entity` with no damager (bds-fixmisc.txt).
    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        let (target, amount) = (Selector::parse(args)?, i32::parse(args)?);
        let mark = args.mark();
        let cause = match args.is_empty() {
            true => DamageCause::Generic,
            false => DamageCause::named(args.word()?).ok_or_else(|| args.error_at(mark))?,
        };
        let by = match args.is_empty() {
            true => None,
            false => {
                let mark = args.mark();
                if !args.word()?.eq_ignore_ascii_case("entity") {
                    return Err(args.error_at(mark));
                }
                if args.is_empty() {
                    return Err(args.error_at(args.mark()));
                }
                Some(Selector::parse(args)?)
            }
        };
        args.finish()?;
        Ok(Self { target, amount, cause, by })
    }
}

impl Command for Damage {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let attacker = match &self.by {
            None => None,
            Some(by) => match targets(ctx, sender, Some(by)) {
                (players, mobs) if players.len() + mobs.len() > 1 => return out.translate("commands.damage.tooManySources", Vec::new(), false),
                (mut players, _) if !players.is_empty() => Some(Attacker::Player(players.remove(0))),
                (_, mut mobs) if !mobs.is_empty() => Some(Attacker::Mob { kind: mobs.remove(0).kind }),
                _ => return no_target(out),
            },
        };
        let (found, mobs) = targets(ctx, sender, Some(&self.target));
        let names = || found.iter().map(|player| player.name.clone()).chain(mobs.iter().map(|mob| display_name(mob))).collect::<Vec<_>>().join(", ");
        // BDS fails `suffocation` for anyone (research/captures/bds-worn.txt).
        if self.cause == DamageCause::Suffocation {
            return out.translate("commands.damage.failed", vec![names()], false);
        }
        let (amount, cause) = (self.amount as f32, self.cause);
        let mut hurt: Vec<String> = found
            .iter()
            .filter(|player| match &attacker {
                Some(by) => ctx.hurt_player_by(player.id, amount, cause, by),
                None => ctx.hurt_player(player.id, amount, cause),
            })
            .map(|player| player.name.clone())
            .collect();
        hurt.extend(mobs.iter().filter(|mob| ctx.hurt_entity(mob.id, amount, cause)).map(|mob| display_name(mob)));
        match hurt.is_empty() {
            true => out.translate("commands.damage.failed", vec![names()], false),
            false => out.translate("commands.damage.success", vec![hurt.join(", ")], true),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(line: &str) -> Result<(DamageCause, bool), ParseError> {
        let sender = Sender::Console;
        Damage::parse(&mut Args::new(line, 0, &sender, &[])).map(|damage| (damage.cause, damage.by.is_some()))
    }

    #[test]
    fn damage_takes_a_damager_after_a_cause_and_refuses_unknown_words() {
        assert_eq!(parsed("@e 2").unwrap(), (DamageCause::Generic, false));
        assert_eq!(parsed("@e 2 entity_attack entity @s").unwrap(), (DamageCause::Attack, true));
        assert!(parsed("@e 2 entity @s").is_err(), "a damager needs a cause before it");
        assert!(parsed("@e 2 entity_attack entity").is_err());
        assert!(parsed("@e 2 banana").is_err());
    }
}
