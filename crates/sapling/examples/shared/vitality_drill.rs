//! Drills of a player's effects, food and harm (docs/research effects, food-effects, items' clouds,
//! item-uses, player-damage): each food's values and leftovers, food effect odds, potion clouds,
//! armour and enchantments against harm, deaths by cause, starving by difficulty, conduits, glides
//! ended by the world, and an ominous trial spawner. On a floor at y -21 round (0, 0).

use std::iter::repeat_n;

/// Packets shown of these drills.
pub const VITALITY_SHOWN: &[&str] = &[
    "UpdateAttributes", "EntityEvent", "SetEntityData", "SetHealth", "Respawn", "LevelSoundEvent", "LevelEvent", "MovePlayer", "SetEntityMotion", "Text", "DeathInfo", "PlayerAction", "MobEffect",
    "SetPlayerGameType", "UpdatePlayerGameType", "RemoveEntity", "AddPlayer", "AddItemEntity", "AddEntity", "InventoryContent", "InventorySlot", "MobEquipment", "MobArmorEquipment",
    "SetSpawnPosition", "CorrectPlayerMovePrediction", "ChangeDimension", "SpawnExperienceOrb", "CompletedUsingItem", "PlayerArmorDamage", "BlockEntityData", "UpdateBlock", "MovementEffect",
    "ClientStartItemCooldown",
];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some((
        match name {
            "foods" => foods(&FOODS[..20]),
            "foods2" => foods2(),
            "foods3" => foods3(),
            "food-odds" => food_odds(),
            "clouds" => clouds(),
            "clouds2" => clouds2(),
            "fx3" => fx3(),
            "hurt2" => hurt2(),
            "deaths" => deaths(),
            "conduit2" => conduit2(),
            "glide2" => glide2(),
            "trialomen" => trial_omen(),
            _ => return None,
        },
        VITALITY_SHOWN,
    ))
}

/// Every food and its data value; the stews' values are their flowers.
const FOODS: [(&str, u8); 44] = [
    ("apple", 0), ("baked_potato", 0), ("beef", 0), ("beetroot", 0), ("beetroot_soup", 0), ("bread", 0), ("carrot", 0), ("chicken", 0), ("cod", 0), ("cooked_beef", 0),
    ("cooked_chicken", 0), ("cooked_cod", 0), ("cooked_mutton", 0), ("cooked_porkchop", 0), ("cooked_rabbit", 0), ("cooked_salmon", 0), ("cookie", 0), ("dried_kelp", 0), ("glow_berries", 0),
    ("golden_carrot", 0), ("melon_slice", 0), ("mushroom_stew", 0), ("mutton", 0), ("porkchop", 0), ("potato", 0), ("pufferfish", 0), ("pumpkin_pie", 0), ("rabbit", 0), ("rabbit_stew", 0),
    ("rotten_flesh", 0), ("salmon", 0), ("spider_eye", 0), ("sweet_berries", 0), ("tropical_fish", 0), ("poisonous_potato", 0), ("chorus_fruit", 0), ("honey_bottle", 0), ("potion", 28),
    ("suspicious_stew", 0), ("suspicious_stew", 4), ("suspicious_stew", 5), ("suspicious_stew", 7), ("suspicious_stew", 9), ("enchanted_golden_apple", 0),
];

fn owned<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

fn wait(lines: &mut Vec<String>, seconds: usize) {
    lines.extend(repeat_n(String::new(), seconds));
}

/// A stone floor 25 square at y -21, air over it, noon, the actor in survival at (0.5, -20, 0.5)
/// looking south, the watcher in its corner.
fn floor() -> Vec<String> {
    owned([
        "kill @e[type=!player]", "gamerule domobspawning false", "gamerule falldamage true", "gamerule keepinventory false", "difficulty easy", "time set noon", "weather clear",
        "fill -12 -21 -12 12 -21 12 stone", "fill -12 -20 -12 12 -12 12 air", "tp @s 0.5 -20 0.5 0 0", "tp @a[name=CmdWatcher] 10.5 -20 10.5", "clear @s", "effect @a clear", "gamemode survival @s", "",
    ])
}

/// Hunger drained to nothing, then one `food` eaten from the hand.
fn eat(lines: &mut Vec<String>, (food, data): (&str, u8)) {
    lines.extend(["effect @s clear".into(), "effect @s hunger 4 255 true".into(), String::new(), String::new(), String::new()]);
    lines.extend([format!("replaceitem entity @s slot.weapon.mainhand 0 {food} 1 {data}"), "!eat".into(), String::new()]);
}

