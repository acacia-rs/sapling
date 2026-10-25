//! The feedback drill of bds_command_probe: the same commands with `sendCommandFeedback` on, off and
//! on again, to see what the rule takes from the actor's answers and what the watcher hears.

pub const FEEDBACK_SHOWN: &[&str] = &["GameRulesChanged"];

const COMMANDS: [&str; 10] =
    ["time query daytime", "time set 1000", "gamemode creative @s", "give @s stone 2", "give @a dirt 1", "xp 3 @s", "effect @s speed 5", "say hello", "tell @a psst", "time set banana"];

pub const MESSAGING_SHOWN: &[&str] = &["SetTitle", "PlaySound", "StopSound", "SpawnParticleEffect", "LevelSoundEvent", "LevelEvent", "SetScore", "SetDisplayObjective"];

/// `/me`, `/tellraw`, `/title`, `/titleraw`, `/playsound`, `/stopsound` and `/particle`, each as
/// it works and as it fails.
pub const MESSAGING: &[&str] = &[
    "me waves hello",
    "me",
    r#"tellraw @a {"rawtext":[{"text":"hi "},{"translate":"commands.give.success","with":["a","b","c"]},{"text":" "},{"selector":"@a"}]}"#,
    r#"tellraw @a {"rawtext":[{"translate":"%%s and %%s","with":{"rawtext":[{"text":"x"},{"selector":"@s"}]}}]}"#,
    r#"tellraw @a {"rawtext":[{"selector":"@e[type=cow]"},{"text":"|"},{"score":{"name":"@s","objective":"obj"}},{"text":"|"}]}"#,
    r#"tellraw CmdWatcher {"rawtext":[{"text":"only you"}]}"#,
    r#"tellraw @a {"rawtext":[]}"#,
    "tellraw @a not json",
    r#"tellraw @a {"text":"x"}"#,
    r#"tellraw @a {"rawtext":"x"}"#,
    r#"tellraw @a {"rawtext":[{"bogus":"x"}]}"#,
    r#"tellraw @a {"rawtext":[{"text":5}]}"#,
    r#"tellraw @a {"rawtext":[{"translate":"x","with":5}]}"#,
    r#"tellraw @a {"rawtext":[{"score":{"name":"@s"}}]}"#,
    r#"tellraw @a {"rawtext":[{"text":"a"}]} extra"#,
    r#"tellraw nobody {"rawtext":[{"text":"x"}]}"#,
    r#"tellraw @x {"rawtext":[{"text":"x"}]}"#,
    "tellraw @a",
    "scoreboard objectives add obj dummy",
    "scoreboard players set @s obj 7",
    "scoreboard players set Fake obj 3",
    r#"tellraw @a {"rawtext":[{"score":{"name":"@s","objective":"obj"}},{"text":"|"},{"score":{"name":"*","objective":"obj"}},{"text":"|"},{"score":{"name":"Fake","objective":"obj"}},{"text":"|"},{"score":{"name":"CmdWatcher","objective":"obj"}},{"text":"|"},{"score":{"name":"@s","objective":"none"}}]}"#,
    "scoreboard objectives remove obj",
    "title @a title Hello there",
    "title @a subtitle Sub",
    "title @a actionbar Bar",
    "title @a times 5 40 10",
    "title @a clear",
    "title @a reset",
    "title @s title",
    "title @a times -1 2 3",
    "title @a times 1 2",
    "title @a banana x",
    "title nobody title x",
    r#"titleraw @a title {"rawtext":[{"text":"Raw"},{"selector":"@s"}]}"#,
    r#"titleraw @a subtitle {"rawtext":[{"text":"RawSub"}]}"#,
    r#"titleraw @a actionbar {"rawtext":[{"translate":"commands.give.success","with":["a","b","c"]}]}"#,
    "titleraw @a times 1 2 3",
    "titleraw @a clear",
    "titleraw @a reset",
    "titleraw @a title notjson",
    "playsound random.orb",
    "playsound random.orb @a",
    "playsound random.orb @s ~ ~ ~ 2 0.5 0.1",
    "playsound random.orb @a ~100 ~ ~",
    "playsound random.orb @a ~100 ~ ~ 1 1 0.5",
    "playsound random.orb @a ~ ~ ~ -1",
    "playsound random.orb @a ~ ~ ~ 1 300",
    "playsound no.such.sound @a",
    "playsound random.orb nobody",
    "stopsound @a random.orb",
    "stopsound @a",
    "stopsound nobody",
    "particle minecraft:heart_particle ~ ~2 ~",
    "particle minecraft:heart_particle",
    "particle no:such ~ ~ ~",
    "particle minecraft:heart_particle 1 2",
    "gamerule sendcommandfeedback false",
    "title @a title quiet",
    "playsound random.orb @a",
    "particle minecraft:heart_particle ~ ~2 ~",
    r#"tellraw @a {"rawtext":[{"text":"quiet"}]}"#,
    "gamerule sendcommandfeedback true",
];

pub fn feedback() -> Vec<String> {
    let mut lines = Vec::new();
    for rule in ["true", "false", "true"] {
        lines.extend([format!("gamerule sendcommandfeedback {rule}"), String::new()]);
        lines.extend(COMMANDS.map(String::from));
        lines.push(String::new());
    }
    lines
}
