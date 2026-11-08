//! The fourteenth wave's group D drills (research/parity/FOURTEENTH-BRIEF.md): mob projectiles, on the
//! flat ground at midnight, the actor survival under resistance 255. `d3ghast`: a held ghast's fireball
//! at a held zombie in its way, then three struck by the actor looking at the ghast, up and aside.
//! `d3shulker`: a shulker's bullet at a held zombie in its way, two struck, the shulker's idle peeks,
//! its floor taken away, and a second one hit three times with a sword. `d3breeze`: a held breeze's
//! charge at a held zombie in its way beside another, two struck, then charges bursting on the actor
//! among doors, a trapdoor, a fence gate, a button and levers 1.5 to 5.6 off. `d3wither`: blue and
//! black skulls struck. `d3slow`: a zombie by the survival watcher under slowness 255, 6 (its speed 0)
//! and 4, a spider and a vindicator under 255. `!d3strike` (`act`) hits a projectile in flight.

use std::time::Duration;

use crate::villages::wait;

fn owned<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

fn start(difficulty: &str) -> Vec<String> {
    let mut out = owned(["kill @e[type=!player]", "gamerule domobspawning false", "gamerule dodaylightcycle false", "gamerule falldamage false", "time set 18000"]);
    out.extend([format!("difficulty {difficulty}"), "gamemode creative @a".into(), "effect @a clear".into(), "fill -8 -60 -8 8 -55 16 air".into(), "tp CmdWatcher 0.5 -60 -14.5".into(), "tp @s 0.5 -60 -12.5".into(), String::new()]);
    out
}

fn held(name: &str) -> String {
    format!("effect @e[name={name}] slowness 1000 255 true")
}

/// The actor survival and unhurt where it stands, at 0.5 -60 0.5 looking along +z.
fn warded() -> Vec<String> {
    owned(["gamemode survival @s", "effect @s resistance 1000 255 true", "effect @s fire_resistance 1000 0 true", "tp @s 0.5 -60 0.5 0 0"])
}

fn end() -> Vec<String> {
    owned(["kill @e[type=!player]", "gamemode creative @s", "effect @s clear", "tp @s 0.5 -60 -12.5", ""])
}

fn ghast() -> Vec<String> {
    let mut out = start("easy");
    out.extend(warded());
    out.extend(["summon zombie z 0.5 -60 6.5".into(), held("z"), "summon ghast g 0.5 -60 14.5".into(), held("g")]);
    wait(&mut out, 6);
    out.extend(owned(["kill @e[name=z]", "tp @s 0.5 -60 0.5 0 0", "!d3strike fireball 0 2 14 12"]));
    wait(&mut out, 4);
    out.extend(owned(["tp @s 0.5 -60 0.5 0 0", "!d3strike fireball 0 8 3 12"]));
    wait(&mut out, 4);
    out.extend(owned(["tp @s 0.5 -60 0.5 0 0", "!d3strike fireball 4 1.62 0 12"]));
    wait(&mut out, 4);
    out.extend(end());
    out
}

/// `d3ghast` again with the zombie in the way, just before the actor, and the struck fireballs sent
/// back at the ghast.
fn ghast_again() -> Vec<String> {
    let mut out = start("easy");
    out.extend(warded());
    out.extend(["summon ghast g 0.5 -60 14.5".into(), held("g"), "summon zombie z 0.5 -60 2.2".into(), held("z")]);
    wait(&mut out, 5);
    out.push("tp @e[name=z] 0.5 -60 2.2".into());
    wait(&mut out, 4);
    out.extend(owned(["kill @e[name=z]", "tp @s 0.5 -60 0.5 0 0", "!d3strike fireball @ghast 0 2 0 12"]));
    wait(&mut out, 6);
    out.extend(owned(["tp @s 0.5 -60 0.5 0 0", "!d3strike fireball @ghast 0 2 0 12"]));
    wait(&mut out, 6);
    out.extend(end());
    out
}