/// Each of `foods` eaten at no hunger and no saturation.
fn foods(foods: &[(&str, u8)]) -> Vec<String> {
    let mut lines = floor();
    foods.iter().for_each(|&food| eat(&mut lines, food));
    lines.extend(owned(["effect @s clear", "gamemode creative @s", "clear @s", ""]));
    lines
}

/// The rest of [`FOODS`], the chorus fruit's jump undone; then a golden apple's absorption hit by 3
/// and by 3 of magic.
fn foods2() -> Vec<String> {
    let mut lines = floor();
    for &food in &FOODS[20..] {
        eat(&mut lines, food);
        if food.0 == "chorus_fruit" {
            lines.push("tp @s 0.5 -20 0.5".into());
        }
    }
    eat(&mut lines, ("golden_apple", 0));
    lines.extend(owned(["", "damage @s 3", "", "damage @s 3 magic", "", "damage @s 3", "", "effect @s clear", "gamemode creative @s", "clear @s", ""]));
    lines
}

/// The suspicious stews [`foods2`] left out, and three more chorus fruits.
fn foods3() -> Vec<String> {
    let mut lines = floor();
    for data in [1, 2, 3, 6, 8, 10, 11, 12, 13, 14] {
        eat(&mut lines, ("suspicious_stew", data));
    }
    for _ in 0..3 {
        eat(&mut lines, ("chorus_fruit", 0));
        lines.push("tp @s 0.5 -20 0.5".into());
    }
    lines.extend(owned(["effect @s clear", "gamemode creative @s", "clear @s", ""]));
    lines
}

/// A raw chicken, a rotten flesh and a poisonous potato eaten twenty times each, hunger drained
/// a second before each: how often each gives its effect.
fn food_odds() -> Vec<String> {
    let mut lines = floor();
    for _ in 0..20 {
        for food in ["chicken", "rotten_flesh", "poisonous_potato"] {
            lines.extend(["effect @s hunger 1 255 true".into(), format!("replaceitem entity @s slot.weapon.mainhand 0 {food} 1"), "!eat".into(), String::new()]);
        }
    }
    lines.extend(owned(["effect @s clear", "gamemode creative @s", "clear @s", ""]));
    lines
}

/// Thrown at the actor's own feet: a splash potion of poison with the watcher 2 off; a lingering
/// potion of harming among a cow 2 off, a cow 3.2 off and a zombified piglin 2.5 off; a lingering
/// potion of poison among a cow 2 off, a cow on a pillar 2 up and 2 off, a spider and a zombified piglin.
fn clouds() -> Vec<String> {
    let mut lines = floor();
    let throw = |lines: &mut Vec<String>, potion: &str| lines.extend([format!("replaceitem entity @s slot.weapon.mainhand 0 {potion}"), "!look 0.01 -1 0".into(), "!use".into()]);
    lines.extend(owned(["tp @a[name=CmdWatcher] 2.5 -20 0.5", ""]));
    throw(&mut lines, "splash_potion 1 25");
    wait(&mut lines, 3);
    lines.extend(owned(["effect @a clear", "tp @a[name=CmdWatcher] 10.5 -20 10.5", "summon cow 2.5 -20 0.5", "summon cow -2.7 -20 0.5", "summon zombie_pigman 0.5 -20 -2", ""]));
    lines.push("effect @e[type=!player] slowness 60 255 true".into());
    throw(&mut lines, "lingering_potion 1 23");
    wait(&mut lines, 8);
    lines.extend(owned(["kill @e[type=!player]", "effect @s clear", "effect @s instant_health 1 4", "", "fill 0 -20 2 0 -19 2 stone"]));
    lines.extend(owned(["summon cow 0.5 -18 2.5", "summon cow 2.5 -20 0.5", "summon spider -2.5 -20 0.5", "summon zombie_pigman 0.5 -20 -2.5", "", "effect @e[type=!player] slowness 60 255 true"]));
    throw(&mut lines, "lingering_potion 1 25");
    wait(&mut lines, 12);
    lines.extend(owned(["kill @e[type=!player]", "fill 0 -20 2 0 -19 2 air", "effect @a clear", "gamemode creative @s", "clear @s", ""]));
    lines
}

