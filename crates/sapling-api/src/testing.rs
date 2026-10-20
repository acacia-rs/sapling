//! A backend that records what handlers ask of it, for this crate's tests.

use std::any::TypeId;
use std::collections::HashMap;

use crate::ctx::State;
use crate::{Backend, Block, BlockPos, BossBar, Difficulty, FormCallback, GameMode, Item, Permission, Player, PlayerId, Position, RuleValue, Sidebar, Slot, Title};

#[derive(Default)]
pub struct Fake {
    pub players: Vec<Player>,
    /// One line per call that changes something.
    pub log: Vec<String>,
    pub forms: Vec<(PlayerId, String, FormCallback)>,
    /// What was set; everything else at y >= 0 is air, and below there is no world.
    pub blocks: HashMap<BlockPos, Block>,
    /// One inventory for every player.
    pub items: HashMap<Slot, Item>,
    /// The world in focus; `None` is "hub".
    pub world: Option<String>,
    rules: HashMap<String, RuleValue>,
    state: HashMap<TypeId, State>,
}

pub fn player(id: u64, name: &str) -> Player {
    Player { id: PlayerId(id), name: name.into(), uuid: uuid::Uuid::nil(), xuid: String::new() }
}

impl Fake {
    pub fn with(names: &[&str]) -> Self {
        Self { players: names.iter().zip(1..).map(|(name, id)| player(id, name)).collect(), ..Self::default() }
    }
}

impl crate::WorldBackend for Fake {
    fn worlds(&self) -> Vec<String> {
        vec!["arena".into(), "hub".into()]
    }
    fn world(&self) -> String {
        self.world.clone().unwrap_or_else(|| "hub".into())
    }
    fn focus(&mut self, world: Option<&str>) -> bool {
        let known = world.is_none_or(|world| self.worlds().iter().any(|w| w == world));
        if known {
            self.world = world.map(str::to_owned);
        }
        known
    }
    fn world_of(&self, player: PlayerId) -> Option<String> {
        self.players.iter().any(|p| p.id == player).then(|| "hub".into())
    }
    fn players_in(&self, world: &str) -> Vec<Player> {
        if world == "hub" { self.players.clone() } else { Vec::new() }
    }
    fn transfer(&mut self, player: PlayerId, world: &str) -> bool {
        self.log.push(format!("transfer {} {world}", player.0));
        world == "arena"
    }
    fn broadcast_in(&mut self, world: &str, message: &str) {
        self.log.push(format!("broadcast in {world} {message}"));
    }
    fn broadcast_all(&mut self, message: &str) {
        self.log.push(format!("broadcast all {message}"));
    }
    fn create_world(&mut self, name: &str, template: &str) -> bool {
        self.log.push(format!("create {name} from {template}"));
        true
    }
    fn remove_world(&mut self, name: &str) -> bool {
        self.log.push(format!("remove {name}"));
        true
    }
    fn reset_world(&mut self, name: &str) -> bool {
        self.log.push(format!("reset {name}"));
        true
    }
    /// Two rules.
    fn game_rules(&self) -> Vec<(String, RuleValue)> {
        let defaults = [("doDayLightCycle", RuleValue::Bool(true)), ("randomTickSpeed", RuleValue::Int(1))];
        defaults.into_iter().map(|(name, value)| (name.to_owned(), self.rules.get(name).copied().unwrap_or(value))).collect()
    }
    fn set_game_rule(&mut self, name: &str, value: RuleValue) -> bool {
        let Some((rule, now)) = self.game_rules().into_iter().find(|(rule, _)| rule.eq_ignore_ascii_case(name)) else { return false };
        let same_kind = std::mem::discriminant(&now) == std::mem::discriminant(&value);
        if same_kind {
            self.log.push(format!("rule {rule} {value}"));
            self.rules.insert(rule, value);
        }
        same_kind
    }
}

