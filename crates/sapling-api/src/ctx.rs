use std::any::{Any, TypeId};

use crate::form::{Form, FormResponse};
use crate::{Block, BlockPos, BossBar, GameMode, Permission, Player, PlayerId, Position, Sidebar, Title};

/// Called with a form's raw answer (`None`: closed) on the shard of the player it was sent to.
pub type FormCallback = Box<dyn FnOnce(&mut ShardCtx<'_>, &Player, Option<&str>) + Send>;

/// What a shard offers its handlers. Implemented by sapling-shard; plugins use [`ShardCtx`].
pub trait Backend {
    fn tick(&self) -> u64;
    fn player_count(&self) -> usize;
    fn players(&self) -> Vec<Player>;
    fn position(&self, player: PlayerId) -> Option<Position>;
    fn spawn(&self) -> [f32; 3];
    fn set_spawn(&mut self, feet: [f32; 3]);
    fn time(&self) -> i64;
    fn set_time(&mut self, time: i64);
    fn block(&self, pos: BlockPos) -> Option<Block>;
    fn set_block(&mut self, pos: BlockPos, block: &Block) -> bool;
    fn kick(&mut self, player: PlayerId, message: &str);
    fn broadcast(&mut self, message: &str);
    fn send_message(&mut self, player: PlayerId, message: &str);
    fn teleport(&mut self, player: PlayerId, feet: [f32; 3]);
    fn permission(&self, player: PlayerId) -> Option<Permission>;
    fn set_permission(&mut self, player: PlayerId, permission: Permission);
    fn game_mode(&self, player: PlayerId) -> Option<GameMode>;
    fn set_game_mode(&mut self, player: PlayerId, mode: GameMode);
    fn send_form(&mut self, player: PlayerId, json: String, on_answer: FormCallback);
    fn set_sidebar(&mut self, player: PlayerId, sidebar: Option<&Sidebar>);
    fn set_boss_bar(&mut self, player: PlayerId, bar: Option<&BossBar>);
    fn send_title(&mut self, player: PlayerId, title: &Title);
    fn send_actionbar(&mut self, player: PlayerId, text: &str);
    fn send_toast(&mut self, player: PlayerId, title: &str, message: &str);
    /// The value kept under `key`, made with `init` if there is none yet.
    fn state(&mut self, key: TypeId, init: &dyn Fn() -> State) -> &mut State;
}

/// A value plugins keep on a shard.
pub type State = Box<dyn Any + Send + Sync>;

/// A handler's access to the shard it runs on, valid only for the call it was passed to. A call
/// naming a player who has left does nothing.
pub struct ShardCtx<'a> {
    backend: &'a mut dyn Backend,
}

impl<'a> ShardCtx<'a> {
    pub fn new(backend: &'a mut dyn Backend) -> Self {
        Self { backend }
    }

    /// Ticks this shard has run (20 per second).
    pub fn tick(&self) -> u64 {
        self.backend.tick()
    }

    /// Players in the world on this shard; those still loading do not count.
    pub fn player_count(&self) -> usize {
        self.backend.player_count()
    }

    /// The players [`ShardCtx::player_count`] counts.
    pub fn players(&self) -> impl Iterator<Item = Player> + use<> {
        self.backend.players().into_iter()
    }

    /// Where a player in the world is; `None` once they left.
    pub fn position(&self, player: PlayerId) -> Option<Position> {
        self.backend.position(player)
    }

    /// Where players appear when they join (feet).
    pub fn spawn(&self) -> [f32; 3] {
        self.backend.spawn()
    }

    /// Moves where players appear; a world with storage saves it as its spawn.
    pub fn set_spawn(&mut self, feet: [f32; 3]) {
        self.backend.set_spawn(feet);
    }

    /// The world's time of day counter: a day is 24000 long (0 sunrise, 6000 noon, 18000
    /// midnight) and the count runs on past it.
    pub fn time(&self) -> i64 {
        self.backend.time()
    }

    /// Sets the time of day for everyone in the world.
    pub fn set_time(&mut self, time: i64) {
        self.backend.set_time(time);
    }

