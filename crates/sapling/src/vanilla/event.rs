//! `/event entity <target> <eventName>`, answered as BDS answered it (research/captures/bds-bb1event.txt): the
//! event fires on each target whose definition names it exactly so (no namespace is added or dropped);
//! one line lists the kinds it fired on, counted, another the kinds that have no such event.

use sapling_api::command::{Args, CommandSpec, CommandSyntax, Overload, ParamKind, Parameter, ParameterSpec, ParseError, Selector, parse_choice};
use sapling_api::{Command, Output, Permission, Sender, ShardCtx};

use super::{no_target, targets};

const PLAYER: &str = "minecraft:player";

pub(crate) struct EventCommand {
    target: Selector,
    event: String,
}

impl CommandSyntax for EventCommand {
    fn spec() -> CommandSpec {
        let entity = ParamKind::Enum { name: "EntityEvents".into(), values: vec!["entity".into()] };
        let parameters = vec![
            ParameterSpec { name: "entity".into(), kind: entity, optional: false },
            ParameterSpec { name: "target".into(), kind: ParamKind::Target, optional: false },
            ParameterSpec { name: "eventName".into(), kind: ParamKind::String, optional: false },
        ];
        CommandSpec { name: "event".into(), aliases: Vec::new(), description: "Triggers an event on the specified object(s)".into(), permission: Permission::Operator, overloads: vec![Overload { parameters }] }
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        parse_choice(args, &[("entity", ())])?;
        let target = Selector::parse(args)?;
        let event = String::parse(args)?;
        args.finish()?;
        Ok(Self { target, event })
    }
}

impl Command for EventCommand {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let (players, entities) = targets(ctx, sender, Some(&self.target));
        if players.is_empty() && entities.is_empty() {
            return no_target(out);
        }
        // TODO: a player's events (`minecraft:gain_raid_omen` gives BDS's player the raid omen) do nothing.
        let mut fired = players.iter().map(|_| (PLAYER.to_owned(), sapling_data::has_event(PLAYER, &self.event))).collect::<Vec<_>>();
        fired.extend(entities.iter().map(|entity| (entity.kind.clone(), ctx.fire_entity_event(entity.id, &self.event))));
        let kinds = |hit: bool| fired.iter().filter(|(_, fired)| *fired == hit).map(|(kind, _)| kind.as_str()).fold(Vec::<&str>::new(), |mut kinds, kind| {
            if !kinds.contains(&kind) {
                kinds.push(kind);
            }
            kinds
        });
        let (hit, missed) = (kinds(true), kinds(false));
        if !hit.is_empty() {
            out.done_to(hit.len() as u32);
            out.translate("commands.event.success", vec![self.event.clone(), hit.join(", ")], true);
        }
        if !missed.is_empty() {
            out.translate("commands.event.error.failed", vec![self.event, missed.join(", ")], false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_takes_entity_a_target_and_an_event() {
        let sender = Sender::Console;
        let parsed = |line: &str| EventCommand::parse(&mut Args::new(line, 0, &sender, &[])).map(|command| command.event).ok();
        assert_eq!(parsed("entity @e[type=sheep] spawn_baby"), Some("spawn_baby".into()));
        assert_eq!([parsed("entity @e[type=sheep]"), parsed("block @s x"), parsed("entity @s a b")], [None, None, None]);
    }
}
