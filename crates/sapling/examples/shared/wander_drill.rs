//! The swimmers, flyers, grazers, amphibians, beasts, creaking and crowd drills of bds_command_probe: mobs
//! left to themselves, watched from near enough that every move is told (research/bds-actor-move-sync.md),
//! and the hostiles that strolled in sapling but attacked on BDS. All in fixed places, as `dolphin`.

/// Both bots in creative, mob spawning off and every mob gone.
const START: [&str; 6] = ["kill @e[type=!player]", "gamerule domobspawning false", "difficulty easy", "gamemode creative @s", "gamemode creative CmdWatcher", "effect @s clear"];
/// Every mob gone, the place filled back with stone, the actor in creative.
const END: [&str; 4] = ["kill @e[type=!player]", "fill -21 -27 -21 21 -14 21 air", "gamemode creative @s", ""];

fn start(time: &str, build: &[&str], stand: &str) -> Vec<String> {
    let mut lines: Vec<String> = START.map(String::from).into();
    lines.push(format!("time set {time}"));
    lines.extend(build.iter().map(|line| line.to_string()));
    lines.extend([format!("tp @s {stand}"), format!("tp CmdWatcher {stand}"), String::new()]);
    lines
}

fn watch(lines: &mut Vec<String>, summons: &[&str], seconds: usize) {
    lines.extend(summons.iter().map(|line| format!("summon {line}")));
    lines.extend(std::iter::repeat_n(String::new(), seconds));
}

/// A pool 21 across and 7 deep walled in stone, both bots on glass 4 over its middle: a cod, a
/// salmon, a pufferfish and a tropical fish in its corners and a squid and a glow squid on its
/// sides, forty seconds.
pub fn swimmers() -> Vec<String> {
    let build = ["fill -11 -27 -11 11 -20 11 stone", "fill -10 -26 -10 10 -20 10 water", "fill -11 -19 -11 11 -14 11 air", "setblock 0 -16 0 glass"];
    let mut lines = start("noon", &build, "0.5 -15 0.5");
    watch(&mut lines, &["cod 6.5 -23 6.5", "salmon -5.5 -23 6.5", "pufferfish 6.5 -23 -5.5", "tropicalfish -5.5 -23 -5.5", "squid 0.5 -24 8.5", "glow_squid 0.5 -24 -7.5"], 40);
    lines.extend(END.map(String::from));
    lines
}

/// A dark stone room 21 across and 12 high, both bots on its floor: a bat, a parrot and an allay,
/// forty seconds.
pub fn flyers() -> Vec<String> {
    let build = ["fill -11 -27 -11 11 -14 11 stone", "fill -10 -26 -10 10 -15 10 air"];
    let mut lines = start("midnight", &build, "0.5 -26 0.5");
    watch(&mut lines, &["bat 5.5 -20 5.5", "parrot -4.5 -24 5.5", "allay 5.5 -22 -4.5"], 40);
    lines.extend(END.map(String::from));
    lines
}

/// A stone floor 41 across, both bots in its middle: a horse, a donkey, a mule and a llama six off,
/// a minute.
pub fn grazers() -> Vec<String> {
    let build = ["fill -20 -27 -20 20 -27 20 stone", "fill -20 -26 -20 20 -16 20 air"];
    let mut lines = start("noon", &build, "0.5 -26 0.5");
    watch(&mut lines, &["horse 6.5 -26 0.5", "donkey -5.5 -26 0.5", "mule 0.5 -26 6.5", "llama 0.5 -26 -5.5"], 60);
    lines.extend(["kill @e[type=!player]", "fill -20 -27 -20 20 -16 20 air", ""].map(String::from));
    lines
}

/// A stone floor 41 across, its west half a pool 3 deep level with the east, both bots at the
/// shore: a turtle, a frog and an axolotl on land and one of each in the water, a minute.
pub fn amphibians() -> Vec<String> {
    let build = ["fill -21 -27 -21 21 -24 21 stone", "fill -20 -26 -20 -1 -24 20 water", "fill -21 -23 -21 21 -14 21 air"];
    let mut lines = start("noon", &build, "0.5 -23 0.5");
    watch(&mut lines, &["turtle 5.5 -23 5.5", "frog 5.5 -23 -4.5", "axolotl 8.5 -23 0.5", "turtle -5.5 -25 5.5", "frog -5.5 -25 -4.5", "axolotl -9.5 -25 0.5"], 60);
    lines.extend(["kill @e[type=!player]", "fill -21 -27 -21 21 -14 21 air", ""].map(String::from));
    lines
}

