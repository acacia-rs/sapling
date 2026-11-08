//! The thirteenth wave's group BE drill (research/parity/THIRTEENTH-BRIEF.md): the order BDS tells mobs'
//! tick-end changes in, as slots are freed and taken. On the flat ground: six named sheep, an event on all;
//! `b` then `d` killed a tick apart and three new sheep summoned; two killed in one command and three more;
//! an event on all after each (`be3slots`).

use crate::villages::wait;

fn owned<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

fn summoned(out: &mut Vec<String>, sheep: &[(&str, &str)]) {
    for (name, at) in sheep {
        out.push(format!("summon sheep {at} 0 0 minecraft:entity_spawned {name}"));
    }
    out.extend(sheep.iter().map(|(name, _)| format!("effect @e[name={name}] slowness 1000 255 true")));
    out.push(String::new());
}

fn slots() -> Vec<String> {
    let mut out = owned(["kill @e[type=!player]", "gamerule domobspawning false", "gamerule dodaylightcycle false", "time set 1000", "tp @a 0.5 -60 0.5", "", "", ""]);
    summoned(&mut out, &[("a", "2.5 -60 0.5"), ("b", "6.5 -60 -4.5"), ("c", "-7.5 -60 5.5"), ("d", "12.5 -60 3.5"), ("e", "0.5 -60 2.5"), ("f", "-4.5 -60 -9.5")]);
    out.extend(owned(["event entity @e[type=sheep] wololo", "", "kill @e[name=b]", "kill @e[name=d]", "", "", ""]));
    summoned(&mut out, &[("g", "5.5 -60 8.5"), ("h", "-10.5 -60 -2.5"), ("i", "9.5 -60 -8.5")]);
    out.extend(owned(["event entity @e[type=sheep] minecraft:on_sheared", "", "kill @e[type=sheep,r=3]", "", "", ""]));
    summoned(&mut out, &[("j", "-2.5 -60 11.5"), ("k", "14.5 -60 -1.5"), ("l", "-12.5 -60 7.5")]);
    out.extend(owned(["event entity @e[type=sheep] minecraft:on_eat_block", "", "effect @e[type=sheep] speed 3 0 true", "", "kill @e[type=!player]", ""]));
    wait(&mut out, 2);
    out
}

const SHOWN: &[&str] = &["CommandOutput", "SetEntityData", "MobEffect", "AddEntity", "RemoveEntity", "EntityEvent"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "be3slots" => (slots(), SHOWN),
        _ => return None,
    })
}
