//! The console's `/help` (`/?`), as BDS's console printed it (research/captures/bds-console-help.txt):
//! seven usage lines a page, every overload of every command, sorted by the name each is listed
//! under (its last alias: `/?`, `/alwaysday`); `help <command>` its name, description and usage.
//! BDS's players have `/help` too, unlisted; sapling has no unlisted command, so only the console does.

use sapling_api::command::{Args, CommandSpec, Commands, Overload, ParamKind};
use sapling_api::{Output, Permission, Sender};

const PER_PAGE: usize = 7;
const NAMES: [&str; 2] = ["help", "?"];

/// The answer to `line` if it is `/help`.
pub(super) fn answer(commands: &Commands, line: &str) -> Option<Output> {
    let line = line.strip_prefix('/').unwrap_or(line);
    let (word, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
    if !NAMES.iter().any(|name| name.eq_ignore_ascii_case(word)) {
        return None;
    }
    let mut out = Output::default();
    let specs: Vec<&CommandSpec> = commands.visible(Permission::Operator).collect();
    let asked = rest.trim();
    match asked.parse::<i64>() {
        _ if asked.is_empty() => page(&mut out, &specs, 1),
        Ok(number) => page(&mut out, &specs, number),
        Err(_) => match specs.iter().find(|spec| named(spec, asked)) {
            Some(spec) => usage(&mut out, spec),
            None if asked.eq_ignore_ascii_case("help") || asked == "?" => help_usage(&mut out),
            None => {
                let sender = Sender::Console;
                let start = line.len() - rest.trim_start().len();
                out.refuse(Args::new(line, start, &sender, &[]).error_at(start));
            }
        },
    }
    Some(out)
}

fn named(spec: &CommandSpec, name: &str) -> bool {
    spec.name.eq_ignore_ascii_case(name) || spec.aliases.iter().any(|alias| alias.eq_ignore_ascii_case(name))
}

fn page(out: &mut Output, specs: &[&CommandSpec], asked: i64) {
    let mut lines: Vec<String> = specs.iter().flat_map(|spec| spec.overloads.iter().map(|overload| line(listed_as(spec), overload))).collect();
    lines.extend(help_lines("?"));
    lines.sort();
    let pages = lines.len().div_ceil(PER_PAGE).max(1);
    let number = usize::try_from(asked).unwrap_or(1).clamp(1, pages);
    out.translate("§2%commands.help.header", vec![number.to_string(), pages.to_string()], true);
    out.print(lines[(number - 1) * PER_PAGE..].iter().take(PER_PAGE).cloned().collect::<Vec<_>>().join("\n"));
    out.translate("§2%commands.help.footer", Vec::new(), true);
}

fn listed_as(spec: &CommandSpec) -> &str {
    spec.aliases.last().unwrap_or(&spec.name)
}

fn usage(out: &mut Output, spec: &CommandSpec) {
    match spec.aliases.is_empty() {
        true => out.print(format!("§e{}:", spec.name)),
        false => out.translate("§e%commands.help.command.aliases", vec![spec.name.clone(), spec.aliases.join(", ")], true),
    }
    out.print(format!("§e{}", spec.description));
    out.translate("commands.generic.usage.noparam", Vec::new(), true);
    spec.overloads.iter().for_each(|overload| out.print(format!("- {}", line(&spec.name, overload))));
}

fn help_usage(out: &mut Output) {
    out.translate("§e%commands.help.command.aliases", vec!["help".into(), "?".into()], true);
    out.translate("§e%commands.help.description", Vec::new(), true);
    out.translate("commands.generic.usage.noparam", Vec::new(), true);
    help_lines("help").into_iter().for_each(|line| out.print(format!("- {line}")));
}

fn help_lines(name: &str) -> [String; 2] {
    [format!("/{name} <page: int>"), format!("/{name} [command: CommandName]")]
}

/// One overload as BDS writes it: `/time set <time: int>`, optional parameters in brackets.
fn line(name: &str, overload: &Overload) -> String {
    let mut words = vec![format!("/{name}")];
    for parameter in &overload.parameters {
        let kind = match &parameter.kind {
            ParamKind::Enum { values, .. } if values.len() == 1 && !parameter.optional => {
                words.push(values[0].clone());
                continue;
            }
            ParamKind::Enum { name, .. } => name.as_str(),
            ParamKind::Int => "int",
            ParamKind::Float => "float",
            ParamKind::String => "string",
            ParamKind::Text => "message",
            ParamKind::Target => "target",
            ParamKind::Value => "value",
            ParamKind::Position | ParamKind::BlockPosition => "x y z",
            ParamKind::BlockStates => "block states",
            ParamKind::Json => "json",
            ParamKind::FilePath => "filepath",
        };
        let (open, close) = if parameter.optional { ('[', ']') } else { ('<', '>') };
        words.push(format!("{open}{}: {kind}{close}", parameter.name));
    }
    words.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use sapling_api::command::ParameterSpec;

    fn spec(name: &str, aliases: &[&str], parameters: Vec<ParameterSpec>) -> CommandSpec {
        let aliases = aliases.iter().map(|alias| alias.to_string()).collect();
        CommandSpec { name: name.into(), aliases, description: "Does it.".into(), permission: Permission::Operator, overloads: vec![Overload { parameters }] }
    }

    #[test]
    fn overloads_read_as_bds_wrote_them() {
        let lock = ParameterSpec { name: "lock".into(), kind: ParamKind::Enum { name: "Boolean".into(), values: vec!["true".into(), "false".into()] }, optional: true };
        assert_eq!(line(listed_as(&spec("daylock", &["alwaysday"], Vec::new())), &Overload { parameters: vec![lock] }), "/alwaysday [lock: Boolean]");
        let clear = ParameterSpec { name: "clear".into(), kind: ParamKind::Enum { name: "CameraClear".into(), values: vec!["clear".into()] }, optional: false };
        let players = ParameterSpec { name: "players".into(), kind: ParamKind::Target, optional: false };
        assert_eq!(line("camera", &Overload { parameters: vec![players, clear] }), "/camera <players: target> clear");
    }

    #[test]
    fn pages_hold_seven_lines_and_end_at_the_last() {
        let mut out = Output::default();
        let specs: Vec<CommandSpec> = (0..10).map(|n| spec(&format!("c{n}"), &[], Vec::new())).collect();
        page(&mut out, &specs.iter().collect::<Vec<_>>(), 9);
        let messages = out.messages();
        assert_eq!((messages[0].parameters.clone(), messages[1].text.as_str()), (vec!["2".to_owned(), "2".to_owned()], "/c5\n/c6\n/c7\n/c8\n/c9"));
    }
}
