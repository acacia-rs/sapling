//! Runs a drill of command lines as an operator bot while a second operator watches, and prints
//! what each was answered: CommandOutput in full, Text, GameRulesChanged as `rule=value`, and the
//! drill's own packets. The ground truth for sapling's vanilla commands (`sapling::vanilla`).
//! `cargo run -p sapling --example bds_command_probe -- <server> [gamerule|kill|replaceitem|defaults|concrete|water|lava|redstone|redstone-phase|conductors|shapes|burnout|repeater-facing|repeaters|comparators|reads|observer-facing|observers|piston-facing|pistons|slime|burn|harm|douse|hit|tools|reach|hurt|smother|hunger|eat|tire|bite|slain|chase|toil|armour|effects|sick|worn|kinds|stroll|flee|around|drink|more|shove|byhand|bedspawn|boom|shoot|wear|provoke|provoke2|provoke3|witch|ender|goat|evoker|shulker|ghast|breeze|vex|stare|dolphin|fx|drink2|shoot2|golem|clock|sounds|loot|loot2|names|sun|weather|spiders|kinds2|pushed|pvp|blaze|duel|placing|item-names|placing2|daylight|clicks|tnt|bonemeal|fire|quench|discs|ores|crops|grows|leaves|trample|clicks2|scaffold|grows2|copper|oresteps|bonemeal2|leafreach|stacks|snowpile]` (offline mode,
//! operators by default; `gamerule` when none is named).
#[path = "shared/burn_drill.rs"]
mod burn;
#[path = "shared/comparator_drill.rs"]
mod comparator;
#[path = "shared/dump.rs"]
#[allow(dead_code)]
mod dump;
#[path = "shared/player_drill.rs"]
mod player;
#[path = "shared/probe.rs"]
#[allow(dead_code)]
mod probe;
#[path = "shared/piston_drill.rs"]
mod piston;
#[path = "shared/redstone_drills.rs"]
mod redstone;

use std::time::{Duration, Instant};

use acacia_bot::{Bot, BotEvent};
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

fn owned(lines: &[&str]) -> Vec<String> {
    lines.iter().map(|line| line.to_string()).collect()
}

