//! Fourteenth wave, group AF (drill2): `af2baby`, a baby cow, pig and sheep beside a held adult of
//! their kind that is moved about: their follow pace and stop distance. `af2wolf`, four wolves tamed
//! and stood up (`!tameall wolf`, `!standall wolf`) behind a wall from a held zombie that hurts the
//! survival actor and that the actor then hits. `af2climb`, a zombie under the actor standing on a
//! block four high, with a ladder, then vines, then scaffolding up its side; `af2shaft`, the zombie
//! walled into the climbable's cell. `af2spear`, a spear rider
//! on its zombie horse and a spear zombie on foot charging the actor from the north (-z), where
//! bds-j3ride's and bds-aa2spear's came from the south.

use std::time::Duration;

use acacia_bot::Bot;
use acacia_bot::proto::types::MetadataFlags1 as Flags;

use crate::villages::wait;

fn owned<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

fn field(time: &str) -> Vec<String> {
    let mut out = owned(["kill @e[type=!player]", "gamerule domobspawning false", "gamerule dodaylightcycle false", "gamerule falldamage false", "weather clear", "difficulty easy"]);
    out.push(format!("time set {time}"));
    out.extend(owned(["gamemode creative @a", "effect @a clear", "clear @s", "fill -14 -60 -14 14 -54 14 air", "fill -14 -61 -14 14 -61 14 grass", "tp CmdWatcher 0.5 -60 -40.5", ""]));
    out
}

/// A held adult `mum` and a baby 7 east of it; the adult moved 7 west, then 12 south.
fn babies() -> Vec<String> {
    let mut out = field("6000");
    out.extend(owned(["tp @s 0.5 -60 -12.5"]));
    // `spawn_adult`: a plain summon is a baby 1 time in 20 (bds-af2baby.txt's pig).
    for (kind, adult) in [("cow", "minecraft:spawn_adult"), ("pig", "minecraft:spawn_adult"), ("sheep", "spawn_adult")] {
        out.extend([format!("summon {kind} 0.5 -60 0.5 0 0 {adult} mum"), "effect @e[name=mum] slowness 1000 255 true".into(), format!("summon {kind} 7.5 -60 0.5 270 0 minecraft:entity_born"), String::new()]);
        wait(&mut out, 8);
        out.extend(owned(["tp @e[name=mum] -6.5 -60 0.5", ""]));
        wait(&mut out, 8);
        out.extend(owned(["tp @e[name=mum] -6.5 -60 12.5", ""]));
        wait(&mut out, 8);
        out.extend([format!("kill @e[type={kind}]"), String::new()]);
    }
    out
}

fn wolves_behind_wall() -> Vec<String> {
    let mut out = field("18000");
    out.extend(owned(["gamemode survival @s", "effect @s resistance 1000 255 true", "tp @s 1.5 -60 0.5 -90 0"]));
    for z in ["-1.5", "-0.5", "0.5", "1.5"] {
        out.push(format!("summon wolf 3.5 -60 {z}"));
    }
    out.extend(owned(["effect @e[type=wolf] slowness 60 255 true", "give @s bone 64", "", "!tameall wolf", "clear @s", "", "!standall wolf", ""]));
    out.extend(owned(["tp @s -0.5 -60 0.5 90 0", "fill 2 -60 -6 2 -58 6 stone", "effect @e[type=wolf] clear", "summon zombie z -1.5 -60 0.5", "effect @e[name=z] slowness 1000 255 true", ""]));
    out.extend(owned(["effect @e[name=z] resistance 1000 3 true", "effect @e[name=z] regeneration 1000 4 true", ""]));
    wait(&mut out, 12);
    out.push("!hit zombie".into());
    wait(&mut out, 20);
    out.extend(owned(["kill @e[type=!player]", "fill 2 -60 -6 2 -58 6 air", "gamemode creative @s", "effect @s clear", ""]));
    out
}

/// The actor on a stone block 5 by 5 and four high, a zombie 10 off, and `climbable` up the block's
/// west side at x 2.
fn climb() -> Vec<String> {
    let mut out = field("18000");
    for climbable in ["ladder [\"facing_direction\"=4]", "vine [\"vine_direction_bits\"=8]", "scaffolding"] {
        out.extend(owned(["fill 3 -60 -2 7 -57 2 stone"]));
        out.push(format!("fill 2 -60 0 2 -57 0 {climbable}"));
        out.extend(owned(["gamemode survival @s", "effect @s resistance 1000 255 true", "tp @s 5.5 -56 0.5 90 0", "", "summon zombie -7.5 -60 0.5 270 0", ""]));
        wait(&mut out, 25);
        out.extend(owned(["kill @e[type=!player]", "gamemode creative @s", "effect @s clear", "fill 2 -60 -2 7 -57 2 air", ""]));
    }
    out
}

