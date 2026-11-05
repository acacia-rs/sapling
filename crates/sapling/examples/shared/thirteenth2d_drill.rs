//! The thirteenth wave's group D drills (research/parity/THIRTEENTH-BRIEF.md), seed 20261018 on BDS: a
//! gateway set by command on a platform in the End (BDS refuses the name) and the worldgen gateway at
//! (1953, 66, 181) visited and stepped into (`d2gate1`); then, on the same world after a restart
//! (`KEEP_WORLD=1`), the worldgen one stepped into again once its column is loaded anew (`d2gate2`).

const GATE_SHOWN: &[&str] = &["MovePlayer", "BlockEntityData", "UpdateBlock", "BlockEvent", "CommandOutput", "ChangeDimension"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "d2gate1" => (gates_set(), GATE_SHOWN),
        "d2gate2" => (gates_loaded(), GATE_SHOWN),
        _ => return None,
    })
}

fn owned<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

fn wait(lines: &mut Vec<String>, seconds: usize) {
    lines.extend(std::iter::repeat_n(String::new(), seconds));
}

/// The command's gateway and the worldgen one, each stepped into.
fn gates_set() -> Vec<String> {
    let mut lines = owned(["kill @e[type=!player]", "gamerule domobspawning false", "execute in the_end run tp @s 43.5 102 43.5", "", "", "fill 40 100 40 47 100 47 obsidian"]);
    wait(&mut lines, 3);
    lines.extend(owned(["tp @s 43.5 101 43.5", "setblock 45 101 41 end_gateway", "", "", "tp @s 45.5 101 41.5", "", "tp @s 43.5 101 43.5", "", ""]));
    lines.extend(owned(["tp @s 1956.5 66 181.5"]));
    wait(&mut lines, 5);
    lines.extend(owned(["tp @s 1953.5 66 181.5", "", "", "tp @s 43.5 101 43.5"]));
    wait(&mut lines, 3);
    lines.extend(owned(["execute in overworld run tp @s 0.5 -26 0.5", "", ""]));
    lines
}

/// After a restart: the worldgen gateway stepped into once its column is loaded again.
fn gates_loaded() -> Vec<String> {
    let mut lines = owned(["execute in the_end run tp @s 1956.5 66 181.5"]);
    wait(&mut lines, 6);
    lines.extend(owned(["tp @s 1953.5 66 181.5", "", "", ""]));
    lines.extend(owned(["execute in overworld run tp @s 0.5 -26 0.5", "", ""]));
    lines
}
