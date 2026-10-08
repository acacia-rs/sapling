//! Runs a drill of command lines as an operator bot while a second operator watches, and prints
//! what each was answered: CommandOutput in full, Text, GameRulesChanged as `rule=value`, and the
//! drill's own packets. The ground truth for sapling's vanilla commands (`sapling::vanilla`).
//! `cargo run -p sapling --example bds_command_probe -- <server> [gamerule|kill|replaceitem|defaults|concrete|water|lava|redstone]` (offline mode,
//! operators by default; `gamerule` when none is named).
#[path = "shared/dump.rs"]
#[allow(dead_code)]
mod dump;
#[path = "shared/probe.rs"]
#[allow(dead_code)]
mod probe;

use std::time::{Duration, Instant};

use acacia_bot::BotEvent;
use acacia_bot::proto::{Packet, packets};
use probe::{SETTLE, connect};

const GAMERULE: &[&str] = &[
    "gamerule",
    "gamerule dodaylightcycle",
    "gamerule dodaylightcycle false",
    "gamerule dodaylightcycle false",
    "gamerule DoDaylightCycle true",
    "gamerule randomtickspeed",
    "gamerule randomtickspeed 5",
    "gamerule randomtickspeed true",
    "gamerule randomtickspeed -3",
    "gamerule dodaylightcycle 3",
    "gamerule nosuchrule true",
    "gamerule spawnradius 2000000000",
    "gamerule playerssleepingpercentage 150",
    "gamerule playerssleepingpercentage -1",
    "gamerule playerssleepingpercentage 100",
    "gamerule randomtickspeed 1",
    "gamerule randomtickspeed 4097",
    "gamerule spawnradius 128",
    "gamerule spawnradius -1",
    "gamerule functioncommandlimit 0",
    "gamerule functioncommandlimit 10001",
    "gamerule maxcommandchainlength -1",
    "gamerule DoDaylightCycle",
    "gamerule playerwaypoints",
    "gamerule playerwaypoints banana",
    "gamerule playerwaypoints off",
    "gamerule playerwaypoints 0",
    "gamerule playerwaypoints everyone",
    "gamerule locatorbar",
    "gamerule locatorbar false",
    "gamerule locatorbar true",
    "gamerule dodaylightcycle true extra",
    "gamerule",
];

/// Mobs killed: the death, the loot and when they go; with `domobloot` off, no loot.
const KILL: &[&str] = &[
    "gamerule domobspawning false",
    "summon chicken ~2 ~ ~",
    "kill @e[type=chicken]",
    "summon pig ~-2 ~ ~",
    "kill @e[type=pig]",
    "summon armor_stand ~ ~ ~2",
    "kill @e[type=armor_stand]",
    "gamerule domobloot false",
    "summon chicken ~2 ~ ~",
    "kill @e[type=chicken]",
    "gamerule domobloot true",
];
/// Items put into a chest's slots, and the ways that fails.
const REPLACEITEM: &[&str] = &[
    "setblock ~2 ~ ~ chest",
    "replaceitem block ~2 ~ ~ slot.container 0 diamond 10",
    "replaceitem block ~2 ~ ~ slot.container 1 diamond",
    "replaceitem block ~2 ~ ~ slot.container 2 stone 70",
    "replaceitem block ~2 ~ ~ slot.container 27 diamond",
    "replaceitem block ~2 ~ ~ slot.container -1 diamond",
    "replaceitem block ~2 ~ ~ slot.container 3 no_such_item",
    "replaceitem block ~2 ~ ~ slot.container 0 air",
    "replaceitem block ~ ~-1 ~ slot.container 0 diamond",
    "replaceitem block ~2 ~ ~ slot.inventory 0 diamond",
];
const REPLACEITEM_SHOWN: &[&str] = &["InventorySlot", "InventoryContent", "BlockEntityData"];

