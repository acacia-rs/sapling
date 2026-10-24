//! What a selector's brackets may say, as read from them: [`Filters`]. Wrong keys and values are
//! syntax errors pointing at the key or value, as BDS's are.

use super::items::ItemTest;
use super::lex::{self, Pair, Span};
use crate::GameMode;
use crate::command::Axis;

/// A value or, with `!`, anything but it.
#[derive(Debug, Clone, PartialEq)]
pub struct Not<T> {
    pub value: T,
    pub negated: bool,
}

impl<T> Not<T> {
    /// Whether `holds`, what the value says of a candidate, passes.
    pub fn passes(&self, holds: bool) -> bool {
        holds != self.negated
    }
}

/// `min..max`, either end open: `3`, `..5`, `3..`, `1..4`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Bounds<T> {
    pub min: Option<T>,
    pub max: Option<T>,
}

impl<T: PartialOrd + Copy> Bounds<T> {
    pub fn holds(&self, value: T) -> bool {
        self.min.is_none_or(|min| value >= min) && self.max.is_none_or(|max| value <= max)
    }

    pub fn is_open(&self) -> bool {
        self.min.is_none() && self.max.is_none()
    }

    /// `3`, `..5`, `3..` or `1..4`; `None` for anything else.
    pub(super) fn read(text: &str) -> Option<Self>
    where
        T: std::str::FromStr,
    {
        let number = |text: &str| text.trim().parse::<T>().ok();
        let bound = |text: &str| if text.trim().is_empty() { Some(None) } else { number(text).map(Some) };
        let bounds = match text.split_once("..") {
            Some((min, max)) => Self { min: bound(min)?, max: bound(max)? },
            None => Self { min: Some(number(text)?), max: Some(number(text)?) },
        };
        (!bounds.is_open()).then_some(bounds)
    }
}

/// An input `/inputpermission` turns on and off, as `haspermission` names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input {
    Camera,
    Movement,
    LateralMovement,
    Sneak,
    Jump,
    Mount,
    Dismount,
    MoveForward,
    MoveBackward,
    MoveLeft,
    MoveRight,
}

const INPUTS: [(&str, Input); 11] = [
    ("camera", Input::Camera),
    ("movement", Input::Movement),
    ("lateral_movement", Input::LateralMovement),
    ("sneak", Input::Sneak),
    ("jump", Input::Jump),
    ("mount", Input::Mount),
    ("dismount", Input::Dismount),
    ("move_forward", Input::MoveForward),
    ("move_backward", Input::MoveBackward),
    ("move_left", Input::MoveLeft),
    ("move_right", Input::MoveRight),
];

/// What a selector's brackets ask of a candidate. Each test of a list must pass: `tag=a,tag=b`
/// is both tags.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Filters {
    /// `x`, `y`, `z`: where distances and volumes count from, `~` from the sender.
    pub origin: [Option<Axis>; 3],
    /// `dx`, `dy`, `dz`: the box from the origin, the axes not given 0.
    pub volume: [Option<f32>; 3],
    /// `rm` and `r`.
    pub distance: Bounds<f32>,
    /// `c`: how many, the nearest first; less than 0, the farthest.
    pub count: Option<i32>,
    pub names: Vec<Not<String>>,
    /// Namespaced.
    pub types: Vec<Not<String>>,
    pub families: Vec<Not<String>>,
    pub tags: Vec<Not<String>>,
    /// `m`; `None` stands for `default`.
    pub modes: Vec<Not<Option<GameMode>>>,
    /// `lm` and `l`: experience level.
    pub level: Bounds<i32>,
    /// `rxm` and `rx`: pitch, -90 up to 90 down.
    pub pitch: Bounds<f32>,
    /// `rym` and `ry`: yaw, -180 to 180.
    pub yaw: Bounds<f32>,
    pub scores: Vec<(String, Not<Bounds<i32>>)>,
    pub items: Vec<ItemTest>,
    pub permissions: Vec<(Input, bool)>,
}

/// Keys that may not come twice.
const ONCE: [&str; 16] = ["x", "y", "z", "dx", "dy", "dz", "r", "rm", "c", "l", "lm", "rx", "rxm", "ry", "rym", "haspermission"];
/// Keys whose value may start with `!`.
const NEGATABLE: [&str; 5] = ["name", "type", "family", "tag", "m"];

impl Filters {
    /// The filters of `text`, the inside of a selector's brackets, which starts at byte `at`.
    pub(super) fn read(text: &str, at: usize) -> Result<Self, Span> {
        let mut filters = Self::default();
        let mut seen = Vec::new();
        for pair in lex::pairs(text, at)? {
            if ONCE.contains(&pair.key) && seen.contains(&pair.key) {
                return Err(pair.key_span());
            }
            if pair.negated && !NEGATABLE.contains(&pair.key) {
                return Err(pair.value_span());
            }
            seen.push(pair.key);
            filters.take(&pair)?;
        }
        Ok(filters)
    }