    /// The block at a position; `None` where the world is not loaded or does not reach.
    pub fn block(&self, pos: BlockPos) -> Option<Block> {
        self.backend.block(pos)
    }

    /// Puts a block into the world and onto the screen of everyone who sees the place. False,
    /// with nothing changed, for a block the game does not have and where [`ShardCtx::block`]
    /// is `None`. Raises no event.
    pub fn set_block(&mut self, pos: BlockPos, block: &Block) -> bool {
        self.backend.set_block(pos, block)
    }

    /// Disconnects a player with `message` on their screen. `on_quit` follows on a later tick.
    pub fn kick(&mut self, player: PlayerId, message: &str) {
        self.backend.kick(player, message);
    }

    /// Shows `message` in the chat of every player in the world. `§` codes format it.
    pub fn broadcast(&mut self, message: &str) {
        self.backend.broadcast(message);
    }

    /// Shows `message` in one player's chat.
    pub fn send_message(&mut self, player: PlayerId, message: &str) {
        self.backend.send_message(player, message);
    }

    /// Moves a player so that their feet are at `feet`, keeping where they look.
    pub fn teleport(&mut self, player: PlayerId, feet: [f32; 3]) {
        self.backend.teleport(player, feet);
    }

    pub fn permission(&self, player: PlayerId) -> Option<Permission> {
        self.backend.permission(player)
    }

    /// Changes which commands a player may use; their client's command list follows.
    pub fn set_permission(&mut self, player: PlayerId, permission: Permission) {
        self.backend.set_permission(player, permission);
    }

    pub fn game_mode(&self, player: PlayerId) -> Option<GameMode> {
        self.backend.game_mode(player)
    }

    pub fn set_game_mode(&mut self, player: PlayerId, mode: GameMode) {
        self.backend.set_game_mode(player, mode);
    }

    /// Opens `form` on a player's screen. `on_answer` runs once, on this shard, when the player
    /// submits or closes it; never if they leave first. An answer the form does not allow counts
    /// as a close (see [`crate::form`]).
    pub fn send_form<F>(&mut self, player: PlayerId, form: F, on_answer: impl FnOnce(&mut ShardCtx<'_>, &Player, FormResponse<F::Response>) + Send + 'static)
    where
        F: Form + Send + 'static,
    {
        let json = form.to_json();
        let callback = move |ctx: &mut ShardCtx<'_>, player: &Player, data: Option<&str>| on_answer(ctx, player, FormResponse::of(&form, data));
        self.backend.send_form(player, json, Box::new(callback));
    }

    /// Shows `sidebar` to a player, or changes the one they have: only lines that differ are sent.
    pub fn set_sidebar(&mut self, player: PlayerId, sidebar: &Sidebar) {
        self.backend.set_sidebar(player, Some(sidebar));
    }

    pub fn remove_sidebar(&mut self, player: PlayerId) {
        self.backend.set_sidebar(player, None);
    }

    /// Shows `bar` to a player, or changes the one they have.
    pub fn set_boss_bar(&mut self, player: PlayerId, bar: &BossBar) {
        self.backend.set_boss_bar(player, Some(bar));
    }

    pub fn remove_boss_bar(&mut self, player: PlayerId) {
        self.backend.set_boss_bar(player, None);
    }

    pub fn send_title(&mut self, player: PlayerId, title: &Title) {
        self.backend.send_title(player, title);
    }

    /// Shows a line above a player's hotbar for a few seconds.
    pub fn send_actionbar(&mut self, player: PlayerId, text: &str) {
        self.backend.send_actionbar(player, text);
    }

    /// Slides a notification in at the top of a player's screen.
    pub fn send_toast(&mut self, player: PlayerId, title: &str, message: &str) {
        self.backend.send_toast(player, title, message);
    }

    /// This shard's `T`, `T::default()` on first use: what handlers, commands and form callbacks
    /// of a plugin share. Keyed by type, so a plugin's own type is its own state.
    pub fn state<T: Default + Send + Sync + 'static>(&mut self) -> &mut T {
        let state = self.backend.state(TypeId::of::<T>(), &|| Box::new(T::default()));
        state.downcast_mut().expect("state is stored under its own type")
    }
}