/// On a stone floor 41 across, the actor in survival with ten more hearts, healed every second,
/// the watcher 8 off: a hoglin and a zoglin 8 off left to come for it; a warden hit once from 2
/// off; a creaking 8 off behind its back, then faced.
pub fn beasts() -> Vec<String> {
    let build = ["fill -20 -27 -20 20 -27 20 stone", "fill -20 -26 -20 20 -16 20 air"];
    let mut lines = start("noon", &build, "0.5 -26 0.5");
    lines.extend(["tp CmdWatcher 8.5 -26 0.5", "gamemode survival @s", "effect @s health_boost 600 4", "effect @s instant_health 1 9"].map(String::from));
    let healed = |seconds: usize| std::iter::repeat_n("effect @s instant_health 1 4".to_owned(), seconds);
    for kind in ["hoglin", "zoglin"] {
        lines.extend(["tp @s 0.5 -26 0.5 facing 0.5 -25 -8".into(), format!("summon {kind} 0.5 -26 -7.5")]);
        lines.extend(healed(12));
        lines.extend(["kill @e[type=!player]", ""].map(String::from));
    }
    lines.extend(["tp @s 0.5 -26 0.5 facing 0.5 -25 -8", "summon warden 0.5 -26 -1.5", "", "!hit warden"].map(String::from));
    lines.extend(healed(20));
    lines.extend(["kill @e[type=!player]", "", "tp @s 0.5 -26 0.5 facing 0.5 -25 8", "summon creaking 0.5 -26 -7.5"].map(String::from));
    lines.extend(healed(8));
    lines.push("tp @s 0.5 -26 0.5 facing 0.5 -24 -8".into());
    lines.extend(healed(6));
    lines.extend(["effect @s clear", "kill @e[type=!player]", "gamemode creative @s", "fill -20 -27 -20 20 -16 20 air", ""].map(String::from));
    lines
}

/// As `beasts` at midnight, the watcher looking away: a creaking 8 off behind the actor's back for
/// ten seconds, then faced for six, then behind its back again for eight.
pub fn creaking() -> Vec<String> {
    let build = ["fill -20 -27 -20 20 -27 20 stone", "fill -20 -26 -20 20 -16 20 air"];
    let mut lines = start("midnight", &build, "0.5 -26 0.5");
    lines.extend(["tp CmdWatcher 8.5 -26 0.5 facing 16 -26 0.5", "gamemode survival @s", "effect @s health_boost 600 4", "effect @s instant_health 1 9"].map(String::from));
    let healed = |seconds: usize| std::iter::repeat_n("effect @s instant_health 1 4".to_owned(), seconds);
    lines.extend(["tp @s 0.5 -26 0.5 facing 0.5 -25 8", "summon creaking 0.5 -26 -7.5"].map(String::from));
    lines.extend(healed(10));
    lines.push("tp @s 0.5 -26 0.5 facing 0.5 -24 -8".into());
    lines.extend(healed(6));
    lines.push("tp @s 0.5 -26 0.5 facing 0.5 -25 8".into());
    lines.extend(healed(8));
    lines.extend(["effect @s clear", "kill @e[type=!player]", "gamemode creative @s", "fill -20 -27 -20 20 -16 20 air", ""].map(String::from));
    lines
}

/// On a stone floor 41 across, both bots in its middle: four cows summoned on one spot 4 off,
/// fifteen seconds; then four pigs in a glass pen 2 across, fifteen seconds.
pub fn crowd() -> Vec<String> {
    let build = ["fill -20 -27 -20 20 -27 20 stone", "fill -20 -26 -20 20 -16 20 air"];
    let mut lines = start("noon", &build, "0.5 -26 0.5");
    watch(&mut lines, &["cow 4.5 -26 0.5", "cow 4.5 -26 0.5", "cow 4.5 -26 0.5", "cow 4.5 -26 0.5"], 15);
    lines.extend(["kill @e[type=!player]", "fill -5 -26 -1 -2 -25 2 glass", "fill -4 -26 0 -3 -25 1 air", ""].map(String::from));
    watch(&mut lines, &["pig -3 -26 1", "pig -3 -26 1", "pig -3 -26 1", "pig -3 -26 1"], 15);
    lines.extend(["kill @e[type=!player]", "fill -20 -27 -20 20 -16 20 air", ""].map(String::from));
    lines
}
