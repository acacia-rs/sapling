use crate::{Block, BlockPos, Player, PlayerId, ShardCtx};

/// Worlds are named. A server's worlds are spread over its shards; the calls of [`ShardCtx`] that
/// name no world (`block`, `set_block`, `spawn`, `set_spawn`, `time`, `set_time`) act on the world
/// of the event being handled: the player's, or in `on_tick` the shard's first.
impl ShardCtx<'_> {
    /// Every world of the server, on this shard or another, sorted by name.
    pub fn worlds(&self) -> Vec<String> {
        self.backend.worlds()
    }

    /// The world the calls that name none act on.
    pub fn world(&self) -> String {
        self.backend.world()
    }

    /// The world a player on this shard is in.
    pub fn world_of(&self, player: PlayerId) -> Option<String> {
        self.backend.world_of(player)
    }

    /// The players in a world of this shard; none for a world on another.
    pub fn players_in(&self, world: &str) -> Vec<Player> {
        self.backend.players_in(world)
    }

    /// Runs `f` with `world` as the world its calls act on. `None`, with `f` not run, for a
    /// world that is not on this shard.
    pub fn within<R>(&mut self, world: &str, f: impl FnOnce(&mut ShardCtx<'_>) -> R) -> Option<R> {
        let before = self.backend.world();
        if !self.backend.focus(Some(world)) {
            return None;
        }
        let result = f(self);
        if !self.backend.focus(Some(&before)) {
            self.backend.focus(None);
        }
        Some(result)
    }

    /// [`ShardCtx::block`] in a named world of this shard.
    pub fn block_in(&mut self, world: &str, pos: BlockPos) -> Option<Block> {
        self.within(world, |ctx| ctx.block(pos)).flatten()
    }

    /// [`ShardCtx::set_block`] in a named world of this shard.
    pub fn set_block_in(&mut self, world: &str, pos: BlockPos, block: &Block) -> bool {
        self.within(world, |ctx| ctx.set_block(pos, block)).unwrap_or(false)
    }

    /// Moves a player to the spawn of another world, on this shard or another. The player stays
    /// online: no `on_quit` or `on_join`, but `on_world_change` once they are there. False, with
    /// nothing done, for a world the server does not have, the world they are in, and a player
    /// who is not in a world of this shard or is already being moved. A world on another shard
    /// can still refuse (removed meanwhile, shard stopped): the player then stays where they
    /// are and `on_transfer_failed` is called.
    pub fn transfer(&mut self, player: PlayerId, world: &str) -> bool {
        self.backend.transfer(player, world)
    }

    /// Shows `message` in the chat of every player in one world, on whichever shard it is.
    pub fn broadcast_in(&mut self, world: &str, message: &str) {
        self.backend.broadcast_in(world, message);
    }

    /// Shows `message` in the chat of every player on the server.
    pub fn broadcast_all(&mut self, message: &str) {
        self.backend.broadcast_all(message);
    }

    /// Makes a world on this shard from a template the server was built with: a copy that
    /// shares the template's blocks until they are changed and is never saved. False for an
    /// unknown template and a name that is taken.
    pub fn create_world(&mut self, name: &str, template: &str) -> bool {
        self.backend.create_world(name, template)
    }

    /// Removes a world of this shard that was made with [`ShardCtx::create_world`]; its players
    /// go to the shard's first world. False for any other world.
    pub fn remove_world(&mut self, name: &str) -> bool {
        self.backend.remove_world(name)
    }

    /// Puts a world made with [`ShardCtx::create_world`] back as its template has it, also on
    /// the screens of the players in it, who stay. False for any other world.
    pub fn reset_world(&mut self, name: &str) -> bool {
        self.backend.reset_world(name)
    }
}

/// The worlds side of [`crate::Backend`]. A world is named; a server's worlds are spread over its
/// shards, and a handler's shard holds some of them.
pub trait WorldBackend {
    /// Every world of the server, sorted.
    fn worlds(&self) -> Vec<String>;
    /// The world the calls that name none act on.
    fn world(&self) -> String;
    /// Makes `world` (`None`: the shard's first world) that world; false if it is not on this shard.
    fn focus(&mut self, world: Option<&str>) -> bool;
    fn world_of(&self, player: PlayerId) -> Option<String>;
    fn players_in(&self, world: &str) -> Vec<Player>;
    fn transfer(&mut self, player: PlayerId, world: &str) -> bool;
    fn broadcast_in(&mut self, world: &str, message: &str);
    fn broadcast_all(&mut self, message: &str);
    fn create_world(&mut self, name: &str, template: &str) -> bool;
    fn remove_world(&mut self, name: &str) -> bool;
    fn reset_world(&mut self, name: &str) -> bool;
}
