use super::Sender;
use crate::Player;

/// Why a command line was not run. The shapes are vanilla's, so the client words them itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// `token` is not what belongs there; `before` and `after` are the line around it. An empty
    /// token at the end of the line means an argument is missing.
    Syntax { before: String, token: String, after: String },
    /// A player argument named nobody who is online.
    NoTarget,
}

/// The rest of a command line, consumed one argument at a time by [`super::Parameter::parse`].
pub struct Args<'a> {
    line: &'a str,
    pos: usize,
    sender: &'a Sender,
    players: &'a [Player],
}

impl<'a> Args<'a> {
    /// `line` is the whole command without its `/`; parsing starts at byte `start` (after the name).
    pub fn new(line: &'a str, start: usize, sender: &'a Sender, players: &'a [Player]) -> Self {
        let mut args = Self { line, pos: start.min(line.len()), sender, players };
        args.skip_spaces();
        args
    }

    pub fn sender(&self) -> &'a Sender {
        self.sender
    }

    /// The players a [`super::Target`] can name.
    pub fn players(&self) -> &'a [Player] {
        self.players
    }

    pub fn is_empty(&self) -> bool {
        self.pos >= self.line.len()
    }

    /// Where the next argument starts, for [`Args::error_at`].
    pub fn mark(&self) -> usize {
        self.pos
    }

    fn skip_spaces(&mut self) {
        let rest = &self.line[self.pos..];
        self.pos += rest.len() - rest.trim_start().len();
    }

    /// The next word, or the next quoted run of words without its quotes.
    pub fn word(&mut self) -> Result<&'a str, ParseError> {
        let start = self.pos;
        let rest = &self.line[start..];
        if rest.is_empty() {
            return Err(self.error_at(start));
        }
        let (word, taken) = match rest.strip_prefix('"') {
            Some(quoted) => {
                let end = quoted.find('"').ok_or_else(|| self.error_at(start))?;
                (&quoted[..end], end + 2)
            }
            None => {
                let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
                (&rest[..end], end)
            }
        };
        self.pos = start + taken;
        self.skip_spaces();
        Ok(word)
    }

    /// The next word without taking it.
    pub fn peek(&self) -> Option<&'a str> {
        let rest = &self.line[self.pos..];
        rest.split_whitespace().next()
    }

    /// The next run from `[` to its `]`, spaces and all; to the end of the line when it has none.
    pub fn bracketed(&mut self) -> Result<&'a str, ParseError> {
        let start = self.pos;
        let rest = self.line[start..].trim_end();
        if rest.is_empty() {
            return Err(self.error_at(start));
        }
        let end = rest.find(']').map_or(rest.len(), |at| at + 1);
        self.pos = start + end;
        self.skip_spaces();
        Ok(&rest[..end])
    }

    /// Everything left, as typed.
    pub fn rest(&mut self) -> Result<&'a str, ParseError> {
        let start = self.pos;
        let rest = self.line[start..].trim_end();
        if rest.is_empty() {
            return Err(self.error_at(start));
        }
        self.pos = self.line.len();
        Ok(rest)
    }

    /// The error for the argument that started at `mark`, as BDS reports one: `/gamemode banana`
    /// is `("/gamemode ", "banana", "")`, a missing argument `("/gamemode", "", "")`. BDS keeps
    /// only the last ten characters before it: `/gamerule randomtickspeed true` is `("tickspeed ", "true", "")`.
    pub fn error_at(&self, mark: usize) -> ParseError {
        const BEFORE_KEPT: usize = 10;
        let rest = &self.line[mark..];
        let end = mark + rest.find(char::is_whitespace).unwrap_or(rest.len());
        let before = format!("/{}", if rest.is_empty() { self.line.trim_end() } else { &self.line[..mark] });
        let cut = before.char_indices().rev().nth(BEFORE_KEPT - 1).map_or(0, |(at, _)| at);
        ParseError::Syntax { before: before[cut..].to_owned(), token: self.line[mark..end].to_owned(), after: self.line[end..].to_owned() }
    }

    /// An error if arguments are left over.
    pub fn finish(&self) -> Result<(), ParseError> {
        if self.is_empty() { Ok(()) } else { Err(self.error_at(self.pos)) }
    }
}
