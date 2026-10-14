//! The generator's questions about structures: where they reach, where they start, and probes.

use crate::OverworldGenerator;
use crate::block::Block;
use crate::{feature, structure};

impl OverworldGenerator {
    /// Structures that may place blocks in the chunk: kind, start chunk, and a box around what
    /// the structure covers (its pieces where ported, a rough area otherwise).
    pub fn structure_areas(&self, chunk_x: i32, chunk_z: i32) -> Vec<(structure::StructureKind, (i32, i32), structure::BoundingBox)> {
        self.structures.areas_touching(&self.density, (chunk_x, chunk_z))
    }

    /// Structure pieces' features in the 3x3 chunks around one, for probes: feature, origin.
    #[doc(hidden)]
    pub fn structure_features_near(&self, chunk_x: i32, chunk_z: i32) -> Vec<(String, (i32, i32, i32))> {
        self.structures.feature_pieces(&self.density, (chunk_x, chunk_z))
    }

    /// Whether structure terrain adjustments reach the chunk, for probes.
    #[doc(hidden)]
    pub fn has_terrain_adjustments(&self, chunk_x: i32, chunk_z: i32) -> bool {
        !self.structures.adjustments(&self.density, (chunk_x, chunk_z)).is_empty()
    }

    #[doc(hidden)]
    pub fn village_pieces(&self, chunk_x: i32, chunk_z: i32) -> Vec<String> {
        self.structures.village_pieces(&self.density, (chunk_x, chunk_z))
    }

    #[doc(hidden)]
    pub fn mineshaft_pieces(&self, chunk_x: i32, chunk_z: i32) -> Vec<(String, structure::BoundingBox)> {
        self.structures.mineshaft_pieces(&self.density, (chunk_x, chunk_z))
    }

    #[doc(hidden)]
    pub fn v2_template_cells(&self, chunk_x: i32, chunk_z: i32) -> Vec<([i32; 3], String, Block, u8, Block)> {
        self.structures.v2_template_cells(&self.density, (chunk_x, chunk_z))
    }

    /// For probes, process-wide and once: the element feature `from` places `to` from here on.
    #[doc(hidden)]
    pub fn swap_feature(from: &str, to: &str) -> bool {
        feature::swap_feature(from, to)
    }

    #[doc(hidden)]
    pub fn v2_pieces(&self, chunk_x: i32, chunk_z: i32) -> Vec<String> {
        self.structures.v2_pieces(&self.density, (chunk_x, chunk_z))
    }

    /// Whether a structure of `kind` starts in the chunk.
    pub fn is_structure_start(&self, kind: structure::StructureKind, chunk_x: i32, chunk_z: i32) -> bool {
        self.structures.is_start(&self.density, kind, (chunk_x, chunk_z))
    }

    /// The start chunk `/locate structure` answers from a block position (grid-placed kinds).
    pub fn locate_structure(&self, kind: structure::StructureKind, x: i32, z: i32) -> Option<(i32, i32)> {
        self.structures.locate(&self.density, kind, (x, z))
    }
}
