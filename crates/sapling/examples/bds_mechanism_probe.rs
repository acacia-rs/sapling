//! Shows what a vanilla server does with falling blocks, pressure plates, buckets, lava and
//! waterlogged blocks: an actor bot runs drills, the odd ones included (a bucket on flowing water,
//! an adventure player's bucket, a spectator on a plate), while a watcher bot stands by. The
//! ground truth for sapling's `blocks/falling.rs`, `plates.rs`, `buckets.rs`, `liquid/` and
//! `waterlog.rs` (docs/research/*.md has what was decompiled before).
//! `cargo run -p sapling --example bds_mechanism_probe -- <server> > mechanisms.txt` (offline mode,
//! operators by default, a flat world)
#[path = "shared/dump.rs"]
mod dump;
#[path = "shared/probe.rs"]
mod probe;

use std::time::Duration;

use acacia_bot::interact::Face;
use probe::{Probe, SETTLE, connect};

/// Every packet that tells of blocks, entities, sounds or items.
const SHOWN: &[&str] = &[
    "UpdateBlock", "UpdateBlockSynced", "UpdateSubchunkBlocks", "LevelEvent", "LevelEventGeneric", "LevelSoundEvent", "BlockEntityData", "AddEntity",
    "RemoveEntity", "MoveEntity", "MoveEntityDelta", "SetEntityMotion", "SetEntityData", "AddItemEntity", "TakeItemEntity", "InventorySlot", "InventoryContent",
    "ItemStackResponse", "SpawnParticleEffect", "Disconnect", "Text",
];
const LONG: Duration = Duration::from_millis(3000);
/// Drills this far apart keep their water and lava apart.
const APART: i32 = 18;

/// A cell of a drill: `origin` plus an offset.
fn at(origin: [i32; 3], [dx, dy, dz]: [i32; 3]) -> [i32; 3] {
    [origin[0] + dx, origin[1] + dy, origin[2] + dz]
}

fn setblock(pos: [i32; 3], block: &str) -> String {
    format!("setblock {} {} {} {block}", pos[0], pos[1], pos[2])
}

impl Probe {
    /// Moves the actor onto `pos` and lets it arrive.
    async fn stand(&mut self, pos: [i32; 3]) -> Result<(), String> {
        self.setup(&[format!("tp @s {}.5 {} {}.5", pos[0], pos[1], pos[2])]).await?;
        self.actor.wait_ticks(10).await.map_err(|e| e.to_string())?;
        self.quiet().await;
        Ok(())
    }

    /// Clears the actor's inventory and gives it `item` in the first slot, selected.
    async fn hold(&mut self, item: &str) -> Result<(), String> {
        self.setup(&["clear @s".into(), format!("give @s {item}")]).await?;
        self.actor.select_hotbar(0).map_err(|e| e.to_string())
    }

    /// A falling block held up at height 2 or 3; its support is broken.
    async fn drop_from(&mut self, label: &str, o: [i32; 3], dx: i32, below: &[(&str, i32)], block: &str) -> Result<(), String> {
        self.stand(at(o, [dx, 0, 0])).await?;
        let mut lines: Vec<String> = below.iter().map(|(name, dy)| setblock(at(o, [dx, *dy, 2]), name)).collect();
        lines.extend([setblock(at(o, [dx, 2, 2]), "stone"), setblock(at(o, [dx, 3, 2]), block)]);
        self.setup(&lines).await?;
        self.break_block(label, at(o, [dx, 2, 2]), LONG).await
    }

    async fn falling(&mut self, o: [i32; 3]) -> Result<(), String> {
        println!("# falling blocks");
        self.drop_from("sand, three cells onto dirt", o, 0, &[], "sand").await?;
        self.drop_from("gravel onto a torch", o, 2, &[("torch [\"torch_facing_direction\"=\"top\"]", 0)], "gravel").await?;
        self.drop_from("sand onto a bottom slab", o, 4, &[("oak_slab", 0)], "sand").await?;
        self.drop_from("an anvil", o, 6, &[], "anvil").await?;
        self.drop_from("concrete powder into water", o, 8, &[("water", -1)], "red_concrete_powder").await?;
        self.drop_from("a snow layer", o, 10, &[], "snow_layer").await?;
        // A stack of three: the bottom one goes first.
        self.stand(at(o, [12, 0, 0])).await?;
        let stack: Vec<String> = [(1, "stone"), (2, "sand"), (3, "sand"), (4, "gravel")].iter().map(|(dy, name)| setblock(at(o, [12, *dy, 2]), name)).collect();
        self.setup(&stack).await?;
        self.break_block("a stack of sand, sand and gravel", at(o, [12, 1, 2]), LONG).await?;
        // Placed by hand beside a block, over a drop: the fall starts on its own.
        self.stand(at(o, [14, 0, 0])).await?;
        self.setup(&[setblock(at(o, [14, 2, 2]), "stone")]).await?;
        self.hold("sand 4").await?;
        self.place_block("sand placed in the air", at(o, [14, 2, 2]), Face::East, LONG).await
    }

