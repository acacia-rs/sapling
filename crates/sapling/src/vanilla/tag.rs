//! `/tag`, answering as BDS did (research/captures/bds-scoreboard.txt, bds-scoreboard2.txt): one
//! target is named, more are counted; tags list newest first, each green.

use sapling_api::command::{Args, CommandSpec, CommandSyntax, Overload, ParamKind, Parameter, ParameterSpec, ParseError, Selector, parse_choice};
use sapling_api::{Command, Output, Permission, Sender, ShardCtx, Tagged};

use super::{display_name, no_target, targets};

pub(crate) struct Tag {
    target: Selector,
    /// `None` lists them.
    change: Option<(bool, String)>,
}

impl CommandSyntax for Tag {
    fn spec() -> CommandSpec {
        let entity = || ParameterSpec { name: "entity".into(), kind: ParamKind::Target, optional: false };
        let action = |name: &str, values: &[&str]| ParameterSpec { name: "action".into(), kind: ParamKind::Enum { name: name.into(), values: values.iter().map(|v| (*v).to_owned()).collect() }, optional: false };
        let name = ParameterSpec { name: "name".into(), kind: ParamKind::String, optional: false };
        let overloads = vec![Overload { parameters: vec![entity(), action("TagChangeAction", &["add", "remove"]), name] }, Overload { parameters: vec![entity(), action("TagListAction", &["list"])] }];
        CommandSpec { name: "tag".into(), aliases: Vec::new(), description: "Manages tags stored in entities".into(), permission: Permission::Operator, overloads }
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        let target = Selector::parse(args)?;
        let change = match parse_choice(args, &[("add", Some(true)), ("remove", Some(false)), ("list", None)])? {
            Some(add) => Some((add, String::parse(args)?)),
            None => None,
        };
        args.finish()?;
        Ok(Self { target, change })
    }
}

fn shown(tags: &[String]) -> String {
    tags.iter().map(|tag| format!("§a{tag}§r")).collect::<Vec<_>>().join(", ")
}

impl Command for Tag {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let (players, entities) = targets(ctx, sender, Some(&self.target));
        let mut found: Vec<(Tagged, String)> = players.into_iter().map(|player| (Tagged::Player(player.id), player.name)).collect();
        found.extend(entities.iter().map(|entity| (Tagged::Entity(entity.id), display_name(entity))));
        if found.is_empty() {
            return no_target(out);
        }
        let Some((add, tag)) = self.change else { return list(ctx, out, &found) };
        let done = found.iter().filter(|(of, _)| if add { ctx.add_tag(*of, &tag) } else { ctx.remove_tag(*of, &tag) }).count();
        let verb = if add { "add" } else { "remove" };
        match (done, found.as_slice()) {
            (0, _) => out.translate(format!("commands.tag.{verb}.failed"), Vec::new(), false),
            (_, [(_, name)]) => out.translate(format!("commands.tag.{verb}.success.single"), vec![tag, name.clone()], true),
            _ => out.translate(format!("commands.tag.{verb}.success.multiple"), vec![tag, done.to_string()], true),
        }
    }
}

fn list(ctx: &mut ShardCtx<'_>, out: &mut Output, found: &[(Tagged, String)]) {
    if let [(of, name)] = found {
        let tags = ctx.tags(*of);
        return match tags.is_empty() {
            true => out.translate("commands.tag.list.single.empty", vec![name.clone()], true),
            false => out.translate("commands.tag.list.single.success", vec![name.clone(), tags.len().to_string(), shown(&tags)], true),
        };
    }
    let mut all: Vec<String> = Vec::new();
    for tag in found.iter().flat_map(|(of, _)| ctx.tags(*of)) {
        if !all.contains(&tag) {
            all.push(tag);
        }
    }
    match all.is_empty() {
        true => out.translate("commands.tag.list.multiple.empty", vec![found.len().to_string()], true),
        false => out.translate("commands.tag.list.multiple.success", vec![found.len().to_string(), all.len().to_string(), shown(&all)], true),
    }
}
