//! The scoreboard of the world in scope, as BDS keeps one per world (its other dimensions share
//! it) and saves it with the world: objectives, the scores players, entities and fake players
//! hold on them, and the objective each display slot shows. Players in the world see the slots
//! change as they change. Vanilla's `/scoreboard` is built on these calls alone.

use crate::command::{Args, Parameter, Selected, Selector};
use crate::{EntityId, Player, PlayerId, Sender, ShardCtx};

/// Who holds a score.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ScoreHolder {
    Player(PlayerId),
    Entity(EntityId),
    /// A name that is nobody: a line with a score (`/scoreboard players set Fake obj 3`).
    Fake(String),
    /// A player or entity with scores on the board that is not in the world on this shard now,
    /// by the `UniqueID` the world keeps it under.
    Absent { unique_id: i64, player: bool },
}

/// Where the client shows an objective.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DisplaySlot {
    /// Right of the screen.
    Sidebar,
    /// The pause screen's player list.
    List,
    /// Under each player's name tag; not sorted.
    BelowName,
}

impl DisplaySlot {
    pub const ALL: [Self; 3] = [Self::Sidebar, Self::List, Self::BelowName];

    /// As the protocol and `/scoreboard objectives setdisplay` name it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Sidebar => "sidebar",
            Self::List => "list",
            Self::BelowName => "belowname",
        }
    }
}

/// `setdisplay`'s order; BDS's default is descending.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortOrder {
    Ascending,
    #[default]
    Descending,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Objective {
    pub name: String,
    pub display_name: String,
    /// `dummy`, the only criteria BDS has.
    pub criteria: String,
}

impl Objective {
    /// A `dummy` objective; an empty display name is the name, as BDS gives it.
    pub fn dummy(name: impl Into<String>, display_name: impl Into<String>) -> Self {
        let (name, display_name) = (name.into(), display_name.into());
        let display_name = if display_name.is_empty() { name.clone() } else { display_name };
        Self { name, display_name, criteria: "dummy".into() }
    }
}

/// What a display slot shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Display {
    pub objective: String,
    pub order: SortOrder,
}

/// `/scoreboard players operation`'s operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScoreOperation {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    Set,
    Min,
    Max,
    Swap,
}

