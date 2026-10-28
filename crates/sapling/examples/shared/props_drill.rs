//! The hangers, standhits, jostle, knockback, flop, perch and goldpickup drills of
//! bds_command_probe: paintings and armour stands broken, players and mobs pushing, throws against
//! knockback resistance, fish out of water and fleeing, a tamed parrot, a piglin and gold on the
//! floor. Laid out as warden_drill.rs.

use super::warden::{SHOWN, healed, start};

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some((
        match name {
            "hangers" => hangers(),
            "standhits" => stand_hits(),
            "jostle" => jostle(),
            "knockback" => knockback(),
            "flop" => flop(),
            "perch" => perch(),
            "goldpickup" => gold_pickup(),
            _ => return None,
        },
        SHOWN,
    ))
}

fn waits(lines: &mut Vec<String>, seconds: usize) {
    lines.extend(std::iter::repeat_n(String::new(), seconds));
}

fn with(mut lines: Vec<String>, more: &[&str]) -> Vec<String> {
    lines.extend(more.iter().map(|line| line.to_string()));
    lines
}

/// Paintings hung on a wall 3 east: one whose wall goes behind it, one hurt by the survival
/// actor's `/damage`, one by a creative actor's, one killed.
pub fn hangers() -> Vec<String> {
    let mut lines = with(start("noon"), &["gamemode creative @s", "fill 3 -26 -8 3 -20 8 stone", "give @s painting 8", ""]);
    for z in [-6, -1, 4] {
        lines.extend([format!("!click 3 1 {z} west"), String::new()]);
    }
    lines.extend(["fill 3 -26 -8 3 -20 -4 air", ""].map(String::from));
    waits(&mut lines, 8);
    lines.extend(["gamemode survival @s", "damage @e[type=painting,x=2,y=-24,z=-1,r=3] 1 entity_attack entity @s", ""].map(String::from));
    waits(&mut lines, 2);
    lines.extend(["gamemode creative @s", "damage @e[type=painting] 1 entity_attack entity @s", ""].map(String::from));
    waits(&mut lines, 2);
    with(lines, &["kill @e[type=painting]", "", "fill 3 -26 -8 3 -20 8 air", "kill @e[type=item]", ""])
}

/// Armour stands in a helmet and with a sword, hit by the survival actor: twice a second apart,
/// twice at once, twice three seconds apart; then once by a creative actor.
pub fn stand_hits() -> Vec<String> {
    let dressed = |x: f32| {
        [
            format!("summon armor_stand {x} -26 2.5"),
            format!("replaceitem entity @e[type=armor_stand,x={x},y=-26,z=2.5,r=0.5] slot.armor.head 0 diamond_helmet"),
            format!("replaceitem entity @e[type=armor_stand,x={x},y=-26,z=2.5,r=0.5] slot.weapon.mainhand 0 iron_sword"),
            "tp @s 0.5 -26 0.5".to_owned(),
            String::new(),
        ]
    };
    let mut lines = with(start("noon"), &["effect @s clear"]);
    lines.extend(dressed(1.5));
    lines.extend(["!hit armor_stand".into(), String::new(), "!hit armor_stand".into()]);
    waits(&mut lines, 3);
    lines.extend(dressed(0.5));
    lines.push("!hit 2armor_stand".into());
    waits(&mut lines, 3);
    lines.extend(dressed(-0.5));
    lines.extend(["!hit armor_stand".into(), String::new(), String::new(), String::new(), "!hit armor_stand".into()]);
    waits(&mut lines, 3);
    lines.push("gamemode creative @s".into());
    lines.extend(dressed(1.5));
    lines.push("!hit armor_stand".into());
    waits(&mut lines, 3);
    with(lines, &["kill @e[type=!player]", ""])
}

