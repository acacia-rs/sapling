//! Mineshaft corridors (`MineshaftCorridor::addChildren` 14831d360, `postProcess` 14831de50,
//! cobwebs 14831d770, pillars and chains 148320130).

use super::{Builder, Kind, Materials, Piece, fence_at, frame};
use crate::block::Block;
use crate::block_entity::BlockEntity;
use crate::random::Mt19937;
use crate::structure::BoundingBox;
use crate::structure::canvas::Canvas;
use crate::structure::piece::{EAST, Frame, NORTH, SOUTH, WEST};

#[derive(Debug, Clone)]
pub struct Corridor {
    rails: bool,
    spider: bool,
    sections: i32,
}

/// Wall torch facings, by orientation, for the torches before and after a support.
const TORCH_BEFORE: [Block; 4] = [Block::TorchSouth, Block::TorchWest, Block::TorchNorth, Block::TorchEast];
const TORCH_AFTER: [Block; 4] = [Block::TorchNorth, Block::TorchEast, Block::TorchSouth, Block::TorchWest];

impl Corridor {
    /// `findCorridorSize`: 4 to 2 sections of 5 blocks, the longest that collides with nothing.
    pub fn find(builder: &Builder, random: &mut Mt19937, (x, y, z): (i32, i32, i32), direction: u8, depth: i32) -> Option<Piece> {
        let count = random.next_int(3) + 2;
        let bb = (1..=count).rev().map(|i| i * 5).find_map(|len| {
            let bb = match direction {
                SOUTH => BoundingBox::new(x, y, z, x + 2, y + 2, z + len - 1),
                WEST => BoundingBox::new(x + 1 - len, y, z, x, y + 2, z + 2),
                NORTH => BoundingBox::new(x, y, z + 1 - len, x + 2, y + 2, z),
                _ => BoundingBox::new(x, y, z, x + len - 1, y + 2, z + 2),
            };
            (!builder.collides(&bb)).then_some(bb)
        })?;
        let rails = random.next_int(3) == 0;
        let spider = !rails && random.next_int(23) == 0;
        let span = if direction == SOUTH || direction == NORTH { bb.z1 - bb.z0 + 1 } else { bb.x1 - bb.x0 + 1 };
        Some(Piece { frame: frame(bb, Some(direction), depth), kind: Kind::Corridor(Corridor { rails, spider, sections: span / 5 }) })
    }