/// As [`clouds`], what its run left open: splash potions of poison at the actor's feet with the
/// watcher 1 to 3.5 off; a lingering potion of poison among a spider and a zombified piglin 1.5 off
/// and a cow 2.9 off; a lingering potion of harming on the actor alone, watched for its pulses.
fn clouds2() -> Vec<String> {
    let mut lines = floor();
    let throw = |lines: &mut Vec<String>, potion: &str| lines.extend([format!("replaceitem entity @s slot.weapon.mainhand 0 {potion}"), "!look 0.01 -1 0".into(), "!use".into()]);
    for off in ["1.5", "2", "2.5", "3", "3.5", "4.5"] {
        lines.extend([format!("tp @a[name=CmdWatcher] {off} -20 0.5"), String::new()]);
        throw(&mut lines, "splash_potion 1 25");
        lines.extend(owned(["", "effect @a clear"]));
    }
    lines.extend(owned(["tp @a[name=CmdWatcher] 10.5 -20 10.5", "summon spider 2 -20 0.5", "summon zombie_pigman -1 -20 0.5", "summon cow 0.5 -20 3.4", ""]));
    lines.push("effect @e[type=!player] slowness 60 255 true".into());
    throw(&mut lines, "lingering_potion 1 25");
    wait(&mut lines, 5);
    lines.extend(owned(["kill @e[type=!player]", "effect @s clear", "effect @s instant_health 1 4", "effect @s resistance 30 1 true", ""]));
    throw(&mut lines, "lingering_potion 1 23");
    wait(&mut lines, 12);
    lines.extend(owned(["kill @e[type=!player]", "effect @a clear", "gamemode creative @s", "clear @s", ""]));
    lines
}

/// Instant damage on the actor in all of iron; a protection IV chestplate and feather falling IV
/// boots put on by hand, then hit by six causes; fatal poison given to a zombified piglin and a cow; a bell
/// rung with a pillager 4 off; an ominous bottle drunk; `/damage` in creative and spectator; speed
/// on the actor and jump boost on the watcher as the actor leaves and joins again.
fn fx3() -> Vec<String> {
    let mut lines = floor();
    for slot in ["head 0 iron_helmet", "chest 0 iron_chestplate", "legs 0 iron_leggings", "feet 0 iron_boots"] {
        lines.push(format!("replaceitem entity @s slot.armor.{slot}"));
    }
    lines.extend(owned(["", "effect @s instant_damage 1 0", "", "effect @s instant_health 1 4", "", "clear @s", ""]));
    for (piece, enchantment) in [("iron_chestplate", "protection"), ("iron_boots", "feather_falling")] {
        lines.extend([format!("replaceitem entity @s slot.weapon.mainhand 0 {piece}"), format!("enchant @s {enchantment} 4"), "!use".into(), String::new()]);
    }
    for harm in ["entity_attack", "fall", "fire", "projectile", "magic", "block_explosion"] {
        lines.extend([format!("damage @s 10 {harm}"), String::new(), "effect @s instant_health 1 4".into(), String::new()]);
    }
    lines.extend(owned(["clear @s", "gamemode creative @s", "summon zombie_pigman 4.5 -20 -3.5", "summon cow -3.5 -20 -3.5", "", "effect @e[type=zombie_pigman] fatal_poison 10 0", "effect @e[type=cow] fatal_poison 10 0"]));
    wait(&mut lines, 4);
    lines.extend(owned(["kill @e[type=!player]", "setblock 2 -20 0 bell [\"attachment\"=\"standing\",\"direction\"=0]", "summon pillager 6.5 -20 0.5", "effect @e[type=pillager] slowness 30 255 true", ""]));
    lines.push("!click 2 0 0".into());
    wait(&mut lines, 5);
    lines.extend(owned(["kill @e[type=!player]", "setblock 2 -20 0 air", "gamemode survival @s", "replaceitem entity @s slot.weapon.mainhand 0 ominous_bottle 1 2", "!eat", "", "", ""]));
    lines.extend(owned(["effect @s clear", "gamemode creative @s", "damage @s 5", "damage @s 5 fall", "", "gamemode spectator @s", "damage @s 5", "damage @s 5 fall", "", "gamemode survival @s"]));
    lines.extend(owned(["effect @s speed 60 1", "effect @a[name=CmdWatcher] jump_boost 60 1", "", "!rejoin"]));
    wait(&mut lines, 4);
    lines.extend(owned(["effect @a clear", "gamemode creative @s", ""]));
    lines
}

