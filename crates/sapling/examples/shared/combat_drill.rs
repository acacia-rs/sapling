//! The combat drills of bds_command_probe: a raised shield, a totem of undying, critical and
//! sprinting hits, a weapon's wear, and what hurts a player where it stands (cactus, berry bushes,
//! powder snow, the void, a falling anvil).

use acacia_bot::Bot;

/// The actor in survival on a wide floor, away from the watcher.
const FLOOR: [&str; 8] = ["kill @e[type=!player]", "gamerule domobspawning false", "difficulty easy", "fill ~-3 ~-1 ~-12 ~12 ~-1 ~40 stone", "fill ~-3 ~ ~-12 ~12 ~6 ~40 air", "gamemode survival @s", "tp @s ~6 ~ ~20", ""];
/// A cow in a pit two deep east of the actor, where it cannot wander off.
const PIT: [&str; 6] = ["kill @e[type=!player]", "", "", "fill ~-3 ~-4 ~-3 ~8 ~-1 ~8 stone", "fill ~-3 ~ ~-3 ~8 ~6 ~8 air", "fill ~1 ~-2 ~ ~1 ~-1 ~ air"];
const HEAL: &str = "effect @s instant_health 1 4";

fn lines(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|line| line.to_string()).collect()
}

fn wait(lines: &mut Vec<String>, seconds: usize) {
    lines.extend(std::iter::repeat_n(String::new(), seconds));
}

fn hand(slot: &str, item: &str) -> String {
    format!("replaceitem entity @s slot.weapon.{slot} 0 {item}")
}

/// The actor, sneaking with a shield in its off hand and then its main hand: hits by an armour
/// stand's name before it, behind it and beside it, of each cause; then a zombie, a skeleton and a
/// vindicator with its axe come at it.
pub fn shield() -> Vec<String> {
    let mut lines = lines(&FLOOR);
    let by = |cause: &str, amount: u32| format!("damage @s {amount} {cause} entity @e[type=armor_stand,c=1]");
    lines.extend(["clear @s".into(), HEAL.into(), hand("offhand", "shield"), "!look 0 1.6 -3".into(), "summon armor_stand ~ ~ ~-2".into(), String::new()]);
    lines.extend([by("entity_attack", 4), String::new(), HEAL.into(), "!sneak".into(), String::new(), String::new()]);
    for (cause, amount) in [("entity_attack", 4), ("entity_attack", 2), ("projectile", 4), ("entity_explosion", 4), ("fire", 4), ("entity_attack", 10)] {
        lines.extend([by(cause, amount), String::new()]);
    }
    lines.extend(["damage @s 4".into(), String::new(), HEAL.into()]);
    for (x, z) in [(0, 3), (2, 0), (2, -2)] {
        lines.extend(["kill @e[type=armor_stand]".into(), format!("summon armor_stand ~{x} ~ ~{z}"), String::new(), by("entity_attack", 4), String::new(), HEAL.into()]);
    }
    for (kind, far, seconds) in [("zombie", 4, 6), ("skeleton", 10, 6), ("vindicator", 4, 8)] {
        lines.extend(["kill @e[type=!player]".into(), HEAL.into(), format!("summon {kind} ~ ~ ~-{far}")]);
        wait(&mut lines, seconds);
    }
    lines.extend(["kill @e[type=!player]", "!unsneak", HEAL, "replaceitem entity @s slot.weapon.offhand 0 air"].map(String::from));
    lines.extend([hand("mainhand", "shield"), "summon armor_stand ~ ~ ~-2".into(), "!sneak".into(), String::new(), by("entity_attack", 4), String::new(), "!unsneak".into(), String::new()]);
    lines.extend([by("entity_attack", 4), String::new()]);
    lines.extend(["kill @e[type=!player]", "gamemode creative @s", "clear @s", ""].map(String::from));
    lines
}

/// The actor, poisoned and quick, with a totem in its off hand, then its main hand, then both, is
/// hurt past its health; then by the void, then `/kill`.
pub fn totem() -> Vec<String> {
    let mut lines = lines(&FLOOR);
    let hurt = |lines: &mut Vec<String>, slots: &[&str], hit: &str| {
        lines.extend(["effect @s clear".into(), HEAL.into(), "effect @s poison 30 0".into(), "effect @s speed 30 1".into(), String::new()]);
        lines.extend(slots.iter().map(|slot| hand(slot, "totem_of_undying")));
        lines.extend([String::new(), hit.to_owned()]);
        wait(lines, 4);
    };
    lines.extend(["clear @s".into()]);
    hurt(&mut lines, &["offhand"], "damage @s 30");
    hurt(&mut lines, &["mainhand"], "damage @s 30 entity_attack");
    hurt(&mut lines, &["mainhand", "offhand"], "damage @s 30 fall");
    lines.extend(["clear @s".into()]);
    hurt(&mut lines, &["offhand"], "damage @s 30 void");
    hurt(&mut lines, &["offhand"], "kill @s");
    lines.extend(["effect @s clear", "gamemode creative @s", "clear @s", ""].map(String::from));
    lines
}

