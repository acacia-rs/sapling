//! Runs a drill of command lines as an operator bot while a second operator watches, and prints
//! what each was answered: CommandOutput in full, Text, GameRulesChanged as `rule=value`, and the
//! drill's own packets. The ground truth for sapling's vanilla commands (`sapling::vanilla`).
//! `cargo run -p sapling --example bds_command_probe -- <server> [gamerule|kill|replaceitem|defaults|concrete|water|lava|redstone|redstone-phase|conductors|shapes|burnout|repeater-facing|repeaters|comparators|reads|observer-facing|observers|piston-facing|pistons|slime|burn|harm|douse|hit|tools|reach|hurt|smother|hunger|eat|tire|bite|slain|chase|toil|armour|effects|sick|worn|kinds|stroll|flee|around|drink|more|shove|byhand|bedspawn|boom|shoot|wear|provoke|provoke2|provoke3|witch|ender|goat|evoker|shulker|ghast|breeze|vex|stare|dolphin|pandas|witch-drinks|goat-cows|mob-effects|mob-effects2|effects3|wind-burst|player-omens|fx|drink2|shoot2|golem|clock|sounds|loot|loot2|names|sun|weather|spiders|kinds2|pushed|pvp|blaze|duel|placing|item-names|placing2|daylight|clicks|tnt|bonemeal|fire|quench|discs|ores|crops|grows|leaves|trample|clicks2|scaffold|grows2|copper|oresteps|bonemeal2|leafreach|stacks|snowpile|worldcmds|itemcmds|bubbles|dripstone|plants|plantmeal|pendingticks|blasts|creeperdrops|holds|rails|cauldrons|cauldronrain|bells|conduits|bookshelves|hives|shield|totem|crit|sprint|swing-wear|thorny|void|anvil]` (offline mode,
//! operators by default; `gamerule` when none is named).
#[path = "shared/batch_drill.rs"]
mod batch;
#[path = "shared/burn_drill.rs"]
mod burn;
#[path = "shared/comparator_drill.rs"]
mod comparator;
#[path = "shared/functions_drill.rs"]
mod functions;
#[path = "shared/command_lists.rs"]
mod command_lists;
#[path = "shared/command_show.rs"]
mod command_show;
#[path = "shared/dump.rs"]
#[allow(dead_code)]
mod dump;
#[path = "shared/liquid_drill.rs"]
mod liquid;
#[path = "shared/player_drill.rs"]
mod player;
#[path = "shared/probe.rs"]
#[allow(dead_code)]
mod probe;
#[path = "shared/piston_drill.rs"]
mod piston;
#[path = "shared/redstone_drills.rs"]
mod redstone;
#[path = "shared/world_command_drill.rs"]
mod world_commands;
#[path = "shared/structure_drill.rs"]
mod structures;
#[path = "shared/nature_drill.rs"]
mod nature;
#[path = "shared/blast_drill.rs"]
mod blast;
#[path = "shared/hold_drill.rs"]
mod hold;
#[path = "shared/rail_drill.rs"]
mod rail;
#[path = "shared/cauldron_drill.rs"]
mod cauldron;
#[path = "shared/block_entity_drill.rs"]
mod keeper;
#[path = "shared/execute_drill.rs"]
mod execute;
#[path = "shared/client_command_drill.rs"]
mod client;
#[path = "shared/maps_drill.rs"]
mod maps;
#[path = "shared/bundle_drill.rs"]
mod bundles;
#[path = "shared/creature_drill.rs"]
mod creatures;
#[path = "shared/companion_drill.rs"]
mod companion;
#[path = "shared/rules_drill.rs"]
mod rules;
#[path = "shared/warden_drill.rs"]
mod warden;
#[path = "shared/props_drill.rs"]
mod props;
#[path = "shared/protocol_drill.rs"]
mod protocol;
#[path = "shared/command_block_drill.rs"]
mod cmdblocks;
#[path = "shared/command_link_drill.rs"]
mod cmdlinks;
#[path = "shared/structure_block_drill.rs"]
mod structblocks;
#[path = "shared/parity_drill.rs"]
mod parity;
#[path = "shared/harm_drill.rs"]
mod harm;
#[path = "shared/fights_drill.rs"]
mod fights;
#[path = "shared/crafting_drill.rs"]
mod crafting;
#[path = "shared/server_drill.rs"]
mod server;
#[path = "shared/redstone_parts_drill.rs"]
mod redstone_parts;
#[path = "shared/block_ticks_drill.rs"]
mod block_ticks;
#[path = "shared/brush_hold.rs"]
mod brush_hold;
#[path = "shared/vitality_drill.rs"]
mod vitality;
#[path = "shared/village_drill.rs"]
mod villages;
#[path = "shared/eggs_drill.rs"]
mod eggs;
#[path = "shared/tack_drill.rs"]
mod tack;
#[path = "shared/growth_drill.rs"]
mod growth;
#[path = "shared/mob_fix_drill.rs"]
mod mobfix;
#[path = "shared/command_fix_drill.rs"]
mod command_fix;
#[path = "shared/leftover_drill.rs"]
mod leftovers;
#[path = "shared/open3_drill.rs"]
mod open3;
#[path = "shared/dwellers_drill.rs"]
mod dwellers;
#[path = "shared/mount_plant_drill.rs"]
mod mount_plant;
#[path = "shared/saddle_drill.rs"]
mod saddles;
#[path = "shared/pillagers_drill.rs"]
mod pillagers;
#[path = "shared/rain_drill.rs"]
mod rain;
#[path = "shared/last3_drill.rs"]
mod last3;
#[path = "shared/last_drill.rs"]
mod last;
#[path = "shared/fifth3_drill.rs"]
mod fifth3;

