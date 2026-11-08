//! The thirteenth wave's group AY drills (research/parity/THIRTEENTH-BRIEF.md), on creature_drill.rs's floor
//! (stone at y -27). `ay2ident`: the actor holds diamonds and stands off the spawn, leaves and joins again
//! (is it the same player to BDS, and does it keep its unique id?). `ay2gdb`, run with BDS_GDB
//! (docs/research/worldgen-bds-debugger.md "Linux"): a zombie stomping eggs from afar and from beside them, two
//! boats summoned at z 4.5, a cat tamed by feeding waking with its owner (each of two sleeps after a click,
//! so in one the cat stands), and the actor riding a harnessed happy ghast holding jump.

use super::creatures::{floor, waits, with};

const SHOWN: &[&str] = &["LevelSoundEvent", "CommandOutput", "MovePlayer", "InventoryContent", "RemoveEntity", "AddPlayer"];
const GDB_SHOWN: &[&str] = &["AddEntity", "AddItemEntity", "MoveEntityDelta", "MovePlayer", "SetEntityData", "LevelEvent", "UpdateBlock", "SetEntityLink", "CommandOutput"];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "ay2ident" => (ident(), SHOWN),
        "ay2gdb" => (traced(), GDB_SHOWN),
        _ => return None,
    })
}

fn ident() -> Vec<String> {
    let lines = with(floor(), &["clear @s", "give @s diamond 3", "tp @s 5.5 -26 3.5", "", "clear @s diamond 0 0", "", "!rejoin", "", "", "clear @s diamond 0 0", "testfor @s[x=5,y=-26,z=3,dx=1,dy=1,dz=1]", "", "!rejoin", "", ""]);
    with(lines, &["clear @s diamond 0 0", "clear @s", "tp @s 0.5 -26 0.5", ""])
}

fn traced() -> Vec<String> {
    let mut lines = with(floor(), &["time set 18000", "gamerule mobgriefing true", "tp CmdWatcher -7.5 -26 -7.5", "tp @s -6.5 -26 -7.5"]);
    for zombie in ["summon zombie -1.5 -26 0.5", "summon zombie 4.5 -26 1.5"] {
        lines.extend(["setblock 4 -26 0 turtle_egg".to_owned(), zombie.into(), String::new()]);
        waits(&mut lines, 12);
        lines.extend(["kill @e[type=zombie]".to_owned(), "setblock 4 -26 0 air".into(), String::new()]);
    }
    for at in ["4.5 -26 4.5", "9.5 -26 4.5", "-0.5 -26 4.5"] {
        lines.extend([format!("summon boat {at}"), String::new(), "kill @e[type=boat]".into()]);
    }
    lines.extend(["time set 12000", "gamemode survival @s", "tp @s 0.5 -26 0.5 -90 0", "clear @s", "give @s cod 64", "summon cat 2.5 -26 0.5 0 0 minecraft:spawn_wild_adult", "effect @e[type=cat] slowness 60 255 true", ""].map(String::from));
    for _ in 0..8 {
        lines.extend(["tp @s 0.5 -26 0.5 -90 0".to_owned(), "!feedall cat".into(), String::new()]);
    }
    lines.extend(["clear @s", "effect @e[type=cat] clear", "setblock -3 -26 3 bed [\"direction\"=0]", "gamerule playerssleepingpercentage 50", ""].map(String::from));
    for _ in 0..2 {
        lines.extend(["tp @s 0.5 -26 0.5 -90 0", "", "!interact cat", "", "time set 13000", "tp @s -2.5 -26 1.5 0 0", "", "!sleep 0 0 2", ""].map(String::from));
        waits(&mut lines, 16);
    }
    lines.extend(["gamerule playerssleepingpercentage 100", "gamemode creative @s", "kill @e[type=!player]", "setblock -3 -26 3 air", "setblock -3 -26 4 air", "time set 6000", ""].map(String::from));
    const OVER: &str = "tp @e[type=happy_ghast] 0.5 -22 0.5";
    lines.extend(["tp CmdWatcher 8.5 -26 -6.5", "tp @s 0.5 -26 0.5 0 0", "summon happy_ghast 0.5 -22 0.5 0 0 minecraft:spawn_adult", "effect @e[type=happy_ghast] slowness 60 255 true", OVER, ""].map(String::from));
    lines.extend(["replaceitem entity @s slot.weapon.mainhand 0 white_harness", OVER, "", "!interact happy_ghast", "", "", "!look 0 1.6 8", OVER, "", "!mount happy_ghast", "", ""].map(String::from));
    lines.extend(["effect @e[type=happy_ghast] clear", "", "!jump", "", "", "", "!stop", "", "!walk", "!jump", "", "", "!stop", "", "!dismount", "", ""].map(String::from));
    with(lines, &["kill @e[type=!player]", "clear @s", "tp @s 0.5 -26 0.5", "tp CmdWatcher 0.5 -26 0.5", ""])
}
