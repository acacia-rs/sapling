//! The thirteenth wave's group BG drills (research/parity/THIRTEENTH-BRIEF.md): the flags BF left. On the
//! flat ground at midnight. `bg2player`: the actor made invisible by `/effect` and raised, sneaking while
//! hidden and after, run out, by a splash potion; the watcher hidden and shown again, each seen by the
//! rejoined actor. `bg2names`: a sheep summoned with a name, a villager by `summon villager <name> <pos>`, a
//! cow named by a name tag renamed at an anvil, a pig standing in fire, a creeper charged by `/event`, a
//! zombie walking at the survival watcher; all seen by the rejoined actor. `bg2tag`: the name tag again.

use crate::villages::wait;

fn owned<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

fn start() -> Vec<String> {
    owned(["kill @e[type=!player]", "gamerule domobspawning false", "gamerule dodaylightcycle false", "time set 18000", "difficulty easy", "gamemode creative @a", "effect @a clear", "clear @a", "tp @s 0.5 -60 0.5", "tp CmdWatcher 3.5 -60 0.5", ""])
}

fn player() -> Vec<String> {
    let mut out = start();
    out.extend(owned([
        "effect @s invisibility 30 0 true", "", "effect @s invisibility 40 1 true", "", "!sneak", "", "!unsneak", "", "effect @s clear", "", "!sneak", "", "!unsneak", "",
        "effect @s invisibility 2 0",
    ]));
    wait(&mut out, 3);
    out.extend(owned(["replaceitem entity @s slot.weapon.mainhand 0 splash_potion 1 7", "!look 0.01 -1 0", "!use", "", "", "effect @s clear", ""]));
    out.extend(owned(["effect CmdWatcher invisibility 60 0 true", "", "!rejoin", "", "", "effect CmdWatcher clear", "", "!rejoin", "", "", "effect @a clear", ""]));
    out
}

fn names() -> Vec<String> {
    let mut out = start();
    out.extend(owned([
        "tp CmdWatcher -8.5 -60 -8.5", "summon sheep 3.5 -60 4.5 0 0 minecraft:entity_spawned a", "", "summon villager v -3.5 -60 4.5", "", "summon cow 1.5 -60 -1.5", "",
        "summon pig 6.5 -60 -3.5", "summon creeper -5.5 -60 -1.5", "effect @e[type=!player] slowness 1000 255 true", "", "setblock 6 -60 -4 fire", "", "event entity @e[type=creeper] minecraft:become_charged", "",
    ]));
    out.extend(owned(["setblock ~2 ~ ~ anvil", "gamemode survival @s", "xp 30L @s", "replaceitem entity @s slot.hotbar 0 name_tag", "", "!click 2 0 0", "", "!ui 0 1", "!anvilcraft! 0 Bob 0", "!close", ""]));
    out.extend(owned(["gamemode creative @s", "", "!interact cow", "", ""]));
    out.extend(owned(["gamemode survival CmdWatcher", "effect CmdWatcher resistance 1000 255 true", "summon zombie -8.5 -60 9.5", "", "", "!rejoin", "", "", "kill @e[type=!player]", "gamemode creative @a", "clear @s", ""]));
    wait(&mut out, 2);
    out
}

/// A cow 1.5 off named by a name tag renamed at an anvil (bg2names' cow stood out of the bot's reach), then
/// seen by the rejoined actor. The actor rejoins before the click: straight after the anvil BDS named nothing.
fn tag() -> Vec<String> {
    let mut out = start();
    out.extend(owned(["summon cow 0.5 -60 -1", "effect @e[type=cow] slowness 1000 255 true", ""]));
    out.extend(owned(["setblock ~2 ~ ~ anvil", "gamemode survival @s", "xp 30L @s", "replaceitem entity @s slot.hotbar 0 name_tag", "", "!click 2 0 0", "", "!ui 0 1", "!anvilcraft! 0 Bob 0", "!close", ""]));
    out.extend(owned(["!rejoin", "", "", "!interact cow", "", "", "!rejoin", "", "", "kill @e[type=!player]", "setblock ~2 ~ ~ air", "gamemode creative @a", "clear @s", ""]));
    out
}

const SHOWN: &[&str] = &["CommandOutput", "SetEntityData", "MobEffect", "AddEntity", "AddPlayer", "RemoveEntity", "LevelSoundEvent", "EntityEvent", "MobEquipment", "MobArmorEquipment", "InventorySlot"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "bg2player" => (player(), SHOWN),
        "bg2names" => (names(), SHOWN),
        "bg2tag" => (tag(), SHOWN),
        _ => return None,
    })
}
