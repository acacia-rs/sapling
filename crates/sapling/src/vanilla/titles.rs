//! `/title` and `/titleraw <target> title|subtitle|actionbar|times|clear|reset`: one SetTitle per
//! target, answered with their names (research/captures/bds-messaging.txt).

use sapling_api::command::{Args, CommandSpec, CommandSyntax, Overload, ParamKind, Parameter, ParameterSpec, ParseError, RawText, Selector, Text};
use sapling_api::{Command, Output, Permission, Sender, ShardCtx, TitleAction};

use super::messaging::resolved;
use super::{no_target, players};

const TEXTS: [&str; 3] = ["title", "subtitle", "actionbar"];

/// What a `/title` line asks for, `T` being its text.
enum Shown<T> {
    Text(usize, T),
    Times([i32; 3]),
    Clear,
    Reset,
}

fn spec<T: Parameter>(name: &str, description: &str) -> CommandSpec {
    let target = ParameterSpec::of::<Selector>("player");
    let texts = ParameterSpec { name: "type".into(), kind: ParamKind::Enum { name: format!("{name}TitleType"), values: TEXTS.map(str::to_owned).to_vec() }, optional: false };
    let times = ["fadeIn", "stay", "fadeOut"].map(ParameterSpec::of::<i32>);
    let overloads = [
        vec![target.clone(), ParameterSpec::literal(name, "clear")],
        vec![target.clone(), ParameterSpec::literal(name, "reset")],
        vec![target.clone(), texts, ParameterSpec::of::<T>("titleText")],
        [target, ParameterSpec::literal(name, "times")].into_iter().chain(times).collect(),
    ];
    CommandSpec { name: name.into(), aliases: Vec::new(), description: description.into(), permission: Permission::Operator, overloads: overloads.map(|parameters| Overload { parameters }).to_vec() }
}

fn parse<T: Parameter>(args: &mut Args<'_>) -> Result<(Selector, Shown<T>), ParseError> {
    let target = Selector::parse(args)?;
    let mark = args.mark();
    let word = args.word()?.to_ascii_lowercase();
    let shown = match word.as_str() {
        "clear" => Shown::Clear,
        "reset" => Shown::Reset,
        "times" => Shown::Times([i32::parse(args)?, i32::parse(args)?, i32::parse(args)?]),
        _ => match TEXTS.iter().position(|text| *text == word) {
            Some(which) => Shown::Text(which, T::parse(args)?),
            None => return Err(args.error_at(mark)),
        },
    };
    Ok((target, shown))
}

/// Does `action` to each target and names them all in the answer under `key`.
fn show(ctx: &mut ShardCtx<'_>, sender: &Sender, target: &Selector, out: &mut Output, key: &str, action: &TitleAction) {
    let found = players(ctx, sender, target);
    if found.is_empty() {
        return no_target(out);
    }
    found.iter().for_each(|player| ctx.title_action(player.id, action));
    let names = found.iter().map(|player| player.name.as_str()).collect::<Vec<_>>().join(", ");
    out.translate(key, vec![names], true);
    out.done_to(found.len() as u32);
}

fn plain<T>(shown: Shown<T>, text: impl FnOnce(usize, T) -> TitleAction) -> TitleAction {
    match shown {
        Shown::Text(which, written) => text(which, written),
        Shown::Times([fade_in, stay, fade_out]) => TitleAction::Times { fade_in, stay, fade_out },
        Shown::Clear => TitleAction::Clear,
        Shown::Reset => TitleAction::Reset,
    }
}

pub(crate) struct Title {
    target: Selector,
    shown: Shown<Option<Text>>,
}

impl CommandSyntax for Title {
    fn spec() -> CommandSpec {
        spec::<Text>("title", "Controls screen titles")
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        parse(args).map(|(target, shown)| Self { target, shown })
    }
}

impl Command for Title {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        if matches!(self.shown, Shown::Text(_, None)) {
            return out.translate("commands.generic.invalidMessage", Vec::new(), false);
        }
        let action = plain(self.shown, |which, text| {
            let text = text.map(|text| text.0).unwrap_or_default();
            [TitleAction::Title, TitleAction::Subtitle, TitleAction::ActionBar][which](text)
        });
        show(ctx, sender, &self.target, out, "commands.title.success", &action);
    }
}

pub(crate) struct TitleRaw {
    target: Selector,
    shown: Shown<RawText>,
}

impl CommandSyntax for TitleRaw {
    fn spec() -> CommandSpec {
        spec::<RawText>("titleraw", "Controls screen titles with JSON messages")
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        parse(args).map(|(target, shown)| Self { target, shown })
    }
}

impl Command for TitleRaw {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let shown = match self.shown {
            Shown::Text(which, text) => Shown::Text(which, resolved(ctx, sender, &text)),
            other => other,
        };
        let action = plain(shown, |which, text| [TitleAction::RawTitle, TitleAction::RawSubtitle, TitleAction::RawActionBar][which](text));
        show(ctx, sender, &self.target, out, "commands.titleraw.success", &action);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn syntax(before: &str, token: &str, after: &str) -> Option<ParseError> {
        Some(ParseError::Syntax { before: before.into(), token: token.into(), after: after.into() })
    }

    #[test]
    fn title_lines_read_as_bds_read_them() {
        let sender = Sender::Console;
        let title = |line: &str| Title::parse(&mut Args::new(line, 5, &sender, &[])).err();
        let raw = |line: &str| TitleRaw::parse(&mut Args::new(line, 8, &sender, &[])).map(|raw| raw.shown).ok();
        assert!(matches!(Title::parse(&mut Args::new("title @a subtitle Sub two", 5, &sender, &[])).map(|t| t.shown), Ok(Shown::Text(1, Some(Text(text)))) if text == "Sub two"));
        assert!(matches!(Title::parse(&mut Args::new("title @s title", 5, &sender, &[])).map(|t| t.shown), Ok(Shown::Text(0, None))), "answered at run");
        assert!(matches!(Title::parse(&mut Args::new("title @a times -1 2 3", 5, &sender, &[])).map(|t| t.shown), Ok(Shown::Times([-1, 2, 3]))));
        assert_eq!(title("title @a times 1 2"), syntax(" times 1 2", "", ""), "BDS's answer");
        assert_eq!(title("title @a banana x"), syntax("/title @a ", "banana", " x"));
        assert!(matches!(raw(r#"titleraw @a actionbar {"rawtext":[]}"#), Some(Shown::Text(2, RawText(parts))) if parts.is_empty()));
        assert!(matches!(raw("titleraw @a reset"), Some(Shown::Reset)) && matches!(raw("titleraw @a CLEAR"), Some(Shown::Clear)));
        assert_eq!(TitleRaw::parse(&mut Args::new("titleraw @a title notjson", 8, &sender, &[])).err(), syntax(" @a title ", "notjson", ""));
    }
}