/// A cow summoned at the survival actor's feet, then 0.3 off; a cow against an armour stand, a
/// shulker and a bat, five seconds each.
pub fn jostle() -> Vec<String> {
    let mut lines = with(start("noon"), &["effect @s clear", "tp @s 0.5 -26 0.5", "summon cow 0.5 -26 0.5"]);
    waits(&mut lines, 5);
    lines.extend(["kill @e[type=cow]", "summon cow 0.8 -26 0.5"].map(String::from));
    waits(&mut lines, 5);
    lines.extend(["kill @e[type=cow]", "tp @s -8.5 -26 -8.5", "summon armor_stand 4.5 -26 4.5", "summon cow 4.8 -26 4.5"].map(String::from));
    waits(&mut lines, 5);
    lines.extend(["kill @e[type=!player]", "summon shulker 4.5 -26 4.5", "summon cow 4.5 -25 4.8"].map(String::from));
    waits(&mut lines, 5);
    with(lines, &["kill @e[type=!player]", ""])
}

/// A creative actor's hit on a cow, a hoglin, a zoglin, a ravager and an iron golem 2 off.
pub fn knockback() -> Vec<String> {
    let mut lines = with(start("noon"), &["gamemode creative @s", "effect @s clear"]);
    for kind in ["cow", "hoglin", "zoglin", "ravager", "iron_golem"] {
        lines.extend([format!("summon {kind} 2.5 -26 0.5"), "tp @s 0.5 -26 0.5 facing 2.5 -25 0.5".into(), String::new(), format!("!hit {kind}")]);
        waits(&mut lines, 3);
        lines.push("kill @e[type=!player]".into());
    }
    lines.push(String::new());
    lines
}

/// A cod, a salmon and a tropical fish summoned onto the dry floor, ten seconds; then three cod in
/// a pool, the survival actor walking in at its edge.
pub fn flop() -> Vec<String> {
    let mut lines = with(start("noon"), &["gamemode creative @s", "summon cod 3.5 -26 0.5", "summon salmon 3.5 -26 3.5", "summon tropicalfish 3.5 -26 -2.5"]);
    waits(&mut lines, 10);
    lines.extend(["kill @e[type=!player]", "fill 2 -27 -6 14 -21 6 stone", "fill 3 -26 -5 13 -22 5 water", "summon cod 8.5 -24 0.5", "summon cod 8.5 -24 2.5", "summon cod 8.5 -24 -1.5"].map(String::from));
    waits(&mut lines, 10);
    lines.extend(["gamemode survival @s", "tp @s 4.5 -26 0.5 facing 8.5 -24 0.5"].map(String::from));
    waits(&mut lines, 10);
    with(lines, &["gamemode creative @s", "kill @e[type=!player]", "fill 2 -27 -6 14 -21 6 air", "fill 2 -27 -6 14 -27 6 stone", ""])
}

/// A parrot by the survival actor fed seeds until tame, then thirty seconds.
pub fn perch() -> Vec<String> {
    let mut lines = with(start("noon"), &["effect @s clear", "summon parrot 2.5 -26 0.5", "give @s wheat_seeds 32", "tp @s 0.5 -26 0.5 facing 2.5 -25.5 0.5", ""]);
    for _ in 0..8 {
        lines.extend(["effect @e[type=parrot] slowness 2 255 true".into(), "tp @s 0.5 -26 0.5 facing @e[type=parrot,c=1]".into(), "!interact parrot".into()]);
    }
    healed(&mut lines, 30);
    with(lines, &["kill @e[type=!player]", "clear @s", ""])
}

/// A piglin 3 off as a gold nugget, a gold ingot, a golden sword and a cobblestone are dropped on
/// the floor between them, twenty seconds; then two piglins after one ingot.
pub fn gold_pickup() -> Vec<String> {
    let mut lines = with(start("noon"), &["gamemode creative @s", "tp @s 0.5 -26 0.5 facing 3.5 -25.5 0.5", "summon piglin 3.5 -26 0.5 0 0 minecraft:spawn_adult"]);
    for item in ["gold_nugget", "gold_ingot", "golden_sword", "cobblestone"] {
        lines.extend(["kill @e[type=item]".into(), "clear @s".into(), format!("give @s {item}"), String::new(), "!drop 0 1".into()]);
        waits(&mut lines, 12);
    }
    lines.extend(["kill @e[type=!player]", "clear @s", "summon piglin 3.5 -26 2.5 0 0 minecraft:spawn_adult", "summon piglin 3.5 -26 -1.5 0 0 minecraft:spawn_adult", "give @s gold_ingot", "", "!drop 0 1"].map(String::from));
    waits(&mut lines, 12);
    with(lines, &["kill @e[type=!player]", "clear @s", ""])
}