/// An iron helmet one use from breaking, hit; grass and a torch broken by hand; a cow hit twice at
/// once; a walk through a water trench; held in stone under resistance until out of breath;
/// killed with `keepInventory`; starved at 3 health on normal, then on hard.
fn hurt2() -> Vec<String> {
    let mut lines = floor();
    lines.extend(owned(["replaceitem entity @s slot.armor.head 0 iron_helmet 1 164", "", "damage @s 4 entity_attack", "", "", "effect @s instant_health 1 4", ""]));
    lines.extend(owned(["setblock 0 -20 2 short_grass", "", "!break 0 0 2", "", "setblock 0 -20 2 torch", "", "!break 0 0 2", ""]));
    lines.extend(owned(["summon cow 0.5 -20 2.5", "effect @e[type=cow] slowness 30 255 true", "", "!hit 2cow", "", "kill @e[type=!player]", ""]));
    lines.extend(owned(["fill -1 -23 3 1 -22 13 stone", "fill 0 -22 4 0 -21 12 water", "tp @s 0.5 -22 4.5 0 0", "", "!walk"]));
    wait(&mut lines, 4);
    lines.extend(owned(["!stop", "tp @s 0.5 -20 0.5 0 0", ""]));
    lines.extend(owned(["effect @s resistance 30 3 true", "tp @s -4.5 -20 -4.5", "fill -5 -20 -5 -5 -19 -5 stone"]));
    wait(&mut lines, 20);
    lines.extend(owned(["fill -5 -20 -5 -5 -19 -5 air", "effect @s clear", "effect @s instant_health 1 4", "tp @s 0.5 -20 0.5", ""]));
    lines.extend(owned(["gamerule keepinventory true", "give @s stone 5", "replaceitem entity @s slot.armor.head 0 iron_helmet", "", "kill @s"]));
    wait(&mut lines, 5);
    lines.extend(owned(["gamerule keepinventory false", "clear @s", "tp @s 0.5 -20 0.5", "effect @s instant_health 1 4", "", "damage @s 17", "effect @s hunger 6 255 true"]));
    wait(&mut lines, 6);
    lines.push("difficulty normal".into());
    wait(&mut lines, 12);
    lines.push("difficulty hard".into());
    wait(&mut lines, 8);
    lines.extend(owned(["difficulty easy", "tp @s 0.5 -20 0.5", "gamemode creative @s", ""]));
    lines
}

/// The actor killed by `/damage` of each cause in turn, and by a zombie's attack.
fn deaths() -> Vec<String> {
    let mut lines = floor();
    let causes = [
        "entity_attack", "fall", "fire", "fire_tick", "lava", "drowning", "magic", "wither", "starve", "void", "contact", "projectile", "entity_explosion", "block_explosion", "lightning", "anvil",
        "falling_block", "freezing", "fly_into_wall", "magma", "stalactite", "stalagmite", "sonic_boom", "campfire", "soul_campfire", "thorns",
    ];
    for cause in causes {
        lines.extend(["tp @s 0.5 -20 0.5".into(), "gamemode survival @s".into(), format!("damage @s 100 {cause}"), String::new(), String::new(), String::new()]);
    }
    lines.extend(owned(["tp @s 0.5 -20 0.5", "summon zombie 3.5 -20 0.5", "", "damage @s 100 entity_attack entity @e[type=zombie,c=1]", "", "", "", "kill @e[type=!player]"]));
    lines.extend(owned(["tp @s 0.5 -20 0.5", "gamemode creative @s", ""]));
    lines
}

/// The 42 cells of a conduit's whole frame about (0, 0, 0), the 16 of its level ring first.
fn frame() -> Vec<[i32; 3]> {
    let mut cells = Vec::new();
    for i in -2i32..=2 {
        for j in -2i32..=2 {
            for k in -2i32..=2 {
                let (l, m, n) = (i.abs(), j.abs(), k.abs());
                let edge = l > 1 || m > 1 || n > 1;
                if edge && ((i == 0 && (m == 2 || n == 2)) || (j == 0 && (l == 2 || n == 2)) || (k == 0 && (l == 2 || m == 2))) {
                    cells.push([i, j, k]);
                }
            }
        }
    }
    cells.sort_by_key(|cell| cell[1] != 0);
    cells
}

