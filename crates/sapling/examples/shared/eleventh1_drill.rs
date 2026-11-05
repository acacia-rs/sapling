//! The eleventh wave's mob drills (research/parity/ELEVENTH-BRIEF.md "Group 1"): `doMobSpawning` off
//! against a spawner and an egg, peaceful against monsters already there (`e1spawn`); cave spiders by
//! night and by day (`e1cave`); a skeleton's arrows with a villager in the way (`e1arrow`); an armour
//! stand taken out of fire (`e1stand`); hurt horses left a while (`e1heal`); a tamed horse's armour
//! with `doMobLoot` off (`e1loot`); a zombie after its prey over a trench, a drop, water and a door
//! (`e1path`); a zombie on land in the rain by a whole conduit (`e1conduit`, open sky:
//! `SAPLING_DRILL_WORLD=open` on sapling).

use super::creatures::{floor, waits, with};

const SHOWN: &[&str] = &[
    "AddEntity", "RemoveEntity", "AddItemEntity", "MoveEntityDelta", "MoveEntityAbsolute", "SetEntityMotion", "SetEntityData", "EntityEvent", "LevelSoundEvent", "LevelEvent", "MobEffect",
    "UpdateAttributes", "SetHealth", "MobEquipment", "MobArmorEquipment", "UpdateBlock", "BlockEntityData", "CommandOutput", "Text",
];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "e1spawn" => (spawning(), SHOWN),
        "e1cave" => (cave_spiders(), SHOWN),
        "e1arrow" => (arrow_in_the_way(), SHOWN),
        "e1stand" => (stand_out_of_fire(), SHOWN),
        "e1heal" => (horses_heal(), SHOWN),
        "e1loot" => (armour_loot(), SHOWN),
        "e1path" => (paths(), SHOWN),
        "e1conduit" => (super::fifth3::drill("conduitrain")?.0.into_iter().take_while(|line| line != "time set midnight").chain(conduit_rain()).collect(), SHOWN),
        _ => return None,
    })
}

fn held(kind: &str) -> String {
    format!("effect @e[type={kind}] slowness 120 255 true")
}

/// Mob spawning off: a spawner given a pig by egg and a cow egg on the floor; then a zombie and a
/// pig left as the difficulty goes peaceful, and a zombie summoned there.
fn spawning() -> Vec<String> {
    let mut lines = with(floor(), &["time set noon", "gamerule domobspawning false", "setblock 3 -26 0 mob_spawner", "replaceitem entity @s slot.weapon.mainhand 0 pig_spawn_egg", "", "!click 3 0 0 west"]);
    waits(&mut lines, 30);
    lines.extend(["testfor @e[type=pig]", "setblock 3 -26 0 air", "kill @e[type=!player]", "replaceitem entity @s slot.weapon.mainhand 0 cow_spawn_egg", "", "!click 0 -1 -3", "", "testfor @e[type=cow]"].map(String::from));
    lines.extend(["kill @e[type=!player]", "time set midnight", "summon zombie 6.5 -26 4.5", "summon pig 6.5 -26 -4.5", &held("zombie"), "", "difficulty peaceful", "", "", "testfor @e[type=zombie]", "testfor @e[type=pig]"].map(String::from));
    lines.extend(["summon zombie 6.5 -26 4.5", "", "", "testfor @e[type=zombie]"].map(String::from));
    with(lines, &["difficulty easy", "kill @e[type=!player]", "replaceitem entity @s slot.weapon.mainhand 0 air", ""])
}

/// The survival actor (healed) on normal: a cave spider 6 off at midnight, watched 20 s, then one
/// at noon.
fn cave_spiders() -> Vec<String> {
    let mut lines = super::warden::start("midnight");
    lines.extend(["difficulty normal", "summon cave_spider 6.5 -26 0.5"].map(String::from));
    super::warden::healed(&mut lines, 20);
    lines.extend(["kill @e[type=!player]", "effect @s clear", "effect @s health_boost 900 4", "effect @s instant_health 1 9", "time set noon", "summon cave_spider 6.5 -26 0.5"].map(String::from));
    super::warden::healed(&mut lines, 20);
    with(lines, &["kill @e[type=!player]", "difficulty easy", "gamemode creative @s", "effect @s clear"])
}

/// A skeleton 12 off the survival actor at midnight, a held villager halfway between; put back
/// once held, as a stroll in its first second took sapling's off the line.
fn arrow_in_the_way() -> Vec<String> {
    let mut lines = super::warden::start("midnight");
    lines.extend(["summon villager 6.5 -26 0.5 0 0 minecraft:spawn_farmer", &held("villager"), "tp @e[type=villager] 6.5 -26 0.5", "summon skeleton 12.5 -26 0.5", &held("skeleton")].map(String::from));
    super::warden::healed(&mut lines, 20);
    with(lines, &["kill @e[type=!player]", "gamemode creative @s", "effect @s clear"])
}

