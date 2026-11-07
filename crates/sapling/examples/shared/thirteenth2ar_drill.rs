//! The thirteenth wave's group AR drills (research/parity/THIRTEENTH-BRIEF.md), on creature_drill.rs's floor
//! (stone at y -27). `ar2look`: mobs of several kinds and states summoned a second apart (villagers by job and
//! born, a zombie villager, a born cow, a jockey, mounts, a tamed horse, a llama), a villager a zombie kills on
//! hard, then the watcher away and back to be shown them all again: what each `AddEntity` carries. `ar2fox`:
//! foxes near a standing and a sneaking player, and born (trusting) ones. `ar2cat`: a cat tamed by feeding,
//! beside blocks it may sit on, then sleeping with its owner until morning.

use super::creatures::{floor, waits, with};

const LOOK_SHOWN: &[&str] = &["AddEntity", "SetEntityData", "SetEntityLink", "RemoveEntity", "CommandOutput"];
const PET_SHOWN: &[&str] = &["AddEntity", "AddItemEntity", "SetEntityData", "MoveEntityDelta", "MoveEntityAbsolute", "EntityEvent", "LevelSoundEvent", "CommandOutput", "RemoveEntity"];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "ar2look" => (looks(), LOOK_SHOWN),
        "ar2fox" => (foxes(), PET_SHOWN),
        "ar2cat" => (cats(), PET_SHOWN),
        _ => return None,
    })
}

const BOTS_AWAY: [&str; 4] = ["time set 18000", "tp CmdWatcher -6.5 -26 -6.5", "tp @s -6.5 -26 -5.5", ""];
const CLEAN: [&str; 5] = ["kill @e[type=!player]", "difficulty easy", "time set 6000", "tp CmdWatcher 0.5 -26 0.5", ""];

fn looks() -> Vec<String> {
    let mut lines = with(floor(), &BOTS_AWAY);
    let summons = [
        "villager", "villager 0 0 minecraft:become_armorer", "villager 0 0 minecraft:become_librarian", "villager 0 0 minecraft:entity_born", "zombie_villager",
        "zombie_villager 0 0 minecraft:as_baby", "cow 0 0 minecraft:entity_born", "zombie_horse 0 0 minecraft:spawn_adult_with_rider", "skeleton_horse", "llama",
        "horse 0 0 minecraft:spawn_tame_adult", "wolf 0 0 minecraft:entity_born", "fox", "cat",
    ];
    for (n, summon) in summons.iter().enumerate() {
        let (x, z) = (-6 + 3 * (n as i32 % 7), if n < 7 { -2 } else { 3 });
        let (kind, event) = summon.split_once(' ').unwrap_or((summon, ""));
        lines.push(format!("summon {kind} {x}.5 -26 {z}.5 {event}").trim_end().to_owned());
        waits(&mut lines, 1);
    }
    lines.extend(
        [
            "difficulty hard", "summon villager 12.5 -26 6.5 0 0 minecraft:become_armorer", "summon zombie 14.5 -26 6.5", "effect @e[type=zombie] slowness 100 255 true", "",
            "damage @e[type=villager_v2,x=12,y=-26,z=6,r=1] 100 entity_attack entity @e[type=zombie,c=1]", "", "",
        ]
        .map(String::from),
    );
    lines.extend(["tp CmdWatcher 400.5 -26 400.5", "", "", "", "tp CmdWatcher -6.5 -26 -6.5", "", "", ""].map(String::from));
    with(lines, &CLEAN)
}

/// The watcher on a pillar 30 east, the actor (survival) at the floor's middle by night: two foxes 4 and 8.9
/// off, watched 6 s; the same with the actor sneaking; then a born (trusting) fox and a plain baby 3 off.
fn foxes() -> Vec<String> {
    let mut lines = with(floor(), &["time set 18000", "setblock 30 -27 0 stone", "tp CmdWatcher 30.5 -26 0.5", "gamemode survival @s", "tp @s 0.5 -26 0.5 -90 0", ""]);
    let pair = ["summon fox 4.5 -26 0.5 0 0 minecraft:entity_spawned", "summon fox 8.5 -26 4.5 0 0 minecraft:entity_spawned"];
    for sneak in [false, true] {
        lines.extend(sneak.then(|| "!sneak".to_owned()));
        lines.extend(pair.map(String::from));
        waits(&mut lines, 6);
        lines.extend(["kill @e[type=fox]".to_owned(), "tp @s 0.5 -26 0.5 -90 0".into(), String::new()]);
        lines.extend(sneak.then(|| "!unsneak".to_owned()));
    }
    lines.extend(["summon fox 3.5 -26 2.5 0 0 minecraft:entity_born", "summon fox 3.5 -26 -1.5 0 0 minecraft:entity_born", "summon fox 3.5 -26 -4.5", ""].map(String::from));
    waits(&mut lines, 8);
    with(lines, &["gamemode creative @s", "kill @e[type=!player]", "time set 6000", ""])
}

/// A wild cat tamed by the actor with cod beside a chest, a lit furnace and a bed, watched 40 s; then the
/// actor sleeps in the bed with the cat near until morning (half the players sleeping skip the night).
fn cats() -> Vec<String> {
    let mut lines = with(floor(), &["time set 12000", "tp CmdWatcher -6.5 -26 -6.5", "gamemode survival @s", "tp @s 0.5 -26 0.5 -90 0", "clear @s", "give @s cod 64", ""]);
    lines.extend(["summon cat 2.5 -26 0.5 0 0 minecraft:spawn_wild_adult", "effect @e[type=cat] slowness 30 255 true", ""].map(String::from));
    for _ in 0..8 {
        lines.extend(["tp @s 0.5 -26 0.5 -90 0", "!feedall cat", ""].map(String::from));
    }
    lines.extend(["testfor @e[type=cat]", "effect @e[type=cat] clear", "setblock 5 -26 3 chest", "setblock 5 -26 -3 lit_furnace", "setblock -3 -26 3 bed [\"direction\"=0]", ""].map(String::from));
    waits(&mut lines, 40);
    lines.extend(["gamerule playerssleepingpercentage 50", "time set 13000", "tp @s -2.5 -26 1.5 0 0", "", "!sleep 0 0 2", ""].map(String::from));
    waits(&mut lines, 25);
    with(lines, &["gamerule playerssleepingpercentage 100", "gamemode creative @s", "kill @e[type=!player]", "setblock -3 -26 3 air", "setblock -3 -26 4 air", "time set 6000", ""])
}
