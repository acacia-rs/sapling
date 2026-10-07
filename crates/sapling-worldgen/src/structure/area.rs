//! The blocks a structure may cover, for attributing cells to it before its pieces are ported:
//! a box around the start chunk, sized from the vanilla templates' reach.

use super::StructureKind;

/// An inclusive block box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundingBox {
    pub x0: i32,
    pub y0: i32,
    pub z0: i32,
    pub x1: i32,
    pub y1: i32,
    pub z1: i32,
}

impl BoundingBox {
    pub fn new(x0: i32, y0: i32, z0: i32, x1: i32, y1: i32, z1: i32) -> Self {
        Self { x0, y0, z0, x1, y1, z1 }
    }

    pub fn intersects(&self, other: &BoundingBox) -> bool {
        self.x1 >= other.x0 && self.x0 <= other.x1 && self.z1 >= other.z0 && self.z0 <= other.z1 && self.y1 >= other.y0 && self.y0 <= other.y1
    }

    pub fn contains(&self, x: i32, y: i32, z: i32) -> bool {
        (self.x0..=self.x1).contains(&x) && (self.y0..=self.y1).contains(&y) && (self.z0..=self.z1).contains(&z)
    }

    pub fn union(&self, o: &BoundingBox) -> Self {
        Self::new(self.x0.min(o.x0), self.y0.min(o.y0), self.z0.min(o.z0), self.x1.max(o.x1), self.y1.max(o.y1), self.z1.max(o.z1))
    }

    pub fn move_y(&mut self, dy: i32) {
        self.y0 += dy;
        self.y1 += dy;
    }

    /// The box of one chunk column, all heights.
    pub fn chunk(chunk_x: i32, chunk_z: i32) -> Self {
        Self::new(chunk_x * 16, -64, chunk_z * 16, chunk_x * 16 + 15, 319, chunk_z * 16 + 15)
    }
}

impl StructureKind {
    /// How many chunks around its start chunk a structure can reach (an upper bound).
    pub fn reach(self) -> i32 {
        match self {
            Self::Mineshaft => 6,
            Self::Village | Self::TrialChambers | Self::AncientCity => 7,
            Self::Stronghold => 8,
            Self::Mansion => 5,
            Self::Monument | Self::TrailRuins | Self::Outpost => 3,
            Self::Temple | Self::Shipwreck | Self::OceanRuins | Self::RuinedPortal => 2,
            Self::BuriedTreasure => 0,
        }
    }

    /// A box sure to hold what a structure starting in `chunk` places, all heights.
    pub fn rough_area(self, (x, z): (i32, i32)) -> BoundingBox {
        let r = self.reach();
        BoundingBox::new((x - r) * 16, -64, (z - r) * 16, (x + r) * 16 + 15, 319, (z + r) * 16 + 15)
    }
}