/// As `climb`, with the zombie summoned in the climbable's cell, walled in on its other three sides.
fn shaft() -> Vec<String> {
    let mut out = field("18000");
    for climbable in ["ladder [\"facing_direction\"=4]", "vine [\"vine_direction_bits\"=8]", "scaffolding"] {
        out.extend(owned(["fill 3 -60 -2 7 -57 2 stone", "fill 1 -60 -1 2 -57 1 stone"]));
        out.push(format!("fill 2 -60 0 2 -57 0 {climbable}"));
        out.extend(owned(["gamemode survival @s", "effect @s resistance 1000 255 true", "tp @s 5.5 -56 0.5 90 0", "", "summon zombie 2.5 -60 0.5 90 0", ""]));
        wait(&mut out, 15);
        out.extend(owned(["kill @e[type=!player]", "gamemode creative @s", "effect @s clear", "fill 1 -60 -2 7 -56 2 air", ""]));
    }
    out
}

/// A spear rider's zombie horse 9 north of the actor, then a zombie with an iron spear 10 north.
fn spears_from_the_north() -> Vec<String> {
    let mut out = field("18000");
    out.extend(owned(["tp @s 0.5 -60 -30.5", "summon zombie_horse 0.5 -60 -40.5", "summon zombie_horse 2.5 -60 -40.5", "kill @e[type=zombie_horse]", ""]));
    out.extend(owned(["gamemode survival @s", "effect @s resistance 1000 3 true", "effect @s health_boost 1000 9 true", "effect @s instant_health 1 9 true", "tp @s 0.5 -60 0.5 180 0"]));
    out.push("summon zombie_horse 0.5 -60 -8.5 0 0 minecraft:spawn_adult_with_rider".into());
    wait(&mut out, 14);
    out.extend(owned(["kill @e[type=!player]", "effect @s instant_health 1 9 true", "tp @s 0.5 -60 0.5 180 0", "summon zombie 0.5 -60 -9.5 0 0", "effect @e[type=zombie] slowness 2 255 true"]));
    out.extend(owned(["replaceitem entity @e[type=zombie] slot.weapon.mainhand 0 iron_spear", ""]));
    wait(&mut out, 25);
    out.extend(owned(["kill @e[type=!player]", "gamemode creative @s", "effect @s clear", ""]));
    out
}

const SHOWN: &[&str] = &["CommandOutput", "AddEntity", "RemoveEntity", "MoveEntityDelta", "MoveEntityAbsolute", "SetEntityData", "EntityEvent", "SetEntityMotion", "LevelSoundEvent", "MobEquipment"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "af2baby" => (babies(), SHOWN),
        "af2wolf" => (wolves_behind_wall(), SHOWN),
        "af2climb" => (climb(), SHOWN),
        "af2shaft" => (shaft(), SHOWN),
        "af2spear" => (spears_from_the_north(), SHOWN),
        _ => return None,
    })
}

/// Reads the actor's packets for `ms`, so its view of the mobs' flags is current.
async fn settle(actor: &mut Bot, ms: u64) {
    let end = tokio::time::Instant::now() + Duration::from_millis(ms);
    while let Ok(Some(_)) = tokio::time::timeout_at(end, actor.next()).await {}
}

fn flags_of(actor: &Bot, id: u64) -> Flags {
    actor.state().entities.iter().find(|entity| entity.runtime_id == id).map_or(Flags::default(), |entity| entity.metadata.flags())
}

fn ids_of(actor: &Bot, kind: &str) -> Vec<u64> {
    let kind = format!("minecraft:{kind}");
    let mut ids: Vec<u64> = actor.state().entities.iter().filter(|entity| entity.kind == kind).map(|entity| entity.runtime_id).collect();
    ids.sort_unstable();
    ids
}

/// `!tameall kind`: uses the held item on each mob of that kind until it shows tamed, 20 tries at
/// most; `!standall kind`: clicks each one that shows sitting once. False for any other line.
pub async fn act(actor: &mut Bot, line: &str) -> bool {
    if let Some(kind) = line.strip_prefix("!tameall ") {
        for id in ids_of(actor, kind) {
            let mut tries = 0;
            while !flags_of(actor, id).contains(Flags::TAMED) && tries < 20 {
                let _ = actor.interact_entity(id).await;
                settle(actor, 400).await;
                tries += 1;
            }
            println!("## tamed {id} after {tries}: {}", flags_of(actor, id).contains(Flags::TAMED));
        }
        return true;
    }
    if let Some(kind) = line.strip_prefix("!standall ") {
        settle(actor, 400).await;
        for id in ids_of(actor, kind) {
            if flags_of(actor, id).contains(Flags::SITTING) {
                let _ = actor.interact_entity(id).await;
                settle(actor, 400).await;
            }
            println!("## {id} sitting: {}", flags_of(actor, id).contains(Flags::SITTING));
        }
        return true;
    }
    false
}