/// No entities: spawns are logged and refused.
impl crate::EntityBackend for Fake {
    fn spawn_entity(&mut self, kind: &str, feet: [f32; 3]) -> Option<crate::EntityId> {
        self.log.push(format!("spawn {kind} at {feet:?}"));
        None
    }
    fn entity(&self, _: crate::EntityId) -> Option<crate::EntityInfo> {
        None
    }
    fn entities(&self) -> Vec<crate::EntityInfo> {
        Vec::new()
    }
    fn move_entity(&mut self, _: crate::EntityId, _: [f32; 3]) -> bool {
        false
    }
    fn set_velocity(&mut self, _: crate::EntityId, _: [f32; 3]) -> bool {
        false
    }
    fn remove_entity(&mut self, _: crate::EntityId) -> bool {
        false
    }
    fn kill_entity(&mut self, _: crate::EntityId) -> bool {
        false
    }

    fn give_entity_effect(&mut self, _: crate::EntityId, _: crate::Effect, _: u32, _: u8) -> bool {
        false
    }
}

impl crate::HealthBackend for Fake {
    fn health(&self, _: PlayerId) -> Option<f32> {
        None
    }
    fn hurt_player(&mut self, _: PlayerId, _: f32, _: crate::DamageCause) -> bool {
        false
    }
    fn kill_player(&mut self, _: PlayerId) -> bool {
        false
    }
}

impl crate::WeatherBackend for Fake {
    fn weather(&self) -> crate::Weather {
        crate::Weather::Clear
    }
    fn set_weather(&mut self, _: crate::Weather, _: Option<u32>) {}
}

impl crate::ExperienceBackend for Fake {
    fn experience(&self, _: PlayerId) -> Option<(i32, f32)> {
        None
    }
    fn give_experience(&mut self, _: PlayerId, _: u32) -> bool {
        false
    }
    fn add_levels(&mut self, _: PlayerId, _: i32) -> bool {
        false
    }
}

impl crate::EffectBackend for Fake {
    fn give_effect(&mut self, _: PlayerId, _: crate::Effect, _: u32, _: u8, _: bool) -> bool {
        false
    }
    fn remove_effect(&mut self, _: PlayerId, _: crate::Effect) -> bool {
        false
    }
    fn clear_effects(&mut self, _: PlayerId) -> bool {
        false
    }
}

impl Backend for Fake {
    fn tick(&self) -> u64 {
        40
    }
    fn player_count(&self) -> usize {
        self.players.len()
    }
    fn players(&self) -> Vec<Player> {
        self.players.clone()
    }
    fn position(&self, player: PlayerId) -> Option<Position> {
        self.players.iter().any(|p| p.id == player).then_some(Position { feet: [1.0, 2.0, 3.0], yaw: 90.0, pitch: 0.0 })
    }
    fn spawn(&self) -> [f32; 3] {
        [0.5, 65.0, 0.5]
    }
    fn set_spawn(&mut self, feet: [f32; 3]) {
        self.log.push(format!("spawn {feet:?}"));
    }
    fn time(&self) -> i64 {
        6000
    }
    fn set_time(&mut self, time: i64) {
        self.log.push(format!("time {time}"));
    }
    fn difficulty(&self) -> Difficulty {
        Difficulty::Peaceful
    }
    fn set_difficulty(&mut self, difficulty: Difficulty) {
        self.log.push(format!("difficulty {difficulty:?}"));
    }
    fn block(&self, pos: BlockPos) -> Option<Block> {
        (pos[1] >= 0).then(|| self.blocks.get(&pos).cloned().unwrap_or_else(Block::air))
    }
    /// No circuits run here.
    fn power_at(&self, _pos: BlockPos) -> u8 {
        0
    }
    /// Neighbours do nothing here: the same as [`Fake::set_block`].
    fn update_block(&mut self, pos: BlockPos, block: &Block) -> bool {
        self.set_block(pos, block)
    }

    /// Nothing drops here: the block only goes.
    fn break_block(&mut self, pos: BlockPos) -> bool {
        let broken = self.blocks.remove(&pos).is_some_and(|block| !block.is_air());
        if broken {
            self.log.push(format!("break {pos:?}"));
        }
        broken
    }

    fn set_container_item(&mut self, pos: BlockPos, slot: usize, item: Option<&Item>) -> bool {
        self.log.push(format!("container {pos:?} {slot} {:?}", item.map(|item| (&item.name, item.count))));
        true
    }