use std::time::{Duration, Instant};

use acacia_bot::Bot;
use command_show::show;
use probe::{SETTLE, clock, connect};
use liquid::{CONCRETE, CONCRETE_SHOWN, LAVA, LAVA_SHOWN, WATER, WATER_SETTLE, WATER_SHOWN};
use command_lists::{DEFAULTS, DEFAULTS_SHOWN, GAMERULE, KILL, KILL_SETTLE, KILL_SHOWN, REPLACEITEM, REPLACEITEM_SHOWN};

/// A drill a scene module names: its lines and the packets shown.
type Named = Option<(Vec<String>, &'static [&'static str])>;

/// Every scene module's `drill`, one a line; a new module adds its line here.
const DRILLS: &[fn(&str) -> Named] = &[
    player::drill,
    structures::drill,
    nature::drill,
    blast::drill,
    hold::drill,
    rail::drill,
    cauldron::drill,
    keeper::drill,
    execute::drill,
    client::drill,
    maps::drill,
    bundles::drill,
    creatures::drill,
    warden::drill,
    props::drill,
    rules::drill,
    protocol::drill,
    cmdblocks::drill,
    cmdlinks::drill,
    structblocks::drill,
    parity::drill,
    companion::drill,
    harm::drill,
    fights::drill,
    crafting::drill,
    server::drill,
    redstone_parts::drill,
    block_ticks::drill,
    vitality::drill,
    villages::drill,
    eggs::drill,
    tack::drill,
    growth::drill,
    mobfix::drill,
    command_fix::drill,
    leftovers::drill,
    open3::drill,
    dwellers::drill,
    mount_plant::drill,
    saddles::drill,
    pillagers::drill,
    rain::drill,
    last3::drill,
    last::drill,
    fifth3::drill,
];

fn owned(lines: &[&str]) -> Vec<String> {
    lines.iter().map(|line| line.to_string()).collect()
}

/// The actor hits the newest mob it knows of the kind `hit` names, twice in a row for `2 kind`,
/// while the watcher's packets are shown.
async fn strike(actor: &mut Bot, watcher: &mut Bot, start: Instant, shown: &[&str], hit: &str) -> Result<(), String> {
    let hit = hit.trim();
    let (times, kind) = hit.strip_prefix('2').map_or((1, hit), |kind| (2, kind));
    let kind = format!("minecraft:{}", kind.trim());
    let id = actor.state().entities.iter().filter(|entity| entity.kind == kind).map(|entity| entity.runtime_id).max().ok_or(format!("no {kind} to hit"))?;
    for _ in 0..times {
        let action = actor.attack(id);
        tokio::pin!(action);
        loop {
            tokio::select! {
                result = &mut action => break println!("## hit {id}: {result:?} at {}", clock::tick_ms(start)),
                event = watcher.next() => show(start, shown, "watcher", event)?,
            }
        }
    }
    Ok(())
}

