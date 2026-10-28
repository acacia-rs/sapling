//! What a player sees on top of the world: sidebar, boss bar, titles. Plain data; a shard turns
//! changes into packets when it is handed to [`crate::ShardCtx`].

use crate::command::RawText;
use crate::{PlayerId, ShardCtx};

/// The scoreboard on the right of the screen: a title over at most [`Sidebar::MAX_LINES`] lines.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Sidebar {
    pub title: String,
    pub lines: Vec<String>,
}

impl Sidebar {
    /// Lines past this many are not shown.
    pub const MAX_LINES: usize = 15;

    pub fn new(title: impl Into<String>) -> Self {
        Self { title: title.into(), lines: Vec::new() }
    }

    pub fn line(mut self, text: impl Into<String>) -> Self {
        self.lines.push(text.into());
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BossBarColour {
    Pink,
    Blue,
    Red,
    Green,
    Yellow,
    #[default]
    Purple,
    White,
}

/// The bar at the top of the screen. A player has at most one.
#[derive(Debug, Clone, PartialEq)]
pub struct BossBar {
    pub text: String,
    /// How full the bar is, 0 to 1.
    pub progress: f32,
    pub colour: BossBarColour,
}

impl BossBar {
    /// A full purple bar.
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into(), progress: 1.0, colour: BossBarColour::default() }
    }

    /// Clamped to 0..=1; not-a-number counts as empty.
    pub fn progress(mut self, progress: f32) -> Self {
        self.progress = if progress.is_nan() { 0.0 } else { progress.clamp(0.0, 1.0) };
        self
    }

    pub fn colour(mut self, colour: BossBarColour) -> Self {
        self.colour = colour;
        self
    }
}

/// Large text across the middle of the screen, with an optional smaller line under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Title {
    pub title: String,
    pub subtitle: Option<String>,
    /// Ticks to fade in, stay and fade out.
    pub fade_in: u32,
    pub stay: u32,
    pub fade_out: u32,
}

impl Title {
    /// Vanilla's timing: half a second in, three and a half shown, one out.
    pub fn new(title: impl Into<String>) -> Self {
        Self { title: title.into(), subtitle: None, fade_in: 10, stay: 70, fade_out: 20 }
    }

    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    pub fn times(mut self, fade_in: u32, stay: u32, fade_out: u32) -> Self {
        (self.fade_in, self.stay, self.fade_out) = (fade_in, stay, fade_out);
        self
    }
}

/// One step of what `/title` and `/titleraw` do to a player's screen, each its own packet: a
/// subtitle and times wait for the next title.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TitleAction {
    Title(String),
    Subtitle(String),
    ActionBar(String),
    RawTitle(RawText),
    RawSubtitle(RawText),
    RawActionBar(RawText),
    /// Ticks to fade in, stay and fade out; BDS passes -1 on as it is given.
    Times { fade_in: i32, stay: i32, fade_out: i32 },
    /// Takes the title off the screen.
    Clear,
    /// Clear, and the subtitle and times back to the client's own.
    Reset,
}

impl ShardCtx<'_> {
    /// Does one [`TitleAction`] to a player's screen, as `/title` does.
    pub fn title_action(&mut self, player: PlayerId, action: &TitleAction) {
        self.backend.title_action(player, action);
    }
}

/// The `/title` side of [`crate::Backend`], and the rest of a client's screen (screen.rs).
pub trait TitleBackend {
    fn title_action(&mut self, player: PlayerId, action: &TitleAction);
    fn screen_action(&mut self, _player: PlayerId, _action: &crate::ScreenAction) {}
    /// The inputs stopped for a player, as UpdateClientInputLocks' bits.
    fn input_locks(&self, _player: PlayerId) -> u32 {
        0
    }
    fn set_input_locks(&mut self, _player: PlayerId, _locks: u32) {}
    fn animate(&mut self, _players: &[PlayerId], _entities: &[crate::EntityId], _animation: &crate::Animation) {}
}

#[cfg(test)]
impl TitleBackend for crate::testing::Fake {
    fn title_action(&mut self, player: PlayerId, action: &TitleAction) {
        self.log.push(format!("title action {} {action:?}", player.0));
    }
}
