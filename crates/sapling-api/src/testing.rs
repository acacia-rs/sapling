//! A backend that records what handlers ask of it, for this crate's tests.

use std::any::TypeId;
use std::collections::HashMap;

use crate::ctx::State;
use crate::{Backend, BossBar, FormCallback, GameMode, Permission, Player, PlayerId, Position, Sidebar, Title};

#[derive(Default)]
pub struct Fake {
    pub players: Vec<Player>,
    /// One line per call that changes something.
    pub log: Vec<String>,
    pub forms: Vec<(PlayerId, String, FormCallback)>,
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
    fn kick(&mut self, player: PlayerId, message: &str) {
        self.log.push(format!("kick {} {message}", player.0));
    }
    fn broadcast(&mut self, message: &str) {
        self.log.push(format!("broadcast {message}"));
    }
    fn send_message(&mut self, player: PlayerId, message: &str) {
        self.log.push(format!("message {} {message}", player.0));
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
