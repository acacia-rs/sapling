//! `/setworldspawn`, `/spawnpoint` and `/clearspawnpoint`, answered as BDS answers
//! (research/captures/bds-worldcmds.txt). A player respawns at their spawn point, else the world's.

use sapling_api::command::{BlockCoords, Selector};
use sapling_api::{BlockPos, Command, Output, Sender, ShardCtx};

use super::{no_target, players};

/// The heights BDS keeps a world spawn between: `/setworldspawn 0 400 0` set y 319.
const HEIGHTS: [i32; 2] = [-64, 319];

/// The block a command means: `at` from the sender, or the sender's own.
fn block_at(ctx: &mut ShardCtx<'_>, sender: &Sender, at: Option<&BlockCoords>) -> BlockPos {
    at.map_or_else(|| ctx.origin(sender).map(|c| c.floor() as i32), |at| at.resolve(ctx, sender))
}

fn shown([x, y, z]: BlockPos) -> [String; 3] {
    [x, y, z].map(|c| c.to_string())
}

#[derive(Command)]
#[command(crate = "crate", name = "setworldspawn", description = "Sets the world spawn", permission = "operator")]
pub(crate) struct SetWorldSpawn {
    spawn_point: Option<BlockCoords>,
}

impl Command for SetWorldSpawn {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let [x, y, z] = block_at(ctx, sender, self.spawn_point.as_ref());
        let at = [x, y.clamp(HEIGHTS[0], HEIGHTS[1]), z];
        ctx.set_spawn([x as f32 + 0.5, at[1] as f32, z as f32 + 0.5]);
        out.translate("commands.setworldspawn.success", shown(at).into(), true);
    }
}

/// `/spawnpoint [player] [spawnPos]`: unlike the world's, a spawn point's height is not kept in
/// the world (`/spawnpoint @s 0 500 0` set y 500).
#[derive(Command)]
#[command(crate = "crate", name = "spawnpoint", description = "Sets the spawn point for a player", permission = "operator")]
pub(crate) struct SpawnPoint {
    player: Option<Selector>,
    spawn_pos: Option<BlockCoords>,
}

impl Command for SpawnPoint {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let at = block_at(ctx, sender, self.spawn_pos.as_ref());
        let set: Vec<String> = players(ctx, sender, self.player.as_ref()).into_iter().filter(|player| ctx.set_spawn_point(player.id, Some(at))).map(|player| player.name).collect();
        let [x, y, z] = shown(at);
        match set.len() {
            0 => no_target(out),
            1 => out.translate("commands.spawnpoint.success.single", vec![set[0].clone(), x, y, z], true),
            count => {
                out.translate("commands.spawnpoint.success.multiple.specific", vec![set.join(", "), x, y, z], true);
                out.done_to(count as u32);
            }
        }
    }
}

#[derive(Command)]
#[command(crate = "crate", name = "clearspawnpoint", description = "Removes the spawn point for a player", permission = "operator")]
pub(crate) struct ClearSpawnPoint {
    player: Option<Selector>,
}

impl Command for ClearSpawnPoint {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let cleared: Vec<String> = players(ctx, sender, self.player.as_ref()).into_iter().filter(|player| ctx.set_spawn_point(player.id, None)).map(|player| player.name).collect();
        match cleared.len() {
            0 => no_target(out),
            1 => out.translate("commands.clearspawnpoint.success.single", cleared, true),
            count => {
                out.translate("commands.clearspawnpoint.success.multiple", vec![cleared.join(", ")], true);
                out.done_to(count as u32);
            }
        }
    }
}
