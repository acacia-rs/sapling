//! A world border that keeps players inside a square, and whether a world is hardcore. Bedrock
//! has no world border (BDS 1.26.52 holds none, docs/research/structures-and-ticking.md): this one
//! is sapling's, off unless a plugin sets it, and clients draw nothing of it.

use crate::ShardCtx;

/// A square about `centre` (block x, z), `radius` blocks from it to each side.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldBorder {
    pub centre: [f32; 2],
    pub radius: f32,
}

impl WorldBorder {
    /// Whether the point (x, z) is inside.
    pub fn holds(&self, [x, z]: [f32; 2]) -> bool {
        (x - self.centre[0]).abs() <= self.radius && (z - self.centre[1]).abs() <= self.radius
    }

    /// The nearest point inside to (x, z).
    pub fn clamp(&self, [x, z]: [f32; 2]) -> [f32; 2] {
        let side = |value: f32, centre: f32| value.clamp(centre - self.radius, centre + self.radius);
        [side(x, self.centre[0]), side(z, self.centre[1])]
    }
}

impl ShardCtx<'_> {
    /// The border of the world in scope, if a plugin set one.
    pub fn world_border(&self) -> Option<WorldBorder> {
        self.backend.world_border()
    }

    /// Sets or takes away the border of the world in scope. A player outside it is put back
    /// inside on its next move; held in memory only.
    pub fn set_world_border(&mut self, border: Option<WorldBorder>) {
        self.backend.set_world_border(border);
    }

    /// Whether the world in scope is hardcore: its players have one life, after which they
    /// watch as spectators, and its difficulty is fixed (level.dat `IsHardcore`).
    pub fn is_hardcore(&self) -> bool {
        self.backend.is_hardcore()
    }
}

/// The border side of [`crate::Backend`].
pub trait BorderBackend {
    fn world_border(&self) -> Option<WorldBorder>;
    fn set_world_border(&mut self, border: Option<WorldBorder>);
    fn is_hardcore(&self) -> bool;
}

#[cfg(test)]
impl BorderBackend for crate::testing::Fake {
    fn world_border(&self) -> Option<WorldBorder> {
        None
    }
    fn set_world_border(&mut self, _: Option<WorldBorder>) {}
    fn is_hardcore(&self) -> bool {
        false
    }
}
