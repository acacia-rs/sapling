//! The third wave's pillager drills (research/parity/MORE-BRIEF.md "Group 1", docs/research/
//! effects-raids.md): from how far a slowed pillager loads and shoots at the survival actor
//! (`pillrange`), its and the actor's moves shown. Places are absolute on the flat ground (y -60).

/// Packets shown: the moves, the arrows, the crossbow's sounds and flags.
const RANGE_SHOWN: &[&str] = &["AddEntity", "RemoveEntity", "MoveEntityDelta", "MoveEntityAbsolute", "MovePlayer", "LevelSoundEvent", "SetEntityData", "EntityEvent", "MobEquipment"];

/// The distances along x the pillager is held at, farthest first.
const DISTANCES: [f32; 8] = [12.0, 11.0, 10.0, 9.5, 9.0, 8.5, 8.0, 7.0];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "pillrange" => (pill_range(), RANGE_SHOWN),
        _ => return None,
    })
}

/// For each distance a lone pillager summoned 40 out, slowed, then held that far east of the
/// actor for 8 s; the actor is put back on its cell every second against the arrows' knockback.
fn pill_range() -> Vec<String> {
    let mut out: Vec<String> = [
        "kill @e[type=!player]", "gamerule domobspawning false", "gamerule dodaylightcycle false", "gamerule doweathercycle false", "weather clear", "time set 1000", "difficulty easy",
        "fill -8 -61 -8 44 -61 8 grass_block", "fill -8 -60 -8 44 -52 8 air", "tp CmdWatcher -4.5 -60 0.5", "gamemode creative CmdWatcher",
        "tp @s 0.5 -60 0.5", "gamemode survival @s", "clear @s", "effect @s resistance 1000 4 true", "effect @s saturation 1000 0 true", "",
    ]
    .map(String::from)
    .into();
    for far in DISTANCES {
        out.extend(["summon pillager 40.5 -60 0.5".to_owned(), "effect @e[type=pillager] slowness 1000 255 true".into()]);
        out.push(format!("tp @e[type=pillager] {} -60 0.5 facing 0.5 -59 0.5", 0.5 + far));
        for _ in 0..8 {
            out.push("tp @s 0.5 -60 0.5".into());
        }
        out.extend(["kill @e[type=pillager]".to_owned(), "kill @e[type=arrow]".into(), "kill @e[type=item]".into(), String::new()]);
    }
    out.extend(["effect @s clear", "gamemode creative @s", ""].map(String::from));
    out
}