    fn take(&mut self, pair: &Pair<'_>) -> Result<(), Span> {
        let bad = || pair.value_span();
        let float = || pair.value.parse::<f32>().ok().filter(|n| n.is_finite()).ok_or_else(bad);
        let not = |value: String| Not { value, negated: pair.negated };
        match pair.key {
            "x" | "y" | "z" => self.origin[axis(pair.key)] = Some(Axis::read(pair.value).ok_or_else(bad)?),
            "dx" | "dy" | "dz" => self.volume[axis(&pair.key[1..])] = Some(float()?),
            "r" => (self.distance.min, self.distance.max) = upper(pair, self.distance.min)?,
            "rm" => self.distance.min = Some(float()?),
            "rx" => (self.pitch.min, self.pitch.max) = upper(pair, self.pitch.min)?,
            "rxm" => self.pitch.min = Some(float()?),
            "ry" => (self.yaw.min, self.yaw.max) = upper(pair, self.yaw.min)?,
            "rym" => self.yaw.min = Some(float()?),
            "l" => (self.level.min, self.level.max) = upper(pair, self.level.min)?,
            "lm" => self.level.min = Some(pair.value.parse::<i32>().map_err(|_| bad())?),
            "c" => self.count = Some(pair.value.parse::<i32>().ok().filter(|count| *count != 0).ok_or_else(bad)?),
            "name" => self.names.push(not(pair.value.to_owned())),
            "type" if !pair.value.is_empty() => self.types.push(not(namespaced(pair.value))),
            "family" if !pair.value.is_empty() => self.families.push(not(pair.value.to_ascii_lowercase())),
            "tag" => self.tags.push(not(pair.value.to_owned())),
            "m" => self.modes.push(Not { value: game_mode(pair.value).ok_or_else(bad)?, negated: pair.negated }),
            "scores" => self.scores.extend(scores(pair)?),
            "hasitem" => self.items.extend(ItemTest::read_all(pair)?),
            "haspermission" => self.permissions = permissions(pair)?,
            "type" | "family" => return Err(bad()),
            _ => return Err(pair.key_span()),
        }
        Ok(())
    }
}

fn axis(key: &str) -> usize {
    match key {
        "x" => 0,
        "y" => 1,
        _ => 2,
    }
}

/// A key that sets the most (`r`, `l`, `rx`, `ry`) as a number, or both ends as a range;
/// `min` is what its key for the least set already.
fn upper<T: std::str::FromStr + PartialOrd + Copy>(pair: &Pair<'_>, min: Option<T>) -> Result<(Option<T>, Option<T>), Span> {
    match pair.value.contains("..") {
        true => Bounds::read(pair.value).map(|bounds: Bounds<T>| (bounds.min.or(min), bounds.max)),
        false => pair.value.trim().parse::<T>().ok().map(|max| (min, Some(max))),
    }
    .ok_or_else(|| pair.value_span())
}

pub(super) fn namespaced(name: &str) -> String {
    let name = name.to_ascii_lowercase();
    if name.contains(':') { name } else { format!("minecraft:{name}") }
}

/// `m`'s value: names, their first letters and the game's numbers for them.
fn game_mode(value: &str) -> Option<Option<GameMode>> {
    Some(match value.to_ascii_lowercase().as_str() {
        "survival" | "s" | "0" => Some(GameMode::Survival),
        "creative" | "c" | "1" => Some(GameMode::Creative),
        "adventure" | "a" | "2" => Some(GameMode::Adventure),
        "spectator" | "6" => Some(GameMode::Spectator),
        "default" | "d" | "5" => None,
        _ => return None,
    })
}

/// `scores={objective=range,other=!range}`.
fn scores(pair: &Pair<'_>) -> Result<Vec<(String, Not<Bounds<i32>>)>, Span> {
    let (inner, at) = lex::inside(pair.value, pair.value_at, '{').ok_or_else(|| pair.value_span())?;
    let read = |score: Pair<'_>| Bounds::read(score.value).map(|bounds| (score.key.to_owned(), Not { value: bounds, negated: score.negated })).ok_or_else(|| score.value_span());
    lex::pairs(inner, at)?.into_iter().map(read).collect()
}

/// `haspermission={camera=enabled,movement=disabled}`.
fn permissions(pair: &Pair<'_>) -> Result<Vec<(Input, bool)>, Span> {
    let (inner, at) = lex::inside(pair.value, pair.value_at, '{').ok_or_else(|| pair.value_span())?;
    let read = |permission: Pair<'_>| {
        let input = INPUTS.iter().find(|(name, _)| *name == permission.key).map(|(_, input)| *input).ok_or_else(|| permission.key_span())?;
        let enabled = match (permission.value, permission.negated) {
            ("enabled", false) => true,
            ("disabled", false) => false,
            _ => return Err(permission.value_span()),
        };
        Ok((input, enabled))
    };
    lex::pairs(inner, at)?.into_iter().map(read).collect()
}
