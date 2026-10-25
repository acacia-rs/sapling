//! The test backend's scoreboard: one board, holders kept as they are named.

use super::{Display, DisplaySlot, Objective, ScoreHolder, ScoreboardBackend};
use crate::testing::Fake;

#[derive(Default)]
pub struct FakeBoard {
    objectives: Vec<Objective>,
    displays: Vec<(DisplaySlot, Display)>,
    scores: Vec<(ScoreHolder, String, i32)>,
}

impl ScoreboardBackend for Fake {
    fn objectives(&self) -> Vec<Objective> {
        self.board.objectives.clone()
    }
    fn add_objective(&mut self, objective: &Objective) -> bool {
        let new = !self.board.objectives.iter().any(|kept| kept.name == objective.name);
        if new {
            self.board.objectives.push(objective.clone());
        }
        new
    }
    fn remove_objective(&mut self, name: &str) -> bool {
        let board = &mut self.board;
        board.scores.retain(|(_, objective, _)| objective != name);
        board.displays.retain(|(_, display)| display.objective != name);
        let before = board.objectives.len();
        board.objectives.retain(|kept| kept.name != name);
        board.objectives.len() < before
    }
    fn display(&self, slot: DisplaySlot) -> Option<Display> {
        self.board.displays.iter().find(|(shown, _)| *shown == slot).map(|(_, display)| display.clone())
    }
    fn set_display(&mut self, slot: DisplaySlot, display: Option<&Display>) -> bool {
        if display.is_some_and(|display| !self.board.objectives.iter().any(|kept| kept.name == display.objective)) {
            return false;
        }
        self.board.displays.retain(|(shown, _)| *shown != slot);
        self.board.displays.extend(display.map(|display| (slot, display.clone())));
        true
    }
    fn score(&self, holder: &ScoreHolder, objective: &str) -> Option<i32> {
        self.board.scores.iter().find(|(who, kept, _)| who == holder && kept == objective).map(|(.., score)| *score)
    }
    fn set_score(&mut self, holder: &ScoreHolder, objective: &str, score: i32) -> bool {
        if !self.board.objectives.iter().any(|kept| kept.name == objective) {
            return false;
        }
        self.reset_score(holder, Some(objective));
        self.board.scores.push((holder.clone(), objective.to_owned(), score));
        true
    }
    fn reset_score(&mut self, holder: &ScoreHolder, objective: Option<&str>) -> bool {
        let before = self.board.scores.len();
        self.board.scores.retain(|(who, kept, _)| !(who == holder && objective.is_none_or(|objective| kept == objective)));
        self.board.scores.len() < before
    }
    fn scores(&self, holder: &ScoreHolder) -> Vec<(String, i32)> {
        self.board.objectives.iter().filter_map(|objective| Some((objective.name.clone(), self.score(holder, &objective.name)?))).collect()
    }
    fn score_holders(&self) -> Vec<ScoreHolder> {
        let mut holders: Vec<ScoreHolder> = Vec::new();
        for (holder, ..) in &self.board.scores {
            if !holders.contains(holder) {
                holders.push(holder.clone());
            }
        }
        holders
    }
    fn holder_name(&self, holder: &ScoreHolder) -> String {
        match holder {
            ScoreHolder::Player(id) => self.players.iter().find(|player| player.id == *id).map_or_else(String::new, |player| player.name.clone()),
            ScoreHolder::Entity(id) => id.0.to_string(),
            ScoreHolder::Fake(name) => name.clone(),
            ScoreHolder::Absent { unique_id, .. } => unique_id.to_string(),
        }
    }
}
