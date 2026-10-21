//! Where a gateway a kill opens leads (`findExitPosition` 1487fd350 and the gateway job,
//! research/dragon-fight-decompile.md 4), as Java's end gateway finds it: 1024 out along the
//! gateway's direction, back by 16 while the column there holds blocks, then on by 16 while it
//! holds none; the end stone nearest the world's middle in that chunk with room over it, the
//! highest block within 16 of that, and ten over it the far gateway. The columns it reads are
//! asked for, and the search waits for them.
// TODO: with no end stone in the chunk BDS also builds an island there (148274c80); this does not.

use sapling_api::BlockPos;
use sapling_world::{ChunkPos, World as Level};

const OUT: f64 = 1024.0;
const STEP: f64 = 16.0;
const STEPS: u8 = 16;
const LOWEST: i32 = 30;
const SPAWN_Y: i32 = 75;
/// Reaches of the highest-block searches: round the far gateway's place, round where a traveller lands.
const FAR_REACH: i32 = 16;
const LANDING_REACH: i32 = 5;
const ABOVE_FAR: i32 = 10;
const TOP: i32 = 255;

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
            let empty = is_empty(level, chunk);
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

/// Whether no block of the column stands above its lowest section.
fn is_empty(level: &Level, chunk: ChunkPos) -> bool {
    (0..16).all(|dx| (0..16).all(|dz| level.surface(chunk.x * 16 + dx, chunk.z * 16 + dz).is_none_or(|y| y < 16)))
}

fn named(level: &Level, [x, y, z]: BlockPos) -> Option<&'static str> {
    level.registry().get(level.block(x, y, z)).map(|state| state.name)
}

fn is_normal_cube(level: &Level, [x, y, z]: BlockPos) -> bool {
    level.registry().get(level.block(x, y, z)).is_some_and(|state| state.is_full_cube())
}

/// The end stone of `chunk` with room for a player over it nearest the world's middle.
fn spawn_in(level: &Level, chunk: ChunkPos) -> Option<BlockPos> {
    let top = (0..16).flat_map(|dx| (0..16).filter_map(move |dz| level.surface(chunk.x * 16 + dx, chunk.z * 16 + dz))).max()?;
    let top = (top >> 4) * 16 + 15;
    let mut best: Option<(f64, BlockPos)> = None;
    for z in chunk.z * 16..chunk.z * 16 + 16 {
        for y in LOWEST..=top {
            for x in chunk.x * 16..chunk.x * 16 + 16 {
                let fits = named(level, [x, y, z]) == Some("minecraft:end_stone") && !is_normal_cube(level, [x, y + 1, z]) && !is_normal_cube(level, [x, y + 2, z]);
                let far = [x, y, z].map(|c| (f64::from(c) + 0.5).powi(2)).iter().sum::<f64>();
                if fits && best.is_none_or(|(least, _)| far < least) {
                    best = Some((far, [x, y, z]));
                }
            }
        }
    }
    best.map(|(_, at)| at)
}

/// The highest full block within `reach` of `at` across (its own column only `with_bedrock`), or
/// `at` itself.
fn highest(level: &Level, at: BlockPos, reach: i32, with_bedrock: bool) -> BlockPos {
    let mut best: Option<BlockPos> = None;
    for dx in -reach..=reach {
        for dz in -reach..=reach {
            if dx == 0 && dz == 0 && !with_bedrock {
                continue;
            }
            let floor = best.map_or(0, |[_, y, _]| y);
            let found = (floor + 1..=TOP).rev().map(|y| [at[0] + dx, y, at[2] + dz]).find(|&pos| is_normal_cube(level, pos) && (with_bedrock || named(level, pos) != Some("minecraft:bedrock")));
            best = found.or(best);
        }
    }
    best.unwrap_or(at)
}

/// Waits for the columns round the far side; then where the far gateway goes and lands people.
fn settle(level: &mut Level, chunk: ChunkPos, at: [f64; 2], near: BlockPos) -> Option<Found> {
    let reach = (FAR_REACH + LANDING_REACH) / 16 + 1;
    let wanted = (-reach..=reach).flat_map(|dx| (-reach..=reach).map(move |dz| ChunkPos::new(chunk.x + dx, chunk.z + dz)));
    if !wanted.chain([ChunkPos::of_block(near[0], near[2])]).fold(true, |all, column| level.request_urgent(column) && all) {
        return None;
    }
    let spawn = spawn_in(level, chunk).unwrap_or([(at[0] + 0.5).floor() as i32, SPAWN_Y, (at[1] + 0.5).floor() as i32]);
    let [x, y, z] = highest(level, spawn, FAR_REACH, true);
    let far = [x, y + ABOVE_FAR, z];
    let up = |[x, y, z]: BlockPos| [x, y + 1, z];
    Some(Found { far, landing: up(highest(level, far, LANDING_REACH, false)), back: up(highest(level, near, LANDING_REACH, false)) })
}
