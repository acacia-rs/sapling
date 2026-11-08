//! `/testfor` and `/spreadplayers`, answered as BDS answers (research/captures/bds-worldcmds.txt).

use std::hash::{BuildHasher, Hasher};

use sapling_api::command::{Axis, Selector};
use sapling_api::{BlockPos, Command, Output, Sender, ShardCtx};

use super::{answer_name, no_target, targets};

/// The top of the world, where `/spreadplayers` looks down for ground without a `maxHeight`.
const TOP: i32 = 319;
const BOTTOM: i32 = -64;
/// BDS's iterations, and the fourth parameter of its `.unsafe` failure.
const ITERATIONS: u32 = 10_000;
/// Where BDS put a target over the ground it found: two blocks up, its feet at y -25 over stone
/// at -27 (bds-t1dspread.txt).
const LANDS_ABOVE: f32 = 2.0;
const UNSAFE: [&str; 9] = ["water", "flowing_water", "lava", "flowing_lava", "fire", "soul_fire", "powder_snow", "campfire", "soul_campfire"];

/// Whether the target names anyone: their names, players then entities.
#[derive(Command)]
#[command(crate = "crate", name = "testfor", description = "Counts entities matching specified conditions", permission = "operator")]
pub(crate) struct TestFor {
    victim: Selector,
}

impl Command for TestFor {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let (players, entities) = targets(ctx, sender, Some(&self.victim));
        let names: Vec<String> = players.into_iter().map(|player| player.name).chain(entities.iter().map(|entity| answer_name(ctx, entity))).collect();
        if names.is_empty() {
            return no_target(out);
        }
        out.translate("commands.testfor.success", vec![names.join(", ")], true);
        out.done_to(names.len() as u32);
    }
}

/// `/spreadplayers <x> <z> <spreadDistance> <maxRange> <victim> [maxHeight]`: the targets are
/// spread over the square `maxRange` around the centre, `spreadDistance` apart ([`spread`]). The centre is counted from the sender's block, as BDS shows it (`0.00` from x 0.5).
#[derive(Command)]
#[command(crate = "crate", name = "spreadplayers", description = "Teleports entities to random locations", permission = "operator")]
pub(crate) struct SpreadPlayers {
    x: Axis,
    z: Axis,
    spread_distance: f32,
    max_range: f32,
    victim: Selector,
    max_height: Option<Axis>,
}

impl Command for SpreadPlayers {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let shown = |n: f32| format!("{n:.2}");
        if self.spread_distance < 0.0 {
            return out.translate("commands.generic.num.tooSmall", vec![shown(self.spread_distance), shown(0.0)], false);
        }
        if self.max_range < self.spread_distance + 1.0 {
            return out.translate("commands.generic.num.tooSmall", vec![shown(self.max_range), shown(self.spread_distance + 1.0)], false);
        }
        let here = ctx.origin(sender).map(f32::floor);
        let centre = [self.x.from(here[0]), self.z.from(here[2])];
        let top = self.max_height.map_or(TOP, |height| (height.from(here[1]).floor() as i32).min(TOP));
        let (players, entities) = targets(ctx, sender, Some(&self.victim));
        let count = players.len() + entities.len();
        if count == 0 {
            return no_target(out);
        }
        let placed = match spread(&mut Roll::new(), count, centre, self.max_range, self.spread_distance, |[x, z]| ground(ctx, [x.floor() as i32, top, z.floor() as i32])) {
            Ok(placed) => placed,
            Err(failure) => {
                let (key, fourth) = match failure {
                    Failure::Crowded(closest) => ("commands.spreadplayers.failure.players", shown(closest)),
                    Failure::Unsafe => ("commands.spreadplayers.failure.players.unsafe", ITERATIONS.to_string()),
                };
                return out.translate(key, vec![count.to_string(), shown(centre[0]), shown(centre[1]), fourth], false);
            }
        };
        let mut places = placed.into_iter();
        for (player, at) in players.iter().zip(&mut places) {
            ctx.teleport(player.id, at);
        }
        for (entity, at) in entities.iter().zip(places) {
            ctx.move_entity(entity.id, at);
        }
        out.translate("commands.spreadplayers.success.players", vec![count.to_string(), shown(centre[0]), shown(centre[1])], true);
    }
}

/// Why every iteration ran out.
#[derive(Debug)]
enum Failure {
    /// The closest two after the last.
    Crowded(f32),
    /// One target, kept off ground it may not stand on.
    Unsafe,
}

