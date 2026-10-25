//! The scoreboard calls of a [`sapling_api::ShardCtx`], on the board of the world in scope.

use sapling_api::{Display, DisplaySlot, EntityId, Objective, ScoreHolder, ScoreboardBackend};
use sapling_world::ScoreIdentity;

use super::{Board, board, board_mut};
use crate::components::{Identity, Layer, NetId};
use crate::handlers::Ctx;
use crate::tags::{Marks, UniqueId};
use crate::{dimensions, entities, worlds};

/// How BDS names a player with scores who is not online (unverified: never seen in a capture).
const OFFLINE: &str = "commands.scoreboard.players.offlinePlayerName";

impl Ctx<'_> {
    /// Reads the board of the world in scope; one not made yet is read from its level's record.
    fn read_board<R>(&self, read: impl FnOnce(&Board) -> R) -> R {
        let layer = worlds::current(self.world);
        if let Some(board) = board(self.world, layer) {
            return read(board);
        }
        let record = self.world.get::<Layer>(dimensions::home(self.world, layer)).and_then(|home| home.world.level().scoreboard.clone());
        read(&record.map_or_else(Board::default, Board::from_record))
    }

    fn write_board<R: Default>(&mut self, write: impl FnOnce(&mut Board) -> R) -> R {
        let layer = worlds::current(self.world);
        board_mut(self.world, layer).map_or_else(R::default, |mut board| write(&mut board))
    }

    /// Whom the board files `holder` under; `None` for a player or entity that is gone or that
    /// its world does not keep.
    fn identity(&self, holder: &ScoreHolder) -> Option<ScoreIdentity> {
        Some(match holder {
            ScoreHolder::Player(player) => ScoreIdentity::Player(self.world.get::<Marks>(self.entity(*player)?)?.unique_id),
            ScoreHolder::Entity(id) => ScoreIdentity::Entity(self.world.get::<UniqueId>(entities::find(self.world, *id)?)?.0),
            ScoreHolder::Fake(name) => ScoreIdentity::Fake(name.clone()),
            ScoreHolder::Absent { unique_id, player: true } => ScoreIdentity::Player(*unique_id),
            ScoreHolder::Absent { unique_id, player: false } => ScoreIdentity::Entity(*unique_id),
        })
    }

    fn holder_of(&self, identity: &ScoreIdentity) -> ScoreHolder {
        match identity {
            ScoreIdentity::Player(unique_id) => {
                let mut players = self.world.iter_entities().filter(|entity| entity.get::<Marks>().is_some_and(|marks| marks.unique_id == *unique_id));
                let online = players.find_map(|entity| Some(entity.get::<Identity>()?.0.id)).filter(|player| self.entity(*player).is_some());
                online.map_or(ScoreHolder::Absent { unique_id: *unique_id, player: true }, ScoreHolder::Player)
            }
            ScoreIdentity::Entity(unique_id) => {
                let mut kept = self.world.iter_entities().filter(|entity| entity.get::<UniqueId>().is_some_and(|unique| unique.0 == *unique_id));
                let here = kept.find_map(|entity| Some(EntityId(entity.get::<NetId>()?.0))).filter(|id| entities::find(self.world, *id).is_some());
                here.map_or(ScoreHolder::Absent { unique_id: *unique_id, player: false }, ScoreHolder::Entity)
            }
            ScoreIdentity::Fake(name) => ScoreHolder::Fake(name.clone()),
        }
    }
}

impl ScoreboardBackend for Ctx<'_> {
    fn objectives(&self) -> Vec<Objective> {
        self.read_board(Board::objectives)
    }

    fn add_objective(&mut self, objective: &Objective) -> bool {
        self.write_board(|board| board.add_objective(objective))
    }

    fn remove_objective(&mut self, name: &str) -> bool {
        self.write_board(|board| board.remove_objective(name))
    }

    fn display(&self, slot: DisplaySlot) -> Option<Display> {
        self.read_board(|board| board.display(slot))
    }

    fn set_display(&mut self, slot: DisplaySlot, display: Option<&Display>) -> bool {
        self.write_board(|board| board.set_display(slot, display))
    }

    fn score(&self, holder: &ScoreHolder, objective: &str) -> Option<i32> {
        let identity = self.identity(holder)?;
        self.read_board(|board| board.score(&identity, objective))
    }

    fn set_score(&mut self, holder: &ScoreHolder, objective: &str, score: i32) -> bool {
        let Some(identity) = self.identity(holder) else { return false };
        self.write_board(|board| board.set_score(&identity, objective, score))
    }

    fn reset_score(&mut self, holder: &ScoreHolder, objective: Option<&str>) -> bool {
        let Some(identity) = self.identity(holder) else { return false };
        self.write_board(|board| board.reset_score(&identity, objective))
    }

    fn scores(&self, holder: &ScoreHolder) -> Vec<(String, i32)> {
        let Some(identity) = self.identity(holder) else { return Vec::new() };
        self.read_board(|board| board.scores(&identity))
    }

    fn score_holders(&self) -> Vec<ScoreHolder> {
        self.read_board(Board::holders).iter().map(|identity| self.holder_of(identity)).collect()
    }

    fn holder_name(&self, holder: &ScoreHolder) -> String {
        match holder {
            ScoreHolder::Player(player) => self.entity(*player).and_then(|entity| self.world.get::<Identity>(entity)).map_or_else(|| OFFLINE.into(), |identity| identity.0.name.clone()),
            ScoreHolder::Entity(id) => entities::find(self.world, *id).and_then(|entity| self.world.get::<UniqueId>(entity)).map_or_else(|| id.0.to_string(), |unique| unique.0.to_string()),
            ScoreHolder::Fake(name) => name.clone(),
            ScoreHolder::Absent { player: true, .. } => OFFLINE.into(),
            ScoreHolder::Absent { unique_id, .. } => unique_id.to_string(),
        }
    }
}