/// Which state `/setblock` gives a block named without states.
const DEFAULTS: &[&str] = &[
    "setblock ~2 ~ ~ chest",
    "testforblock ~2 ~ ~ chest [\"minecraft:cardinal_direction\"=\"north\"]",
    "testforblock ~2 ~ ~ chest [\"minecraft:cardinal_direction\"=\"south\"]",
    "setblock ~3 ~ ~ furnace",
    "testforblock ~3 ~ ~ furnace [\"minecraft:cardinal_direction\"=\"north\"]",
    "testforblock ~3 ~ ~ furnace [\"minecraft:cardinal_direction\"=\"south\"]",
    "setblock ~4 ~ ~ oak_stairs",
    "testforblock ~4 ~ ~ oak_stairs [\"weirdo_direction\"=0]",
    "setblock ~5 ~ ~ hopper",
    "testforblock ~5 ~ ~ hopper [\"facing_direction\"=0]",
    // Blocks whose creative item records another state than their first.
    "setblock ~6 ~ ~ dropper",
    "testforblock ~6 ~ ~ dropper [\"facing_direction\"=3]",
    "setblock ~7 ~ ~ piston",
    "testforblock ~7 ~ ~ piston [\"facing_direction\"=1]",
    "setblock ~8 ~ ~ amethyst_cluster",
    "testforblock ~8 ~ ~ amethyst_cluster [\"minecraft:block_face\"=\"up\"]",
    "setblock ~9 ~ ~ brown_mushroom_block",
    "testforblock ~9 ~ ~ brown_mushroom_block [\"huge_mushroom_bits\"=14]",
    "setblock ~12 ~ ~ pointed_dripstone",
    "testforblock ~12 ~ ~ pointed_dripstone [\"hanging\"=true]",
    "setblock ~11 ~ ~ small_dripleaf_block",
    "testforblock ~11 ~ ~ small_dripleaf_block [\"upper_block_bit\"=true]",
    "testforblock ~11 ~ ~ small_dripleaf_block [\"minecraft:cardinal_direction\"=\"east\"]",
    "setblock ~12 ~ ~ stonecutter_block",
    "testforblock ~12 ~ ~ stonecutter_block [\"minecraft:cardinal_direction\"=\"north\"]",
    "setblock ~13 ~ ~ suspicious_sand",
    "testforblock ~13 ~ ~ suspicious_sand [\"hanging\"=true]",
];
const DEFAULTS_SHOWN: &[&str] = &["UpdateBlock"];

