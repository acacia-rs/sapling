//! The toil, armour and effects drills of bds_command_probe: a player tired by work, hit in armour
//! and under `/effect` (docs/research/player-damage.md, effects.md).

/// The actor, in survival: breaks four blocks of dirt and one of stone by hand, hits a cow three
/// times and an armour stand once, and swings at nothing.
pub fn toil() -> Vec<String> {
    let mut lines: Vec<String> = ["kill @e[type=!player]", "", "fill ~-3 ~-1 ~-3 ~8 ~-1 ~8 stone", "fill ~-3 ~ ~-3 ~8 ~6 ~8 air", "gamemode survival @s", ""].map(String::from).into();
    for at in 2..6 {
        lines.extend([format!("setblock ~1 ~ ~{at} dirt"), format!("!break 1 0 {at}"), String::new()]);
    }
    lines.extend(["setblock ~1 ~1 ~ stone", "!break 1 1 0", "", "summon cow ~2 ~ ~"].map(String::from));
    for _ in 0..3 {
        lines.extend(["!hit cow", "", ""].map(String::from));
    }
    lines.extend(["kill @e[type=cow]", "summon armor_stand ~2 ~ ~", "", "!hit armor_stand", "", "damage @s 2", "", "", "gamemode creative @s", "kill @e[type=!player]", ""].map(String::from));
    lines
}

/// The actor, in survival, hit for 5 by `/damage` as an entity's attack: bare, in a leather and an
/// iron chestplate, in all of iron, of diamond and of netherite; then for 2 in the netherite by
/// each other cause.
pub fn armour() -> Vec<String> {
    let mut lines: Vec<String> = ["kill @e[type=!player]", "", "fill ~-3 ~-1 ~-3 ~8 ~-1 ~8 stone", "fill ~-3 ~ ~-3 ~8 ~6 ~8 air", "gamemode survival @s", ""].map(String::from).into();
    let wear = |lines: &mut Vec<String>, part: &str, item: &str| lines.push(format!("replaceitem entity @s slot.armor.{part} 0 {item}"));
    let hit = |lines: &mut Vec<String>, harm: &str| lines.extend([format!("damage @s {harm}"), String::new(), "effect @s instant_health 1 4".into(), String::new()]);
    hit(&mut lines, "5 entity_attack");
    for chestplate in ["leather_chestplate", "iron_chestplate"] {
        wear(&mut lines, "chest", chestplate);
        hit(&mut lines, "5 entity_attack");
    }
    for metal in ["iron", "diamond", "netherite"] {
        for (part, piece) in [("head", "helmet"), ("chest", "chestplate"), ("legs", "leggings"), ("feet", "boots")] {
            wear(&mut lines, part, &format!("{metal}_{piece}"));
        }
        hit(&mut lines, "5 entity_attack");
    }
    for cause in ["", " fall", " fire", " drowning", " suffocation", " lava"] {
        hit(&mut lines, &format!("2{cause}"));
    }
    lines.extend(["gamemode creative @s", "clear @s", ""].map(String::from));
    lines
}

/// The actor, in survival, under `/effect`: healed and harmed at once by two strengths, mended by
/// regeneration at two, poisoned to its last health, withered, made quick, and cleared.
pub fn effects() -> Vec<String> {
    let mut lines: Vec<String> = ["kill @e[type=!player]", "", "fill ~-3 ~-1 ~-3 ~8 ~-1 ~8 stone", "fill ~-3 ~ ~-3 ~8 ~6 ~8 air", "gamemode survival @s", ""].map(String::from).into();
    let wait = |lines: &mut Vec<String>, seconds: usize| lines.extend(std::iter::repeat_n(String::new(), seconds));
    lines.extend(["damage @s 18", "", "effect @s instant_health 1 0", "", "effect @s instant_health 1 1", "", "effect @s instant_health 1 4", ""].map(String::from));
    lines.extend(["effect @s instant_damage 1 0", "", "effect @s instant_health 1 4", "", "effect @s instant_damage 1 1", "", "damage @s 4", ""].map(String::from));
    lines.push("effect @s regeneration 6 0".into());
    wait(&mut lines, 8);
    lines.push("effect @s regeneration 3 2 true".into());
    wait(&mut lines, 5);
    lines.extend(["effect @s instant_health 1 4", "", "effect @s poison 30 0"].map(String::from));
    wait(&mut lines, 28);
    lines.extend(["effect @s clear", "", "effect @s instant_health 1 4", "", "effect @s poison 3 1"].map(String::from));
    wait(&mut lines, 5);
    lines.extend(["effect @s instant_health 1 4", "", "effect @s wither 5 0"].map(String::from));
    wait(&mut lines, 7);
    lines.extend(["effect @s instant_health 1 4", "", "effect @s speed 3 1", "", "effect @s speed 3 0", "", "effect @s speed 0", "", "effect @s nosuch 3 1", "effect @s regeneration", "effect @s regeneration 5 300", "effect @s clear", "effect @s clear", ""].map(String::from));
    lines.extend(["gamemode creative @s", "effect @s poison 3 1", "", "", "", "", ""].map(String::from));
    lines
}

/// The wake drill: the actor, in survival by night, lies in a bed and leaves it by `!wake` (a
/// client's `PlayerAction StopSleeping`), lies down again and is woken by the day, and again and
/// is woken by the bed broken under it (docs/research/beds-and-containers.md 3). An armour stand
/// marks where it lay down from, since it stands up elsewhere.
pub fn wake() -> Vec<String> {
    let mut lines: Vec<String> = ["kill @e[type=!player]", "time set 18000", "fill ~-2 ~-1 ~-2 ~4 ~-1 ~2 stone", "fill ~-2 ~ ~-2 ~4 ~3 ~2 air", "gamemode survival @s", "summon armor_stand ~ ~ ~", ""].map(String::from).into();
    for leave in ["!wake", "time set 1000", "setblock ~1 ~ ~ air"] {
        lines.extend(["fill ~1 ~ ~ ~1 ~ ~1 air", "setblock ~1 ~ ~ bed", "", "!sleep 1 0 0", "", "", leave, "", "", ""].map(String::from));
        lines.extend(["tp @s @e[type=armor_stand]", "time set 18000", ""].map(String::from));
    }
    lines.extend(["time set 1000", "fill ~-2 ~ ~-2 ~4 ~1 ~2 air", "gamemode creative @s", ""].map(String::from));
    lines
}
