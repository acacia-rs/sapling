//! The fourteenth wave's group N drills (research/parity/FOURTEENTH-BRIEF.md): what a BDS world folder
//! keeps, each left in the world as the server stops and then read with sapling-leveldb's `actor_dump`,
//! `block_entity_dump` and `records`. `f14nkeep`, on creature_drill.rs's floor: farm animals and a wolf
//! (their `properties`), a wandering trader, a villager given bread at its feet, a dropper and a
//! dispenser filled, a dropped stack, and the actor riding a tamed horse. `f14nwither`: a wither past
//! its charge, hurt below half (armoured) 3 s before the stop; `f14nwither2` (KEEP_WORLD=1) the actor,
//! warded, watched by it for 60 s, to time its first dash after the load. `f14nvillage`: village_drill's
//! village, a grown villager hit twice (standing), then a raid whose first raider the actor slays.

use super::creatures::{floor, waits, with};
use crate::villages::{UNSKILLED, VILLAGE_SHOWN, village, wait};

const SHOWN: &[&str] = &["AddEntity", "RemoveEntity", "SetEntityData", "EntityEvent", "MoveEntityDelta", "MoveEntityAbsolute", "SetEntityMotion", "AddItemEntity", "SetEntityLink", "CommandOutput", "LevelEvent"];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "f14nkeep" => (keep(), SHOWN),
        "f14nwither" => (wither(), SHOWN),
        "f14nwither2" => (wither_kept(), SHOWN),
        "f14nvillage" => (raided_village(), VILLAGE_SHOWN),
        "f14nnest" => (nests(), SHOWN),
        _ => return None,
    })
}

fn nests() -> Vec<String> {
    let mut lines = with(floor(), &["time set noon", "setblock 2 -26 -4 bee_nest", "setblock 4 -26 -4 beehive", "fill 6 -27 2 10 -27 6 grass_block", "place feature minecraft:cherry_tree_with_beehive_feature 8 -26 4", "testfor @e[type=bee]"]);
    waits(&mut lines, 10);
    with(lines, &["testfor @e[type=bee]", "kill @e[type=!player]", "fill -8 -26 -8 16 -16 8 air", ""])
}

fn keep() -> Vec<String> {
    let mut lines = with(floor(), &[
        "gamerule mobgriefing false", "tp CmdWatcher -6.5 -26 -6.5",
        "summon pig -4.5 -26 -5.5", "summon cow -2.5 -26 -5.5", "summon chicken -0.5 -26 -5.5", "summon wolf 1.5 -26 -5.5", "summon frog 3.5 -26 -5.5",
        "effect @e[type=!player] slowness 1000 255 true",
        "summon wandering_trader 12.5 -26 -5.5", "effect @e[type=wandering_trader] slowness 1000 255 true",
        "summon villager 12.5 -26 5.5 0 0 minecraft:become_unskilled", "effect @e[type=villager_v2] slowness 1000 255 true",
        "setblock -6 -26 6 dropper", "setblock -4 -26 6 dispenser",
        "replaceitem block -6 -26 6 slot.container 0 dirt 5", "replaceitem block -4 -26 6 slot.container 1 arrow 7",
        "tp @s 11.5 -26 5.5 90 0", "give @s bread 3", "", "!drop 0 3",
    ]);
    waits(&mut lines, 4);
    lines = with(lines, &["tp @s 0.5 -26 2.5", "give @s cobblestone 4", "", "!drop 0 4", "", "summon horse 4.5 -26 4.5 0 0 minecraft:spawn_tame_adult", "replaceitem entity @e[type=horse] slot.saddle 0 saddle", "tp @s 4.5 -26 2.5", "", "!mount horse"]);
    waits(&mut lines, 6);
    lines
}

fn wither() -> Vec<String> {
    let mut lines = with(floor(), &["gamerule mobgriefing false", "summon wither 6.5 -26 0.5"]);
    waits(&mut lines, 12);
    lines.extend(["damage @e[type=wither] 200", ""].map(String::from));
    waits(&mut lines, 3);
    lines
}

fn wither_kept() -> Vec<String> {
    let mut lines = with(Vec::new(), &["gamemode survival @s", "effect @s resistance 120 4 true", "effect @s regeneration 120 4 true", "tp @s 0.5 -26 0.5"]);
    waits(&mut lines, 60);
    with(lines, &["gamemode creative @s", "kill @e[type=wither]", "gamerule mobgriefing true", ""])
}

fn raided_village() -> Vec<String> {
    let mut out = village(&UNSKILLED);
    out.extend(["tp @s -4.5 -60 2.5 facing -4.5 -59 4.5", "", "!hit villager_v2", "", "!hit villager_v2", ""].map(String::from));
    out.push("effect @s bad_omen 600 0".into());
    wait(&mut out, 50);
    out.extend(["damage @e[family=monster,c=1] 200 entity_attack entity CmdActor", "testfor @e[family=monster]"].map(String::from));
    wait(&mut out, 3);
    out
}