impl ScoreOperation {
    pub const SYMBOLS: [(&'static str, Self); 9] =
        [("+=", Self::Add), ("-=", Self::Subtract), ("*=", Self::Multiply), ("/=", Self::Divide), ("%=", Self::Remainder), ("=", Self::Set), ("<", Self::Min), (">", Self::Max), ("><", Self::Swap)];

    /// The target's and the source's scores after. As BDS's C++ ints: they wrap, division and
    /// remainder truncate toward zero (`-7 /= 2` is -3, `7 %= -3` is 1) and by zero leave the
    /// target as it was (research/captures/bds-scoreboard2.txt).
    pub fn apply(self, target: i32, source: i32) -> (i32, i32) {
        let target = match self {
            Self::Add => target.wrapping_add(source),
            Self::Subtract => target.wrapping_sub(source),
            Self::Multiply => target.wrapping_mul(source),
            Self::Divide => target.checked_div(source).unwrap_or(target),
            Self::Remainder => target.checked_rem(source).unwrap_or(target),
            Self::Set => source,
            Self::Min => target.min(source),
            Self::Max => target.max(source),
            Self::Swap => return (source, target),
        };
        (target, source)
    }
}

impl ShardCtx<'_> {
    /// Every objective, oldest first.
    pub fn objectives(&self) -> Vec<Objective> {
        self.backend.objectives()
    }

    pub fn objective(&self, name: &str) -> Option<Objective> {
        self.backend.objectives().into_iter().find(|objective| objective.name == name)
    }

    /// False, with nothing changed, where one of that name is there already.
    pub fn add_objective(&mut self, objective: &Objective) -> bool {
        self.backend.add_objective(objective)
    }

    /// Takes the objective, its scores and any slot it was shown in away; false where there was none.
    pub fn remove_objective(&mut self, name: &str) -> bool {
        self.backend.remove_objective(name)
    }

    pub fn display(&self, slot: DisplaySlot) -> Option<Display> {
        self.backend.display(slot)
    }

    /// Shows an objective in a slot, or empties it (`None`); false for an objective there is not.
    pub fn set_display(&mut self, slot: DisplaySlot, display: Option<&Display>) -> bool {
        self.backend.set_display(slot, display)
    }

    pub fn score(&self, holder: &ScoreHolder, objective: &str) -> Option<i32> {
        self.backend.score(holder, objective)
    }

    /// False for an objective there is not and a player or entity that is gone.
    pub fn set_score(&mut self, holder: &ScoreHolder, objective: &str, score: i32) -> bool {
        self.backend.set_score(holder, objective, score)
    }

    /// Takes a holder's score on `objective` away, or all of them (`None`); false where it had none.
    pub fn reset_score(&mut self, holder: &ScoreHolder, objective: Option<&str>) -> bool {
        self.backend.reset_score(holder, objective)
    }

    /// A holder's scores, by objective in [`ShardCtx::objectives`]' order.
    pub fn scores(&self, holder: &ScoreHolder) -> Vec<(String, i32)> {
        self.backend.scores(holder)
    }

    /// Everyone with a score, as `/scoreboard players list` names them.
    pub fn score_holders(&self) -> Vec<ScoreHolder> {
        self.backend.score_holders()
    }

    /// How `/scoreboard` names a holder: a player's name, an entity's `UniqueID`, a fake's name.
    pub fn holder_name(&self, holder: &ScoreHolder) -> String {
        self.backend.holder_name(holder)
    }

    /// Who a score command's target names for `sender`: `*` everyone with a score, a selector its
    /// players and entities, a name the online player of that name or else a fake player.
    pub fn select_holders(&mut self, sender: &Sender, target: &Selector) -> Vec<ScoreHolder> {
        match target {
            Selector::Named(name) if name == "*" => self.score_holders(),
            Selector::Named(name) => match self.players().find(|player| player.name.eq_ignore_ascii_case(name)) {
                Some(player) => vec![ScoreHolder::Player(player.id)],
                None => vec![ScoreHolder::Fake(name.clone())],
            },
            Selector::Variable(..) => self.select(sender, target).into_iter().map(|found| match found {
                Selected::Player(player) => ScoreHolder::Player(player.id),
                Selected::Entity(entity) => ScoreHolder::Entity(entity.id),
            }).collect(),
        }
    }

    /// What a rawtext `score` part shows for `name` (a selector or a name; not `*`, which is
    /// whoever reads it) on `objective`: the scores of those it names that have one, `, `-joined
    /// (research/captures/bds-scoreboard.txt); `None` where none has.
    pub fn score_text(&mut self, sender: &Sender, name: &str, objective: &str) -> Option<String> {
        let online: Vec<Player> = self.players().collect();
        let target = Selector::parse(&mut Args::new(name, 0, sender, &online)).ok()?;
        let scores: Vec<String> = self.select_holders(sender, &target).iter().filter_map(|holder| self.score(holder, objective)).map(|score| score.to_string()).collect();
        (!scores.is_empty()).then(|| scores.join(", "))
    }
}

/// The scoreboard side of [`crate::Backend`], on the world in scope.
pub trait ScoreboardBackend {
    fn objectives(&self) -> Vec<Objective>;
    fn add_objective(&mut self, objective: &Objective) -> bool;
    fn remove_objective(&mut self, name: &str) -> bool;
    fn display(&self, slot: DisplaySlot) -> Option<Display>;
    fn set_display(&mut self, slot: DisplaySlot, display: Option<&Display>) -> bool;
    fn score(&self, holder: &ScoreHolder, objective: &str) -> Option<i32>;
    fn set_score(&mut self, holder: &ScoreHolder, objective: &str, score: i32) -> bool;
    fn reset_score(&mut self, holder: &ScoreHolder, objective: Option<&str>) -> bool;
    fn scores(&self, holder: &ScoreHolder) -> Vec<(String, i32)>;
    fn score_holders(&self) -> Vec<ScoreHolder>;
    fn holder_name(&self, holder: &ScoreHolder) -> String;
}

#[cfg(test)]
pub(crate) mod fake;
#[cfg(test)]
mod tests;
