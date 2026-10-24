use std::any::{Any, TypeId};

use crate::form::{Form, FormResponse};
use crate::effect::EffectBackend;
use crate::entity::EntityBackend;
use crate::health::HealthBackend;
use crate::world::WorldBackend;
use crate::{Block, BlockPos, BossBar, Difficulty, GameMode, Item, Permission, Player, PlayerId, Position, Sidebar, Slot, Title};

/// Called with a form's raw answer (`None`: closed) on the shard of the player it was sent to.
pub type FormCallback = Box<dyn FnOnce(&mut ShardCtx<'_>, &Player, Option<&str>) + Send>;

/// What a shard offers its handlers. Implemented by sapling-shard; plugins use [`ShardCtx`].
pub trait Backend: WorldBackend + EntityBackend + HealthBackend + EffectBackend + crate::ExperienceBackend + crate::WeatherBackend + crate::ChatBackend {
    fn tick(&self) -> u64;
    fn player_count(&self) -> usize;
    fn players(&self) -> Vec<Player>;
    fn position(&self, player: PlayerId) -> Option<Position>;
    fn spawn(&self) -> [f32; 3];
    fn set_spawn(&mut self, feet: [f32; 3]);
    fn time(&self) -> i64;
    fn set_time(&mut self, time: i64);
    fn difficulty(&self) -> Difficulty;
    fn set_difficulty(&mut self, difficulty: Difficulty);
    fn block(&self, pos: BlockPos) -> Option<Block>;
    fn power_at(&self, pos: BlockPos) -> u8;
    fn set_block(&mut self, pos: BlockPos, block: &Block) -> bool;
    fn update_block(&mut self, pos: BlockPos, block: &Block) -> bool;
    fn break_block(&mut self, pos: BlockPos) -> bool;
    fn set_container_item(&mut self, pos: BlockPos, slot: usize, item: Option<&Item>) -> bool;
    fn container_size(&mut self, pos: BlockPos) -> Option<usize>;
    fn kick(&mut self, player: PlayerId, message: &str);
    fn broadcast(&mut self, message: &str);
    fn send_message(&mut self, player: PlayerId, message: &str);
    fn send_translation(&mut self, player: PlayerId, key: &str, parameters: &[String]);
    fn teleport(&mut self, player: PlayerId, feet: [f32; 3]);
    /// Told to the player with its next teleport.
    fn look(&mut self, _player: PlayerId, _yaw: f32, _pitch: f32) {}
    /// See `ShardCtx::dragon_fight`.
    fn dragon_fight(&self) -> Option<crate::DragonFight> { None }
    fn permission(&self, player: PlayerId) -> Option<Permission>;
    fn set_permission(&mut self, player: PlayerId, permission: Permission);
    fn game_mode(&self, player: PlayerId) -> Option<GameMode>;
    fn set_game_mode(&mut self, player: PlayerId, mode: GameMode);
    fn item(&self, player: PlayerId, slot: Slot) -> Option<Item>;
    fn set_item(&mut self, player: PlayerId, slot: Slot, item: Option<&Item>) -> bool;
    fn give(&mut self, player: PlayerId, item: &Item) -> u8;
    fn clear_inventory(&mut self, player: PlayerId) -> u32;
    fn held_slot(&self, player: PlayerId) -> Option<u8>;
    fn send_form(&mut self, player: PlayerId, json: String, on_answer: FormCallback);
    fn set_sidebar(&mut self, player: PlayerId, sidebar: Option<&Sidebar>);
    fn set_boss_bar(&mut self, player: PlayerId, bar: Option<&BossBar>);
    fn send_title(&mut self, player: PlayerId, title: &Title);
    fn send_actionbar(&mut self, player: PlayerId, text: &str);
    fn send_toast(&mut self, player: PlayerId, title: &str, message: &str);
    /// The value kept under `key`, made with `init` if there is none yet.
    fn state(&mut self, key: TypeId, init: &dyn Fn() -> State) -> &mut State;
    /// Keeps a change to the shard's listeners until the event that made it is done.
    fn defer(&mut self, change: crate::ListenerChange);
}

/// A value plugins keep on a shard.
pub type State = Box<dyn Any + Send + Sync>;

/// A handler's access to the shard it runs on, valid only for the call it was passed to. A call
/// naming a player who has left does nothing.
pub struct ShardCtx<'a> {
    pub(crate) backend: &'a mut dyn Backend,
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

    pub fn difficulty(&self) -> Difficulty {
        self.backend.difficulty()
    }

    /// Sets the difficulty for everyone on the shard, and for who joins later. It is not saved.
    pub fn set_difficulty(&mut self, difficulty: Difficulty) {
        self.backend.set_difficulty(difficulty);
    }

    /// The block at a position; `None` where the world is not loaded or does not reach.
    pub fn block(&self, pos: BlockPos) -> Option<Block> {
        self.backend.block(pos)
    }

    /// The redstone signal, 0 to 15, the block at a position got when circuits were last worked
    /// out (every second tick); 0 where nothing powers it.
    pub fn power_at(&self, pos: BlockPos) -> u8 {
        self.backend.power_at(pos)
    }

    /// Puts a block into the world and onto the screen of everyone who sees the place. False,
    /// with nothing changed, for a block the game does not have and where [`ShardCtx::block`]
    /// is `None`. Raises no event.
    pub fn set_block(&mut self, pos: BlockPos, block: &Block) -> bool {
        self.backend.set_block(pos, block)
    }

    /// [`ShardCtx::set_block`], and then what is around the block reacts as to a player's place:
    /// liquids move, sand over a gap falls, a torch that lost its block breaks. What vanilla's
    /// `/setblock` does. A cell that already holds `block` is left alone, and that is true too.
    pub fn update_block(&mut self, pos: BlockPos, block: &Block) -> bool {
        self.backend.update_block(pos, block)
    }

    /// Breaks the block at `pos` as a hand would: its fragments fly, it leaves what it drops, and
    /// what is around reacts as to [`ShardCtx::update_block`]. False for air and a place that is
    /// not loaded. What vanilla's `/setblock .. destroy` does first.
    pub fn break_block(&mut self, pos: BlockPos) -> bool {
        self.backend.break_block(pos)
    }

    /// Puts `item` (`None`: nothing) into slot `slot` of the container block at `pos` (a chest,
    /// a furnace, a hopper; a double chest's slots run on through its other half), on the screens
    /// of those who have it open too. False for a block that holds no items, a slot it does not
    /// have, and an item the game does not have.
    pub fn set_container_item(&mut self, pos: BlockPos, slot: usize, item: Option<&Item>) -> bool {
        self.backend.set_container_item(pos, slot, item)
    }

    /// How many slots the container block at `pos` has (54 for a double chest); `None` for a
    /// block that holds no items. `&mut` because a container nobody has open is read in.
    pub fn container_size(&mut self, pos: BlockPos) -> Option<usize> {
        self.backend.container_size(pos)
    }

    /// Disconnects a player with `message` on their screen. `on_quit` follows on a later tick.
    pub fn kick(&mut self, player: PlayerId, message: &str) {
        self.backend.kick(player, message);
    }

    /// Shows `message` in the chat of every player on this shard, whatever world they are in
    /// (see [`ShardCtx::broadcast_in`] and [`ShardCtx::broadcast_all`]). `§` codes format it.
    pub fn broadcast(&mut self, message: &str) {
        self.backend.broadcast(message);
    }

    /// Shows `message` in one player's chat.
    pub fn send_message(&mut self, player: PlayerId, message: &str) {
        self.backend.send_message(player, message);
    }

    /// Shows one player the client's line under translation key `key` (`commands.tp.successVictim`),
    /// in their language, with `parameters` filled in.
    pub fn send_translation(&mut self, player: PlayerId, key: &str, parameters: &[String]) {
        self.backend.send_translation(player, key, parameters);
    }

    /// Moves a player so that their feet are at `feet`, keeping where they look.
    pub fn teleport(&mut self, player: PlayerId, feet: [f32; 3]) {
        self.backend.teleport(player, feet);
    }

    /// [`Self::teleport`], turning the player to look along `yaw` and `pitch`.
    pub fn teleport_facing(&mut self, player: PlayerId, feet: [f32; 3], [yaw, pitch]: [f32; 2]) {
        self.backend.look(player, yaw, pitch);
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

    /// What a slot of a player's inventory holds; `None` when it is empty or does not exist.
    pub fn item(&self, player: PlayerId, slot: Slot) -> Option<Item> {
        self.backend.item(player, slot)
    }

    /// Puts a stack into a slot, replacing what it held, or empties it. False, with nothing
    /// changed, for a slot that does not exist and an item the game does not have.
    pub fn set_item(&mut self, player: PlayerId, slot: Slot, item: Option<&Item>) -> bool {
        self.backend.set_item(player, slot, item)
    }

    /// Adds a stack to a player's hotbar and inventory, onto stacks of the same item first, and
    /// returns how many did not fit.
    pub fn give(&mut self, player: PlayerId, item: &Item) -> u8 {
        self.backend.give(player, item)
    }

    /// Empties a player's hotbar, inventory, armour and offhand; how many items went.
    pub fn clear_inventory(&mut self, player: PlayerId) -> u32 {
        self.backend.clear_inventory(player)
    }

    /// The hotbar slot a player has in hand (0 to 8).
    pub fn held_slot(&self, player: PlayerId) -> Option<u8> {
        self.backend.held_slot(player)
    }

    pub fn held_item(&self, player: PlayerId) -> Option<Item> {
        self.backend.item(player, Slot::Main(self.backend.held_slot(player)?))
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
    /// of a plugin share. Keyed by type, so a plugin's own type is its own state. Each shard has
    /// its own: nothing here follows a player to a world on another shard.
    pub fn state<T: Default + Send + Sync + 'static>(&mut self) -> &mut T {
        let state = self.backend.state(TypeId::of::<T>(), &|| Box::new(T::default()));
        state.downcast_mut().expect("state is stored under its own type")
    }
}
