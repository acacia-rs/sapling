//! Parameters vanilla's commands take: positions as three numbers, each of which may be `~` with
//! an offset from where the sender is; a block's states in brackets; and a selector of players or
//! entities (`@s`, `@p`, `@a`, `@e[type=minecraft:pig]`, a player's name).

use super::{Args, ParamKind, Parameter, ParseError};

/// One axis of a position: a number, or `~` and an offset from the sender's own.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Axis {
    At(f32),
    Off(f32),
}

impl Axis {
    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        let mark = args.mark();
        let word = args.word()?;
        let number = |text: &str| if text.is_empty() { Some(0.0) } else { text.parse::<f32>().ok().filter(|n| n.is_finite()) };
        let axis = match word.strip_prefix('~') {
            Some(offset) => number(offset).map(Self::Off),
            None => word.parse::<f32>().ok().filter(|n| n.is_finite()).map(Self::At),
        };
        axis.ok_or_else(|| args.error_at(mark))
    }

    fn from(self, origin: f32) -> f32 {
        match self {
            Self::At(at) => at,
            Self::Off(off) => origin + off,
        }
    }
}

fn three(args: &mut Args<'_>) -> Result<[Axis; 3], ParseError> {
    Ok([Axis::parse(args)?, Axis::parse(args)?, Axis::parse(args)?])
}

/// A point: `x y z`, where a whole number is the middle of its block across (as vanilla reads
/// `/tp 3 64 3`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Coords(pub [Axis; 3]);

impl Coords {
    /// The point, `~` counted from `origin`.
    pub fn from(&self, origin: [f32; 3]) -> [f32; 3] {
        std::array::from_fn(|axis| match self.0[axis] {
            Axis::At(at) if axis != 1 && at.fract() == 0.0 => at + 0.5,
            other => other.from(origin[axis]),
        })
    }
}

impl Parameter for Coords {
    fn kind() -> ParamKind {
        ParamKind::Position
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        three(args).map(Self)
    }
}

/// A block: `x y z`, `~` counted from the block the sender stands in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlockCoords(pub [Axis; 3]);

impl BlockCoords {
    pub fn from(&self, origin: [f32; 3]) -> [i32; 3] {
        std::array::from_fn(|axis| self.0[axis].from(origin[axis].floor()).floor() as i32)
    }
}

impl Parameter for BlockCoords {
    fn kind() -> ParamKind {
        ParamKind::BlockPosition
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        three(args).map(Self)
    }
}

/// A block's states as typed, `["liquid_depth"=3,"direction"=1]`, in the registry's own form
/// (`direction=1,liquid_depth=3`, quotes gone, sorted by name).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct States(pub String);

impl Parameter for States {
    fn kind() -> ParamKind {
        ParamKind::BlockStates
    }

    fn begins(next: &str) -> bool {
        next.starts_with('[')
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        let mark = args.mark();
        let bracketed = args.bracketed()?;
        let inner = bracketed.strip_prefix('[').and_then(|inner| inner.strip_suffix(']')).ok_or_else(|| args.error_at(mark))?;
        let mut pairs: Vec<String> = inner
            .split(',')
            .filter(|pair| !pair.trim().is_empty())
            .map(|pair| pair.split_once('=').map(|(key, value)| format!("{}={}", key.trim().trim_matches('"'), state_value(value.trim()))))
            .collect::<Option<_>>()
            .ok_or_else(|| args.error_at(mark))?;
        pairs.sort();
        Ok(Self(pairs.join(",")))
    }
}

/// A state's value as the block table stores it: an unquoted `true` or `false` is a bool state,
/// kept as 1 or 0.
fn state_value(value: &str) -> &str {
    match value {
        "true" => "1",
        "false" => "0",
        _ => value.trim_matches('"'),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Sender;

    fn parsed<P: Parameter>(line: &str) -> Result<P, ParseError> {
        let sender = Sender::Console;
        P::parse(&mut Args::new(line, 0, &sender, &[]))
    }

    #[test]
    fn positions_count_tildes_from_the_sender_and_centre_whole_numbers() {
        let here = [10.25, 64.0, -3.75];
        assert_eq!(parsed::<Coords>("~ ~1 ~-0.25").unwrap().from(here), [10.25, 65.0, -4.0]);
        assert_eq!(parsed::<Coords>("3 64 -2").unwrap().from(here), [3.5, 64.0, -1.5]);
        assert_eq!(parsed::<BlockCoords>("~-4 ~-1 ~2").unwrap().from(here), [6, 63, -2]);
        assert!(parsed::<Coords>("1 2").is_err() && parsed::<Coords>("1 two 3").is_err());
    }

    #[test]
    fn states_and_selectors_read_as_vanilla_writes_them() {
        assert_eq!(parsed::<States>(r#"["torch_facing_direction"="top", "a"=1]"#).unwrap(), States("a=1,torch_facing_direction=top".into()));
        assert!(parsed::<States>("liquid_depth=3").is_err(), "brackets are required");
        assert_eq!(parsed::<States>(r#"["open_bit"=true,"b"=false,"c"="true"]"#).unwrap(), States("b=0,c=true,open_bit=1".into()), "bools as the table keeps them");
        let sender = Sender::Console;
        let mut args = Args::new(r#"["a"=1, "b"=2] destroy"#, 0, &sender, &[]);
        let (states, after) = (States::parse(&mut args).unwrap(), args.word().unwrap());
        assert_eq!((states, after), (States("a=1,b=2".into()), "destroy"), "they end at their bracket");
        let mut args = Args::new("destroy", 0, &sender, &[]);
        assert_eq!(Option::<States>::parse(&mut args), Ok(None), "left out when the next word is not one");
        assert_eq!(parsed::<Selector>("@e[type=chicken]").unwrap(), Selector::Entities { kind: Some("minecraft:chicken".into()) });
        assert_eq!([parsed::<Selector>("@s").unwrap(), parsed::<Selector>("Alex").unwrap()], [Selector::Sender, Selector::Named("Alex".into())]);
        assert!(parsed::<Selector>("@x").is_err());
    }
}

/// Who or what a command is aimed at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selector {
    /// `@s`: the sender.
    Sender,
    /// `@p`: the player nearest the sender.
    Nearest,
    /// `@a`: every player.
    Players,
    /// `@e`, optionally of one kind (`type=`, namespaced or not).
    Entities { kind: Option<String> },
    /// A player by name.
    Named(String),
}

impl Parameter for Selector {
    fn kind() -> ParamKind {
        ParamKind::Target
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        let mark = args.mark();
        let word = args.word()?;
        let Some(selector) = word.strip_prefix('@') else { return Ok(Self::Named(word.to_owned())) };
        let (base, filters) = selector.split_once('[').map_or((selector, ""), |(base, filters)| (base, filters.trim_end_matches(']')));
        let filter = |key: &str| filters.split(',').filter_map(|pair| pair.split_once('=')).find(|(name, _)| name.trim() == key).map(|(_, value)| value.trim().trim_matches('"').to_owned());
        let namespaced = |kind: String| if kind.contains(':') { kind } else { format!("minecraft:{kind}") };
        Ok(match base {
            "s" => Self::Sender,
            "p" => Self::Nearest,
            "a" => Self::Players,
            "e" => Self::Entities { kind: filter("type").map(namespaced) },
            _ => return Err(args.error_at(mark)),
        })
    }
}
