//! The lines of the portal probe's drills. A line is a command, watched for 1.2 s or for the
//! milliseconds after a final ` #`; or `!light x y z face [ms]` (the held item used on a block),
//! `!wait ms`, `!portals reach` or `!where`.

use std::time::Duration;

const WAIT: Duration = Duration::from_millis(1200);

pub enum Step<'a> {
    Command(&'a str, Duration),
    Light([i32; 3], &'a str, Duration),
    Wait(Duration),
    Portals(i32),
    Where,
}

pub fn step(line: &str) -> Step<'_> {
    let words: Vec<&str> = line.split(' ').collect();
    let int = |at: usize| words.get(at).and_then(|word| word.parse::<i64>().ok());
    let ms = |at: usize| int(at).map_or(WAIT, |ms| Duration::from_millis(ms as u64));
    match words[0] {
        "!light" => Step::Light([0, 1, 2].map(|i| int(i + 1).unwrap_or(0) as i32), words.get(4).copied().unwrap_or("up"), ms(5)),
        "!wait" => Step::Wait(ms(1)),
        "!portals" => Step::Portals(int(1).unwrap_or(8) as i32),
        "!where" => Step::Where,
        _ => match line.rsplit_once(" #") {
            Some((command, wait)) => Step::Command(command, wait.parse().map_or(WAIT, Duration::from_millis)),
            None => Step::Command(line, WAIT),
        },
    }
}

fn owned(lines: &[&str]) -> Vec<String> {
    lines.iter().map(|line| line.to_string()).collect()
}

/// A 2 x 3 frame along x with its lower left inner cell at `(x, y, z)`, flint and steel in hand
/// and the actor in reach of it, two blocks in front.
fn frame(x: i32, y: i32, z: i32) -> Vec<String> {
    vec![
        format!("fill {} {} {z} {} {} {z} obsidian", x - 1, y - 1, x + 2, y + 3),
        format!("fill {x} {y} {z} {} {} {z} air", x + 1, y + 2),
        "replaceitem entity @s slot.weapon.mainhand 0 flint_and_steel".into(),
        format!("tp @s {x}.5 {} {}.5", y - 1, z - 2),
    ]
}