    /// A chest wherever one was set.
    fn container_size(&mut self, pos: BlockPos) -> Option<usize> {
        self.blocks.get(&pos).filter(|block| block.name == "minecraft:chest").map(|_| 27)
    }

    fn set_block(&mut self, pos: BlockPos, block: &Block) -> bool {
        let known = pos[1] >= 0 && block.name.starts_with("minecraft:");
        if known {
            self.blocks.insert(pos, block.clone());
            self.log.push(format!("block {pos:?} {} [{}]", block.name, block.properties));
        }
        known
    }
    fn kick(&mut self, player: PlayerId, message: &str) {
        self.log.push(format!("kick {} {message}", player.0));
    }
    fn broadcast(&mut self, message: &str) {
        self.log.push(format!("broadcast {message}"));
    }
    fn send_message(&mut self, player: PlayerId, message: &str) {
        self.log.push(format!("message {} {message}", player.0));
    }
    fn send_translation(&mut self, player: PlayerId, key: &str, parameters: &[String]) {
        self.log.push(format!("translation {} {key} {parameters:?}", player.0));
    }
    fn teleport(&mut self, player: PlayerId, feet: [f32; 3]) {
        self.log.push(format!("teleport {} {feet:?}", player.0));
    }
    fn permission(&self, _: PlayerId) -> Option<Permission> {
        Some(Permission::Member)
    }
    fn set_permission(&mut self, player: PlayerId, permission: Permission) {
        self.log.push(format!("permission {} {permission:?}", player.0));
    }
    fn game_mode(&self, _: PlayerId) -> Option<GameMode> {
        Some(GameMode::Survival)
    }
    fn set_game_mode(&mut self, player: PlayerId, mode: GameMode) {
        self.log.push(format!("game mode {} {mode:?}", player.0));
    }
    fn item(&self, _: PlayerId, slot: Slot) -> Option<Item> {
        self.items.get(&slot).cloned()
    }
    fn set_item(&mut self, player: PlayerId, slot: Slot, item: Option<&Item>) -> bool {
        self.log.push(format!("item {} {slot:?} {:?}", player.0, item.map(|item| (&item.name, item.count))));
        match item {
            Some(item) => drop(self.items.insert(slot, item.clone())),
            None => drop(self.items.remove(&slot)),
        }
        true
    }
    fn give(&mut self, player: PlayerId, item: &Item) -> u8 {
        self.log.push(format!("give {} {} x{}", player.0, item.name, item.count));
        0
    }
    fn clear_inventory(&mut self, player: PlayerId) -> u32 {
        self.log.push(format!("clear {}", player.0));
        let removed = self.items.values().map(|item| u32::from(item.count)).sum();
        self.items.clear();
        removed
    }
    fn held_slot(&self, _: PlayerId) -> Option<u8> {
        Some(2)
    }
    fn send_form(&mut self, player: PlayerId, json: String, on_answer: FormCallback) {
        self.forms.push((player, json, on_answer));
    }
    fn set_sidebar(&mut self, player: PlayerId, sidebar: Option<&Sidebar>) {
        self.log.push(format!("sidebar {} {:?}", player.0, sidebar.map(|s| (&s.title, &s.lines))));
    }
    fn set_boss_bar(&mut self, player: PlayerId, bar: Option<&BossBar>) {
        self.log.push(format!("boss bar {} {:?}", player.0, bar.map(|b| (&b.text, b.progress, b.colour))));
    }
    fn send_title(&mut self, player: PlayerId, title: &Title) {
        self.log.push(format!("title {} {} {:?}", player.0, title.title, title.subtitle));
    }
    fn send_actionbar(&mut self, player: PlayerId, text: &str) {
        self.log.push(format!("actionbar {} {text}", player.0));
    }
    fn send_toast(&mut self, player: PlayerId, title: &str, message: &str) {
        self.log.push(format!("toast {} {title} {message}", player.0));
    }
    fn state(&mut self, key: TypeId, init: &dyn Fn() -> State) -> &mut State {
        self.state.entry(key).or_insert_with(init)
    }
}
