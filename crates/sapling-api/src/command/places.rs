//! Parameters vanilla's commands take: positions as three numbers, each of which may be `~` with
//! an offset from where the sender is or `^` with one along where it faces, and a block's states
//! in brackets.

use super::{Args, ParamKind, Parameter, ParseError};
use crate::{Sender, ShardCtx};

/// One axis of a position: a number, `~` and an offset from the sender's own, or `^` and an offset
/// along where the sender faces (left, up, forward; all three axes or none).
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum Axis {
    At(f32),
    Off(f32),
    Local(f32),
}

impl Axis {
    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        let mark = args.mark();
        let word = args.word()?;
        match word.strip_prefix('^') {
            Some(offset) => offset_of(offset).map(Self::Local),
            None => Self::read(word),
        }
        .ok_or_else(|| args.error_at(mark))
    }

    pub(crate) fn read(word: &str) -> Option<Self> {
        match word.strip_prefix('~') {
            Some(offset) => offset_of(offset).map(Self::Off),
            None => number(word).map(Self::At),
        }
    }

    /// The number, `~` counted from `origin`; `^` too, which only [`Coords::resolve`] turns.
    pub fn from(self, origin: f32) -> f32 {
        match self {
            Self::At(at) => at,
            Self::Off(off) | Self::Local(off) => origin + off,
        }
    }

    fn local(self) -> Option<f32> {
        if let Self::Local(off) = self { Some(off) } else { None }
    }
}

fn number(text: &str) -> Option<f32> {
    text.parse::<f32>().ok().filter(|n| n.is_finite())
}

fn offset_of(text: &str) -> Option<f32> {
    if text.is_empty() { Some(0.0) } else { number(text) }
}

impl Parameter for Axis {
    fn kind() -> ParamKind {
        ParamKind::Value
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        Self::parse(args)
    }
}

fn three(args: &mut Args<'_>) -> Result<[Axis; 3], ParseError> {
    let first = Axis::parse(args)?;
    let mut next = || {
        let mark = args.mark();
        let axis = Axis::parse(args)?;
        if axis.local().is_some() == first.local().is_some() { Ok(axis) } else { Err(args.error_at(mark)) }
    };
    Ok([first, next()?, next()?])
}

/// The offsets of an all-`^` position.
fn local(axes: &[Axis; 3]) -> Option<[f32; 3]> {
    Some([axes[0].local()?, axes[1].local()?, axes[2].local()?])
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

    /// The point for `sender`: `~` from [`ShardCtx::origin`], `^` along where it faces.
    pub fn resolve(&self, ctx: &ShardCtx<'_>, sender: &Sender) -> [f32; 3] {
        match local(&self.0) {
            Some(offsets) => ctx.local_point(sender, offsets),
            None => self.from(ctx.origin(sender)),
        }
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

    /// The block for `sender`, as [`Coords::resolve`] finds a point.
    pub fn resolve(&self, ctx: &ShardCtx<'_>, sender: &Sender) -> [i32; 3] {
        match local(&self.0) {
            Some(offsets) => ctx.local_point(sender, offsets).map(|at| at.floor() as i32),
            None => self.from(ctx.origin(sender)),
        }
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
    fn carets_count_along_where_the_sender_faces() {
        let mut fake = crate::testing::Fake::with(&["Alex"]);
        fake.feet.insert(crate::PlayerId(1), [1.5, 2.0, 3.5]);
        let ctx = ShardCtx::new(&mut fake);
        let alex = Sender::Player(crate::testing::player(1, "Alex"));
        // Yaw 90 faces west (research/captures/bds-execute.txt: `rotated 90 0 run tp @s ^ ^ ^2`).
        let [x, y, z] = parsed::<Coords>("^ ^1 ^2").unwrap().resolve(&ctx, &alex);
        assert!((x + 0.5).abs() < 1e-5 && (y - 3.0).abs() < 1e-5 && (z - 3.5).abs() < 1e-5, "{x} {y} {z}");
        assert_eq!(parsed::<BlockCoords>("^1 ^ ^").unwrap().resolve(&ctx, &alex), [1, 2, 4], "left of west is south");
        assert_eq!(parsed::<BlockCoords>("~ ~ ~1").unwrap().resolve(&ctx, &alex), [1, 2, 4]);
        assert!(parsed::<Coords>("~ ^ ~").is_err() && parsed::<Coords>("^ ^ 3").is_err());
    }

    #[test]
    fn states_read_as_vanilla_writes_them() {
        assert_eq!(parsed::<States>(r#"["torch_facing_direction"="top", "a"=1]"#).unwrap(), States("a=1,torch_facing_direction=top".into()));
        assert!(parsed::<States>("liquid_depth=3").is_err(), "brackets are required");
        assert_eq!(parsed::<States>(r#"["open_bit"=true,"b"=false,"c"="true"]"#).unwrap(), States("b=0,c=true,open_bit=1".into()), "bools as the table keeps them");
        let sender = Sender::Console;
        let mut args = Args::new(r#"["a"=1, "b"=2] destroy"#, 0, &sender, &[]);
        let (states, after) = (States::parse(&mut args).unwrap(), args.word().unwrap());
        assert_eq!((states, after), (States("a=1,b=2".into()), "destroy"), "they end at their bracket");
        let mut args = Args::new("destroy", 0, &sender, &[]);
        assert_eq!(Option::<States>::parse(&mut args), Ok(None), "left out when the next word is not one");
    }
}

