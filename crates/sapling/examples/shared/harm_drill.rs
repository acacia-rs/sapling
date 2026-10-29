//! The catalyst, harm2 and burn2 drills of bds_command_probe (docs/research/sculk.md "Catalyst",
//! mob-damage.md): many mobs at once in stone pens on a floor at y -27, at fixed places.

/// Packets shown of these drills and of fights_drill.rs's.
pub const SHOWN: &[&str] = &[
    "UpdateAttributes", "EntityEvent", "SetEntityData", "SetHealth", "LevelSoundEvent", "LevelEvent", "LevelEventGeneric", "MovePlayer", "SetEntityMotion", "Animate", "RemoveEntity",
    "AddEntity", "AddItemEntity", "MoveEntityDelta", "MoveEntityAbsolute", "CorrectPlayerMovePrediction", "MobEquipment", "MobArmorEquipment", "MobEffect", "UpdateBlock",
    "UpdateSubchunkBlocks", "InventorySlot", "InventoryContent", "DeathInfo",
];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some((
        match name {
            "catalyst" => catalyst(),
            "harm2" => harm2(),
            "burn2" => burn2(),
            _ => return None,
        },
        SHOWN,
    ))
}

/// A cell's middle; `format!("{x}.5")` is the wrong cell for negative x.
pub fn mid(cell: i32) -> f32 {
    cell as f32 + 0.5
}

/// Every mob gone, the floor laid at y -27 with 10 clear over it, the actor at 0 -26 `actor_z`
/// in creative, the watcher in a corner, at `time` on easy.
pub fn start(time: &str, actor_z: i32) -> Vec<String> {
    let mut lines: Vec<String> = [
        "kill @e[type=!player]",
        "gamerule domobspawning false",
        "gamerule falldamage true",
        "weather clear",
        "difficulty easy",
        "gamemode creative CmdWatcher",
        "gamemode creative @s",
        "effect @s clear",
        "clear @s",
        "fill -20 -27 -20 20 -27 20 stone",
        "fill -20 -26 -20 20 -16 0 air",
        "fill -20 -26 0 20 -16 20 air",
        "tp CmdWatcher 18.5 -26 18.5",
    ]
    .map(String::from)
    .into();
    lines.extend([format!("time set {time}"), format!("tp @s 0.5 -26 {}", mid(actor_z)), String::new()]);
    lines
}

/// A stone pen around cell `x`, `z`, its walls `height` high from y -26 and its inside clear.
pub fn pen(lines: &mut Vec<String>, x: i32, z: i32, height: i32) {
    let top = -27 + height;
    lines.push(format!("fill {} -26 {} {} {top} {} stone", x - 1, z - 1, x + 1, z + 1));
    lines.push(format!("fill {x} -26 {z} {x} {top} {z} air"));
}

/// `summon` of `kind`, which may carry a rotation and spawn event after its name, at x y z.
pub fn summon(kind: &str, x: f32, y: f32, z: f32) -> String {
    let (kind, event) = kind.split_once(' ').map_or((kind, String::new()), |(kind, event)| (kind, format!(" {event}")));
    format!("summon {kind} {x} {y} {z}{event}")
}

pub fn wait(lines: &mut Vec<String>, seconds: usize) {
    lines.extend(std::iter::repeat_n(String::new(), seconds));
}

/// A catalyst in the floor at 0 -27 -8 and deaths by `/kill` around it: a cow 3 off, cows 8.4, 9.9
/// and 10.4 off (range 10 against Java's 8), a baby cow and a snow golem (no reward), then four
/// zombies killed at once (merging cursors), and the spread left to run.
pub fn catalyst() -> Vec<String> {
    let mut lines = start("midnight", 4);
    lines.extend(["setblock 0 -27 -8 sculk_catalyst", ""].map(String::from));
    let kill = |lines: &mut Vec<String>, summon: String, kind: &str| {
        lines.extend([summon, String::new(), format!("kill @e[type={kind}]")]);
        wait(lines, 4);
    };
    for dx in [3.0, 8.4, 9.9, 10.4] {
        kill(&mut lines, format!("summon cow {} -26 -7.5", 0.5 + dx), "cow");
    }
    kill(&mut lines, "summon cow -2.5 -26 -7.5 0 0 minecraft:entity_born".into(), "cow");
    kill(&mut lines, "summon snow_golem -2.5 -26 -7.5".into(), "snow_golem");
    lines.extend(["summon zombie 2.5 -26 -9.5", "summon zombie 2.5 -26 -5.5", "summon zombie -1.5 -26 -9.5", "summon zombie -1.5 -26 -5.5"].map(String::from));
    kill(&mut lines, String::new(), "zombie");
    wait(&mut lines, 12);
    lines.extend(["kill @e[type=!player]", "fill -12 -27 -20 12 -27 4 stone", ""].map(String::from));
    lines
}