/// A conduit at (20, -18, 0) waterlogged by the actor's bucket, in a cube of water with its level
/// ring of 16: the actor in rain 31 and 32 off along x and 32.5 off on the diagonal, and on dry land
/// 6 off in clear weather. Then the whole frame of 42: a zombie held in a pool 5 off, then a
/// zombie on land in rain.
fn conduit2() -> Vec<String> {
    let mut lines = floor();
    lines.extend(owned(["fill 13 -21 -30 60 -21 30 stone", "fill 13 -20 -30 60 -10 30 air", "gamemode creative @s", "setblock 20 -18 0 conduit", "tp @s 17.5 -20 0.5 -90 0"]));
    lines.extend(owned(["replaceitem entity @s slot.weapon.mainhand 0 water_bucket", "", "!click 3 2 0 west", "", "fill 18 -20 -2 22 -16 2 water replace air"]));
    let cells = frame();
    lines.extend(cells[..16].iter().map(|[x, y, z]| format!("setblock {} {} {} prismarine", 20 + x, -18 + y, z)));
    lines.extend(owned(["clear @s", "weather rain", "tp @s 51.5 -20 0.5", "effect @s clear"]));
    wait(&mut lines, 5);
    for at in ["52.5 -20 0.5", "43.5 -20 23.5"] {
        lines.extend([format!("tp @s {at}"), "effect @s clear".into()]);
        wait(&mut lines, 5);
    }
    lines.extend(owned(["weather clear", "tp @s 26.5 -20 0.5", "effect @s clear"]));
    wait(&mut lines, 5);
    lines.extend(cells[16..].iter().map(|[x, y, z]| format!("setblock {} {} {} prismarine", 20 + x, -18 + y, z)));
    lines.extend(owned(["tp @s 20.5 -20 8.5", "time set midnight", "fill 24 -21 -1 26 -21 1 stone", "fill 24 -20 -1 26 -19 1 water", "summon zombie 25.5 -20 0.5", "effect @e[type=zombie] slowness 60 255 true"]));
    wait(&mut lines, 10);
    lines.extend(owned(["kill @e[type=!player]", "fill 23 -20 -8 33 -19 8 air", "weather rain", "", "summon zombie 20.5 -20 -6.5", "effect @e[type=zombie] slowness 60 255 true"]));
    wait(&mut lines, 10);
    lines.extend(owned(["kill @e[type=!player]", "weather clear", "time set noon", "fill 13 -20 -30 60 -10 30 air", "tp @s 0.5 -20 0.5", "effect @s clear", ""]));
    lines
}

/// Glides 40 up, ended by a teleport into a pool, by levitation and by a minecart ridden; then a
/// cod bucket given by command poured out.
fn glide2() -> Vec<String> {
    let mut lines = floor();
    lines.extend(owned(["fill -4 -23 -4 4 -22 4 stone", "fill -3 -22 -3 3 -21 3 water", "replaceitem entity @s slot.armor.chest 0 elytra", "tp @s 0.5 20 0.5 0 10", "!glide", "", ""]));
    lines.extend(owned(["tp @s 0.5 -21 0.5", "", "", "", "tp @s 0.5 20 0.5 0 10", "!glide", "", "", "effect @s levitation 3", ""]));
    wait(&mut lines, 4);
    lines.extend(owned(["effect @s clear", "gamerule falldamage false", "tp @s 0.5 20 0.5 0 10", "!glide", "", "summon minecart ~ ~ ~", "ride @s start_riding @e[type=minecart,c=1] teleport_rider", ""]));
    wait(&mut lines, 4);
    lines.extend(owned(["ride @s stop_riding", "kill @e[type=!player]", "tp @s 0.5 -20 0.5 0 0", "gamerule falldamage true", "replaceitem entity @s slot.armor.chest 0 air"]));
    lines.extend(owned(["replaceitem entity @s slot.weapon.mainhand 0 cod_bucket", "", "!click 0 -1 2", "", "", "kill @e[type=!player]", "gamemode creative @s", "clear @s", ""]));
    lines
}

/// An ominous bottle drunk 20 off a trial spawner, then the actor (resistant, survival) beside it
/// while it turns ominous and spawns.
fn trial_omen() -> Vec<String> {
    let mut lines = floor();
    lines.extend(owned(["effect @s resistance 120 4 true", "setblock 0 -20 -10 trial_spawner", "tp @s 0.5 -20 10.5", "replaceitem entity @s slot.weapon.mainhand 0 ominous_bottle 1 0", "!eat", "", ""]));
    lines.push("tp @s 0.5 -20 -5.5".into());
    wait(&mut lines, 30);
    lines.extend(owned(["kill @e[type=!player]", "setblock 0 -20 -10 air", "effect @s clear", "gamemode creative @s", "clear @s", ""]));
    lines
}
