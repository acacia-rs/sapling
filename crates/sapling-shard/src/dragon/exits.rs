//! Where a gateway a kill opens leads (`findExitPosition` 1487fd350 and the gateway job,
//! research/dragon-fight-decompile.md 4, research/dragon-extras-decompile.md 1): 1024 out along
//! the gateway's direction, back by 16 while the column there holds blocks, then on by 16 while it
//! holds none; BDS's spawn search round that point (an end island at y 75 where it finds none), the
//! tallest block within 16 of it and ten over that the far gateway; the near gateway lands a
//! traveller one over the spawn search round the far gateway, the far one on the main island's top
//! within 48 of the near gateway. The columns it reads are asked for, and the search waits for them.

use sapling_api::BlockPos;
use sapling_world::{ChunkPos, World as Level};

use super::exit_spawn::{island, tallest, valid_spawn};

const OUT: f64 = 1024.0;
const STEP: f64 = 16.0;
const STEPS: u8 = 16;
const FAR_REACH: i32 = 16;
const ABOVE_FAR: i32 = 10;
/// Chunks round a point its spawn search reads: round the far side, and round the near gateway.
const FAR_SPAWN: i32 = 1;
const BACK_SPAWN: i32 = 3;

enum Phase {
    Back,
    Forward,
    Found,
}

/// A search for one gateway's exit.
pub(super) struct Search {
    dir: [f64; 2],
    at: [f64; 2],
    phase: Phase,
    left: u8,
}

/// What a search found: the far gateway's cell, where the near gateway lands a traveller, and
/// where the far one does.
pub(super) struct Found {
    pub far: BlockPos,
    pub landing: BlockPos,
    pub back: BlockPos,
}

impl Search {
    pub fn new([x, _, z]: BlockPos) -> Self {
        let length = f64::from(x).hypot(f64::from(z));
        let dir = if length < 1e-4 { [0.0; 2] } else { [f64::from(x) / length, f64::from(z) / length] };
        Self { dir, at: dir.map(|c| c * OUT), phase: Phase::Back, left: STEPS }
    }

    fn chunk(&self) -> ChunkPos {
        ChunkPos::of_block(self.at[0].floor() as i32, self.at[1].floor() as i32)
    }

    /// Steps the search over loaded columns, asking for the next; `Some` once it has all it reads.
    pub fn step(&mut self, level: &mut Level, near: BlockPos) -> Option<Found> {
        loop {
            let chunk = self.chunk();
            if !level.request_urgent(chunk) {
                return None;
            }
            let empty = is_empty(level, self.at);
            let go = match self.phase {
                Phase::Back if !empty && self.left > 0 => -STEP,
                Phase::Back => {
                    (self.phase, self.left) = (Phase::Forward, STEPS);
                    continue;
                }
                Phase::Forward if empty && self.left > 0 => STEP,
                Phase::Forward => {
                    self.phase = Phase::Found;
                    continue;
                }
                Phase::Found => return settle(level, chunk, self.at, near),
            };
            self.left -= 1;
            self.at = [self.at[0] + self.dir[0] * go, self.at[1] + self.dir[1] * go];
        }
    }
}

/// Whether the column at the search's point holds no block. BDS tests that column's height, not
/// Java's whole chunk (research/captures/bds-dragon-exits.txt: it walked past a chunk with one block).
fn is_empty(level: &Level, [x, z]: [f64; 2]) -> bool {
    level.surface(x.floor() as i32, z.floor() as i32).is_none()
}

/// Whether every column within `chunks` round `at`'s chunk can be read, asking for those not loaded.
fn loaded(level: &mut Level, at: BlockPos, chunks: i32) -> bool {
    let middle = ChunkPos::of_block(at[0], at[2]);
    let wanted = (-chunks..=chunks).flat_map(|dx| (-chunks..=chunks).map(move |dz| ChunkPos::new(middle.x + dx, middle.z + dz)));
    wanted.fold(true, |all, column| level.request_urgent(column) && all)
}

/// Waits for the columns round the far side; then where the far gateway goes and lands people.
fn settle(level: &mut Level, chunk: ChunkPos, [x, z]: [f64; 2], near: BlockPos) -> Option<Found> {
    let exit = [x.floor() as i32, 0, z.floor() as i32];
    let reads = FAR_SPAWN + (FAR_REACH + 15) / 16 + 1;
    if !loaded(level, [chunk.x * 16, 0, chunk.z * 16], reads) || !loaded(level, near, BACK_SPAWN + 1) {
        return None;
    }
    let (spawn, built) = match valid_spawn(level, exit, true, FAR_SPAWN) {
        Some(spawn) => (spawn, Vec::new()),
        None => island(exit),
    };
    if let Some(end_stone) = level.block_id("minecraft:end_stone") {
        for &[x, y, z] in &built {
            level.set_block(x, y, z, end_stone);
        }
    }
    let [fx, fy, fz] = tallest(level, spawn, FAR_REACH, true);
    let far = [fx, fy + ABOVE_FAR, fz];
    // BDS reads round the far gateway through the near side's view, which sees none of it: the
    // tallest-block search there falls back to the spawn (bds-dragon-again.txt's first exit).
    let landing = valid_spawn(level, far, false, FAR_SPAWN).unwrap_or([0; 3]);
    let back = valid_spawn(level, near, false, BACK_SPAWN).unwrap_or([0; 3]);
    Some(Found { far, landing: [landing[0], landing[1] + 1, landing[2]], back })
}
