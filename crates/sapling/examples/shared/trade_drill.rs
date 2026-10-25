//! The trade drills of bds_command_probe: a villager taking a job site, the trade screen and its
//! offers, trades until a villager levels up, and a wandering trader (docs/research/villagers.md).

pub const TRADE_SHOWN: &[&str] = &[
    "UpdateTrade", "SetEntityData", "AddEntity", "RemoveEntity", "EntityEvent", "ActorEvent", "ItemStackResponse", "InventorySlot", "InventoryContent", "SpawnExperienceOrb",
    "UpdateAttributes", "ContainerOpen", "ContainerClose", "SetEntityLink", "LevelSoundEvent", "LevelEvent",
];

fn lines(all: &[&str]) -> Vec<String> {
    all.iter().map(|line| line.to_string()).collect()
}

/// Seconds of nothing but watching.
fn wait(lines: &mut Vec<String>, seconds: usize) {
    lines.extend(std::iter::repeat_n(String::new(), seconds));
}

/// An unemployed villager beside a blast furnace, without and then with a bed: whether and when it
/// takes the job; then the furnace broken.
pub fn job() -> Vec<String> {
    let mut all = lines(&[
        "gamerule domobspawning false",
        "gamerule dodaylightcycle false",
        "time set 1000",
        "fill ~-8 ~-1 ~-8 ~8 ~-1 ~8 grass_block",
        "fill ~-8 ~ ~-8 ~8 ~3 ~8 air",
        "setblock ~-5 ~ ~5 bed [\"direction\"=0]",
        "setblock ~5 ~ ~-5 bell",
        "summon villager ~-3 ~ ~3 0 0 minecraft:become_unskilled",
    ]);
    wait(&mut all, 20);
    all.push("setblock ~5 ~ ~3 blast_furnace".into());
    wait(&mut all, 60);
    all.push("setblock ~-5 ~ ~4 lectern".into());
    wait(&mut all, 10);
    all.extend(lines(&["setblock ~5 ~ ~3 air", "setblock ~-5 ~ ~4 air"]));
    wait(&mut all, 20);
    all
}

/// An armorer's screen, five trades of coal until it levels up, a restock while it works, and a
/// wandering trader's screen.
pub fn trade() -> Vec<String> {
    let mut all = lines(&[
        "gamerule domobspawning false",
        "gamerule dodaylightcycle false",
        "time set 1000",
        "gamemode s",
        "fill ~-8 ~-1 ~-8 ~8 ~-1 ~8 grass_block",
        "fill ~-8 ~ ~-8 ~8 ~3 ~8 air",
        "clear @s",
        "give @s coal 64",
        "give @s coal 64",
        "give @s emerald 64",
        "summon villager ~2 ~ ~2 0 0 minecraft:become_armorer",
        "setblock ~4 ~ ~2 blast_furnace",
        "",
        "!trade villager_v2 0 1",
    ]);
    wait(&mut all, 3);
    all.push("!trade villager_v2 0 7".into());
    wait(&mut all, 10);
    all.push("!trade villager_v2 0 1".into());
    wait(&mut all, 3);
    all.extend(lines(&["summon wandering_trader ~1 ~ ~-1", "", "", "!trade wandering_trader 0 1"]));
    wait(&mut all, 5);
    all
}
