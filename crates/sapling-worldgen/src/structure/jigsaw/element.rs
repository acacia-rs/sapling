//! Pool elements (`StructurePoolElement` 145e18510, feature elements 145e137d0): their boxes and
//! jigsaw blocks as placed at an origin and rotation.

use super::super::BoundingBox;
use super::super::template::{Capped, Rotation, Rule, Template};
use crate::block::Block;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Projection {
    Rigid,
    TerrainMatching,
}

pub enum ElementKind {
    Template {
        template: &'static Template,
        rules: &'static [Rule],
        projection: Projection,
        /// Foundation flags (element +0x121): 1 fill under the bottom layer, 2 plug holes below.
        post: u8,
        /// Pool aliases applied to the template's jigsaws.
        aliases: Option<&'static Vec<(String, String)>>,
        /// v2 elements place as Java does (`template::JavaPlacement`), with these capped processors.
        java: Option<&'static [Capped]>,
    },
    Feature(String),
    /// A template BDS's registration misspells: valid, but with no blocks or jigsaws.
    Missing,
    Empty,
}

pub struct Element {
    pub name: String,
    pub kind: ElementKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

impl Dir {
    fn parse(name: &str) -> Dir {
        match name {
            "down" => Dir::Down,
            "up" => Dir::Up,
            "north" => Dir::North,
            "south" => Dir::South,
            "west" => Dir::West,
            "east" => Dir::East,
            _ => panic!("bad direction {name}"),
        }
    }

    pub fn offset(self) -> (i32, i32, i32) {
        match self {
            Dir::Down => (0, -1, 0),
            Dir::Up => (0, 1, 0),
            Dir::North => (0, 0, -1),
            Dir::South => (0, 0, 1),
            Dir::West => (-1, 0, 0),
            Dir::East => (1, 0, 0),
        }
    }

    pub fn opposite(self) -> Dir {
        match self {
            Dir::Down => Dir::Up,
            Dir::Up => Dir::Down,
            Dir::North => Dir::South,
            Dir::South => Dir::North,
            Dir::West => Dir::East,
            Dir::East => Dir::West,
        }
    }

    pub fn vertical(self) -> bool {
        matches!(self, Dir::Down | Dir::Up)
    }

    fn rotate(self, rotation: Rotation) -> Dir {
        let turns = rotation as usize;
        let ring = [Dir::North, Dir::East, Dir::South, Dir::West];
        match ring.iter().position(|&d| d == self) {
            Some(i) => ring[(i + turns) % 4],
            None => self,
        }
    }
}

/// A jigsaw block of a placed element (`JigsawBlockInfo`).
#[derive(Debug, Clone)]
pub struct JigsawInfo {
    pub pos: (i32, i32, i32),
    pub front: Dir,
    pub top: Dir,
    pub name: &'static str,
    pub target: &'static str,
    pub pool: &'static str,
    pub aligned: bool,
    pub final_state: Block,
    pub selection_priority: i32,
    pub placement_priority: i32,
}

impl JigsawInfo {
    /// `JigsawBlockInfo` compatibility (142fd8d10, v2 145e16f50): this one's target is the other's
    /// name (`any_name` skips that), they face each other, and aligned vertical joints share their top.
    pub fn can_attach(&self, other: &JigsawInfo, any_name: bool) -> bool {
        (any_name || self.target == other.name) && self.front == other.front.opposite() && !(self.front.vertical() && self.aligned && self.top != other.top)
    }
}

impl Element {
    pub fn is_valid(&self) -> bool {
        !matches!(self.kind, ElementKind::Empty)
    }

    pub fn projection(&self) -> Projection {
        match self.kind {
            ElementKind::Template { projection, .. } => projection,
            _ => Projection::Rigid,
        }
    }

    pub fn bbox(&self, (x, y, z): (i32, i32, i32), rotation: Rotation) -> BoundingBox {
        let (sx, sy, sz) = match self.kind {
            ElementKind::Template { template, .. } => template.size,
            ElementKind::Feature(_) => return feature_bbox((x, y, z), rotation),
            _ => (1, 1, 1),
        };
        let (dx, dy, dz) = rotation.apply((sx - 1, sy - 1, sz - 1));
        BoundingBox::new(x.min(x + dx), y.min(y + dy), z.min(z + dz), x.max(x + dx), y.max(y + dy), z.max(z + dz))
    }

}

/// A feature element's box: its size is (0, 0, 0) (140906d00), which the shared box code
/// (145e1aa90) turns into a box inverted on every axis, one off the origin by rotation. Such a
/// box meets one chunk, or none when the origin is on that chunk's edge, and fills no columns.
fn feature_bbox((x, y, z): (i32, i32, i32), rotation: Rotation) -> BoundingBox {
    let turn = rotation as usize;
    let (x0, x1) = if matches!(turn, 1 | 2) { (x + 1, x) } else { (x, x - 1) };
    let (z0, z1) = if matches!(turn, 2 | 3) { (z + 1, z) } else { (z, z - 1) };
    BoundingBox::new(x0, y, z0, x1, y - 1, z1)
}

impl Element {
    /// The element's jigsaws in template order, as placed at `origin`.
    pub fn jigsaws(&self, origin: (i32, i32, i32), rotation: Rotation) -> Vec<JigsawInfo> {
        match &self.kind {
            ElementKind::Template { template, aliases, .. } => template
                .jigsaws
                .iter()
                .map(|j| {
                    let (dx, dy, dz) = rotation.apply(template.blocks[j.block].pos);
                    let (front, top) = j.orientation.split_once('_').unwrap();
                    let pool = aliases.and_then(|a| a.iter().find(|(from, _)| *from == j.pool.trim_start_matches("minecraft:")));
                    let pool: &'static str = match pool {
                        Some((_, to)) => to.as_str(),
                        None => j.pool.as_str(),
                    };
                    JigsawInfo {
                        pos: (origin.0 + dx, origin.1 + dy, origin.2 + dz),
                        front: Dir::parse(front).rotate(rotation),
                        top: Dir::parse(top).rotate(rotation),
                        name: j.name.as_str(),
                        target: j.target.as_str(),
                        pool,
                        aligned: j.joint == "aligned",
                        final_state: j.final_state.block(rotation),
                        selection_priority: j.selection_priority,
                        placement_priority: j.placement_priority,
                    }
                })
                .collect(),
            ElementKind::Feature(_) => vec![JigsawInfo {
                pos: origin,
                front: Dir::Down,
                top: Dir::South,
                name: "minecraft:bottom",
                target: "minecraft:empty",
                pool: "minecraft:empty",
                aligned: false,
                final_state: Block::Air,
                selection_priority: 0,
                placement_priority: 0,
            }],
            ElementKind::Missing | ElementKind::Empty => Vec::new(),
        }
    }
}