/// The lines of the drill `name`.
pub fn drill(name: &str) -> Option<Vec<String>> {
    let mut lines = owned(&["gamemode creative", "gamerule domobspawning false", "tp @s 0.5 -60 0.5 #2000", "!where"]);
    match name {
        // There and back by command.
        "travel" => lines.extend(owned(&[
            "execute in nether run tp @s 0.5 70 0.5 #9000",
            "!where",
            "!portals 6",
            "tp @s 8.5 70 8.5 #2000",
            "!where",
            "execute in overworld run tp @s 0.5 -60 0.5 #9000",
            "!where",
        ])),
        // A frame lit by flint and steel, then stood in, in creative.
        "light" => {
            lines.extend(frame(4, -59, 5));
            lines.extend(owned(&["!light 4 -60 5 up 2000", "!portals 6", "tp @s 4.5 -59 5.5 #14000", "!where", "!portals 16", "!wait 12000", "!where"]));
        }
        // The same in survival, leaving early once.
        "survival" => {
            lines.extend(frame(4, -59, 5));
            lines.extend(owned(&["!light 4 -60 5 up 2000", "gamemode survival", "tp @s 4.5 -59 5.5 #1500", "tp @s 0.5 -60 0.5 #1500", "tp @s 4.5 -59 5.5 #14000", "!where", "!portals 16"]));
        }
        // Frame sizes, shapes and sparks.
        "frames" => lines.extend(frames()),
        // A frame broken, then a portal block broken.
        "break" => {
            lines.extend(frame(4, -59, 5));
            lines.extend(owned(&["!light 4 -60 5 up 2000", "setblock 3 -58 5 air #2000", "!portals 6"]));
            lines.extend(frame(10, -59, 5));
            lines.extend(owned(&["!light 10 -60 5 up 2000", "setblock 10 -58 5 air #2000", "!portals 6", "setblock 11 -60 5 air #2000", "!portals 6"]));
        }
        // Through a portal and back through the one made at the other end.
        "back" => {
            lines.extend(frame(4, -59, 5));
            lines.extend(owned(&["!light 4 -60 5 up 2000", "tp @s 4.5 -59 5.5 #14000", "!where", "!portals 16", "!wait 14000", "!where", "!portals 16"]));
        }
        // A second overworld portal 160 blocks east, 20 in the Nether: does it find the first's?
        "far" => {
            lines.extend(frame(4, -59, 5));
            lines.extend(owned(&["!light 4 -60 5 up 2000", "tp @s 4.5 -59 5.5 #14000", "!where", "!portals 24", "execute in overworld run tp @s 164.5 -60 3.5 #6000"]));
            lines.extend(frame(164, -59, 5));
            lines.extend(owned(&["!light 164 -60 5 up 2000", "tp @s 164.5 -59 5.5 #14000", "!where", "!portals 32"]));
        }
        // Four Nether portals made in one run, 250 apart there: the direction draws and the sites.
        "makes" => {
            for x in [0, 2000, 4000, 6000] {
                lines.push(format!("execute in overworld run tp @s {}.5 -60 3.5 #6000", x + 4));
                lines.push(format!("fill {} -61 1 {} -61 7 stone", x, x + 8));
                lines.extend(frame(x + 4, -59, 5));
                lines.extend([format!("!light {} -60 5 up 2000", x + 4), format!("tp @s {}.5 -59 5.5 #14000", x + 4), "!where".into(), "!portals 24".into()]);
            }
        }
        _ => return None,
    }
    Some(lines)
}

/// Frames of other sizes and ways to light them, along z from x 20 on, each lit at its lower left.
fn frames() -> Vec<String> {
    let mut lines = vec!["replaceitem entity @s slot.weapon.mainhand 0 flint_and_steel".to_string()];
    // (label, inner width, inner height, lit with fire by command)
    let shapes = [("1x2", 1, 2, false), ("2x3", 2, 3, false), ("2x2", 2, 2, false), ("21x21", 21, 21, false), ("22x3", 22, 3, false), ("3x22", 3, 22, false), ("setblock-fire", 2, 3, true)];
    let mut z = 10;
    for (label, width, height, by_command) in shapes {
        let (x, y) = (20, -59);
        lines.push(format!("say frame {label}"));
        lines.push(format!("tp @s {}.5 -60 {}.5 #1000", x - 3, z - 2));
        lines.push(format!("fill {} {} {z} {} {} {z} obsidian", x - 1, y - 1, x + width, y + height));
        lines.push(format!("fill {x} {y} {z} {} {} {z} air", x + width - 1, y + height - 1));
        lines.push(match by_command {
            true => format!("setblock {x} {y} {z} fire #2000"),
            false => format!("!light {x} {} {z} up 2000", y - 1),
        });
        lines.push("!portals 24".into());
        z += 6;
    }
    // A frame along z, corners missing, lit on its side wall.
    let (x, y, z) = (60, -59, 10);
    lines.push("say frame along z, no corners".into());
    lines.push(format!("tp @s {}.5 -60 {}.5 #1000", x - 2, z - 2));
    lines.push(format!("fill {x} {} {z} {x} {} {} obsidian", y - 1, y + 3, z + 3));
    lines.push(format!("fill {x} {y} {} {x} {} {} air", z + 1, y + 2, z + 2));
    for corner in [[x, y - 1, z], [x, y - 1, z + 3], [x, y + 3, z], [x, y + 3, z + 3]] {
        lines.push(format!("setblock {} {} {} air", corner[0], corner[1], corner[2]));
    }
    lines.push(format!("!light {x} {} {} up 2000", y - 1, z + 1));
    lines.push("!portals 24".into());
    lines
}