/// The actor hits the newest mob it knows of the kind `hit` names, twice in a row for `2 kind`,
/// while the watcher's packets are shown.
async fn strike(actor: &mut Bot, watcher: &mut Bot, start: Instant, shown: &[&str], hit: &str) -> Result<(), String> {
    let (times, kind) = hit.strip_prefix('2').map_or((1, hit), |kind| (2, kind));
    let kind = format!("minecraft:{}", kind.trim());
    let id = actor.state().entities.iter().filter(|entity| entity.kind == kind).map(|entity| entity.runtime_id).max().ok_or(format!("no {kind} to hit"))?;
    for _ in 0..times {
        let action = actor.attack(id);
        tokio::pin!(action);
        loop {
            tokio::select! {
                result = &mut action => break println!("## hit {id}: {result:?} at {}", start.elapsed().as_millis()),
                event = watcher.next() => show(start, shown, "watcher", event)?,
            }
        }
    }
    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let server = args.next().ok_or("usage: bds_command_probe <server> [gamerule|kill|replaceitem|defaults|concrete|water|lava|redstone|redstone-phase|conductors|shapes|burnout|repeater-facing|repeaters|comparators|reads|observer-facing|observers|piston-facing|pistons|slime|burn|harm|douse|hit|tools|reach|hurt|smother|hunger|eat|tire|bite|slain|chase|toil|armour|effects|sick|worn|kinds|stroll|flee|around|drink|more|shove|byhand|bedspawn|boom|shoot|wear|provoke|provoke2|provoke3|witch|ender|goat|evoker|shulker|ghast|breeze|vex|stare|dolphin|fx|drink2|shoot2|golem|clock|sounds|loot|loot2|names|sun|weather|spiders|kinds2|pushed|pvp|blaze|duel|placing|item-names|placing2|daylight|clicks|tnt|bonemeal|fire|quench|discs|ores|crops|grows|leaves|trample|clicks2|scaffold|grows2|copper|oresteps|bonemeal2|leafreach|stacks|snowpile|holders]")?;
    let (lines, shown, settle) = match args.next().as_deref() {
        None | Some("gamerule") => (owned(GAMERULE), &[][..], SETTLE),
        Some("kill") => (owned(KILL), KILL_SHOWN, KILL_SETTLE),
        Some("replaceitem") => (owned(REPLACEITEM), REPLACEITEM_SHOWN, SETTLE),
        Some("defaults") => (owned(DEFAULTS), DEFAULTS_SHOWN, SETTLE),
        Some("concrete") => (owned(CONCRETE), CONCRETE_SHOWN, KILL_SETTLE),
        Some("water") => (owned(WATER), WATER_SHOWN, WATER_SETTLE),
        Some("lava") => (owned(LAVA), LAVA_SHOWN, WATER_SETTLE),
        Some("redstone") => (owned(redstone::REDSTONE), redstone::REDSTONE_SHOWN, KILL_SETTLE),
        Some("redstone-phase") => (owned(redstone::REDSTONE_PHASE), redstone::REDSTONE_SHOWN, redstone::PHASE_SETTLE),
        Some("conductors") => (redstone::conductors(), redstone::REDSTONE_SHOWN, redstone::CONDUCTORS_SETTLE),
        Some("burnout") => (owned(redstone::BURNOUT), redstone::REDSTONE_SHOWN, KILL_SETTLE),
        Some("repeater-facing") => (redstone::repeater_facing(), redstone::REDSTONE_SHOWN, KILL_SETTLE),
        Some("repeaters") => (redstone::repeaters(), redstone::REDSTONE_SHOWN, KILL_SETTLE),
        Some("comparators") => (redstone::comparators(), redstone::REDSTONE_SHOWN, KILL_SETTLE),
        Some("reads") => (comparator::reads(), redstone::REDSTONE_SHOWN, KILL_SETTLE),
        Some("observer-facing") => (redstone::observer_facing(), redstone::REDSTONE_SHOWN, KILL_SETTLE),
        Some("observers") => (redstone::observers(), redstone::REDSTONE_SHOWN, KILL_SETTLE),
        Some("piston-facing") => (redstone::piston_facing(), redstone::PISTON_SHOWN, KILL_SETTLE),
        Some("pistons") => (redstone::pistons(), redstone::PISTON_SHOWN, KILL_SETTLE),
        Some("slime") => (piston::slime(), redstone::PISTON_SHOWN, KILL_SETTLE),
        Some("burn") => (burn::burn(), burn::BURN_SHOWN, burn::BURN_SETTLE),
        Some("harm") => (burn::harm(), burn::BURN_SHOWN, burn::BURN_SETTLE),
        Some("douse") => (burn::douse(), burn::BURN_SHOWN, burn::BURN_SETTLE),
        Some("hit") => (burn::hit(), burn::HIT_SHOWN, burn::BURN_SETTLE),
        Some("tools") => (burn::tools(), burn::HIT_SHOWN, burn::BURN_SETTLE),
        Some("reach") => (burn::reach(), burn::HIT_SHOWN, burn::BURN_SETTLE),
        Some("shapes") => (redstone::shapes(), redstone::REDSTONE_SHOWN, redstone::CONDUCTORS_SETTLE),
        Some(other) => match player::drill(other) {
            Some((lines, shown)) => (lines, shown, burn::BURN_SETTLE),
            None => return Err(format!("no drill {other}").into()),
        },
    };
    let (mut actor, mut watcher) = (connect(&server, "CmdActor").await?, connect(&server, "CmdWatcher").await?);
    watcher.wait_ticks(60).await?;
    actor.wait_ticks(60).await?;
    for bot in [&mut actor, &mut watcher] {
        while let Ok(Some(_)) = tokio::time::timeout(Duration::from_millis(5), bot.next()).await {}
    }
    let start = Instant::now();
    for line in &lines {
        println!("## /{line}");
        match line.strip_prefix("!hit") {
            Some(hit) => strike(&mut actor, &mut watcher, start, shown, hit).await?,
            None if player::act(&mut actor, line).await => {}
            None if !line.is_empty() => drop(actor.client().command(line)),
            None => {}
        }
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
