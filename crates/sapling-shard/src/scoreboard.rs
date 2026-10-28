//! Each world's scoreboard, kept on its overworld layer as BDS keeps one per level: taken from
//! the level's `scoreboard` record when first needed and handed back to it to be saved. What a
//! change shows clients is noted as a [`Change`] and sent after the tick's handlers (sync.rs).
//! Holders are filed by [`ScoreIdentity`]: a player's or entity's `UniqueID`, or a fake name.

mod backend;
mod sync;

use std::collections::{BTreeMap, HashMap};

use bevy_ecs::prelude::*;
use sapling_api::{Display, DisplaySlot, Objective, SortOrder};
use sapling_world::{DisplayRecord, ObjectiveRecord, ScoreIdentity, ScoreboardRecord};

use crate::components::Layer;

pub(crate) use sync::{send, show_sidebar};

/// What players seeing the board are to be told, in the order it happened.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Change {
    Removed(String),
    /// A slot shows `objective` now, or nothing; `replaced` when it showed another before.
    Displayed { slot: DisplaySlot, shown: Option<(Objective, SortOrder)>, replaced: bool },
    Score { objective: String, id: i64, score: Option<i32> },
}

struct Kept {
    objective: Objective,
    /// By scoreboard id.
    scores: BTreeMap<i64, i32>,
}

#[derive(Component, Default)]
pub(crate) struct Board {
    /// Oldest first.
    objectives: Vec<Kept>,
    displays: HashMap<DisplaySlot, Display>,
    ids: HashMap<ScoreIdentity, i64>,
    holders: BTreeMap<i64, ScoreIdentity>,
    last_id: i64,
    pub(crate) changes: Vec<Change>,
}

fn slot_named(name: &str) -> Option<DisplaySlot> {
    DisplaySlot::ALL.into_iter().find(|slot| slot.name() == name)
}

/// BDS's `SortOrder` byte: 0 ascending, 1 descending.
pub(crate) fn order_byte(order: SortOrder) -> i8 {
    match order {
        SortOrder::Ascending => 0,
        SortOrder::Descending => 1,
    }
}

impl Board {
    fn from_record(record: ScoreboardRecord) -> Self {
        let mut board = Self { last_id: record.last_unique_id, ..Self::default() };
        for (id, identity) in record.entries {
            board.ids.insert(identity.clone(), id);
            board.holders.insert(id, identity);
        }
        for kept in record.objectives {
            let objective = Objective { name: kept.name, display_name: kept.display_name, criteria: kept.criteria };
            board.objectives.push(Kept { objective, scores: kept.scores.into_iter().filter(|(id, _)| board.holders.contains_key(id)).collect() });
        }
        for display in record.displays {
            let order = if display.sort_order == 0 { SortOrder::Ascending } else { SortOrder::Descending };
            if let Some(slot) = slot_named(&display.slot).filter(|_| board.kept(&display.objective).is_some()) {
                board.displays.insert(slot, Display { objective: display.objective, order });
            }
        }
        board
    }

    /// As BDS saves it: only holders with a score are entries.
    pub(crate) fn record(&self) -> ScoreboardRecord {
        let objectives = self.objectives.iter().map(|kept| {
            let Objective { name, display_name, criteria } = kept.objective.clone();
            ObjectiveRecord { name, display_name, criteria, scores: kept.scores.iter().map(|(id, score)| (*id, *score)).collect() }
        });
        let displays = DisplaySlot::ALL.into_iter().filter_map(|slot| {
            let display = self.displays.get(&slot)?;
            Some(DisplayRecord { slot: slot.name().into(), objective: display.objective.clone(), sort_order: order_byte(display.order) })
        });
        let entries = self.holders.iter().filter(|(id, _)| self.objectives.iter().any(|kept| kept.scores.contains_key(id))).map(|(id, holder)| (*id, holder.clone()));
        let mut objectives: Vec<ObjectiveRecord> = objectives.collect();
        objectives.sort_by(|a, b| a.name.cmp(&b.name));
        ScoreboardRecord { objectives, displays: displays.collect(), entries: entries.collect(), last_unique_id: self.last_id }
    }

    fn kept(&self, name: &str) -> Option<&Kept> {
        self.objectives.iter().find(|kept| kept.objective.name == name)
    }

    fn shown(&self, objective: &str) -> bool {
        self.displays.values().any(|display| display.objective == objective)
    }

    pub(crate) fn objectives(&self) -> Vec<Objective> {
        self.objectives.iter().map(|kept| kept.objective.clone()).collect()
    }

    pub(crate) fn add_objective(&mut self, objective: &Objective) -> bool {
        let new = self.kept(&objective.name).is_none();
        if new {
            self.objectives.push(Kept { objective: objective.clone(), scores: BTreeMap::new() });
        }
        new
    }

    /// BDS tells every client, whether the objective was shown or not; a shown one's scores are
    /// taken away first, newest first (research/captures/bds-scoreboard-join.txt).
    pub(crate) fn remove_objective(&mut self, name: &str) -> bool {
        let Some(at) = self.objectives.iter().position(|kept| kept.objective.name == name) else { return false };
        if self.shown(name) {
            let gone = self.scores_on(name).into_iter().map(|(id, _)| Change::Score { objective: name.to_owned(), id, score: None });
            self.changes.extend(gone.collect::<Vec<_>>());
        }
        self.objectives.remove(at);
        self.displays.retain(|_, display| display.objective != name);
        self.changes.push(Change::Removed(name.to_owned()));
        true
    }

    pub(crate) fn display(&self, slot: DisplaySlot) -> Option<Display> {
        self.displays.get(&slot).cloned()
    }