/// When concrete powder sets, on a stone stage: put beside water, dropped into a pool two deep and
/// one deep, water put beside it and on it.
const CONCRETE: &[&str] = &[
    "fill ~2 ~-4 ~6 ~14 ~-1 ~8 stone",
    "fill ~2 ~ ~6 ~14 ~4 ~8 air",
    // Water only in holes in the stone, where it stays.
    "setblock ~3 ~-1 ~7 water",
    "setblock ~4 ~-1 ~7 red_concrete_powder",
    "fill ~6 ~-2 ~7 ~6 ~-1 ~7 water",
    "setblock ~6 ~3 ~7 red_concrete_powder",
    "setblock ~13 ~-1 ~7 water",
    "setblock ~13 ~3 ~7 red_concrete_powder",
    "setblock ~8 ~-1 ~7 red_concrete_powder",
    "setblock ~9 ~-1 ~7 water",
    // Last: this water spreads over the stage.
    "setblock ~11 ~-1 ~7 red_concrete_powder",
    "setblock ~11 ~ ~7 water",
];
/// What water does to an item, an orb and an armour stand dropped into a pool three deep.
const WATER: &[&str] = &[
    "fill ~2 ~-5 ~6 ~8 ~-1 ~12 stone",
    "fill ~2 ~ ~6 ~8 ~4 ~12 air",
    "fill ~3 ~-4 ~7 ~7 ~-1 ~9 water",
    "setblock ~5 ~2 ~8 dirt",
    "setblock ~5 ~2 ~8 air destroy",
    "summon xp_orb ~6.5 ~1 ~8.5",
    "summon armor_stand ~3.5 ~1 ~8.5",
];
const WATER_SHOWN: &[&str] = &["AddItemEntity", "AddEntity", "RemoveEntity", "MoveEntityDelta", "SetEntityMotion", "MotionPredictionHints", "LevelSoundEvent", "SetEntityData"];
/// The water drill over a pool of lava, which also burns what falls in.
const LAVA: &[&str] = &[
    "fill ~2 ~-5 ~6 ~8 ~-1 ~12 stone",
    "fill ~2 ~ ~6 ~8 ~4 ~12 air",
    "fill ~3 ~-4 ~7 ~7 ~-1 ~9 lava",
    "setblock ~5 ~2 ~8 dirt",
    "setblock ~5 ~2 ~8 air destroy",
    // Mid-pool: /summon throws an orb up to about two blocks.
    "summon xp_orb ~5.5 ~1 ~8.5",
    "summon armor_stand ~3.5 ~1 ~8.5",
];
const LAVA_SHOWN: &[&str] = &[
    "AddItemEntity",
    "AddEntity",
    "RemoveEntity",
    "MoveEntityDelta",
    "SetEntityMotion",
    "MotionPredictionHints",
    "LevelSoundEvent",
    "EntityEvent",
    "SetEntityData",
    "LevelEvent",
];
/// Redstone on a stone stage: a lamp by a power block, dust from a power block to a lamp 15 away
/// and 16 away, a lever powering through the block it is on, a torch put out by power into its
/// block, a door and a hopper powered, and pressed buttons left to spring back.
const REDSTONE: &[&str] = &[
    "fill ~2 ~-1 ~4 ~22 ~-1 ~16 stone",
    "fill ~2 ~ ~4 ~22 ~4 ~16 air",
    "setblock ~3 ~ ~5 redstone_lamp",
    "setblock ~4 ~ ~5 redstone_block",
    "setblock ~4 ~ ~5 air",
    "fill ~3 ~ ~7 ~17 ~ ~7 redstone_wire",
    "setblock ~18 ~ ~7 redstone_lamp",
    "fill ~3 ~ ~9 ~18 ~ ~9 redstone_wire",
    "setblock ~19 ~ ~9 redstone_lamp",
    "setblock ~2 ~ ~7 redstone_block",
    "setblock ~2 ~ ~9 redstone_block",
    "setblock ~2 ~ ~7 air",
    "setblock ~3 ~ ~12 stone",
    "setblock ~4 ~ ~12 redstone_lamp",
    "setblock ~3 ~1 ~12 lever [\"lever_direction\"=\"up_north_south\",\"open_bit\"=true]",
    "setblock ~3 ~1 ~12 lever [\"lever_direction\"=\"up_north_south\",\"open_bit\"=false]",
    "setblock ~7 ~ ~12 stone",
    "setblock ~7 ~1 ~12 redstone_torch",
    "setblock ~7 ~2 ~12 redstone_lamp",
    "setblock ~6 ~ ~12 redstone_block",
    "setblock ~6 ~ ~12 air",
    "setblock ~10 ~ ~12 iron_door",
    "setblock ~11 ~ ~12 redstone_block",
    "setblock ~11 ~ ~12 air",
    "setblock ~13 ~ ~12 hopper",
    "setblock ~14 ~ ~12 redstone_block",
    "setblock ~14 ~ ~12 air",
    "setblock ~16 ~ ~12 stone",
    "setblock ~17 ~ ~12 redstone_lamp",
    "setblock ~16 ~1 ~12 stone_button [\"facing_direction\"=1,\"button_pressed_bit\"=true]",
    "setblock ~16 ~1 ~12 wooden_button [\"facing_direction\"=1,\"button_pressed_bit\"=true]",
    "testforblock ~16 ~1 ~12 wooden_button [\"button_pressed_bit\"=true]",
];
/// A block of redstone put by a lamp, an iron door and a hopper and taken away again, 20 times,
/// [`PHASE_SETTLE`] apart so the toggles fall on both ticks of the 2-tick evaluation (research C1).
const REDSTONE_PHASE: &[&str] = &[
    "fill ~2 ~-1 ~4 ~6 ~-1 ~6 stone",
    "fill ~2 ~ ~4 ~6 ~2 ~6 air",
    "setblock ~3 ~ ~5 redstone_lamp",
    "setblock ~5 ~ ~5 iron_door",
    "setblock ~4 ~ ~6 hopper",
    "setblock ~4 ~ ~5 redstone_block",
    "setblock ~4 ~ ~5 air",
    "setblock ~4 ~ ~5 redstone_block",
    "setblock ~4 ~ ~5 air",
    "setblock ~4 ~ ~5 redstone_block",
    "setblock ~4 ~ ~5 air",
    "setblock ~4 ~ ~5 redstone_block",
    "setblock ~4 ~ ~5 air",
    "setblock ~4 ~ ~5 redstone_block",
    "setblock ~4 ~ ~5 air",
    "setblock ~4 ~ ~5 redstone_block",
    "setblock ~4 ~ ~5 air",
    "setblock ~4 ~ ~5 redstone_block",
    "setblock ~4 ~ ~5 air",
    "setblock ~4 ~ ~5 redstone_block",
    "setblock ~4 ~ ~5 air",
    "setblock ~4 ~ ~5 redstone_block",
    "setblock ~4 ~ ~5 air",
    "setblock ~4 ~ ~5 redstone_block",
    "setblock ~4 ~ ~5 air",
    "setblock ~4 ~ ~5 redstone_block",
    "setblock ~4 ~ ~5 air",
    "setblock ~4 ~ ~5 redstone_block",
    "setblock ~4 ~ ~5 air",
    "setblock ~4 ~ ~5 redstone_block",
    "setblock ~4 ~ ~5 air",
    "setblock ~4 ~ ~5 redstone_block",
    "setblock ~4 ~ ~5 air",
    "setblock ~4 ~ ~5 redstone_block",
    "setblock ~4 ~ ~5 air",
    "setblock ~4 ~ ~5 redstone_block",
    "setblock ~4 ~ ~5 air",
];
/// 10.5 ticks: a lamp is out 6 after its power goes, and each toggle lands half a tick later in
/// the 2-tick cycle than the one before.
const PHASE_SETTLE: Duration = Duration::from_millis(525);
const REDSTONE_SHOWN: &[&str] = &["UpdateBlock", "UpdateBlockSynced", "UpdateSubchunkBlocks", "LevelSoundEvent", "LevelEvent", "BlockEvent"];

