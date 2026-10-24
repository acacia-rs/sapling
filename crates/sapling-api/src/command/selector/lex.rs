//! Splitting a selector's brackets into `key=value` pairs, keeping braces, brackets and quotes
//! whole. Every piece keeps the byte of the line it starts at, for the error that names it.

/// Bytes `start..end` of the command line: what an error points at.
pub(super) type Span = (usize, usize);

/// One `key=value`; `value` is without its `!` and quotes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Pair<'a> {
    pub key: &'a str,
    pub key_at: usize,
    pub value: &'a str,
    pub value_at: usize,
    pub negated: bool,
}

impl Pair<'_> {
    pub fn key_span(&self) -> Span {
        (self.key_at, self.key_at + self.key.len())
    }

    pub fn value_span(&self) -> Span {
        (self.value_at, self.value_at + self.value.len())
    }
}

/// `text` (starting at byte `at` of the line) trimmed, and where what is left starts.
fn trimmed(text: &str, at: usize) -> (&str, usize) {
    let start = text.len() - text.trim_start().len();
    (text.trim(), at + start)
}

/// The pieces of `text` between its top-level commas, trimmed: none for a blank `text`.
pub(super) fn split(text: &str, at: usize) -> Result<Vec<(&str, usize)>, Span> {
    let (mut pieces, mut depth, mut quoted, mut start) = (Vec::new(), 0i32, false, 0);
    for (i, c) in text.char_indices() {
        match c {
            '"' => quoted = !quoted,
            '[' | '{' if !quoted => depth += 1,
            ']' | '}' if !quoted => depth -= 1,
            ',' if !quoted && depth == 0 => {
                pieces.push(trimmed(&text[start..i], at + start));
                start = i + 1;
            }
            _ => {}
        }
        if depth < 0 {
            return Err((at + i, at + i + 1));
        }
    }
    if quoted || depth != 0 {
        return Err((at + text.len(), at + text.len()));
    }
    if !pieces.is_empty() || !text.trim().is_empty() {
        pieces.push(trimmed(&text[start..], at + start));
    }
    match pieces.iter().find(|(piece, _)| piece.is_empty()) {
        Some(&(_, empty)) => Err((empty, empty)),
        None => Ok(pieces),
    }
}

/// The `key=value` pairs of `text`, the inside of brackets or braces starting at byte `at`.
pub(super) fn pairs(text: &str, at: usize) -> Result<Vec<Pair<'_>>, Span> {
    split(text, at)?.into_iter().map(|(piece, at)| pair(piece, at)).collect()
}

fn pair(piece: &str, at: usize) -> Result<Pair<'_>, Span> {
    let Some(equals) = piece.find('=') else { return Err((at, at + piece.len())) };
    let key = piece[..equals].trim_end();
    let (value, mut value_at) = trimmed(&piece[equals + 1..], at + equals + 1);
    let (negated, value) = match value.strip_prefix('!') {
        Some(rest) => {
            let (rest, rest_at) = trimmed(rest, value_at + 1);
            value_at = rest_at;
            (true, rest)
        }
        None => (false, value),
    };
    let value = match value.strip_prefix('"').and_then(|inner| inner.strip_suffix('"')) {
        Some(inner) => {
            value_at += 1;
            inner
        }
        None => value,
    };
    if key.is_empty() {
        return Err((at, at + equals + 1));
    }
    Ok(Pair { key, key_at: at, value, value_at, negated })
}

/// The inside of `{...}` (or `[...]` with `open` `[`), and where it starts; `None` when `value`
/// is not wrapped so.
pub(super) fn inside(value: &str, at: usize, open: char) -> Option<(&str, usize)> {
    let close = if open == '{' { '}' } else { ']' };
    value.strip_prefix(open).and_then(|inner| inner.strip_suffix(close)).map(|inner| (inner, at + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pairs_keep_nested_values_whole_and_know_where_they_are() {
        let text = r#"type=pig, name=!"a, b" ,scores={x=1..,y=!2},hasitem=[{item=a},{item=b}]"#;
        let found = pairs(text, 10).unwrap();
        let keys: Vec<_> = found.iter().map(|pair| (pair.key, pair.value, pair.negated)).collect();
        assert_eq!(keys, [("type", "pig", false), ("name", "a, b", true), ("scores", "{x=1..,y=!2}", false), ("hasitem", "[{item=a},{item=b}]", false)]);
        assert_eq!((found[1].key_span(), found[1].value_span()), ((20, 24), (27, 31)));
        assert_eq!(&text[found[1].value_at - 10..found[1].value_at - 10 + 4], "a, b");
        assert_eq!(pairs("", 0), Ok(Vec::new()));
        assert_eq!(pairs("  ", 0), Ok(Vec::new()));
    }

    #[test]
    fn broken_pairs_point_at_what_is_wrong() {
        assert_eq!(pairs("type=pig,,c=1", 0), Err((9, 9)), "an empty pair");
        assert_eq!(pairs("type=pig,", 0), Err((9, 9)), "a trailing comma");
        assert_eq!(pairs("type", 0), Err((0, 4)), "no value");
        assert_eq!(pairs("=pig", 0), Err((0, 1)), "no key");
        assert_eq!(pairs("scores={x=1", 0), Err((11, 11)), "an open brace");
        assert_eq!(pairs("name=\"ab", 0), Err((8, 8)), "an open quote");
        assert_eq!(inside("{a=1}", 7, '{'), Some(("a=1", 8)));
        assert_eq!(inside("a=1", 7, '{'), None);
    }
}
