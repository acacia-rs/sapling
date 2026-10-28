//! The lines of the portal probe's drills. A line is a command, watched for 1.2 s or for the
//! milliseconds after a final ` #`; or `!light x y z face [ms]` (the held item used on a block),
//! `!wait ms`, `!portals reach`, `!where` or `!throw yaw pitch [ms]`; the End's drills and steps
//! are shared/end_drill.rs, the eye of ender's shared/eye_drill.rs.

use std::time::Duration;

const WAIT: Duration = Duration::from_millis(1200);

pub enum Step<'a> {
    Command(&'a str, Duration),
    Light([i32; 3], &'a str, Duration),
    Wait(Duration),
    Portals(i32),
    Where,
    Box([i32; 6]),
    Top(i32, i32),
    /// The held item used in the air with the eyes at (yaw, pitch).
    Throw(f32, f32, Duration),
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
        "!box" => Step::Box(std::array::from_fn(|i| int(i + 1).unwrap_or(0) as i32)),
        "!top" => Step::Top(int(1).unwrap_or(0) as i32, int(2).unwrap_or(0) as i32),
        "!throw" => Step::Throw(int(1).unwrap_or(0) as f32, int(2).unwrap_or(0) as f32, ms(3)),
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
        // Overworld portals made in a cleared void: none to stand on (the no-site platform), then
        // a floor under leaf litter and under snow three high (the site search's empty cells).
        "voids" => {
            for (nx, cover) in [(100, None), (150, Some("leaf_litter")), (200, Some(r#"snow_layer ["height"=3]"#))] {
                lines.extend(void_trip(nx, 78, cover));
            }
        }
        // No-site platforms across a section boundary on y as well (the floor at 79), where two of
        // the three sections share a bucket of eight: the sections' order.
        "collide" => {
            for nx in [240, 264] {
                lines.extend(void_trip(nx, 77, None));
            }
        }
        // 21 x 21 frames lit across four sections each, along x and along z: the sections' order.
        "sections" => {
            lines.push("replaceitem entity @s slot.weapon.mainhand 0 flint_and_steel".into());
            for k in 0..6 {
                let z = 28 + 32 * k;
                lines.push(format!("tp @s 17.5 -60 {}.5 #1000", z - 2));
                lines.push(format!("fill 19 -60 {z} 41 -38 {z} obsidian"));
                lines.push(format!("fill 20 -59 {z} 40 -39 {z} air"));
                lines.push(format!("!light 20 -60 {z} up 2000"));
                let x = 60 + 32 * k;
                lines.push(format!("tp @s {}.5 -60 19.5 #1000", x - 2));
                lines.push(format!("fill {x} -60 19 {x} -38 41 obsidian"));
                lines.push(format!("fill {x} -59 20 {x} -39 40 air"));
                lines.push(format!("!light {x} -60 20 up 2000"));
            }
        }
        _ => lines.extend(super::end::drill(name).or_else(|| super::eye::drill(name))?),
    }
    Some(lines)
}

/// Through a Nether portal at x `nx`, height `ny`, z 0 into an overworld cleared round where it
/// arrives, (8 nx + 15, ny + 1.62, 4): the floor taken out for 20 blocks round, then, with `cover`,
/// a stone floor at y 78 under a layer of it. The platform crosses sections on x.
fn void_trip(nx: i32, ny: i32, cover: Option<&str>) -> Vec<String> {
    let x = 8 * nx + 15;
    let mut lines = vec![format!("execute in overworld run tp @s {x}.5 -60 40.5 #6000")];
    for (z0, z1) in [(-16, -1), (0, 15), (16, 24)] {
        lines.push(format!("fill {} -64 {z0} {} -61 {z1} air", x - 20, x + 20));
    }
    if let Some(cover) = cover {
        lines.push(format!("fill {} 78 2 {} 78 7 stone", x - 5, x - 1));
        lines.push(format!("fill {} 79 2 {} 79 7 {cover}", x - 5, x - 1));
    }
    lines.push(format!("execute in nether run tp @s {nx}.5 90 -3.5 #9000"));
    lines.push(format!("fill {} {} -6 {} {} 2 air", nx - 3, ny - 2, nx + 4, ny + 6));
    lines.push(format!("fill {} {} -6 {} {} 2 netherrack", nx - 3, ny - 2, nx + 4, ny - 2));
    lines.extend(frame(nx, ny, 0));
    lines.extend([format!("!light {nx} {} 0 up 2000", ny - 1), "!wait 6000".into(), format!("tp @s {}.9 {ny} 0.5 #14000", nx + 1), "!where".into(), "!portals 24".into()]);
    lines.push(format!("!box {} 76 -16 {} 84 24", x - 20, x + 20));
    lines
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