/// An armour stand in fire 3 s, then moved onto stone and watched; one put in fire and left.
fn stand_out_of_fire() -> Vec<String> {
    let mut lines = with(floor(), &["gamerule dofiretick false", "setblock 4 -27 4 netherrack", "setblock 4 -26 4 fire", "setblock 4 -27 -4 netherrack", "setblock 4 -26 -4 fire", "summon armor_stand 4.5 -26 4.5", "summon armor_stand 4.5 -26 -3.5"]);
    waits(&mut lines, 3);
    lines.push("tp @e[type=armor_stand,x=4,y=-26,z=4,r=1] 8.5 -26 4.5".into());
    waits(&mut lines, 14);
    with(lines, &["kill @e[type=!player]", "fill -8 -27 -8 16 -27 8 stone", "fill -8 -26 -8 16 -16 8 air", "gamerule dofiretick true", ""])
}

/// Six grown horses and two cows held still, each hurt 6, watched 100 s by night.
fn horses_heal() -> Vec<String> {
    let mut lines = with(floor(), &["time set midnight"]);
    for x in [2, 4, 6, 8, 10, 12] {
        lines.push(format!("summon horse {x}.5 -26 4.5 0 0 minecraft:spawn_adult"));
    }
    lines.extend(["summon cow 4.5 -26 -4.5", "summon cow 8.5 -26 -4.5", &held("horse"), &held("cow"), "", "damage @e[type=horse] 6", "damage @e[type=cow] 6"].map(String::from));
    waits(&mut lines, 100);
    with(lines, &["kill @e[type=!player]"])
}

/// A tamed horse in a saddle and iron armour killed with `doMobLoot` off, then one with it on.
fn armour_loot() -> Vec<String> {
    let mut lines = with(floor(), &["time set noon"]);
    for loot in ["false", "true"] {
        lines.extend([
            format!("gamerule domobloot {loot}"),
            "summon horse 4.5 -26 0.5 0 0 minecraft:spawn_tame_adult".into(),
            held("horse"),
            "replaceitem entity @e[type=horse] slot.saddle 0 saddle".into(),
            "replaceitem entity @e[type=horse] slot.armor 0 iron_horse_armor".into(),
            String::new(),
            "kill @e[type=horse]".into(),
        ]);
        waits(&mut lines, 3);
        lines.extend(["testfor @e[type=item]".into(), "kill @e[type=item]".into()]);
    }
    with(lines, &["gamerule domobloot true", "kill @e[type=!player]"])
}

/// A zombie at midnight 12 off the survival actor (healed): with a trench 2 deep across between
/// them, then a drop of 3 (the zombie on a shelf), then a strip of water 3 wide, then a wall with a
/// shut wooden door; each watched 15 s, the zombie killed between.
fn paths() -> Vec<String> {
    let mut lines = super::warden::start("midnight");
    let reset = ["kill @e[type=!player]", "fill -20 -27 -20 20 -27 20 stone", "fill -20 -26 -20 20 -16 0 air", "fill -20 -26 0 20 -16 20 air", "fill -20 -29 -20 20 -28 20 air", "tp @s 0.5 -26 0.5"];
    let ways: [&[&str]; 4] = [
        &["fill -20 -29 -6 20 -29 -5 stone", "fill -20 -27 -6 20 -27 -5 air"],
        &["fill -20 -26 -20 20 -24 -6 stone"],
        &["fill -20 -28 -7 20 -28 -5 stone", "fill -20 -27 -7 20 -27 -5 water"],
        &["fill -20 -26 -6 20 -24 -6 cobblestone", "setblock 0 -26 -6 wooden_door [\"direction\"=1]", "setblock 0 -25 -6 wooden_door [\"direction\"=1,\"upper_block_bit\"=true]"],
    ];
    for (way, feet) in ways.into_iter().zip([-26, -23, -26, -26]) {
        lines.extend(reset.map(String::from));
        lines.extend(way.iter().map(|line| line.to_string()));
        lines.push(format!("summon zombie 0.5 {feet} -11.5"));
        super::warden::healed(&mut lines, 15);
    }
    lines.extend(reset.map(String::from));
    with(lines, &["gamemode creative @s", "effect @s clear"])
}

/// In rain at midnight: a held zombie on glass 4 north of the conduit and 1 up, where
/// `conduitdepth` took one in water; then one on glass over the conduit's cube.
fn conduit_rain() -> Vec<String> {
    let mut lines: Vec<String> = ["time set midnight", "weather rain", "fill 19 -19 -6 21 -14 -4 air", "fill 19 -19 -6 21 -19 -4 glass", "", "", "summon zombie 20.5 -18 -4.5", &held("zombie")].map(String::from).into();
    lines.push("testforblock 20 -18 -5 air".into());
    waits(&mut lines, 12);
    lines.extend(["testfor @e[type=zombie]", "kill @e[type=zombie]", "fill 19 -19 -6 21 -19 -4 air", "setblock 20 -15 0 glass", "summon zombie 20.5 -14 0.5", &held("zombie")].map(String::from));
    waits(&mut lines, 12);
    with(lines, &["testfor @e[type=zombie]", "kill @e[type=!player]", "weather clear", "time set noon", "fill 13 -20 -30 60 -10 30 air", "tp @s 0.5 -20 0.5", ""])
}