/// Does the actor's `!` line while the watcher's packets are shown, so they keep their times.
async fn act(actor: &mut Bot, watcher: &mut Bot, start: Instant, shown: &[&str], line: &str) -> Result<(), String> {
    let action = async {
        // A module that does `!` lines adds its line here; the player's is the fallback.
        let done = maps::act(actor, line).await
            || bundles::act(actor, line).await
            || creatures::act(actor, line).await
            || cmdblocks::act(actor, line).await
            || structblocks::act(actor, line).await
            || crafting::act(actor, line).await
            || block_ticks::act(actor, line).await
            || tack::act(actor, line).await;
        if !done {
            player::act(actor, line).await;
        }
    };
    tokio::pin!(action);
    loop {
        tokio::select! {
            _ = &mut action => return Ok(()),
            event = watcher.next() => show(start, shown, "watcher", event)?,
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let server = args.next().ok_or("usage: bds_command_probe <server> [gamerule|kill|replaceitem|defaults|concrete|water|lava|redstone|redstone-phase|conductors|shapes|burnout|repeater-facing|repeaters|comparators|reads|observer-facing|observers|piston-facing|pistons|slime|burn|harm|douse|hit|tools|reach|hurt|smother|hunger|eat|tire|bite|slain|chase|toil|armour|effects|sick|worn|kinds|stroll|flee|around|drink|more|shove|byhand|bedspawn|boom|shoot|wear|provoke|provoke2|provoke3|witch|ender|goat|evoker|shulker|ghast|breeze|vex|stare|dolphin|pandas|witch-drinks|goat-cows|mob-effects|mob-effects2|effects3|wind-burst|player-omens|fx|drink2|shoot2|golem|clock|sounds|loot|loot2|names|sun|weather|spiders|kinds2|pushed|pvp|blaze|duel|placing|item-names|placing2|daylight|clicks|tnt|bonemeal|fire|quench|discs|ores|crops|grows|leaves|trample|clicks2|scaffold|grows2|copper|oresteps|bonemeal2|leafreach|stacks|snowpile|holders|messaging|scoreboard|scoreboard2|bubbles|dripstone|plants|plantmeal|pendingticks|blasts|creeperdrops|holds|rails|cauldrons|cauldronrain|bells|conduits|bookshelves|hives|shield|totem|crit|sprint|swing-wear|thorny|void|anvil]")?;
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
        Some("swim") => (burn::swim(), burn::HIT_SHOWN, burn::BURN_SETTLE),
        Some("batch") => (batch::batch(), batch::BATCH_SHOWN, burn::BURN_SETTLE),
        Some("reach") => (burn::reach(), burn::HIT_SHOWN, burn::BURN_SETTLE),
        Some("shapes") => (redstone::shapes(), redstone::REDSTONE_SHOWN, redstone::CONDUCTORS_SETTLE),
        Some("worldcmds") => (world_commands::world(), world_commands::WORLD_SHOWN, SETTLE),
        Some("itemcmds") => (world_commands::items(), world_commands::ITEM_SHOWN, SETTLE),
        Some("functions") => (functions::functions(), functions::FUNCTIONS_SHOWN, SETTLE),
        Some(other) => match DRILLS.iter().find_map(|drill| drill(other)) {
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
    clock::mark_start();
    for line in &lines {
        println!("## /{line}");
        match line.strip_prefix("!hit") {
            Some(hit) => strike(&mut actor, &mut watcher, start, shown, hit).await?,
            // The actor leaves and joins again; what it is sent as it comes is shown.
            None if line == "!rejoin" => {
                actor.disconnect().await;
                tokio::time::sleep(Duration::from_secs(2)).await;
                actor = connect(&server, "CmdActor").await?;
            }
            None if line.starts_with('!') => act(&mut actor, &mut watcher, start, shown, line).await?,
            None if !line.is_empty() => drop(actor.client().command(line)),
            None => {}
        }
        let end = tokio::time::Instant::now() + clock::scaled(settle);
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
