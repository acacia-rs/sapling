//! The dragon probe's drills. A line is a command, watched for 1.2 s or for the milliseconds after
//! a final ` #`; or one of the probe's own steps:
//! `!wait ms`, `!where`, `!box x0 y0 z0 x1 y1 z1`, `!top x z`, `!spikes` (the top of every spike),
//! `!entities` (what the actor's client tracks), `!use x y z face [ms]` (the held item on a block;
//! `!light` the same, as shared/end_drill.rs writes it),
//! `!hit kind [ms]` (the nearest of the kind, hit), `!crystal [ms]` (beside the nearest crystal on a
//! spike, then hit it), `!enter [ms]` (onto the nearest end portal cell), `!summon [ms]` (an end
//! crystal on each side of the exit portal), `!perch [ms]` (until the dragon sits, at most that
//! long), `!near dx dy dz [ms]` (to the dragon's feet and that much off).

use std::time::Duration;

const WAIT: Duration = Duration::from_millis(1200);

pub enum Step<'a> {
    Command(&'a str, Duration),
    Wait(Duration),
    Where,
    Box([i32; 6]),
    Top(i32, i32),
    Spikes,
    Entities,
    Use([i32; 3], &'a str, Duration),
    Hit(&'a str, Duration),
    Crystal(Duration),
    Enter(Duration),
    Summon(Duration),
    Perch(Duration),
    Near([i32; 3], Duration),
    Swat(Duration),
}

pub fn step(line: &str) -> Step<'_> {
    let words: Vec<&str> = line.split(' ').collect();
    let int = |at: usize| words.get(at).and_then(|word| word.parse::<i64>().ok());
    let ms = |at: usize| int(at).map_or(WAIT, |ms| Duration::from_millis(ms as u64));
    match words[0] {
        "!wait" => Step::Wait(ms(1)),
        "!where" => Step::Where,
        "!box" => Step::Box(std::array::from_fn(|i| int(i + 1).unwrap_or(0) as i32)),
        "!top" => Step::Top(int(1).unwrap_or(0) as i32, int(2).unwrap_or(0) as i32),
        "!spikes" => Step::Spikes,
        "!entities" => Step::Entities,
        "!use" | "!light" => Step::Use([0, 1, 2].map(|i| int(i + 1).unwrap_or(0) as i32), words.get(4).copied().unwrap_or("up"), ms(5)),
        "!hit" => Step::Hit(words.get(1).copied().unwrap_or("ender_dragon"), ms(2)),
        "!crystal" => Step::Crystal(ms(1)),
        "!enter" => Step::Enter(ms(1)),
        "!summon" => Step::Summon(ms(1)),
        "!perch" => Step::Perch(ms(1)),
        "!swat" => Step::Swat(ms(1)),
        "!near" => Step::Near([0, 1, 2].map(|i| int(i + 1).unwrap_or(0) as i32), ms(4)),
        _ => match line.rsplit_once(" #") {
            Some((command, wait)) => Step::Command(command, wait.parse().map_or(WAIT, Duration::from_millis)),
            None => Step::Command(line, WAIT),
        },
    }
}

fn owned(lines: &[&str]) -> Vec<String> {
    lines.iter().map(|line| line.to_string()).collect()
}

/// Into the End over the exit portal's pillar, where the fight starts, and a look round.
const ARRIVE: &[&str] = &["gamemode creative", "gamerule domobspawning false", "execute in the_end run tp @s 0.5 90 0.5 #15000", "!where", "!entities", "!spikes", "!box -4 58 -4 4 67 4"];
/// Off the pillar, where neither the egg nor the portal comes.
const ASIDE: &str = "tp @s 8.5 70 0.5 #3000";

/// The lines of the drill `name`.
pub fn drill(name: &str) -> Option<Vec<String>> {
    let mut lines = owned(ARRIVE);
    match name {
        // The fight as it starts and goes on by itself.
        "dragon-watch" => lines.extend(owned(&["!wait 60000", "!entities", "!spikes"])),
        // Crystals hit, the dragon hurt and killed, the way home.
        "dragon-fight" => {
            lines.extend(owned(&["!wait 10000", "!crystal 4000", "!entities", "!crystal 4000", "!entities", ASIDE]));
            lines.extend(owned(&["damage @e[type=ender_dragon] 10 #3000", "!entities", "damage @e[type=ender_dragon] 30 entity_explosion #3000", "!entities"]));
            lines.extend(owned(&["kill @e[type=ender_dragon] #30000", "!entities", "!box -4 58 -4 4 67 4", "!spikes", "!wait 5000", "!enter 20000", "!where", "!entities"]));
            lines.extend(owned(&["execute in the_end run tp @s 8.5 70 0.5 #10000", "!enter 20000", "!where"]));
        }
        // A kill, then a summoning on the exit portal, and that dragon's kill.
        "dragon-again" => {
            lines.extend(owned(&[ASIDE, "kill @e[type=ender_dragon] #30000", "!box -4 58 -4 4 67 4", "replaceitem entity @s slot.weapon.mainhand 0 end_crystal 4", "tp @s 0.5 70 6.5 #3000"]));
            lines.extend(owned(&["!summon 40000", "!entities", "!spikes", "!wait 20000", "!entities", ASIDE, "kill @e[type=ender_dragon] #30000", "!entities", "!box -4 58 -4 4 67 4"]));
        }
        // A survival player on the island by the portal, for the dragon to flame, strafe, charge
        // and hit; kept alive by health boost and regeneration, so each harm shows whole.
        "dragon-survival" => {
            lines.extend(owned(&["tp @s 6.5 63 0.5 #2000", "effect @s health_boost 9999 9 true", "effect @s regeneration 9999 4 true", "gamemode survival", "!wait 150000", "!entities", "gamemode creative"]));
        }
        // The first gateway's line out (0, z), where the exit search walks, and round the exits BDS found.
        "dragon-exits" => {
            lines.extend(owned(&["tp @s 0.5 110 -1100.5 #12000"]));
            lines.extend((0..14).map(|i| format!("!top 0 {}", -1024 - 16 * i)));
            lines.extend(["!top -13 -1123", "!top -13 -1124", "!top 7 -1092", "!top 0 -1100", "!top 8 -1120"].map(String::from));
        }
        // Experience orbs on a floor in the air, a survival player at its middle: single orbs 4 to 24
        // blocks off, then ten in one cell 30 off (merging), watched to rest.
        "orbs" => {
            lines = owned(&["gamemode creative", "gamerule domobspawning false", "fill -40 99 -12 40 99 12 stone", "tp @s 0.5 100 0.5 #3000", "gamemode survival"]);
            for (x, z) in [(4.5, 0.5), (-7.5, 0.5), (0.5, 9.5), (12.5, 0.5), (0.5, -11.5), (-16.5, 0.5), (24.5, 0.5)] {
                lines.push(format!("summon xp_orb {x} 101 {z} #8000"));
            }
            lines.extend((0..10).map(|_| "summon xp_orb -30.5 101 0.5 #50".to_string()));
            lines.extend(owned(&["!wait 30000", "!entities", "gamemode creative"]));
        }
        // The first gateway's line out (x 0, z -1024 to -1296) cleared of end stone, so its exit search
        // finds no spawn and builds an island at y 75; the island and the exit looked at.
        "exit-island" => {
            lines.push("tp @s 0.5 110 -1135.5 #12000".into());
            lines.extend((0..10).map(|i| format!("fill -20 0 {} 20 100 {} air", -1100 - 7 * i, -1106 - 7 * i)));
            lines.push("tp @s 0.5 110 -1265.5 #12000".into());
            lines.extend((0..14).map(|i| format!("!top 0 {}", -1072 - 16 * i)));
            lines.extend((0..5).map(|i| format!("fill -16 0 {} 16 100 {} air", -1260 - 9 * i, -1268 - 9 * i)));
            lines.extend(owned(&[ASIDE, "kill @e[type=ender_dragon] #40000", "tp @s 0.5 100 -1279.5 #12000"]));
            lines.extend((0..14).map(|i| format!("!top 0 {}", -1072 - 16 * i)));
            lines.push("!box -9 64 -1289 9 86 -1271".into());
            lines.extend(owned(&["tp @s 0.5 110 -1135.5 #12000", "!top 0 -1120", "!top -6 -1123", "!top -13 -1123", "!top 0 -1136", "!box -8 64 -1128 8 76 -1112"]));
        }
        // A second island: a kill and a summoning, the second gateway's exit (585, 71, -850) cleared
        // round, the second kill, and that island looked at.
        "exit-island2" => {
            lines.extend(owned(&[ASIDE, "kill @e[type=ender_dragon] #30000", "replaceitem entity @s slot.weapon.mainhand 0 end_crystal 4", "tp @s 0.5 70 6.5 #3000", "!summon 40000", "!wait 20000"]));
            lines.push("tp @s 585.5 110 -845.5 #12000".into());
            lines.extend((0..18).map(|i| format!("fill 550 0 {} 620 100 {} air", -882 + 4 * i, -879 + 4 * i)));
            lines.extend(owned(&[ASIDE, "kill @e[type=ender_dragon] #40000", "tp @s 585.5 100 -845.5 #12000", "!box 570 64 -860 600 76 -830"]));
        }
        // The survival drill, the actor hitting each fireball that comes within 3 blocks of its eyes.
        "fireball-hit" => {
            lines.extend(owned(&["tp @s 6.5 63 0.5 #2000", "effect @s health_boost 9999 9 true", "effect @s regeneration 9999 4 true", "gamemode survival", "!swat 150000", "!entities", "gamemode creative"]));
        }
        // Into the End through an end portal, the dragon killed, home through the exit portal.
        "dragon-portal" => {
            // In the air over the default world's ground, on a floor of its own.
            lines = owned(&["gamemode creative", "gamerule domobspawning false", "fill 5 99 3 16 99 16 stone", "tp @s 11.5 100 6.5 #3000"]);
            lines.extend(crate::end::ring(10, 100, 10, true));
            lines.extend(owned(&["tp @s 11.5 100 11.5 #15000", "!where", "tp @s 8.5 70 0.5 #10000", "kill @e[type=ender_dragon] #14000", "!enter 20000", "!where"]));
        }
        // The fight of the world the last drill left (KEEP_WORLD=1), as the restarted server resumes it.
        "dragon-resume" => lines.extend(owned(&["!wait 20000", "!entities"])),
        // Home through the exit portal in survival, twice.
        "dragon-home" => {
            lines.extend(owned(&[ASIDE, "kill @e[type=ender_dragon] #14000", "gamerule falldamage false", "gamemode survival", "!enter 20000", "!where"]));
            lines.extend(owned(&["gamemode creative", "execute in the_end run tp @s 8.5 70 0.5 #10000", "gamemode survival", "!enter 20000", "!where", "gamemode creative"]));
        }
        // Hits on the perched dragon from the podium's floor, on each side of the pillar.
        "dragon-hits" => {
            for off in ["2 -3 0", "0 -3 2", "-2 -3 0", "0 -3 -2"] {
                lines.extend(["!perch 60000".to_string(), format!("!near {off} 300"), "!hit ender_dragon 300".into(), "!hit ender_dragon 600".into(), "!entities".into(), ASIDE.into()]);
            }
        }
        _ => return None,
    }
    Some(lines)
}
