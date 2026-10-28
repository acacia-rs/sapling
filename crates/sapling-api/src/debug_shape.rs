//! Debug shapes: lines, boxes, spheres, circles, text and arrows drawn in the world on one
//! player's client (ServerScriptDebugDrawer), each under an id of the plugin's choosing.

use crate::{PlayerId, ShardCtx};

/// What is drawn, with its own sizes.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ShapeKind {
    Line { to: [f32; 3] },
    /// A box from the shape's place, `bounds` long along each axis.
    Box { bounds: [f32; 3] },
    Sphere { segments: u8 },
    Circle { segments: u8 },
    Text(String),
    Arrow { to: [f32; 3], head_length: f32, head_radius: f32, segments: u8 },
}

#[derive(Debug, Clone, PartialEq)]
pub struct DebugShape {
    pub at: [f32; 3],
    pub kind: ShapeKind,
    /// Alpha, red, green, blue.
    pub colour: [u8; 4],
    pub scale: f32,
    /// Degrees about x, y and z.
    pub rotation: [f32; 3],
    /// Seconds until the client takes it away by itself; `None`: kept until erased.
    pub seconds: Option<f32>,
}

impl DebugShape {
    /// An opaque white shape at `at`, unscaled and unturned.
    pub fn new(at: [f32; 3], kind: ShapeKind) -> Self {
        Self { at, kind, colour: [0xff; 4], scale: 1.0, rotation: [0.0; 3], seconds: None }
    }

    pub fn colour(mut self, argb: [u8; 4]) -> Self {
        self.colour = argb;
        self
    }

    pub fn scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    pub fn rotation(mut self, degrees: [f32; 3]) -> Self {
        self.rotation = degrees;
        self
    }

    pub fn for_seconds(mut self, seconds: f32) -> Self {
        self.seconds = Some(seconds);
        self
    }

    /// The colour as the packet's one ARGB number.
    pub fn argb(&self) -> i32 {
        i32::from_be_bytes(self.colour)
    }
}

impl ShardCtx<'_> {
    /// Draws `shape` on a player's client under `id`, replacing what that id showed.
    pub fn draw_shape(&mut self, player: PlayerId, id: u64, shape: &DebugShape) {
        self.backend.debug_shape(player, id, Some(shape));
    }

    /// Takes away what a player's client shows under `id`.
    pub fn erase_shape(&mut self, player: PlayerId, id: u64) {
        self.backend.debug_shape(player, id, None);
    }
}
