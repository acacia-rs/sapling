//! The builder's worlds: the default one, others by shard, dimensions, templates and where
//! players join.

use std::sync::Arc;

use sapling_api::{Difficulty, GameMode};
use sapling_shard::ShardConfig;
use sapling_world::{Template, World, WorldFolder};

use super::ServerBuilder;
use crate::layout::{self, WorldSpec};

impl ServerBuilder {
    /// The default world, on shard 0, and where in it players appear (feet position). Without
    /// one it is a stone platform in the void. Handlers know it as `"default"`, or by
    /// [`ServerBuilder::default_world_name`]; players join it unless
    /// [`ServerBuilder::join_world`] names another.
    pub fn world(mut self, world: World, spawn: [f32; 3]) -> Self {
        self.world = Some(world);
        self.shard.spawn = spawn;
        self
    }

    /// The default world opened from a folder, with the spawn, default game mode and difficulty
    /// its `level.dat` names. Call [`ServerBuilder::game_mode`] after it to override the folder's.
    /// Its Nether is [`WorldFolder::nether`], for [`ServerBuilder::nether`].
    pub fn folder(mut self, opened: WorldFolder) -> Self {
        let game_mode = opened.game_type.and_then(|saved| [(0, GameMode::Survival), (1, GameMode::Creative), (2, GameMode::Adventure), (6, GameMode::Spectator)].into_iter().find(|(id, _)| *id == saved));
        let difficulty = opened.difficulty.and_then(|saved| [Difficulty::Peaceful, Difficulty::Easy, Difficulty::Normal, Difficulty::Hard].get(usize::try_from(saved).ok()?).copied());
        self.shard.game_mode = game_mode.map_or(self.shard.game_mode, |(_, mode)| mode);
        self.shard.difficulty = difficulty.unwrap_or(self.shard.difficulty);
        self.world(opened.world, opened.spawn)
    }

    /// The default world's Nether: a world of dimension 1 (`World::with_storage_in`,
    /// `WorldFolder::nether`) on the same shard, named `<default>:nether`, reached by portal and
    /// `/execute in nether`, sharing the default world's time and rules.
    pub fn nether(mut self, world: World) -> Self {
        self.dimensions.push((None, world));
        self
    }

    /// [`ServerBuilder::nether`] (or another dimension) of the world named `of`.
    pub fn dimension_of(mut self, of: impl Into<String>, world: World) -> Self {
        self.dimensions.push((Some(of.into()), world));
        self
    }

    /// The name handlers know the default world by (default `"default"`).
    pub fn default_world_name(mut self, name: impl Into<String>) -> Self {
        self.shard.world_id = name.into();
        self
    }

    /// Another world on shard 0, for `ShardCtx::transfer`.
    pub fn add_world(self, name: impl Into<String>, world: World, spawn: [f32; 3]) -> Self {
        self.add_world_on(0, name, world, spawn)
    }

    /// Another world on the shard of this index. Shards are threads: the server runs as many as
    /// the highest index named, plus one, and each needs a world.
    pub fn add_world_on(mut self, shard: usize, name: impl Into<String>, world: World, spawn: [f32; 3]) -> Self {
        self.worlds.push(WorldSpec { name: name.into(), world, spawn, shard });
        self
    }

    /// The stone platform in the void as another world on the shard of this index.
    pub fn add_platform_on(self, shard: usize, name: impl Into<String>) -> Self {
        self.add_world_on(shard, name, layout::platform_world(), ShardConfig::default().spawn)
    }

    /// A map every shard can make worlds from at runtime (`ShardCtx::create_world`), with where
    /// players appear in such a world (feet). The worlds share its blocks until they change them.
    pub fn template(mut self, name: impl Into<String>, template: Arc<Template>, spawn: [f32; 3]) -> Self {
        self.templates.push((name.into(), template, spawn));
        self
    }

    /// The world players enter when they join the server (default: the default world).
    pub fn join_world(mut self, name: impl Into<String>) -> Self {
        self.join = Some(name.into());
        self
    }
}