fn shulker() -> Vec<String> {
    let mut out = start("easy");
    out.extend(warded());
    out.extend(["summon zombie z 0.5 -60 3.5".into(), held("z"), "summon shulker s 0.5 -60 7.5".into()]);
    wait(&mut out, 8);
    out.extend(owned(["kill @e[name=z]", "tp @s 0.5 -60 0.5 0 0", "!d3strike shulker_bullet 0 1.62 5 10"]));
    wait(&mut out, 2);
    out.extend(owned(["tp @s 0.5 -60 0.5 0 0", "!d3strike shulker_bullet 3 1.62 3 10"]));
    wait(&mut out, 2);
    out.extend(owned(["gamemode creative @s", "tp @s 0.5 -60 -12.5"]));
    wait(&mut out, 12);
    out.push("setblock 0 -61 7 air".into());
    wait(&mut out, 4);
    out.extend(owned(["kill @e[type=shulker]", "setblock 0 -61 7 grass", "summon shulker t 0.5 -60 7.5", "replaceitem entity @s slot.weapon.mainhand 0 netherite_sword", "gamemode survival @s", "tp @s 0.5 -60 5.5 0 0", ""]));
    for _ in 0..3 {
        out.extend(owned(["!hit shulker", ""]));
    }
    wait(&mut out, 3);
    out.extend(owned(["replaceitem entity @s slot.weapon.mainhand 0 air"]));
    out.extend(end());
    out
}

fn breeze() -> Vec<String> {
    let mut out = start("easy");
    out.extend(warded());
    out.extend(["summon zombie z 0.5 -60 4.5".into(), held("z"), "summon zombie y 2.5 -60 4.5".into(), held("y"), "summon breeze b 0.5 -60 8.5".into(), held("b")]);
    wait(&mut out, 5);
    out.extend(owned(["kill @e[type=zombie]", "tp @s 0.5 -60 0.5 0 0", "!d3strike breeze_wind_charge_projectile 0 1.9 8 8"]));
    wait(&mut out, 2);
    out.extend(owned(["tp @s 0.5 -60 0.5 0 0", "!d3strike breeze_wind_charge_projectile 5 1.62 0 8"]));
    wait(&mut out, 2);
    out.extend(owned([
        "setblock 2 -60 1 wooden_door [\"direction\"=0]",
        "setblock 2 -59 1 wooden_door [\"direction\"=0,\"upper_block_bit\"=true]",
        "setblock -2 -60 1 iron_door [\"direction\"=0]",
        "setblock -2 -59 1 iron_door [\"direction\"=0,\"upper_block_bit\"=true]",
        "setblock 1 -60 2 trapdoor",
        "setblock -1 -60 2 fence_gate",
        "setblock 1 -60 0 wooden_button [\"facing_direction\"=1]",
        "setblock -1 -60 0 lever [\"lever_direction\"=\"up_north_south\"]",
        "setblock 3 -60 0 lever [\"lever_direction\"=\"up_north_south\"]",
        "setblock -4 -60 0 lever [\"lever_direction\"=\"up_north_south\"]",
        "setblock 6 -60 0 lever [\"lever_direction\"=\"up_north_south\"]",
    ]));
    for _ in 0..3 {
        out.push("tp @s 0.5 -60 0.5 0 0".into());
        wait(&mut out, 3);
    }
    out.extend(end());
    out
}

fn wither() -> Vec<String> {
    let mut out = start("normal");
    out.extend(warded());
    out.push("summon wither w 0.5 -55 8.5".into());
    wait(&mut out, 12);
    out.extend(owned(["tp @s 0.5 -60 0.5 0 0", "!d3strike wither_skull_dangerous 0 1.62 5 20"]));
    wait(&mut out, 3);
    out.extend(owned(["tp @s 0.5 -60 0.5 0 0", "!d3strike wither_skull 0 1.62 5 10"]));
    wait(&mut out, 3);
    out.extend(owned(["tp @s 0.5 -60 0.5 0 0", "!d3strike wither_skull_dangerous 5 1.62 0 20"]));
    wait(&mut out, 3);
    out.extend(end());
    out
}