    /// Clearing an empty slot tells nobody (research/captures/bds-scoreboard.txt).
    pub(crate) fn set_display(&mut self, slot: DisplaySlot, display: Option<&Display>) -> bool {
        let shown = match display {
            Some(display) => match self.kept(&display.objective) {
                Some(kept) => Some((kept.objective.clone(), display.order)),
                None => return false,
            },
            None => None,
        };
        let before = match display {
            Some(display) => self.displays.insert(slot, display.clone()),
            None => self.displays.remove(&slot),
        };
        if shown.is_some() || before.is_some() {
            self.changes.push(Change::Displayed { slot, shown, replaced: before.is_some() });
        }
        true
    }

    pub(crate) fn id_of(&self, holder: &ScoreIdentity) -> Option<i64> {
        self.ids.get(holder).copied()
    }

    /// The fake holder `name` becomes the player `unique`, unless that player holds scores of its
    /// own (sync.rs `claim`); the id they keep.
    pub(crate) fn claim(&mut self, name: &str, unique: i64) -> Option<i64> {
        let player = ScoreIdentity::Player(unique);
        if self.ids.contains_key(&player) {
            return None;
        }
        let id = self.ids.remove(&ScoreIdentity::Fake(name.to_owned()))?;
        self.ids.insert(player.clone(), id);
        self.holders.insert(id, player);
        Some(id)
    }

    pub(crate) fn holder(&self, id: i64) -> Option<&ScoreIdentity> {
        self.holders.get(&id)
    }

    pub(crate) fn score(&self, holder: &ScoreIdentity, objective: &str) -> Option<i32> {
        self.kept(objective)?.scores.get(&self.id_of(holder)?).copied()
    }

    /// Every change is told while the objective is shown, a score that stays too, as on BDS.
    pub(crate) fn set_score(&mut self, holder: &ScoreIdentity, objective: &str, score: i32) -> bool {
        if self.kept(objective).is_none() {
            return false;
        }
        let id = match self.id_of(holder) {
            Some(id) => id,
            None => {
                self.last_id += 1;
                self.ids.insert(holder.clone(), self.last_id);
                self.holders.insert(self.last_id, holder.clone());
                self.last_id
            }
        };
        let shown = self.shown(objective);
        let Some(kept) = self.objectives.iter_mut().find(|kept| kept.objective.name == objective) else { return false };
        kept.scores.insert(id, score);
        if shown {
            self.changes.push(Change::Score { objective: objective.to_owned(), id, score: Some(score) });
        }
        true
    }

    pub(crate) fn reset_score(&mut self, holder: &ScoreIdentity, objective: Option<&str>) -> bool {
        let Some(id) = self.id_of(holder) else { return false };
        let mut gone = Vec::new();
        for kept in &mut self.objectives {
            if objective.is_none_or(|objective| kept.objective.name == objective) && kept.scores.remove(&id).is_some() {
                gone.push(kept.objective.name.clone());
            }
        }
        for objective in &gone {
            if self.shown(objective) {
                self.changes.push(Change::Score { objective: objective.clone(), id, score: None });
            }
        }
        !gone.is_empty()
    }

    /// A holder's scores in objective order.
    pub(crate) fn scores(&self, holder: &ScoreIdentity) -> Vec<(String, i32)> {
        let Some(id) = self.id_of(holder) else { return Vec::new() };
        self.objectives.iter().filter_map(|kept| Some((kept.objective.name.clone(), *kept.scores.get(&id)?))).collect()
    }

    /// Holders with any score, newest first: BDS listed them about so.
    pub(crate) fn holders(&self) -> Vec<ScoreIdentity> {
        let scored = |id: &i64| self.objectives.iter().any(|kept| kept.scores.contains_key(id));
        self.holders.iter().rev().filter(|(id, _)| scored(id)).map(|(_, holder)| holder.clone()).collect()
    }

    /// An objective's scores by scoreboard id, newest first, as BDS sent them on `setdisplay`.
    pub(crate) fn scores_on(&self, objective: &str) -> Vec<(i64, i32)> {
        self.kept(objective).map_or_else(Vec::new, |kept| kept.scores.iter().rev().map(|(id, score)| (*id, *score)).collect())
    }

    /// Each slot shown, with its objective.
    pub(crate) fn displays(&self) -> Vec<(DisplaySlot, Objective, SortOrder)> {
        DisplaySlot::ALL.into_iter().filter_map(|slot| {
            let display = self.displays.get(&slot)?;
            Some((slot, self.kept(&display.objective)?.objective.clone(), display.order))
        }).collect()
    }
}

/// The board of `layer`'s world, made from its level's record the first time.
pub(crate) fn board_mut(world: &mut World, layer: Entity) -> Option<Mut<'_, Board>> {
    let home = crate::dimensions::home(world, layer);
    if world.get::<Board>(home).is_none() {
        let record = world.get_mut::<Layer>(home)?.world.level_mut().scoreboard.take();
        world.entity_mut(home).insert(record.map_or_else(Board::default, Board::from_record));
    }
    world.get_mut::<Board>(home)
}

pub(crate) fn board(world: &World, layer: Entity) -> Option<&Board> {
    world.get::<Board>(crate::dimensions::home(world, layer))
}

/// Hands each board to its level to be saved with it.
pub(crate) fn keep_all(world: &mut World) {
    let boards: Vec<(Entity, ScoreboardRecord)> = world.query::<(Entity, &Board)>().iter(world).map(|(layer, board)| (layer, board.record())).collect();
    for (layer, record) in boards {
        if let Some(mut layer) = world.get_mut::<Layer>(layer) {
            layer.world.level_mut().scoreboard = Some(record);
        }
    }
}

#[cfg(test)]
mod tests;