    /// `spawner_placed`: whether the piece placed its spawner, here or in an earlier chunk.
    pub fn post_process(&self, f: &Frame, materials: Materials, canvas: &mut Canvas, random: &mut Mt19937, spawner_placed: &mut bool) {
        let (air, planks) = (Block::Air, materials.planks());
        let length = self.sections * 5;
        f.generate_box(canvas, (0, 0, 0), (2, 1, length - 1), air, air, false);
        f.generate_maybe_box(canvas, random, 0.8, (0, 2, 0), (2, 2, length - 1), (air, air), false, false);
        if self.spider {
            f.generate_maybe_box(canvas, random, 0.6, (0, 0, 0), (2, 1, length - 1), (Block::Web, air), false, true);
        }
        super::probe::mark("sections", random);
        let orientation = f.orientation.unwrap_or(SOUTH) as usize;
        for k in 0..self.sections {
            let z = k * 5 + 2;
            let support = supported(f, canvas, z);
            super::probe::mark(if support { "support" } else { "bare" }, random);
            if support {
                for x in [0, 2] {
                    for y in 0..=1 {
                        let at = f.world(x, y, z);
                        f.place(canvas, fence_at(materials, canvas, at), x, y, z);
                        if canvas.bb.contains(at.0, at.1, at.2) && materials.is_fence(canvas.get(at.0, at.1, at.2)) {
                            canvas.shaft_fences.insert(at);
                        }
                    }
                }
                if random.next_int(4) == 0 {
                    f.generate_box(canvas, (0, 2, z), (0, 2, z), planks, air, false);
                    f.generate_box(canvas, (2, 2, z), (2, 2, z), planks, air, false);
                } else {
                    f.generate_box(canvas, (0, 2, z), (2, 2, z), planks, air, false);
                    if random.next_float() < 0.05 {
                        f.place(canvas, TORCH_BEFORE[orientation], 1, 2, z - 1);
                    }
                    if random.next_float() < 0.05 {
                        f.place(canvas, TORCH_AFTER[orientation], 1, 2, z + 1);
                    }
                }
            }
            let base = k * 5;
            super::probe::mark("webs", random);
            for (chance, x, at) in [
                (0.1, 0, z),
                (0.1, 2, base + 1),
                (0.1, 0, base + 3),
                (0.1, 2, base + 3),
                (0.05, 0, base),
                (0.05, 2, base),
                (0.05, 0, base + 4),
                (0.05, 2, base + 4),
            ] {
                // BDS tests the cell above one block further along than the cobweb it places.
                if f.interior(canvas, x, 3, at) {
                    f.maybe_block_if_not_floating(canvas, random, chance, (x, 2, at - 1), Block::Web);
                }
            }
            if self.spider && !*spawner_placed {
                let spawner = random.next_int(3) + base + 1;
                let (wx, wy, wz) = f.world(1, 0, spawner);
                if canvas.bb.contains(wx, wy, wz) && canvas.interior(wx, wy, wz) {
                    canvas.set(wx, wy, wz, Block::MobSpawner);
                    canvas.set_entity(wx, wy, wz, BlockEntity::Spawner("minecraft:cave_spider"));
                    *spawner_placed = true;
                }
            }
        }
        for x in 0..=2 {
            for z in 0..length.max(1) {
                let (wx, wy, wz) = f.world(x, -1, z);
                if canvas.interior(wx, wy + 1, wz) && matches!(canvas.get(wx, wy, wz), Block::Air | Block::IronChain) {
                    canvas.set_unclipped(wx, wy, wz, planks);
                }
            }
        }
        let mut pillars = vec![2];
        if self.sections > 1 {
            pillars.push(length - 3);
        }
        for z in pillars {
            for x in [0, 2] {
                if f.get(canvas, x, -1, z) == planks {
                    pillar_or_chain(f, materials, canvas, (x, z));
                }
            }
        }
        if self.rails {
            super::probe::mark("rails", random);
            let rail = if matches!(f.orientation, Some(WEST) | Some(EAST)) { Block::RailEastWest } else { Block::RailNorthSouth };
            for z in 0..length.max(1) {
                if full_solid(f.get(canvas, 1, -1, z)) {
                    let chance = if f.interior(canvas, 1, 0, z) { 0.7 } else { 0.9 };
                    if random.next_float() < chance {
                        f.place(canvas, rail, 1, 0, z);
                    }
                }
            }
        }
    }
}

/// Every block over the support's span, one above its top, is not air (read past the chunk).
fn supported(f: &Frame, canvas: &Canvas, z: i32) -> bool {
    let ((x0, y, z0), (x1, _, z1)) = (f.world(0, 3, z), f.world(2, 3, z));
    (x0..=x1).all(|x| (z0..=z1).all(|z| canvas.get(x, y, z) != Block::Air))
}

/// Blocks a rail can lie on and a fence joins: full solid cubes.
pub fn full_solid(block: Block) -> bool {
    block.is_solid()
        && !matches!(block, Block::SnowLayer | Block::Web | Block::MobSpawner | Block::IronChain)
        && !(Block::RailNorthSouth..=Block::DarkOakFence15).contains(&block)
        && !(Block::StoneBrickSlab..=Block::EndPortal).contains(&block)
        && !(Block::NetherBrickFence0..=Block::NetherBrickStairsNorthOuterRight).contains(&block)
}

/// A sturdy bottom centre that is not a falling block (148320130; Java's `canHangChainBelow`):
/// full cubes, and fences and chains, so a support's fence ends the search with nothing hung.
fn holds_chain(block: Block) -> bool {
    (full_solid(block) || (Block::OakFence0..=Block::DarkOakFence15).contains(&block) || block == Block::IronChain) && !block.falls()
}

/// Air, liquids and the thin plants structures may overwrite (141f5b7e0).
fn replaceable(block: Block) -> bool {
    matches!(block, Block::Air | Block::Water | Block::Lava | Block::ShortGrass | Block::SnowLayer) || block >= Block::Seagrass
}

