//! What commands change on a player's client besides titles (hud.rs): the camera and its shake,
//! fog, HUD elements, the recipes it knows, the inputs it may use, and animations it plays. Plain
//! data; the shard sends the packets as BDS sends them (research/captures/bds-screen.txt,
//! bds-controls.txt).

use crate::command::Input;
use crate::{EntityId, PlayerId, ShardCtx};

/// One change to a player's client, each its own packet.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ScreenAction {
    /// Shakes the view by `intensity` (0 to 4) for `seconds`: turning it, or moving it.
    Shake { intensity: f32, seconds: f32, rotational: bool },
    StopShake,
    /// The player's whole fog stack, bottom first.
    Fog(Vec<String>),
    /// HUD elements, by the client's numbers, hidden or back as they were.
    Hud { elements: Vec<i32>, hide: bool },
    Camera(Camera),
    /// The camera back to the player's own.
    ClearCamera,
    /// Fades the view to `colour` (red, green, blue, 0 to 1) and back: seconds in, held, out.
    /// Each left out is the client's own.
    CameraFade { seconds: Option<[f32; 3]>, colour: Option<[f32; 3]> },
    /// The field of view in degrees, eased over `ease_seconds` by the easing named as `/camera`
    /// names it (`linear`, `in_out_sine`, ..).
    CameraFov { degrees: f32, ease_seconds: f32, ease: String },
    /// The field of view back to the player's own.
    ClearCameraFov,
    /// Recipes, by id, the player's recipe book gains or loses.
    Recipes { ids: Vec<String>, change: RecipeChange },
    Music(Music),
}

/// What `/music` asks of the client's music player.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Music {
    /// A track now, or once the one playing ends (`queued`), at `volume` (0 to 1), faded in.
    Play { track: String, volume: f32, fade_seconds: f32, looped: bool, queued: bool },
    Stop { fade_seconds: f32 },
    Volume(f32),
}

/// What [`ScreenAction::Recipes`] does to a recipe book, as UnlockedRecipes says it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RecipeChange {
    /// Those a new player knows.
    Initial,
    Learnt,
    Forgotten,
    /// Every recipe forgotten; the list is empty.
    AllForgotten,
}

/// `/camera <player> set`: a preset, by its place in the server's list (`minecraft:free` is 3),
/// and what it is changed by.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Camera {
    pub preset: u32,
    /// The easing, by its place in vanilla's list (`linear`, `spring`, `in_quad`, ..), and seconds.
    pub ease: Option<(u8, f32)>,
    pub position: Option<[f32; 3]>,
    /// Pitch and yaw.
    pub rotation: Option<[f32; 2]>,
    pub facing: Option<[f32; 3]>,
    /// Back to the preset's own place and turn.
    pub default: bool,
}

/// An animation an entity plays once, as `/playanimation` asks.
#[derive(Debug, Clone, PartialEq)]
pub struct Animation {
    pub name: String,
    pub next_state: String,
    pub stop_expression: String,
    pub controller: String,
    pub blend_out_seconds: f32,
}

impl Animation {
    /// `name` with vanilla's defaults: on to `default` once any animation is done.
    pub fn new(name: impl Into<String>) -> Self {
        let owned = |text: &str| text.to_owned();
        Self { name: name.into(), next_state: owned("default"), stop_expression: owned("query.any_animation_finished"), controller: owned("__runtime_controller"), blend_out_seconds: 0.0 }
    }
}

impl ShardCtx<'_> {
    /// Does one [`ScreenAction`] to a player's client.
    pub fn screen_action(&mut self, player: PlayerId, action: &ScreenAction) {
        self.backend.screen_action(player, action);
    }

    /// Whether a player may use `input`; every input is allowed until `/inputpermission` stops it.
    pub fn input_allowed(&self, player: PlayerId, input: Input) -> bool {
        self.backend.input_locks(player) & input.lock() == 0
    }

    /// Allows or stops `input` for a player, as `/inputpermission set` does.
    pub fn allow_input(&mut self, player: PlayerId, input: Input, allowed: bool) {
        let locks = self.backend.input_locks(player);
        self.backend.set_input_locks(player, if allowed { locks & !input.lock() } else { locks | input.lock() });
    }

    /// Plays `animation` on these players and entities, for everyone who sees them.
    pub fn animate(&mut self, players: &[PlayerId], entities: &[EntityId], animation: &Animation) {
        self.backend.animate(players, entities, animation);
    }
}
