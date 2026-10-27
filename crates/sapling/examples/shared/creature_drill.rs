//! Drills of mobs' own ways: what a world folder keeps of them, bees, golems, guardians and the wither.

/// Packets shown of these drills.
pub const CREATURE_SHOWN: &[&str] = &["AddEntity", "RemoveEntity", "MoveEntityDelta", "SetEntityMotion", "SetEntityData", "EntityEvent", "ActorEvent", "LevelSoundEvent", "LevelEvent", "MobEffect", "AddItemEntity", "UpdateBlock", "BlockEntityData", "MobEquipment", "MobArmorEquipment", "AddPainting", "UpdateSubchunkBlocks", "UpdateAttributes", "InventorySlot", "InventoryContent", "Animate"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some((
        match name {
            "keepmobs" => keep_mobs(),
            "keephive" => keep_hive(),
            "witherdeath" => wither_death(),
            "witherdash" => wither_dash(),
            "snowgolem" => snow_golem(),
            "drowned" => drowned(),
            "barter" => barter(),
            "guardians" => guardians(),
            "paintings" => paintings(),
            "armorstand" => armor_stand(),
            _ => return None,
        },
        CREATURE_SHOWN,
    ))
}

/// `!interact kind`: uses the held item on the newest mob of that kind it knows of; `!drop slot
/// count`: throws that many of a main inventory slot. False for any other line.
pub async fn act(actor: &mut acacia_bot::Bot, line: &str) -> bool {
    if let Some(kind) = line.strip_prefix("!interact ") {
        let kind = format!("minecraft:{kind}");
        match actor.state().entities.iter().filter(|entity| entity.kind == kind).map(|entity| entity.runtime_id).max() {
            Some(id) => println!("## interacted: {:?}", actor.interact_entity(id).await),
            None => println!("## no {kind} to interact with"),
        }
        return true;
    }
    if let Some(words) = line.strip_prefix("!drop ") {
        let numbers: Vec<u8> = words.split(' ').filter_map(|word| word.parse().ok()).collect();
        if let [slot, count] = numbers[..] {
            println!("## dropped: {:?}", actor.drop_item(acacia_bot::items::SlotRef::Main(slot), count).await);
        }
        return true;
    }
    false
}

fn floor() -> Vec<String> {
    // An absolute height: a bot that joined falling was once left in the void below the floor.
    ["kill @e[type=!player]", "gamerule domobspawning false", "gamemode creative @s", "tp @s 0.5 -60 0.5 -90 0", "fill ~-8 ~-1 ~-8 ~16 ~-1 ~8 stone", "fill ~-8 ~ ~-8 ~16 ~10 ~8 air", "clear @s", ""]
        .map(String::from)
        .into()
}

fn waits(lines: &mut Vec<String>, seconds: usize) {
    lines.extend(std::iter::repeat_n(String::new(), seconds));
}

/// Left in the world when the bots go, for the folder's actors to be read (sapling-leveldb's
/// `actor_dump`): a wither past its charge and hurt below half, one still charging, a zombie
/// and a skeleton under effects, and a TNT lit just before.
pub fn keep_mobs() -> Vec<String> {
    let mut lines = floor();
    lines.extend(["summon wither ~10 ~ ~"].map(String::from));
    waits(&mut lines, 14);
    lines.extend(["damage @e[type=wither] 200", ""].map(String::from));
    waits(&mut lines, 5);
    lines.extend(
        [
            "summon wither ~10 ~ ~5",
            "summon zombie ~4 ~ ~-4",
            "summon skeleton ~4 ~ ~-6",
            "effect @e[type=zombie] speed 100 1",
            "effect @e[type=zombie] resistance 200 2 true",
            "effect @e[type=skeleton] slowness 50 0",
            "",
            "summon tnt ~3 ~ ~3",
        ]
        .map(String::from),
    );
    lines
}

/// Bees by a hive at night, which go in to shelter; left in the world when the bots go, for the
/// hive's `Occupants` to be read. A bee by a flower at noon first, for its flight and its pollen.
pub fn keep_hive() -> Vec<String> {
    let mut lines = floor();
    lines.extend(["time set noon", "setblock ~6 ~ ~ beehive [\"direction\"=3]", "setblock ~3 ~ ~4 poppy", "setblock ~4 ~ ~4 dandelion", "summon bee ~4 ~1 ~2"].map(String::from));
    waits(&mut lines, 30);
    lines.extend(["summon bee ~5 ~1 ~-2", "summon bee ~5 ~2 ~-3", "time set midnight"].map(String::from));
    waits(&mut lines, 30);
    lines
}

