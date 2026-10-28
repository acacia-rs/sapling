//! The message of `/say`, `/me` and `/tell`: the rest of the line, with each selector in it
//! replaced when the command runs by the names it finds, `, `-joined, or by nothing
//! (research/captures/bds-saysel.txt). A selector is `@` and a base's whole name, `@s@s` being two
//! and `@here` or `mail@example.com` none, with its filters; inside double quotes it stays text. A
//! selector that does not parse is the command's syntax error.

use std::ops::Range;

use sapling_api::command::{Args, ParamKind, Parameter, ParseError, Selector};
use sapling_api::{Sender, ShardCtx};

use super::{display_name, targets};

const BASES: [&str; 7] = ["s", "p", "a", "r", "e", "n", "initiator"];

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Message {
    text: String,
    selectors: Vec<(Range<usize>, Selector)>,
}

impl Message {
    /// The text as `sender` sends it, its selectors replaced.
    pub(super) fn resolve(&self, ctx: &mut ShardCtx<'_>, sender: &Sender) -> String {
        let (mut out, mut from) = (String::new(), 0);
        for (span, selector) in &self.selectors {
            out.push_str(&self.text[from..span.start]);
            out.push_str(&names(ctx, sender, selector).join(", "));
            from = span.end;
        }
        out.push_str(&self.text[from..]);
        out
    }
}

/// Who `selector` names for `sender`, players first, as BDS writes them into text.
pub(super) fn names(ctx: &mut ShardCtx<'_>, sender: &Sender, selector: &Selector) -> Vec<String> {
    let (players, entities) = targets(ctx, sender, Some(selector));
    players.into_iter().map(|player| player.name).chain(entities.iter().map(display_name)).collect()
}

impl Parameter for Message {
    fn kind() -> ParamKind {
        ParamKind::Text
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        let start = args.mark();
        let text = args.rest()?;
        let mut selectors = Vec::new();
        for at in selector_starts(text) {
            let end = selector_end(text, at);
            let selector = parse_selector(args, start + at, start + end)?;
            selectors.push((at..end, selector));
        }
        Ok(Self { text: text.to_owned(), selectors })
    }
}

/// Where each selector of `text` starts: an `@` outside quotes followed by a base's whole name.
fn selector_starts(text: &str) -> Vec<usize> {
    let mut quoted = false;
    let mut starts = Vec::new();
    for (at, c) in text.char_indices() {
        match c {
            '"' => quoted = !quoted,
            '@' if !quoted && BASES.contains(&base_name(&text[at + 1..])) => starts.push(at),
            _ => {}
        }
    }
    starts
}

fn base_name(after: &str) -> &str {
    &after[..after.find(|c: char| !c.is_ascii_alphabetic()).unwrap_or(after.len())]
}

/// Where the selector starting at `at` ends: after its base, or after its filters' `]`; an
/// unclosed `[` runs to the end.
fn selector_end(text: &str, at: usize) -> usize {
    let base_end = at + 1 + base_name(&text[at + 1..]).len();
    if !text[base_end..].starts_with('[') {
        return base_end;
    }
    let (mut depth, mut quoted) = (0, false);
    for (offset, c) in text[base_end..].char_indices() {
        match c {
            '"' => quoted = !quoted,
            '[' if !quoted => depth += 1,
            ']' if !quoted => {
                depth -= 1;
                if depth == 0 {
                    return base_end + offset + 1;
                }
            }
            _ => {}
        }
    }
    text.len()
}

/// The selector at `start..end` of the line, parsed where it stands so errors point into the line.
// The selector parser reads to the next space: what follows it is blanked, keeping offsets.
fn parse_selector(args: &Args<'_>, start: usize, end: usize) -> Result<Selector, ParseError> {
    let line = args.line();
    let next = line[end..].chars().next().map_or(0, char::len_utf8);
    let blanked = format!("{}{}{}", &line[..end], " ".repeat(next), &line[end + next..]);
    let mut own = Args::new(&blanked, start, args.sender(), args.players());
    Selector::parse(&mut own)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spans(text: &str) -> Vec<&str> {
        selector_starts(text).into_iter().map(|at| &text[at..selector_end(text, at)]).collect()
    }

    #[test]
    fn selectors_are_found_as_bds_resolved_them() {
        assert_eq!(spans("hi @s, @a."), ["@s", "@a"]);
        assert_eq!(spans("@s@s x a@s"), ["@s", "@s", "@s"]);
        assert_eq!(spans("mail@example.com @here @ @x"), Vec::<&str>::new());
        assert_eq!(spans(r#"hi "@s" @e[type=cow,name="a]b"] @initiator"#), [r#"@e[type=cow,name="a]b"]"#, "@initiator"]);
        assert_eq!(spans("hi @s[ y"), ["@s[ y"]);
    }

    #[test]
    fn a_broken_selector_is_the_lines_syntax_error() {
        let sender = Sender::Console;
        let parse = |line: &str| Message::parse(&mut Args::new(line, 4, &sender, &[]));
        let syntax = |before: &str, token: &str, after: &str| ParseError::Syntax { before: before.into(), token: token.into(), after: after.into() };
        assert_eq!(parse("say hi @s[ y"), Err(syntax("y hi @s[ y", "", "")));
        assert_eq!(parse("say hi @s[bad=1] y"), Err(syntax("say hi @s[", "bad", "=1] y")));
        assert_eq!(parse("say a@s b").map(|message| message.selectors.len()), Ok(1));
    }
}
