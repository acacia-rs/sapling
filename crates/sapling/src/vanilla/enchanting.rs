//! `/enchant`, answered as BDS answers (research/captures/bds-itemcmds.txt); which enchantment
//! goes on what is `sapling_data::enchant`'s table.

use sapling_api::command::{Args, CommandSpec, CommandSyntax, Overload, ParamKind, Parameter, ParameterSpec, ParseError, Selector};
use sapling_api::{Command, Output, Permission, Sender, ShardCtx};
use sapling_data::enchant::{self, Kind};

use super::{no_target, players};

/// An enchantment as typed: its number, which may name none, or its word.
#[derive(Debug, PartialEq)]
enum Named {
    Id(i32),
    Kind(&'static Kind),
}

/// `/enchant <player> <enchantmentId|enchantmentName> [level]`: the item in each player's hand
/// gets the enchantment at the level, 1 without one.
pub(crate) struct Enchant {
    player: Selector,
    enchantment: Named,
    level: Option<i32>,
}

impl CommandSyntax for Enchant {
    fn spec() -> CommandSpec {
        let names = ParamKind::Enum { name: "Enchant".into(), values: enchant::KINDS.iter().map(|kind| kind.word.to_owned()).collect() };
        let overload = |enchantment: ParameterSpec| Overload { parameters: vec![ParameterSpec::of::<Selector>("player"), enchantment, ParameterSpec::of::<Option<i32>>("level")] };
        CommandSpec {
            name: "enchant".into(),
            aliases: Vec::new(),
            description: "Adds an enchantment to a player's selected item".into(),
            permission: Permission::Operator,
            overloads: vec![overload(ParameterSpec::of::<i32>("enchantmentId")), overload(ParameterSpec { kind: names, ..ParameterSpec::of::<String>("enchantmentName") })],
        }
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        let player = Selector::parse(args)?;
        let mark = args.mark();
        let word = args.word()?;
        let enchantment = match word.parse::<i32>() {
            Ok(id) => Named::Id(id),
            Err(_) => Named::Kind(enchant::named(word).ok_or_else(|| args.error_at(mark))?),
        };
        let level = Option::<i32>::parse(args)?;
        args.finish()?;
        Ok(Self { player, enchantment, level })
    }
}

impl Command for Enchant {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let kind = match self.enchantment {
            Named::Kind(kind) => kind,
            Named::Id(id) => match i16::try_from(id).ok().and_then(enchant::kind_of) {
                Some(kind) => kind,
                None => return out.translate("commands.enchant.notFound", vec![id.to_string()], false),
            },
        };
        let enchanted = players(ctx, sender, Some(&self.player));
        if enchanted.is_empty() {
            return no_target(out);
        }
        let level = self.level.unwrap_or(1);
        if !(1..=i32::from(kind.max)).contains(&level) {
            let bound = if level < 1 { ("commands.generic.num.tooSmall", 1) } else { ("commands.generic.num.tooBig", kind.max) };
            out.translate(bound.0, vec![level.to_string(), bound.1.to_string()], false);
            return out.translate("commands.enchant.invalidLevel", vec![format!("%{}", kind.key), level.to_string()], false);
        }
        for player in &enchanted {
            let (Some(slot), Some(mut item)) = (ctx.held_slot(player.id), ctx.held_item(player.id)) else {
                out.translate("commands.enchant.noItem", vec![player.name.clone()], false);
                continue;
            };
            // Another level of one it has only goes on above it: `knockback 1` after `knockback 2` failed.
            let fits = |&(id, at): &(i16, i16)| enchant::kind_of(id).is_none_or(|other| enchant::compatible(kind, other) && (other.id != kind.id || at < level as i16));
            if !enchant::goes_on(kind, &item.name) || !item.enchantments.iter().all(fits) {
                out.translate("commands.enchant.cantEnchant", vec![player.name.clone()], false);
                continue;
            }
            item.enchantments.retain(|(id, _)| *id != kind.id);
            item.enchantments.push((kind.id, level as i16));
            ctx.set_item(player.id, sapling_api::Slot::Main(slot), Some(&item));
            out.translate("commands.enchant.success", vec![player.name.clone()], true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_enchantment_is_a_number_or_a_word() {
        let sender = Sender::Console;
        let parsed = |line: &str| Enchant::parse(&mut Args::new(line, 0, &sender, &[])).map(|enchant| (enchant.enchantment, enchant.level));
        assert_eq!(parsed("@s 12 2").unwrap(), (Named::Id(12), Some(2)));
        assert_eq!(parsed("@s 99").unwrap(), (Named::Id(99), None));
        assert_eq!(parsed("@s Sharpness 5").unwrap(), (Named::Kind(enchant::named("sharpness").unwrap()), Some(5)));
        assert!(parsed("@s nosuch").is_err());
    }
}