fn slow() -> Vec<String> {
    let mut out = start("normal");
    out.extend(owned(["tp @s 0.5 -60 -20.5", "gamemode survival CmdWatcher", "tp CmdWatcher 0.5 -60 0.5 0 0"]));
    for (kind, level) in [("zombie", 255), ("zombie", 6), ("zombie", 4), ("spider", 255), ("vindicator", 255)] {
        out.extend([format!("summon {kind} m 0.5 -60 7.5"), format!("effect @e[name=m] slowness 1000 {level} true"), "tp @e[name=m] 0.5 -60 1.6".into(), "effect CmdWatcher instant_health 1 4".into(), "tp CmdWatcher 0.5 -60 0.5 0 0".into()]);
        wait(&mut out, 4);
        out.extend(owned(["kill @e[name=m]", "effect CmdWatcher instant_health 1 4", ""]));
    }
    out.extend(owned(["kill @e[type=!player]", "tp CmdWatcher 0.5 -60 -14.5", "gamemode creative CmdWatcher", ""]));
    out
}

/// Runs the actor's own ticks for `ticks`.
async fn run(actor: &mut acacia_bot::Bot, ticks: u32) {
    let end = tokio::time::Instant::now() + crate::probe::clock::scaled(Duration::from_millis(50 * u64::from(ticks)));
    while let Ok(Some(_)) = tokio::time::timeout_at(end, actor.next()).await {}
}

/// The actor looks at its feet plus `offset`, or at the newest `at` (a kind) plus it.
fn look(actor: &mut acacia_bot::Bot, at: Option<&str>, [dx, dy, dz]: [f32; 3]) {
    let Some([x, y, z]) = actor.movement().and_then(|movement| movement.position()) else { return };
    let base = match at {
        Some(kind) => match actor.state().entities.iter().filter(|entity| entity.kind == kind).max_by_key(|entity| entity.runtime_id).map(|entity| [entity.position.x, entity.position.y, entity.position.z]) {
            Some(position) => position,
            None => return,
        },
        None => [x, y, z],
    };
    if let Some(controls) = actor.controls() {
        controls.look_at([x, y + 1.62, z], [base[0] + dx, base[1] + dy, base[2] + dz]);
    }
}

/// Does `!d3strike <kind> [@<kind>] dx dy dz secs`: the actor looks at its feet (or the newest
/// `@<kind>`) plus the offset and, within `secs`, hits the first `<kind>` that comes after the
/// line as soon as it is in the bot's reach.
pub async fn act(actor: &mut acacia_bot::Bot, line: &str) -> bool {
    let Some(words) = line.strip_prefix("!d3strike ") else { return false };
    let mut words: Vec<&str> = words.split(' ').collect();
    let kind = format!("minecraft:{}", words.remove(0));
    let at = words.first().and_then(|word| word.strip_prefix('@')).map(|kind| format!("minecraft:{kind}"));
    let numbers: Vec<f32> = words.iter().filter_map(|word| word.parse().ok()).collect();
    let &[dx, dy, dz, secs] = &numbers[..] else { return true };
    look(actor, at.as_deref(), [dx, dy, dz]);
    let before: Vec<u64> = actor.state().entities.iter().filter(|entity| entity.kind == kind).map(|entity| entity.runtime_id).collect();
    let end = tokio::time::Instant::now() + crate::probe::clock::scaled(Duration::from_secs_f32(secs));
    while tokio::time::Instant::now() < end {
        run(actor, 1).await;
        look(actor, at.as_deref(), [dx, dy, dz]);
        let newest = actor.state().entities.iter().filter(|entity| entity.kind == kind && !before.contains(&entity.runtime_id)).map(|entity| entity.runtime_id).min();
        if let Some(id) = newest {
            if actor.click_entity_aimed(id, true).is_ok() {
                println!("## struck {id}");
                return true;
            }
        }
    }
    println!("## struck none");
    true
}

const SHOWN: &[&str] = &[
    "CommandOutput", "AddEntity", "RemoveEntity", "MoveEntityDelta", "MoveEntityAbsolute", "SetEntityData", "EntityEvent", "SetEntityMotion", "LevelSoundEvent", "LevelEvent", "UpdateAttributes", "MobEffect",
    "UpdateBlock", "UpdateSubchunkBlocks", "MovePlayer", "Animate",
];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "d3ghast" => (ghast(), SHOWN),
        "d3ghast2" => (ghast_again(), SHOWN),
        "d3shulker" => (shulker(), SHOWN),
        "d3breeze" => (breeze(), SHOWN),
        "d3wither" => (wither(), SHOWN),
        "d3slow" => (slow(), SHOWN),
        _ => return None,
    })
}
