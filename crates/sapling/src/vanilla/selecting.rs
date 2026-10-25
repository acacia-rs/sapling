//! `/testfor` and `/spreadplayers`, answered as BDS answers (research/captures/bds-worldcmds.txt).

use std::hash::{BuildHasher, Hasher};

use sapling_api::command::{Axis, Selector};
use sapling_api::{BlockPos, Command, Output, Sender, ShardCtx};

use super::{display_name, no_target, targets};

/// The top of the world, where `/spreadplayers` looks down for ground without a `maxHeight`.
const TOP: i32 = 319;
const BOTTOM: i32 = -64;
/// Tries at a free place for each target before giving up.
const TRIES: u32 = 64;

/// Whether the target names anyone: their names, players then entities.
#[derive(Command)]
#[command(crate = "crate", name = "testfor", description = "Counts entities matching specified conditions", permission = "operator")]
pub(crate) struct TestFor {
    victim: Selector,
}

impl Command for TestFor {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let (players, entities) = targets(ctx, sender, Some(&self.victim));
        let names: Vec<String> = players.into_iter().map(|player| player.name).chain(entities.iter().map(|entity| display_name(entity))).collect();
        if names.is_empty() {
            return no_target(out);
        }
        out.translate("commands.testfor.success", vec![names.join(", ")], true);
        out.done_to(names.len() as u32);
    }
}

/// `/spreadplayers <x> <z> <spreadDistance> <maxRange> <victim> [maxHeight]`: each target goes on
/// top of the ground at a random block within `maxRange` of the centre, `spreadDistance` from the
/// others. The centre is counted from the sender's block, as BDS shows it (`0.00` from x 0.5).
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
        let mut roll = Roll::new();
        let mut placed: Vec<[f32; 3]> = Vec::with_capacity(count);
        for _ in 0..count {
            let free = (0..TRIES).find_map(|_| {
                let [x, z] = [0, 1].map(|axis| (centre[axis] + (roll.next() * 2.0 - 1.0) * self.max_range).floor());
                let far = |other: &[f32; 3]| (other[0] - x - 0.5).hypot(other[2] - z - 0.5) >= self.spread_distance;
                let ground = placed.iter().all(far).then(|| ground(ctx, [x as i32, top, z as i32]))??;
                Some([x + 0.5, ground as f32 + 1.0, z + 0.5])
            });
            match free {
                Some(at) => placed.push(at),
                None => {
                    let parameters = vec![count.to_string(), shown(centre[0]), shown(centre[1]), TRIES.to_string()];
                    return out.translate("commands.spreadplayers.failure.players.unsafe", parameters, false);
                }
            }
        }
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

/// The highest block that is not air at or below `from`, unless it is a liquid or not loaded.
fn ground(ctx: &ShardCtx<'_>, [x, from, z]: BlockPos) -> Option<i32> {
    let y = (BOTTOM..=from).rev().find(|&y| ctx.block([x, y, z]).is_none_or(|block| !block.is_air()))?;
    let block = ctx.block([x, y, z])?;
    (!["minecraft:water", "minecraft:lava", "minecraft:flowing_water", "minecraft:flowing_lava"].contains(&block.name.as_str())).then_some(y)
}

/// Numbers in [0, 1), from a seed the process picks at random.
struct Roll(u64);

impl Roll {
    fn new() -> Self {
        Self(std::collections::hash_map::RandomState::new().build_hasher().finish() | 1)
    }

    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }
}