fn with(mut lines: Vec<String>, more: &[&str]) -> Vec<String> {
    lines.extend(more.iter().map(|line| line.to_string()));
    lines
}

/// A summoned wither past its charge, then killed: its 200 dying frames and what it leaves.
pub fn wither_death() -> Vec<String> {
    let mut lines = with(floor(), &["summon wither ~8 ~ ~"]);
    waits(&mut lines, 12);
    lines.push("kill @e[type=wither]".into());
    waits(&mut lines, 14);
    lines
}

/// A summoned wither past its charge, hurt below half (armoured), with an iron golem to go at.
pub fn wither_dash() -> Vec<String> {
    let mut lines = with(floor(), &["summon wither ~6 ~ ~"]);
    waits(&mut lines, 12);
    lines.extend(["damage @e[type=wither] 200", "summon iron_golem ~14 ~ ~6"].map(String::from));
    waits(&mut lines, 25);
    lines
}

/// A snow golem with a zombie ten blocks off, on plains: its snowballs and its trail.
pub fn snow_golem() -> Vec<String> {
    let mut lines = with(floor(), &["time set noon", "summon snow_golem ~4 ~ ~", "summon zombie ~12 ~ ~"]);
    waits(&mut lines, 20);
    lines
}

/// A pool east of the actor, who is in survival, warded and dry, at midnight.
fn pool(more: &[&str]) -> Vec<String> {
    with(floor(), &[&["fill ~3 ~-1 ~-6 ~14 ~5 ~6 water", "time set midnight", "gamemode survival @s", "effect @s resistance 120 4 true", "effect @s instant_health 1 10 true"], more].concat())
}

/// A drowned with a trident in the pool, eight blocks off.
pub fn drowned() -> Vec<String> {
    let mut lines = pool(&["summon drowned ~8 ~1 ~", "replaceitem entity @e[type=drowned] slot.weapon.mainhand 0 trident"]);
    waits(&mut lines, 15);
    with(lines, &["gamemode creative @s"])
}

/// An adult piglin handed a gold ingot, and one that finds an ingot thrown: their admiring and
/// what they barter.
pub fn barter() -> Vec<String> {
    let mut lines = with(floor(), &["summon piglin ~3 ~ ~2 0 0 minecraft:spawn_adult", "give @s gold_ingot 4", "", "!interact piglin"]);
    waits(&mut lines, 10);
    lines.extend(["kill @e[type=piglin]", "kill @e[type=item]", "summon piglin ~3 ~ ~-2 0 0 minecraft:spawn_adult", "", "!drop 0 1"].map(String::from));
    waits(&mut lines, 12);
    lines
}

/// A guardian in the pool with the actor at its edge, then an elder guardian for a minute.
pub fn guardians() -> Vec<String> {
    let mut lines = pool(&["summon guardian ~8 ~1 ~"]);
    waits(&mut lines, 12);
    lines.extend(["kill @e[type=guardian]", "summon elder_guardian ~8 ~1 ~"].map(String::from));
    for _ in 0..13 {
        waits(&mut lines, 5);
        lines.push("effect @s instant_health 1 10 true".into());
    }
    with(lines, &["gamemode creative @s"])
}

/// Paintings hung on a wall's west face: four on open wall, one in a 1 by 1 nook, one in a 2 by
/// 1 and one in a 3 by 2.
pub fn paintings() -> Vec<String> {
    let mut lines = with(floor(), &["fill ~3 ~ ~-8 ~3 ~6 ~8 stone", "give @s painting 16"]);
    for z in [-6, -2, 2, 6] {
        lines.extend([format!("!click 3 2 {z} west"), String::new()]);
    }
    lines.extend(["fill ~3 ~ ~-8 ~3 ~6 ~8 air", "kill @e[type=painting]", "fill ~3 ~ ~-8 ~3 ~6 ~8 stone"].map(String::from));
    for (top, south, z) in [(1, 0, -6), (1, 1, -2), (2, 2, 3)] {
        lines.push(format!("fill ~2 ~ ~{} ~2 ~{} ~{} stone", z - 1, top + 1, z + south + 1));
        lines.push(format!("fill ~2 ~1 ~{z} ~2 ~{top} ~{} air", z + south));
        lines.extend([format!("!click 3 1 {z} west"), String::new()]);
    }
    lines
}

/// An armour stand given a helmet and a sword by the actor's clicks, then clicked empty-handed.
pub fn armor_stand() -> Vec<String> {
    let mut lines = with(floor(), &["summon armor_stand ~3 ~ ~ 90", "give @s diamond_helmet", "", "!interact armor_stand", "", "give @s iron_sword", "", "!interact armor_stand", "", "!interact armor_stand", ""]);
    waits(&mut lines, 2);
    lines
}
