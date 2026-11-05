//! The thirteenth wave's group W drills (research/parity/THIRTEENTH-BRIEF.md). On a default world of seed
//! 20261003 (sapling `SAPLING_DRILL_WORLD=generated SAPLING_DRILL_SEED=20261003`, BDS with server-side
//! generation): `w3despawn`, natural mobs by the swamp hut, then the players 80 east, out of the tick
//! region of most of them though their chunks stay loaded, counted each 5 s for 60 s.

#[path = "thirteenth3w_world.rs"]
mod world;

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "w3despawn" => (world::despawn(), &["CommandOutput", "AddEntity", "RemoveEntity"]),
        _ => return None,
    })
}