/// `fillPillarDownOrChainUp` (148320130) under a floor plank: a log column down to the first
/// solid block within 10, else a fence and chains up to the first solid block within 20.
fn pillar_or_chain(f: &Frame, materials: Materials, canvas: &mut Canvas, (x, z): (i32, i32)) {
    let (wx, top, wz) = f.world(x, 0, z);
    if !canvas.bb.contains(wx, top - 1, wz) {
        return;
    }
    let (mut down, mut up) = (true, true);
    let mut j = 1;
    while down || up {
        if down {
            let y = top - 1 - j;
            let block = canvas.get(wx, y, wz);
            if !replaceable(block) || block == Block::Lava {
                down = false;
                if !matches!(block, Block::Lava | Block::RailNorthSouth | Block::RailEastWest) {
                    (y + 1..=top - 2).for_each(|y| canvas.set(wx, y, wz, materials.log()));
                    return;
                }
            } else {
                down = j <= 10 && super::MIN_Y + 1 < y;
            }
        }
        if up {
            let y = top - 1 + j;
            let block = canvas.get(wx, y, wz);
            if !replaceable(block) && holds_chain(block) {
                let fence = fence_at(materials, canvas, (wx, top, wz));
                canvas.set(wx, top, wz, fence);
                canvas.shaft_fences.insert((wx, top, wz));
                (top + 1..=top + j - 2).for_each(|y| canvas.set(wx, y, wz, Block::IronChain));
                return;
            }
            up = replaceable(block) && y < 319 && j < 21;
        }
        j += 1;
    }
}

/// `MineshaftCorridor::addChildren`: one piece past the far end (or turning off its last
/// section), then side branches every 5 blocks while shallow.
pub fn children(builder: &mut Builder, index: usize, random: &mut Mt19937) {
    let (bb, orientation, depth) = {
        let f = &builder.pieces[index].frame;
        (f.bb, f.orientation.unwrap_or(SOUTH), f.depth)
    };
    let pick = random.next_int(4);
    let y = random.next_int(3) + bb.y0 - 1;
    let (at, direction) = match (orientation, pick) {
        (SOUTH, 0 | 1) => ((bb.x0, y, bb.z1 + 1), SOUTH),
        (SOUTH, 2) => ((bb.x0 - 1, y, bb.z1 - 3), WEST),
        (SOUTH, _) => ((bb.x1 + 1, y, bb.z1 - 3), EAST),
        (WEST, 0 | 1) => ((bb.x0 - 1, y, bb.z0), WEST),
        (WEST, 2) => ((bb.x0, y, bb.z0 - 1), NORTH),
        (WEST, _) => ((bb.x0, y, bb.z1 + 1), SOUTH),
        (NORTH, 0 | 1) => ((bb.x0, y, bb.z0 - 1), NORTH),
        (NORTH, 2) => ((bb.x0 - 1, y, bb.z0), WEST),
        (NORTH, _) => ((bb.x1 + 1, y, bb.z0), EAST),
        (_, 0 | 1) => ((bb.x1 + 1, y, bb.z0), EAST),
        (_, 2) => ((bb.x1 - 3, y, bb.z0 - 1), NORTH),
        (_, _) => ((bb.x1 - 3, y, bb.z1 + 1), SOUTH),
    };
    builder.add(random, at, direction, depth);
    if depth >= 8 {
        return;
    }
    if orientation == SOUTH || orientation == NORTH {
        let mut z = bb.z0 + 6;
        while z <= bb.z1 {
            match random.next_int(5) {
                0 => builder.add(random, (bb.x0 - 1, bb.y0, z - 3), WEST, depth + 1),
                1 => builder.add(random, (bb.x1 + 1, bb.y0, z - 3), EAST, depth + 1),
                _ => {}
            }
            z += 5;
        }
    } else {
        let mut x = bb.x0 + 6;
        while x <= bb.x1 {
            match random.next_int(5) {
                0 => builder.add(random, (x - 3, bb.y0, bb.z0 - 1), NORTH, depth + 1),
                1 => builder.add(random, (x - 3, bb.y0, bb.z1 + 1), SOUTH, depth + 1),
                _ => {}
            }
            x += 5;
        }
    }
}