/// Long enough to see a thing sink or rise through the pool.
const WATER_SETTLE: Duration = Duration::from_millis(8000);
const CONCRETE_SHOWN: &[&str] = &["UpdateBlock", "UpdateBlockSynced", "UpdateSubchunkBlocks", "AddEntity", "RemoveEntity", "AddItemEntity", "LevelEvent", "LevelSoundEvent"];

const KILL_SHOWN: &[&str] = &["EntityEvent", "AddEntity", "AddItemEntity", "RemoveEntity", "LevelSoundEvent", "SetEntityData", "Animate", "MobEffect"];
/// How long each line of the kill drill is watched: a dead mob goes a second after.
const KILL_SETTLE: Duration = Duration::from_millis(2500);

fn show(start: Instant, shown: &[&str], who: &str, event: Option<BotEvent>) -> Result<(), String> {
    let packet = match event {
        Some(BotEvent::Packet(packet)) => packet,
        Some(BotEvent::Disconnected(reason)) => return Err(format!("{who} disconnected: {reason:?}")),
        Some(_) => return Ok(()),
        None => return Err(format!("{who} is gone")),
    };
    let ms = start.elapsed().as_millis();
    if packet.id == packets::CommandOutput::ID {
        let answer = packet.decode::<packets::CommandOutput>().map_err(|e| e.to_string())?;
        let lines: Vec<String> = answer.output.iter().map(|line| format!("{} success={} {:?}", line.message_id, line.success, line.parameters)).collect();
        println!("{ms:>7} {who:<7} CommandOutput success_count={} data={:?} {}", answer.success_count, answer.data, lines.join(" | "));
    } else if packet.id == packets::GameRulesChanged::ID {
        let rules = packet.decode::<packets::GameRulesChanged>().map_err(|e| e.to_string())?;
        let shown: Vec<String> = rules.rules.iter().map(|rule| format!("{}={:?}", rule.name, rule.value)).collect();
        println!("{ms:>7} {who:<7} GameRulesChanged {}", shown.join(" "));
    } else if let Some(name) = acacia_bot::proto::packet_name(packet.id).filter(|name| ["Text", "SetTime", "SyncWorldClocks"].contains(name) || shown.contains(name)) {
        let text: String = dump::dump(&packet).unwrap_or_default().chars().take(SHOWN).collect();
        println!("{ms:>7} {who:<7} {name} {text}");
    }
    Ok(())
}

/// Characters of a drill's own packets shown.
const SHOWN: usize = 4000;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let server = args.next().ok_or("usage: bds_command_probe <server> [gamerule|kill|replaceitem|defaults|concrete|water|lava|redstone|redstone-phase]")?;
    let (lines, shown, settle) = match args.next().as_deref() {
        None | Some("gamerule") => (GAMERULE, &[][..], SETTLE),
        Some("kill") => (KILL, KILL_SHOWN, KILL_SETTLE),
        Some("replaceitem") => (REPLACEITEM, REPLACEITEM_SHOWN, SETTLE),
        Some("defaults") => (DEFAULTS, DEFAULTS_SHOWN, SETTLE),
        Some("concrete") => (CONCRETE, CONCRETE_SHOWN, KILL_SETTLE),
        Some("water") => (WATER, WATER_SHOWN, WATER_SETTLE),
        Some("lava") => (LAVA, LAVA_SHOWN, WATER_SETTLE),
        Some("redstone") => (REDSTONE, REDSTONE_SHOWN, KILL_SETTLE),
        Some("redstone-phase") => (REDSTONE_PHASE, REDSTONE_SHOWN, PHASE_SETTLE),
        Some(other) => return Err(format!("no drill {other}").into()),
    };
    let (mut actor, mut watcher) = (connect(&server, "CmdActor").await?, connect(&server, "CmdWatcher").await?);
    watcher.wait_ticks(60).await?;
    actor.wait_ticks(60).await?;
    for bot in [&mut actor, &mut watcher] {
        while let Ok(Some(_)) = tokio::time::timeout(Duration::from_millis(5), bot.next()).await {}
    }
    let start = Instant::now();
    for line in lines {
        println!("## /{line}");
        actor.client().command(line);
        let end = tokio::time::Instant::now() + settle;
        loop {
            tokio::select! {
                event = actor.next() => show(start, shown, "actor", event)?,
                event = watcher.next() => show(start, shown, "watcher", event)?,
                _ = tokio::time::sleep_until(end) => break,
            }
        }
    }
    actor.disconnect().await;
    watcher.disconnect().await;
    Ok(())
}
