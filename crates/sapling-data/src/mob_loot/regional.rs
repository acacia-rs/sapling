//! What a new mob's equipment table reads of its world: the difficulty, for
//! `random_difficulty_chance` (vtable 14a87d2c0: a chance per difficulty, each defaulting to
//! `default_chance`), and the special multiplier, for `random_regional_difficulty_chance` (vtable
//! 14a87d2a0: `max_chance` times it) and `enchant_random_gear` (vtable 14a7489e0, item_data.rs).
//! A pool with `tiers` (`LootPool::addRandomItems` 14127b8e0) takes the entry `nextInt(initial_range)`
//! moved one on for each of its `bonus_rolls` drawing under `bonus_chance`, the entries' conditions unasked.

use serde_json::Value;

use crate::enchant::Draws;

/// How full the moon is on each day of its cycle (bds-blocks 14a75c000).
const MOON_FULLNESS: [f32; 8] = [1.0, 0.75, 0.5, 0.25, 0.0, 0.25, 0.5, 0.75];

const DAY: i64 = 24_000;
const HARD: u8 = 3;
const EASY: u8 = 1;

/// The difficulty (0 peaceful to 3 hard) and special multiplier a mob is born under.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Regional {
    pub difficulty: u8,
    pub multiplier: f32,
}

impl Regional {
    /// As the world is at `time` on `difficulty`.
    pub fn at(difficulty: u8, time: i64) -> Self {
        Self { difficulty, multiplier: special_multiplier(difficulty, time) }
    }
}

/// How full the moon is at `time`.
pub fn moon_fullness(time: i64) -> f32 {
    MOON_FULLNESS[(time / DAY).unsigned_abs() as usize % MOON_FULLNESS.len()]
}

/// `Level::getSpecialMultiplier` (Linux 0xc8a2090, Level vtable slot 40): Java's clamped regional
/// difficulty with the chunk's inhabited time fixed at half and the world's age capped after a tick.
pub fn special_multiplier(difficulty: u8, time: i64) -> f32 {
    if difficulty == 0 {
        return 0.0;
    }
    let aged = (time as f32 - 0.5).clamp(0.0, 1.0) * 0.25;
    let mut more = (moon_fullness(time) * 0.25).min(aged) + if difficulty == HARD { 0.5 } else { 0.375 };
    if difficulty == EASY {
        more *= 0.5;
    }
    let regional = f32::from(difficulty) * (0.75 + aged + more);
    if regional < 2.0 {
        0.0
    } else if regional > 4.0 {
        1.0
    } else {
        (regional - 2.0) * 0.5
    }
}

/// A `random_difficulty_chance` condition's chance on `difficulty`.
pub(super) fn difficulty_chance(condition: &Value, difficulty: u8) -> f32 {
    let key = ["peaceful", "easy", "normal", "hard"].get(usize::from(difficulty)).copied().unwrap_or("hard");
    condition.get(key).or_else(|| condition.get("default_chance")).and_then(Value::as_f64).unwrap_or(0.0) as f32
}

/// The entry a `tiers` pool takes, if it has that many.
pub(super) fn tier<'a>(pool: &'a Value, draws: &mut dyn Draws) -> Option<&'a Value> {
    let tiers = &pool["tiers"];
    let range = tiers["initial_range"].as_i64().unwrap_or(0) as i32;
    let mut at = if range >= 1 { draws.int(range) } else { 0 };
    for _ in 0..tiers["bonus_rolls"].as_i64().unwrap_or(0) {
        if draws.float() < tiers["bonus_chance"].as_f64().unwrap_or(0.0) as f32 {
            at += 1;
        }
    }
    pool["entries"].as_array()?.get(usize::try_from(at).ok()?)
}
