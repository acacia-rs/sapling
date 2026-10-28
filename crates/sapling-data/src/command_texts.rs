//! The English a command's answer reads as on BDS's console (`data/command-texts.tsv`, README
//! "Command texts"): en_US.lang's `commands.*` lines. The file is made from your own BDS, so a
//! build without it has none, and an answer then reads as its key and parameters.

use std::collections::HashMap;
use std::sync::LazyLock;

static TEXTS: LazyLock<HashMap<&'static str, &'static str>> =
    LazyLock::new(|| include_str!(concat!(env!("OUT_DIR"), "/command-texts.tsv")).lines().filter_map(|line| line.split_once('\t')).collect());

/// `text` (a key, a key behind colour codes and `%`, or a pattern of its own) in English with
/// `parameters` put in, as BDS's console printed it.
pub fn command_text(text: &str, parameters: &[String]) -> String {
    let mut rest = text;
    while let Some(code) = rest.strip_prefix('§') {
        let mut chars = code.chars();
        chars.next();
        rest = chars.as_str();
    }
    let prefix = &text[..text.len() - rest.len()];
    let key = rest.strip_prefix('%').unwrap_or(rest);
    match TEXTS.get(key) {
        Some(pattern) => format!("{prefix}{}", fill(pattern, parameters)),
        None if rest.contains('%') => format!("{prefix}{}", fill(&rest.replace("%%", "%"), parameters)),
        None if parameters.is_empty() => text.to_owned(),
        None => format!("{text} {}", parameters.join(" ")),
    }
}

/// `pattern` with `%s`/`%d` taking the parameters in turn and `%1$s`/`%1$d` the one named.
fn fill(pattern: &str, parameters: &[String]) -> String {
    let (mut out, mut next, mut rest) = (String::new(), 0, pattern);
    while let Some(at) = rest.find('%') {
        out.push_str(&rest[..at]);
        let spec = &rest[at + 1..];
        let numbered = spec.find('$').filter(|end| *end > 0 && spec[..*end].bytes().all(|b| b.is_ascii_digit()));
        let (index, after) = match numbered {
            Some(end) => (spec[..end].parse::<usize>().ok().map(|n| n.saturating_sub(1)), &spec[end + 1..]),
            None => (Some(next), spec),
        };
        match after.chars().next() {
            Some(kind @ ('s' | 'd')) => {
                out.push_str(index.and_then(|index| parameters.get(index)).map_or("", String::as_str));
                next += usize::from(numbered.is_none());
                rest = &after[kind.len_utf8()..];
            }
            _ => {
                out.push('%');
                rest = spec;
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn patterns_take_their_parameters_in_turn_or_by_number() {
        assert_eq!(fill("Set max players to %1$d.", &strings(&["5"])), "Set max players to 5.");
        assert_eq!(fill("Syntax error: Unexpected \"%2$s\": at \"%1$s>>%2$s<<%3$s\"", &strings(&["/help ", "seed", ""])), "Syntax error: Unexpected \"seed\": at \"/help >>seed<<\"");
        assert_eq!(fill("Unknown command: %s. Please", &strings(&["seed"])), "Unknown command: seed. Please");
        assert_eq!(fill("100% %s", &strings(&["sure"])), "100% sure");
    }

    #[test]
    fn a_text_without_a_known_key_reads_as_itself() {
        assert_eq!(command_text("%%s, %%s", &strings(&["Fake", "Alice"])), "Fake, Alice");
        assert_eq!(command_text("no.such.key", &strings(&["a", "b"])), "no.such.key a b");
        assert_eq!(command_text("plain", &[]), "plain");
    }
}
