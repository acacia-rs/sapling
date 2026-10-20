use std::fmt;

use crate::{Block, BlockPos, Player, PlayerId, ShardCtx};

/// A game rule's value: a switch or a number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleValue {
    Bool(bool),
    Int(i32),
}

/// As `/gamerule` shows it: `true`, `5`.
impl fmt::Display for RuleValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bool(on) => write!(f, "{on}"),
            Self::Int(n) => write!(f, "{n}"),
        }
    }
}

/// Worlds are named. A server's worlds are spread over its shards; the calls of [`ShardCtx`] that
/// name no world (`block`, `set_block`, `spawn`, `set_spawn`, `time`, `set_time`, the game rules)
/// act on the world of the event being handled: the player's, or in `on_tick` the shard's first.
impl ShardCtx<'_> {
    /// The world's game rules, each under its own name (`doDayLightCycle`), in vanilla's order.
    /// Sapling acts on `doDayLightCycle`; the others are kept and saved for plugins to read.
    pub fn game_rules(&self) -> Vec<(String, RuleValue)> {
        self.backend.game_rules()
    }

    /// The game rule `name`, in any case: its own name and value.
    pub fn game_rule(&self, name: &str) -> Option<(String, RuleValue)> {
        self.game_rules().into_iter().find(|(rule, _)| rule.eq_ignore_ascii_case(name))
    }

    /// Sets the game rule `name`, in any case, and tells the world's players of a change; a world
    /// with storage saves it. False, with nothing changed, for no such rule or a value of the other kind.
    pub fn set_game_rule(&mut self, name: &str, value: RuleValue) -> bool {
        self.backend.set_game_rule(name, value)
    }

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

    /// The dimension of the world a player on this shard is in: 0 overworld, 1 nether, 2 end.
    pub fn dimension_of(&self, player: PlayerId) -> Option<i32> {
        self.backend.dimension_of(player)
    }

    /// Puts a player's feet at `feet` in `dimension` of its world (its overworld's Nether, or
    /// back), changing dimension as BDS does; in its own dimension this is a teleport. False for
    /// a world without that dimension and a player who is not here or is being moved.
    pub fn travel(&mut self, player: PlayerId, dimension: i32, feet: [f32; 3]) -> bool {
        self.backend.travel(player, dimension, feet)
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
    fn game_rules(&self) -> Vec<(String, RuleValue)>;
    fn set_game_rule(&mut self, name: &str, value: RuleValue) -> bool;
    fn dimension_of(&self, _player: PlayerId) -> Option<i32> {
        None
    }
    fn travel(&mut self, _player: PlayerId, _dimension: i32, _feet: [f32; 3]) -> bool {
        false
    }
}