/// Cows penned in barriers beside the actor hit by the fist while it jumps, then stands; the
/// watcher hit as the actor jumps.
pub fn crit() -> Vec<String> {
    let mut lines = lines(&PIT);
    let pen = ["kill @e[type=cow]", "fill ~1 ~ ~-1 ~3 ~1 ~1 barrier", "fill ~2 ~ ~ ~2 ~1 ~ air", "summon cow ~2 ~ ~", ""];
    lines.extend(["gamerule pvp true", "difficulty easy", "gamemode survival @a", "clear @a", "effect @a instant_health 1 4", hand("mainhand", "air").as_str()].map(String::from));
    lines.extend(pen.map(String::from));
    lines.push("!jump".into());
    (0..6).for_each(|_| lines.extend(["!hit cow", ""].map(String::from)));
    lines.extend(["!stop", "", "!hit cow", ""].map(String::from));
    lines.extend(pen.map(String::from));
    lines.extend([hand("mainhand", "wooden_sword"), "!jump".into()]);
    (0..4).for_each(|_| lines.extend(["!hit cow", ""].map(String::from)));
    lines.extend(["!stop", "", hand("mainhand", "air").as_str(), "effect @a instant_health 1 4"].map(String::from));
    lines.extend(["!jump".into()]);
    (0..4).for_each(|_| lines.extend(["!hit player", ""].map(String::from)));
    lines.extend(["!stop", "kill @e[type=!player]", "gamemode creative @a", "clear @a", ""].map(String::from));
    lines
}

/// The actor sprints a few ticks toward a cow in a pit and hits it as it runs, then the same
/// walking; then sprints at the watcher and hits it.
pub fn sprint() -> Vec<String> {
    let mut lines = lines(&["kill @e[type=!player]", "", "", "gamerule pvp true", "difficulty easy", "gamemode survival @a", "clear @a", "effect @a instant_health 1 4"]);
    lines.extend(["fill ~-3 ~-4 ~-12 ~3 ~-1 ~12 stone", "fill ~-3 ~ ~-12 ~3 ~6 ~12 air", "tp @s ~ ~ ~6", ""].map(String::from));
    for run in ["!sprinthit cow", "!walkhit cow"] {
        lines.extend(["fill ~ ~-2 ~-4 ~ ~-1 ~-4 air", "summon cow ~ ~-2 ~-4", "!look 0 1.6 -4", "", run, "", "", "kill @e[type=cow]", "fill ~-3 ~-4 ~-12 ~3 ~-1 ~-1 stone", "tp @s ~ ~ ~2", ""].map(String::from));
    }
    lines.extend(["tp @s ~ ~ ~-3", "!look 0 1.6 -6", "", "!sprinthit player", "", "", "effect @a instant_health 1 4", "", "!walkhit player", "", ""].map(String::from));
    lines.extend(["kill @e[type=!player]", "gamemode creative @a", "clear @a", ""].map(String::from));
    lines
}

/// A cow in the pit hit by weapons and tools near their end: a wooden sword, axe and pickaxe worn
/// to three uses left, a stick, a sword in a hit that does not land, a sword in creative.
pub fn swing_wear() -> Vec<String> {
    let mut lines = lines(&PIT);
    lines.extend(["gamemode survival @s".into(), "clear @s".into()]);
    for tool in ["wooden_sword", "wooden_axe", "wooden_pickaxe", "stick"] {
        lines.extend([hand("mainhand", &format!("{tool} 1 57")), String::new()]);
        (0..3).for_each(|_| lines.extend(["kill @e[type=cow]", "summon cow ~1 ~-2 ~", "", "!hit cow", ""].map(String::from)));
    }
    lines.extend(["kill @e[type=cow]".into(), "summon cow ~1 ~-2 ~".into(), hand("mainhand", "iron_sword 1 10"), String::new(), "!hit2 cow".into(), String::new()]);
    lines.extend(["gamemode creative @s".into(), hand("mainhand", "iron_sword 1 10"), String::new(), "!hit cow".into(), String::new()]);
    lines.extend(["kill @e[type=!player]", "clear @s", ""].map(String::from));
    lines
}