/// BDS's spreading (bdsre 0x143b47550): each target starts at a random point of the square;
/// each iteration pushes every one a block away from the mean of those nearer than `distance`
/// and keeps it in the square; once none moves, a target off safe ground starts again. Each lands
/// at `ground(x, z)`, the middle of its block.
fn spread(roll: &mut Roll, count: usize, centre: [f32; 2], range: f32, distance: f32, ground: impl Fn([f32; 2]) -> Option<i32>) -> Result<Vec<[f32; 3]>, Failure> {
    let [low, high] = [-range, range].map(|by| centre.map(|c| c + by));
    let random = |roll: &mut Roll| -> [f32; 2] { std::array::from_fn(|axis| low[axis] + roll.next() * (high[axis] - low[axis])) };
    let mut at: Vec<[f32; 2]> = (0..count).map(|_| random(roll)).collect();
    let mut closest = f32::MAX;
    for _ in 0..ITERATIONS {
        let mut moved = false;
        closest = f32::MAX;
        for i in 0..count {
            let (mut push, mut near) = ([0.0f32; 2], 0);
            for j in (0..count).filter(|&j| j != i) {
                let apart = (at[j][0] - at[i][0]).hypot(at[j][1] - at[i][1]);
                closest = closest.min(apart);
                if apart < distance {
                    push = std::array::from_fn(|axis| push[axis] + at[j][axis] - at[i][axis]);
                    near += 1;
                }
            }
            if near > 0 {
                let mean = push.map(|sum| sum / near as f32);
                let length = mean[0].hypot(mean[1]);
                if length <= 0.0 {
                    at[i] = random(roll);
                } else if length >= 1e-4 {
                    at[i] = std::array::from_fn(|axis| at[i][axis] - mean[axis] / length);
                }
                moved = true;
            }
            let kept = std::array::from_fn(|axis| at[i][axis].clamp(low[axis], high[axis]));
            moved |= kept != at[i];
            at[i] = kept;
        }
        if !moved {
            let landed: Vec<Option<i32>> = at.iter().map(|&point| ground(point)).collect();
            if landed.iter().all(Option::is_some) {
                return Ok(at.iter().zip(landed).map(|(&[x, z], y)| [x.floor() + 0.5, y.unwrap_or_default() as f32 + LANDS_ABOVE, z.floor() + 0.5]).collect());
            }
            for (point, landed) in at.iter_mut().zip(&landed) {
                if landed.is_none() {
                    *point = random(roll);
                }
            }
        }
    }
    // BDS tells the two apart by whether any pair was measured (0x143b49292).
    Err(if closest == f32::MAX { Failure::Unsafe } else { Failure::Crowded(closest) })
}

/// The highest block at or below `from` that is not air or a carpet, unless a target may not
/// stand on it (bds-t1dspread.txt: fire, water, lava, powder snow and a campfire were refused;
/// glass, a carpet and stone were not) or it is not loaded.
fn ground(ctx: &ShardCtx<'_>, [x, from, z]: BlockPos) -> Option<i32> {
    let y = (BOTTOM..=from).rev().find(|&y| ctx.block([x, y, z]).is_none_or(|block| !block.is_air() && !block.name.ends_with("_carpet")))?;
    let block = ctx.block([x, y, z])?;
    (!UNSAFE.contains(&block.name.trim_start_matches("minecraft:"))).then_some(y)
}

/// Numbers in [0, 1), from a seed the process picks at random.
pub(super) struct Roll(u64);

impl Roll {
    pub(super) fn new() -> Self {
        Self(std::collections::hash_map::RandomState::new().build_hasher().finish() | 1)
    }

    pub(super) fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn targets_spread_as_bds_spread_them() {
        let roll = &mut Roll(0x9E37_79B9_7F4A_7C15);
        let placed = spread(roll, 6, [40.0, 0.0], 3.0, 2.0, |_| Some(-27)).ok().expect("room for six");
        assert!(placed.iter().all(|&[x, y, z]| (36.5..=43.5).contains(&x) && (-3.5..=3.5).contains(&z) && y == -25.0), "{placed:?}");
        // BDS's closest was 0.00; a simulation of the rule gave 0.00 in 14 of 15 draws.
        let crowded = spread(roll, 20, [0.0, 0.0], 6.0, 5.0, |_| Some(0));
        assert!(matches!(crowded, Err(Failure::Crowded(closest)) if closest < 1.0), "bds-spread.txt: 0.00, {crowded:?}");
        assert!(matches!(spread(roll, 1, [0.0, 0.0], 2.0, 1.0, |_| None), Err(Failure::Unsafe)));
    }
}
