//! `/execute`'s sub-commands as BDS reads them (research/captures/bds-execute.txt,
//! bds-execute-if.txt): a word, its arguments, the next sub-command; a condition may end the
//! line, `run` takes the rest of it.

use sapling_api::command::{Args, Axis, BlockCoords, Coords, Parameter, ParseError, Selector, States};

use crate::vanilla::names::BlockName;

pub(super) const DIMENSIONS: [&str; 3] = ["overworld", "nether", "the_end"];
const OPERATORS: [(&str, Order); 5] = [("<=", Order::AtMost), (">=", Order::AtLeast), ("<", Order::Less), (">", Order::More), ("=", Order::Equal)];

pub(super) enum Step {
    As(Selector),
    At(Selector),
    In(i32),
    Positioned(Coords),
    PositionedAs(Selector),
    Rotated([Axis; 2]),
    RotatedAs(Selector),
    Facing(Coords),
    /// Towards the eyes when true, else the feet.
    FacingEntity(Selector, bool),
    /// x, y, z.
    Align([bool; 3]),
    /// The eyes when true.
    Anchored(bool),
    /// `unless` when false.
    If(bool, Condition),
}

pub(super) enum Condition {
    Block(BlockCoords, BlockName, Option<States>),
    /// Begin, end, destination; `masked` when true.
    Blocks([BlockCoords; 3], bool),
    Entity(Selector),
    Score(Score, Compare),
}

/// A score holder and objective as typed; `typed` names it when it names nobody.
pub(super) struct Score {
    pub target: Selector,
    pub typed: String,
    pub objective: String,
}

pub(super) enum Compare {
    Matches(Range),
    Against(Order, Score),
}

#[derive(Clone, Copy)]
pub(super) enum Order {
    Less,
    AtMost,
    Equal,
    AtLeast,
    More,
}

impl Order {
    pub fn holds(self, left: i32, right: i32) -> bool {
        match self {
            Self::Less => left < right,
            Self::AtMost => left <= right,
            Self::Equal => left == right,
            Self::AtLeast => left >= right,
            Self::More => left > right,
        }
    }
}

/// `5`, `1..4`, `5..`, `..5`, each perhaps after `!`.
pub(super) struct Range {
    pub min: i32,
    pub max: i32,
    pub not: bool,
}

impl Range {
    pub fn holds(&self, score: i32) -> bool {
        (self.min..=self.max).contains(&score) != self.not
    }

    pub(super) fn read(word: &str) -> Option<Self> {
        let (not, word) = word.strip_prefix('!').map_or((false, word), |rest| (true, rest));
        let bound = |text: &str, open: i32| if text.is_empty() { Some(open) } else { text.parse().ok() };
        let (min, max) = match word.split_once("..") {
            Some((min, max)) if !(min.is_empty() && max.is_empty()) => (bound(min, i32::MIN)?, bound(max, i32::MAX)?),
            Some(_) => return None,
            None => {
                let only = word.parse().ok()?;
                (only, only)
            }
        };
        Some(Self { min, max, not })
    }
}

/// The steps of a line and, after `run`, the command and where it starts.
pub(super) fn steps(args: &mut Args<'_>) -> Result<(Vec<Step>, Option<(String, usize)>), ParseError> {
    let mut steps = Vec::new();
    loop {
        let mark = args.mark();
        let step = match args.word()?.to_ascii_lowercase().as_str() {
            "as" => Step::As(Selector::parse(args)?),
            "at" => Step::At(Selector::parse(args)?),
            "in" => Step::In(choice(args, &DIMENSIONS)? as i32),
            "positioned" if next_is(args, "as") => Step::PositionedAs(after_word(args)?),
            "positioned" => Step::Positioned(Coords::parse(args)?),
            "rotated" if next_is(args, "as") => Step::RotatedAs(after_word(args)?),
            "rotated" => Step::Rotated([angle(args)?, angle(args)?]),
            "facing" if next_is(args, "entity") => Step::FacingEntity(after_word(args)?, choice(args, &["feet", "eyes"])? == 1),
            "facing" => Step::Facing(Coords::parse(args)?),
            "align" => Step::Align(axes(args.word()?).ok_or_else(|| ParseError::Invalid("commands.execute.align.invalidInput".into()))?),
            "anchored" => Step::Anchored(choice(args, &["feet", "eyes"])? == 1),
            word @ ("if" | "unless") => Step::If(word == "if", condition(args)?),
            "run" => {
                let at = args.mark();
                return Ok((steps, Some((args.rest()?.to_owned(), at))));
            }
            _ => return Err(args.error_at(mark)),
        };
        let ends = matches!(step, Step::If(..)) && args.is_empty();
        steps.push(step);
        if ends {
            return Ok((steps, None));
        }
    }
}

fn next_is(args: &Args<'_>, word: &str) -> bool {
    args.peek().is_some_and(|next| next.eq_ignore_ascii_case(word))
}

fn after_word(args: &mut Args<'_>) -> Result<Selector, ParseError> {
    args.word()?;
    Selector::parse(args)
}

/// The index of the word of `words` that comes next.
fn choice(args: &mut Args<'_>, words: &[&str]) -> Result<usize, ParseError> {
    let mark = args.mark();
    let word = args.word()?;
    words.iter().position(|one| word.eq_ignore_ascii_case(one)).ok_or_else(|| args.error_at(mark))
}

/// A yaw or pitch: a number or `~` and an offset.
fn angle(args: &mut Args<'_>) -> Result<Axis, ParseError> {
    let mark = args.mark();
    match <Axis as Parameter>::parse(args)? {
        Axis::Local(_) => Err(args.error_at(mark)),
        axis => Ok(axis),
    }
}

/// `xz`, `y`, `zyx`: each axis at most once.
fn axes(word: &str) -> Option<[bool; 3]> {
    let mut chosen = [false; 3];
    for c in word.chars() {
        let axis = "xyz".find(c.to_ascii_lowercase())?;
        if std::mem::replace(&mut chosen[axis], true) {
            return None;
        }
    }
    (!word.is_empty()).then_some(chosen)
}

fn condition(args: &mut Args<'_>) -> Result<Condition, ParseError> {
    Ok(match choice(args, &["block", "blocks", "entity", "score"])? {
        0 => Condition::Block(BlockCoords::parse(args)?, BlockName::parse(args)?, Option::<States>::parse(args)?),
        1 => Condition::Blocks([BlockCoords::parse(args)?, BlockCoords::parse(args)?, BlockCoords::parse(args)?], choice(args, &["all", "masked"])? == 1),
        2 => Condition::Entity(Selector::parse(args)?),
        _ => {
            let target = score(args)?;
            let mark = args.mark();
            let word = args.word()?;
            if word.eq_ignore_ascii_case("matches") {
                let mark = args.mark();
                return Ok(Condition::Score(target, Compare::Matches(Range::read(args.word()?).ok_or_else(|| args.error_at(mark))?)));
            }
            // BDS reads `==` as `=` and stops at the second.
            let (op, order) = OPERATORS.iter().find(|(op, _)| word.starts_with(op)).ok_or_else(|| args.error_at(mark))?;
            if word.len() > op.len() {
                return Err(args.error_span(mark + op.len(), mark + word.len()));
            }
            Condition::Score(target, Compare::Against(*order, score(args)?))
        }
    })
}

fn score(args: &mut Args<'_>) -> Result<Score, ParseError> {
    let mark = args.mark();
    let target = Selector::parse(args)?;
    let typed = args.line()[mark..args.mark()].trim().to_owned();
    Ok(Score { target, typed, objective: args.word()?.to_owned() })
}
