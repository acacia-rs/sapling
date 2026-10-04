use super::{Args, ParamKind, ParseError};
use crate::{GameMode, Player};

/// A type a command field can have. `#[derive(CommandEnum)]` writes it for an enum of words.
pub trait Parameter: Sized {
    /// Whether the argument may be left out; only `Option` says yes.
    const OPTIONAL: bool = false;

    fn kind() -> ParamKind;
    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError>;
}

macro_rules! number {
    ($kind:ident: $($number:ty),*) => {$(
        impl Parameter for $number {
            fn kind() -> ParamKind {
                ParamKind::$kind
            }

            fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
                let mark = args.mark();
                args.word()?.parse().map_err(|_| args.error_at(mark))
            }
        }
    )*};
}
number!(Int: i8, i16, i32, i64, u8, u16, u32, u64, usize);

macro_rules! float {
    ($($float:ty),*) => {$(
        impl Parameter for $float {
            fn kind() -> ParamKind {
                ParamKind::Float
            }

            fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
                let mark = args.mark();
                args.word()?.parse().ok().filter(|f: &$float| f.is_finite()).ok_or_else(|| args.error_at(mark))
            }
        }
    )*};
}
float!(f32, f64);

/// The words of an enum parameter and what each stands for; the parser `#[derive(CommandEnum)]` uses.
pub fn parse_choice<T: Clone>(args: &mut Args<'_>, choices: &[(&str, T)]) -> Result<T, ParseError> {
    let mark = args.mark();
    let word = args.word()?;
    choices.iter().find(|(name, _)| name.eq_ignore_ascii_case(word)).map(|(_, value)| value.clone()).ok_or_else(|| args.error_at(mark))
}

fn enum_kind(name: &str, values: &[&str]) -> ParamKind {
    ParamKind::Enum { name: name.to_owned(), values: values.iter().map(|v| (*v).to_owned()).collect() }
}

impl Parameter for bool {
    fn kind() -> ParamKind {
        enum_kind("Boolean", &["true", "false"])
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        parse_choice(args, &[("true", true), ("false", false)])
    }
}

impl Parameter for GameMode {
    fn kind() -> ParamKind {
        enum_kind("GameMode", &["survival", "creative", "adventure", "spectator"])
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        let modes = [("survival", Self::Survival), ("creative", Self::Creative), ("adventure", Self::Adventure), ("spectator", Self::Spectator)];
        parse_choice(args, &modes)
    }
}

impl Parameter for String {
    fn kind() -> ParamKind {
        ParamKind::String
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        args.word().map(str::to_owned)
    }
}

/// Everything to the end of the line; only useful as the last parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Text(pub String);

impl Parameter for Text {
    fn kind() -> ParamKind {
        ParamKind::Text
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        args.rest().map(|rest| Self(rest.to_owned()))
    }
}

/// A player who is online, named by the sender or `@s` for the sender itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target(pub Player);

impl Parameter for Target {
    fn kind() -> ParamKind {
        ParamKind::Target
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        let mark = args.mark();
        let word = args.word()?;
        let found = match word.strip_prefix('@') {
            Some("s") => args.sender().player(),
            // TODO: @a, @p, @r and selector arguments.
            Some(_) => return Err(args.error_at(mark)),
            None => args.players().iter().find(|p| p.name.eq_ignore_ascii_case(word)),
        };
        found.cloned().map(Self).ok_or(ParseError::NoTarget)
    }
}

impl<P: Parameter> Parameter for Option<P> {
    const OPTIONAL: bool = true;

    fn kind() -> ParamKind {
        P::kind()
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        if args.is_empty() { Ok(None) } else { P::parse(args).map(Some) }
    }
}