    /// The actor steps onto the plate at `plate`, stays, and steps off again.
    async fn step_on(&mut self, label: &str, home: [i32; 3], plate: [i32; 3], name: &str) -> Result<(), String> {
        self.setup(&[setblock(plate, name)]).await?;
        println!("## {label}: onto {name} at {plate:?}");
        self.command(&format!("tp @s {}.5 {} {}.5", plate[0], plate[1], plate[2]), LONG).await?;
        println!("## {label}: off it, now {}", self.name(plate));
        self.command(&format!("tp @s {}.5 {} {}.5", home[0], home[1], home[2]), LONG).await?;
        println!("## -> {}", self.name(plate));
        Ok(())
    }

    async fn plates(&mut self, o: [i32; 3]) -> Result<(), String> {
        println!("# pressure plates");
        self.stand(o).await?;
        self.step_on("a player", o, at(o, [0, 0, 3]), "stone_pressure_plate").await?;
        self.step_on("a player, weighted", o, at(o, [2, 0, 3]), "light_weighted_pressure_plate").await?;
        self.step_on("a player, wooden", o, at(o, [4, 0, 3]), "wooden_pressure_plate").await?;
        self.setup(&["gamemode spectator".into()]).await?;
        self.step_on("a spectator", o, at(o, [6, 0, 3]), "stone_pressure_plate").await?;
        self.setup(&["gamemode creative".into()]).await?;
        for (dx, name, mob) in [(8, "wooden_pressure_plate", "chicken"), (10, "stone_pressure_plate", "armor_stand"), (12, "heavy_weighted_pressure_plate", "pig")] {
            let plate = at(o, [dx, 0, 3]);
            self.setup(&[setblock(plate, name)]).await?;
            self.command(&format!("summon {mob} {}.5 {} {}.5", plate[0], plate[1], plate[2]), LONG).await?;
            self.command(&format!("kill @e[type={mob}]"), LONG).await?;
            println!("## -> {}", self.name(plate));
        }
        Ok(())
    }

    async fn buckets(&mut self, o: [i32; 3]) -> Result<(), String> {
        println!("# buckets");
        self.stand(o).await?;
        let (pit, lava_pit, stairs, snow, grass) = (at(o, [0, -1, 2]), at(o, [2, -1, 2]), at(o, [-2, 0, 2]), at(o, [1, 0, -2]), at(o, [-1, 0, -2]));
        self.setup(&["gamemode survival".into(), setblock(pit, "water"), setblock(lava_pit, "air"), setblock(stairs, "oak_stairs"), setblock(snow, "snow_layer"), setblock(grass, "short_grass")]).await?;
        self.hold("bucket 2").await?;
        self.click("survival, an empty bucket of two on a source", pit, Face::Up, SETTLE).await?;
        self.click("survival, again, on what is left", pit, Face::Up, SETTLE).await?;
        self.setup(&[setblock(at(o, [0, 0, 3]), "water [\"liquid_depth\"=3]")]).await?;
        self.click("survival, an empty bucket on flowing water", at(o, [0, 0, 3]), Face::Up, SETTLE).await?;
        self.hold("water_bucket").await?;
        self.click("survival, a water bucket on stairs", stairs, Face::Up, SETTLE).await?;
        self.hold("bucket").await?;
        self.click("survival, an empty bucket on the wet stairs", stairs, Face::Up, SETTLE).await?;
        self.hold("lava_bucket").await?;
        self.click("survival, a lava bucket into a pit", at(lava_pit, [0, -1, 0]), Face::Up, SETTLE).await?;
        self.hold("bucket").await?;
        self.click("survival, an empty bucket on the lava", lava_pit, Face::Up, SETTLE).await?;
        self.hold("water_bucket").await?;
        self.click("survival, a water bucket on a snow layer", snow, Face::Up, SETTLE).await?;
        self.hold("water_bucket").await?;
        self.click("survival, a water bucket into grass", at(grass, [0, -1, 0]), Face::Up, SETTLE).await?;
        self.hold("lava_bucket").await?;
        self.click("survival, a lava bucket aimed into the ground", at(o, [-1, -1, 1]), Face::Down, SETTLE).await?;
        self.setup(&["gamemode creative".into()]).await?;
        self.hold("water_bucket").await?;
        self.click("creative, a water bucket", at(o, [1, -1, 1]), Face::Up, SETTLE).await?;
        self.setup(&["gamemode adventure".into()]).await?;
        self.hold("lava_bucket").await?;
        self.click("adventure, a lava bucket", at(o, [3, -1, 0]), Face::Up, SETTLE).await?;
        self.setup(&["gamemode creative".into()]).await
    }

