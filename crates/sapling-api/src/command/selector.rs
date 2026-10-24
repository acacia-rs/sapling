//! A selector of players or entities as vanilla writes one: `@s`, `@p`, `@r`, `@a`, `@e`, with
//! arguments in brackets (`@e[type=pig,c=2,r=10]`), or a player's name. What it names is the
//! command's to work out (sapling's vanilla commands); this is what was written.
// TODO: dx/dy/dz volumes, levels, scores, tags, families and rotations.

use super::places::Axis;
use super::{Args, ParamKind, Parameter, ParseError};
use crate::GameMode;

/// Who or what a command is aimed at.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Selector {
    /// An `@` selector and what its brackets ask.
    Pick(Pick, Filter),
    /// A player by name.
    Named(String),
}

/// What an `@` selector starts from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Pick {
    /// `@s`: the sender.
    Sender,
    /// `@p`: the player nearest, one unless `c=` says more.
    Nearest,
    /// `@r`: a player at random, one unless `c=` says more.
    Random,
    /// `@a`: every player.
    Players,
    /// `@e`: every entity, players too.
    Entities,
}

/// A selector's arguments. A text kept with a leading `!` names everything but it.
#[derive(Debug, Clone, Default, PartialEq)]
#[non_exhaustive]
pub struct Filter {
    /// `type=`, namespaced.
    pub kind: Option<String>,
    /// `name=`.
    pub name: Option<String>,
    /// `c=`: how many, the nearest first; below 0, the farthest first.
    pub count: Option<i32>,
    /// `rm=` and `r=`: how near and how far, in blocks.
    pub radius: [Option<f32>; 2],
    /// `x=`, `y=` and `z=`: where nearness is counted from, in place of the command's own place.
    pub at: [Option<Axis>; 3],
    /// `m=`, and whether it is the mode wanted (`m=!creative`: every other).
    pub game_mode: Option<(GameMode, bool)>,
}

impl Selector {
    /// `@s` with nothing in brackets.
    pub fn sender() -> Self {
        Self::Pick(Pick::Sender, Filter::default())
    }

    /// Whether an entity of `kind` is one `type=` names (`wanted`, `!` kept).
    pub fn takes(wanted: Option<&str>, kind: &str) -> bool {
        Filter::matches(wanted, kind)
    }
}

impl Filter {
    /// Whether `value` is one `wanted` (`!` kept) names; anything where nothing is asked.
    pub fn matches(wanted: Option<&str>, value: &str) -> bool {
        match wanted.map(|wanted| wanted.strip_prefix('!').map_or((wanted, true), |other| (other, false))) {
            Some((wanted, is_wanted)) => wanted.eq_ignore_ascii_case(value) == is_wanted,
            None => true,
        }
    }

    /// Whether a player in `mode` is one `m=` names.
    pub fn takes_mode(&self, mode: GameMode) -> bool {
        self.game_mode.is_none_or(|(wanted, is_wanted)| (wanted == mode) == is_wanted)
    }

    /// Whether something `distance` blocks off is within `rm=` and `r=`.
    pub fn within(&self, distance: f32) -> bool {
        let [least, most] = self.radius;
        least.is_none_or(|least| distance >= least) && most.is_none_or(|most| distance <= most)
    }
}

fn game_mode(value: &str) -> Option<GameMode> {
    Some(match value.to_ascii_lowercase().as_str() {
        "0" | "s" | "survival" => GameMode::Survival,
        "1" | "c" | "creative" => GameMode::Creative,
        "2" | "a" | "adventure" => GameMode::Adventure,
        "6" | "spectator" => GameMode::Spectator,
        _ => return None,
    })
}

/// The arguments between a selector's brackets; `None` for one it cannot read.
fn filter(arguments: &str) -> Option<Filter> {
    let mut filter = Filter::default();
    for pair in arguments.split(',').filter(|pair| !pair.trim().is_empty()) {
        let (key, value) = pair.split_once('=')?;
        let value = value.trim().trim_matches('"');
        let (not, plain) = value.strip_prefix('!').map_or(("", value), |plain| ("!", plain));
        match key.trim() {
            "type" => filter.kind = Some(format!("{not}{}", if plain.contains(':') { plain.to_owned() } else { format!("minecraft:{plain}") })),
            "name" => filter.name = Some(value.to_owned()),
            "c" => filter.count = Some(value.parse().ok()?),
            "r" => filter.radius[1] = Some(value.parse().ok()?),
            "rm" => filter.radius[0] = Some(value.parse().ok()?),
            "x" => filter.at[0] = Some(Axis::read(value)?),
            "y" => filter.at[1] = Some(Axis::read(value)?),
            "z" => filter.at[2] = Some(Axis::read(value)?),
            "m" => filter.game_mode = Some((game_mode(plain)?, not.is_empty())),
            _ => {}
        }
    }
    Some(filter)
}

impl Parameter for Selector {
    fn kind() -> ParamKind {
        ParamKind::Target
    }

    fn parse(args: &mut Args<'_>) -> Result<Self, ParseError> {
        let mark = args.mark();
        let word = args.word()?;
        let Some(selector) = word.strip_prefix('@') else { return Ok(Self::Named(word.to_owned())) };
        let (base, arguments) = selector.split_once('[').map_or((selector, ""), |(base, arguments)| (base, arguments.trim_end_matches(']')));
        let pick = match base {
            "s" => Pick::Sender,
            "p" => Pick::Nearest,
            "r" => Pick::Random,
            "a" => Pick::Players,
            "e" => Pick::Entities,
            _ => return Err(args.error_at(mark)),
        };
        filter(arguments).map(|filter| Self::Pick(pick, filter)).ok_or_else(|| args.error_at(mark))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Sender;

    fn parsed(line: &str) -> Result<Selector, ParseError> {
        let sender = Sender::Console;
        Selector::parse(&mut Args::new(line, 0, &sender, &[]))
    }

    fn filter_of(line: &str) -> Filter {
        match parsed(line) {
            Ok(Selector::Pick(_, filter)) => filter,
            other => panic!("{line}: {other:?}"),
        }
    }

    #[test]
    fn selectors_read_as_vanilla_writes_them() {
        assert_eq!(filter_of("@e[type=chicken]").kind.as_deref(), Some("minecraft:chicken"));
        assert_eq!(filter_of("@e[type=!player]").kind.as_deref(), Some("!minecraft:player"));
        let takes = |wanted: Option<&str>| [Selector::takes(wanted, "minecraft:player"), Selector::takes(wanted, "minecraft:pig")];
        assert_eq!([takes(None), takes(Some("minecraft:pig")), takes(Some("!minecraft:player"))], [[true, true], [false, true], [false, true]]);
        assert_eq!([parsed("@s").unwrap(), parsed("Alex").unwrap()], [Selector::sender(), Selector::Named("Alex".into())]);
        assert!(parsed("@x").is_err() && parsed("@e[c=two]").is_err() && parsed("@a[m=flying]").is_err());
        let filter = filter_of("@e[type=pig,c=-2,r=10,rm=1,x=~3,name=\"Bob\",m=!creative]");
        assert_eq!((filter.count, filter.radius, filter.at[0], filter.name.as_deref()), (Some(-2), [Some(1.0), Some(10.0)], Some(Axis::Off(3.0)), Some("Bob")));
        assert!(!filter.takes_mode(GameMode::Creative) && filter.takes_mode(GameMode::Survival) && filter.within(5.0) && !filter.within(0.5));
        assert!(matches!(parsed("@r[c=2]"), Ok(Selector::Pick(Pick::Random, Filter { count: Some(2), .. }))));
    }
}
