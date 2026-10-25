//! Structure templates: boxes of blocks, what they hold and the entities in them, saved under a
//! name (`mystructure:` when it has no namespace) and loaded elsewhere, turned and mirrored, as
//! `/structure` does. A world keeps those saved to disk with it, and reads the `.mcstructure`
//! files of its folder's `structures` directory.

use crate::{BlockPos, ShardCtx};

/// A clockwise turn about the vertical, seen from above.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Rotation {
    #[default]
    None,
    Clockwise90,
    Clockwise180,
    Clockwise270,
}

/// `X` runs the box's z the other way (it mirrors across the x axis), `Z` its x.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Mirror {
    #[default]
    None,
    X,
    Z,
    XZ,
}

/// How a structure is loaded.
#[derive(Clone, Debug, PartialEq)]
pub struct StructureLoad {
    pub rotation: Rotation,
    pub mirror: Mirror,
    pub include_entities: bool,
    pub include_blocks: bool,
    /// Whether a block put where a liquid is keeps it.
    pub waterlogged: bool,
    /// The share of its blocks placed, 0 to 1, picked by `seed`.
    pub integrity: f32,
    /// Picks the same blocks every time; an empty one picks anew each load.
    pub seed: String,
}

impl Default for StructureLoad {
    fn default() -> Self {
        Self { rotation: Rotation::None, mirror: Mirror::None, include_entities: true, include_blocks: true, waterlogged: false, integrity: 1.0, seed: String::new() }
    }
}

/// Why a structure was not saved or loaded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StructureError {
    /// No structure of that name.
    NotFound,
    /// Part of the box is not loaded or outside the world.
    Unloaded,
    /// The saved template could not be read.
    Unreadable,
}

impl ShardCtx<'_> {
    /// Saves the box between two corners of the world in scope as `name`, its entities (players
    /// aside) too where `include_entities`, its blocks where `include_blocks`. `to_disk` keeps it
    /// with the world; otherwise it lasts until the server stops.
    pub fn save_structure(&mut self, name: &str, from: BlockPos, to: BlockPos, include_entities: bool, include_blocks: bool, to_disk: bool) -> Result<(), StructureError> {
        self.backend.save_structure(name, from, to, include_entities, include_blocks, to_disk)
    }

    /// Places the structure `name` with its lowest corner at `at`.
    pub fn load_structure(&mut self, name: &str, at: BlockPos, how: &StructureLoad) -> Result<(), StructureError> {
        self.backend.load_structure(name, at, how)
    }

    /// The cells a structure spans along x, y and z, as saved (before any turn).
    pub fn structure_size(&mut self, name: &str) -> Option<[i32; 3]> {
        self.backend.structure_size(name)
    }

    /// Forgets the structure `name`, from memory and from the world's disk; false where there
    /// was none of that name.
    pub fn delete_structure(&mut self, name: &str) -> bool {
        self.backend.delete_structure(name)
    }

    /// The names of the structures that can be loaded, sorted.
    pub fn structures(&self) -> Vec<String> {
        self.backend.structures()
    }

    /// A structure as `.mcstructure` bytes.
    pub fn structure_template(&mut self, name: &str) -> Option<Vec<u8>> {
        self.backend.structure_template(name)
    }

    /// Keeps `template` (`.mcstructure` bytes) as `name`, in memory or with the world;
    /// [`StructureError::Unreadable`] where it is no template.
    pub fn set_structure_template(&mut self, name: &str, template: Vec<u8>, to_disk: bool) -> Result<(), StructureError> {
        self.backend.set_structure_template(name, template, to_disk)
    }
}

/// The structure side of [`crate::Backend`].
pub trait StructureBackend {
    fn save_structure(&mut self, name: &str, from: BlockPos, to: BlockPos, include_entities: bool, include_blocks: bool, to_disk: bool) -> Result<(), StructureError>;
    fn load_structure(&mut self, name: &str, at: BlockPos, how: &StructureLoad) -> Result<(), StructureError>;
    fn structure_size(&mut self, name: &str) -> Option<[i32; 3]>;
    fn delete_structure(&mut self, name: &str) -> bool;
    fn structures(&self) -> Vec<String>;
    fn structure_template(&mut self, name: &str) -> Option<Vec<u8>>;
    fn set_structure_template(&mut self, name: &str, template: Vec<u8>, to_disk: bool) -> Result<(), StructureError>;
}

#[cfg(test)]
impl StructureBackend for crate::testing::Fake {
    fn save_structure(&mut self, _: &str, _: BlockPos, _: BlockPos, _: bool, _: bool, _: bool) -> Result<(), StructureError> {
        Err(StructureError::Unloaded)
    }
    fn load_structure(&mut self, _: &str, _: BlockPos, _: &StructureLoad) -> Result<(), StructureError> {
        Err(StructureError::NotFound)
    }
    fn structure_size(&mut self, _: &str) -> Option<[i32; 3]> {
        None
    }
    fn delete_structure(&mut self, _: &str) -> bool {
        false
    }
    fn structures(&self) -> Vec<String> {
        Vec::new()
    }
    fn structure_template(&mut self, _: &str) -> Option<Vec<u8>> {
        None
    }
    fn set_structure_template(&mut self, _: &str, _: Vec<u8>, _: bool) -> Result<(), StructureError> {
        Err(StructureError::Unreadable)
    }
}