    /// Walls a source of `liquid` in at `pos` on four sides.
    fn pool(pos: [i32; 3], liquid: &str) -> Vec<String> {
        let walls = [[-1, 0, 0], [1, 0, 0], [0, 0, -1], [0, 0, 1]].map(|side| setblock(at(pos, side), "stone"));
        walls.into_iter().chain([setblock(pos, liquid)]).collect()
    }

    async fn lava(&mut self, o: [i32; 3]) -> Result<(), String> {
        println!("# lava");
        self.stand(o).await?;
        let (lava, water) = (at(o, [2, 0, 2]), at(o, [-2, 0, 2]));
        let mut lines = Self::pool(lava, "lava");
        lines.extend(Self::pool(water, "water"));
        self.setup(&lines).await?;
        // The lava's far wall goes: it runs out onto the dirt at lava's pace.
        self.break_block("a lava source's east wall", at(lava, [1, 0, 0]), Duration::from_millis(6000)).await?;
        // Both walls between them go: the water gets to the lava source first.
        self.break_block("the lava's west wall", at(lava, [-1, 0, 0]), Duration::ZERO).await?;
        self.break_block("the water's east wall", at(water, [1, 0, 0]), Duration::from_millis(6000)).await?;
        // Lava held over a water pit by nothing: a neighbour's change sets it falling.
        let column = at(o, [0, 0, -3]);
        self.setup(&[setblock(at(column, [0, -1, 0]), "water"), setblock(at(column, [0, 3, 0]), "lava")]).await?;
        self.command(&setblock(at(column, [1, 3, 0]), "stone"), Duration::from_millis(9000)).await
    }

    async fn waterlogging(&mut self, o: [i32; 3]) -> Result<(), String> {
        println!("# waterlogging");
        self.stand(o).await?;
        let (source, other, stream) = (at(o, [0, 0, 2]), at(o, [3, 0, 2]), at(o, [-3, 0, 2]));
        let mut lines = Self::pool(source, "water");
        lines.extend(Self::pool(other, "water"));
        lines.extend([setblock(stream, "water"), setblock(at(stream, [0, 0, -1]), "stone"), setblock(at(stream, [-1, 0, 0]), "stone"), setblock(at(stream, [1, 0, 0]), "stone")]);
        lines.extend([setblock(at(stream, [0, 0, 3]), "oak_stairs"), setblock(at(stream, [1, 0, 2]), "short_grass")]);
        self.setup(&lines).await?;
        self.hold("oak_stairs 4").await?;
        self.place_block("stairs into a source", at(source, [0, -1, 0]), Face::Up, SETTLE).await?;
        self.break_block("the wet stairs", source, SETTLE).await?;
        self.hold("stone 4").await?;
        self.place_block("stone into a source", at(other, [0, -1, 0]), Face::Up, SETTLE).await?;
        // The stream runs south from its source into stairs and past grass.
        self.settle(LONG).await?;
        println!("## the stream ran: {} | {} | {}", self.name(at(stream, [0, 0, 2])), self.name(at(stream, [0, 0, 3])), self.name(at(stream, [1, 0, 2])));
        self.hold("oak_stairs 4").await?;
        self.place_block("stairs into flowing water", at(stream, [0, -1, 1]), Face::Up, LONG).await
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = std::env::args().nth(1).ok_or("usage: bds_mechanism_probe <server>")?;
    let (mut actor, mut watcher) = (connect(&server, "MechActor").await?, connect(&server, "MechWatcher").await?);
    watcher.wait_ticks(60).await?;
    actor.wait_ticks(60).await?;
    let feet = actor.movement().and_then(|m| m.position()).ok_or("the actor has no position")?.map(|c| c.floor() as i32);
    let mut p = Probe { start: std::time::Instant::now(), actor, watcher, shown: SHOWN };
    let span = APART * 3;
    p.setup(&[
        "gamemode creative".into(),
        "gamerule domobspawning false".into(),
        format!("fill {} {} {} {} {} {} dirt", feet[0] - span, feet[1] - 1, feet[2] - 8, feet[0] + span, feet[1] - 1, feet[2] + 8),
        format!("fill {} {} {} {} {} {} air", feet[0] - span, feet[1], feet[2] - 8, feet[0] + span, feet[1] + 6, feet[2] + 8),
    ])
    .await?;
    println!("## actor feet {feet:?}");
    let origin = |n: i32| [feet[0] + (n - 2) * APART, feet[1], feet[2]];
    p.falling(origin(0)).await?;
    p.plates(origin(1)).await?;
    p.buckets(origin(2)).await?;
    p.lava(origin(3)).await?;
    p.waterlogging(origin(4)).await?;
    p.actor.disconnect().await;
    p.watcher.disconnect().await;
    Ok(())
}