/// Cows dropped 8 onto stone, hay, slime, water and a bed, a baby cow and a zombie onto stone;
/// cows under glass, leaves, stone and a slab one block up, one shut in water then let out, fish on
/// dry stone; then the actor in survival dropped 8 onto stone, hay and water, and stood in fire.
pub fn harm2() -> Vec<String> {
    let mut lines = start("midnight", 12);
    let drops = [("stone", "cow"), ("hay_block", "cow"), ("slime", "cow"), ("water", "cow"), ("stone", "cow 0 0 minecraft:entity_born"), ("stone", "zombie")];
    for (at, (landing, kind)) in drops.iter().enumerate() {
        let x = -16 + 4 * at as i32;
        pen(&mut lines, x, -14, 11);
        lines.push(format!("setblock {x} -26 -14 {landing}"));
        lines.push(summon(kind, mid(x), -17.0, -13.5));
    }
    lines.extend(["fill 7 -26 -15 9 -16 -12 stone", "fill 8 -26 -14 8 -16 -13 air", "setblock 8 -26 -14 bed [\"direction\"=0,\"head_piece_bit\"=false]", "setblock 8 -26 -13 bed [\"direction\"=0,\"head_piece_bit\"=true]", "summon cow 8.5 -17 -13.5"].map(String::from));
    wait(&mut lines, 5);
    lines.extend(["kill @e[type=!player]", "fill -18 -26 -16 10 -16 -11 air", ""].map(String::from));
    for (at, roof) in ["glass", "oak_leaves [\"persistent_bit\"=true]", "stone", "oak_slab [\"minecraft:vertical_half\"=\"bottom\"]"].iter().enumerate() {
        let x = -16 + 4 * at as i32;
        pen(&mut lines, x, -6, 2);
        lines.extend([format!("setblock {x} -25 -6 {roof}"), format!("summon cow {} -26 -5.5", mid(x))]);
    }
    pen(&mut lines, 0, -6, 3);
    lines.extend(["fill 0 -26 -6 0 -25 -6 water", "setblock 0 -24 -6 stone", "summon cow 0.5 -26 -5.5"].map(String::from));
    for (at, fish) in ["cod", "salmon", "tropical_fish", "pufferfish"].iter().enumerate() {
        let x = 4 + 4 * at as i32;
        pen(&mut lines, x, -6, 2);
        lines.push(format!("summon {fish} {} -26 -5.5", mid(x)));
    }
    wait(&mut lines, 16);
    lines.extend(["fill 0 -26 -6 0 -25 -6 air", ""].map(String::from));
    wait(&mut lines, 12);
    lines.extend(["kill @e[type=!player]", "fill -18 -26 -8 18 -16 -4 air", "gamemode survival @s", "effect @s instant_health 1 4", ""].map(String::from));
    for (at, landing) in ["stone", "hay_block", "water"].iter().enumerate() {
        let x = -8 + 4 * at as i32;
        pen(&mut lines, x, 4, 11);
        lines.extend([format!("setblock {x} -26 4 {landing}"), format!("tp @s {} -17 4.5", mid(x))]);
        wait(&mut lines, 3);
        lines.extend(["tp @s 0.5 -26 12.5", "effect @s instant_health 1 4", ""].map(String::from));
    }
    pen(&mut lines, 6, 4, 3);
    lines.extend(["setblock 6 -26 4 fire", "tp @s 6.5 -26 4.5", "", "", "tp @s 0.5 -26 12.5"].map(String::from));
    wait(&mut lines, 10);
    lines.extend(["gamemode creative @s", "effect @s clear", "kill @e[type=!player]", "fill -18 -26 2 18 -16 6 air", ""].map(String::from));
    lines
}

/// Cows on soul fire, magma, a campfire and a soul campfire, cows put in fire and in lava for a
/// second (burn times), one in lava with fire resistance; zombies plain, in diamond armour and
/// baby, and an armour stand, put in lava; then a burning cow in the rain.
pub fn burn2() -> Vec<String> {
    let mut lines = start("midnight", 12);
    let cells: [(&str, &str); 7] = [
        ("soul_soil", "soul_fire"),
        ("magma", "air"),
        ("stone", "campfire"),
        ("stone", "soul_campfire"),
        ("netherrack", "fire"),
        ("stone", "lava"),
        ("stone", "lava"),
    ];
    for (at, (floor, inside)) in cells.iter().enumerate() {
        let x = -16 + 4 * at as i32;
        pen(&mut lines, x, -14, 3);
        lines.extend([format!("setblock {x} -27 -14 {floor}"), format!("summon cow {} -26 -13.5", mid(x))]);
        if at == 6 {
            lines.push("effect @e[type=cow,x=8.5,y=-26,z=-13.5,r=1] fire_resistance 60 0 true".into());
        }
        lines.push(format!("setblock {x} -26 -14 {inside}"));
    }
    lines.extend(["", "setblock 0 -26 -14 air", "setblock 4 -26 -14 air"].map(String::from));
    wait(&mut lines, 12);
    lines.extend(["kill @e[type=!player]", "fill -18 -27 -16 18 -27 -12 stone", "fill -18 -26 -16 18 -16 -12 air", ""].map(String::from));
    let kinds = ["zombie", "zombie", "zombie 0 0 minecraft:as_baby", "armor_stand"];
    for (at, kind) in kinds.iter().enumerate() {
        let x = -8 + 4 * at as i32;
        pen(&mut lines, x, -6, 3);
        lines.push(summon(kind, mid(x), -26.0, -5.5));
    }
    for slot in [("head", "helmet"), ("chest", "chestplate"), ("legs", "leggings"), ("feet", "boots")] {
        lines.push(format!("replaceitem entity @e[type=zombie,x=-3.5,y=-26,z=-5.5,r=1] slot.armor.{} 0 diamond_{}", slot.0, slot.1));
    }
    lines.push(String::new());
    lines.extend((0..4).map(|at| format!("setblock {} -26 -6 lava", -8 + 4 * at)));
    wait(&mut lines, 5);
    lines.extend(["kill @e[type=!player]", "fill -10 -26 -8 6 -16 -4 air", ""].map(String::from));
    pen(&mut lines, 0, 4, 3);
    lines.extend(["setblock 0 -27 4 netherrack", "summon cow 0.5 -26 4.5", "setblock 0 -26 4 fire", "", "setblock 0 -26 4 air", "weather rain"].map(String::from));
    wait(&mut lines, 10);
    lines.extend(["weather clear", "kill @e[type=!player]", "fill -2 -27 2 2 -27 6 stone", "fill -2 -26 2 2 -16 6 air", ""].map(String::from));
    lines
}
