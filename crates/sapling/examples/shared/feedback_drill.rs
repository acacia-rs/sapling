//! The feedback drill of bds_command_probe: the same commands with `sendCommandFeedback` on, off and
//! on again, to see what the rule takes from the actor's answers and what the watcher hears.

pub const FEEDBACK_SHOWN: &[&str] = &["GameRulesChanged"];

const COMMANDS: [&str; 10] =
    ["time query daytime", "time set 1000", "gamemode creative @s", "give @s stone 2", "give @a dirt 1", "xp 3 @s", "effect @s speed 5", "say hello", "tell @a psst", "time set banana"];

pub fn feedback() -> Vec<String> {
    let mut lines = Vec::new();
    for rule in ["true", "false", "true"] {
        lines.extend([format!("gamerule sendcommandfeedback {rule}"), String::new()]);
        lines.extend(COMMANDS.map(String::from));
        lines.push(String::new());
    }
    lines
}