/// The actor walks into a cactus, then through a row of ripe sweet berry bushes and stands in
/// one, then sinks into powder snow and stays.
pub fn thorny() -> Vec<String> {
    let mut lines = lines(&FLOOR);
    lines.extend(["clear @s", HEAL, "setblock ~2 ~-1 ~ sand", "setblock ~2 ~ ~ cactus", "!look 2 1.6 0", "", "!walk"].map(String::from));
    wait(&mut lines, 4);
    lines.extend(["!stop", "setblock ~1 ~ ~ air", "setblock ~2 ~ ~ air", "tp @s ~-1 ~ ~", HEAL, ""].map(String::from));
    lines.extend(["fill ~ ~-1 ~-1 ~ ~-1 ~-6 grass_block", "fill ~ ~ ~-1 ~ ~ ~-6 sweet_berry_bush [\"growth\"=3]", "!look 0 1.6 -6", "", "!walk"].map(String::from));
    wait(&mut lines, 3);
    lines.extend(["!stop"].map(String::from));
    wait(&mut lines, 3);
    lines.extend(["fill ~-1 ~ ~-8 ~1 ~ ~8 air", "tp @s ~ ~ ~-10", HEAL, "", "fill ~ ~-2 ~ ~ ~-1 ~ powder_snow", ""].map(String::from));
    wait(&mut lines, 16);
    lines.extend(["tp @s ~ ~3 ~", "fill ~ ~-5 ~ ~ ~-4 ~ stone", ""].map(String::from));
    wait(&mut lines, 3);
    lines.extend(["gamemode creative @s", "clear @s", ""].map(String::from));
    lines
}

/// The actor falls into the void and dies there.
pub fn void() -> Vec<String> {
    let mut lines = lines(&FLOOR);
    lines.extend(["clear @s", HEAL, "tp @s ~ -110 ~"].map(String::from));
    wait(&mut lines, 8);
    lines.extend(["gamemode creative @s", ""].map(String::from));
    lines
}

/// Anvils dropped on the actor from six and sixteen blocks up, then one onto its helmet.
pub fn anvil() -> Vec<String> {
    let mut lines = lines(&FLOOR);
    lines.extend(["clear @s", HEAL].map(String::from));
    for (up, helmet) in [(6, false), (16, false), (16, true)] {
        if helmet {
            lines.push("replaceitem entity @s slot.armor.head 0 iron_helmet".into());
        }
        lines.extend([HEAL.into(), format!("setblock ~ ~{up} ~ anvil"), String::new(), String::new(), String::new(), "tp @s ~2 ~ ~".into(), String::new()]);
    }
    lines.extend(["gamemode creative @s", "clear @s", ""].map(String::from));
    lines
}

/// The lines of the combat drill `name`, if it is one.
pub fn named(name: &str) -> Option<Vec<String>> {
    Some(match name {
        "shield" => shield(),
        "totem" => totem(),
        "crit" => crit(),
        "sprint" => sprint(),
        "swing-wear" => swing_wear(),
        "thorny" => thorny(),
        "void" => void(),
        "anvil" => anvil(),
        _ => return None,
    })
}

/// `!sneak` and `!unsneak`; `!sprinthit kind` and `!walkhit kind` run (sprinting or not) for five
/// ticks, hit the newest of that kind it knows and stop. False for any other line.
pub async fn act(actor: &mut Bot, line: &str) -> bool {
    if let Some(sneak) = match line {
        "!sneak" => Some(true),
        "!unsneak" => Some(false),
        _ => None,
    } {
        if let Some(controls) = actor.controls() {
            controls.sneak = sneak;
        }
        return true;
    }
    let Some((sprint, kind)) = line.strip_prefix("!sprinthit ").map(|kind| (true, kind)).or(line.strip_prefix("!walkhit ").map(|kind| (false, kind))) else { return false };
    if let Some(controls) = actor.controls() {
        (controls.forward, controls.sprint) = (1.0, sprint);
    }
    let _ = actor.wait_ticks(5).await;
    let kind = format!("minecraft:{kind}");
    match actor.state().entities.iter().filter(|entity| entity.kind == kind).map(|entity| entity.runtime_id).max() {
        Some(id) => println!("## hit {id}: {:?}", actor.attack(id).await),
        None => println!("## no {kind} to hit"),
    }
    if let Some(controls) = actor.controls() {
        (controls.forward, controls.sprint) = (0.0, false);
    }
    true
}
