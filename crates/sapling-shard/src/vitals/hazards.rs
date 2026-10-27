//! What else hurts a player where it stands: a cactus it touches, a ripe sweet berry bush it moves
//! through, the void under the world. Each cost BDS's player 1, 1 and 4 a hit, a hit each time
//! its guard let one in (research/captures/bds-thorny.txt, bds-void.txt), with the cause
//! `contact`, `contact` and `void`.

use sapling_world::World as Level;

use crate::body::Body;
use crate::creatures::touches;

const CACTUS: &[&str] = &["minecraft:cactus"];
const BUSH: &str = "minecraft:sweet_berry_bush";
const GROWTH: &str = "growth";
pub(super) const PRICKS: f32 = 1.0;
pub(super) const VOID_HARM: f32 = 4.0;
/// The void hurts this far under the bottom of the world (BDS `OutOfWorldSystem` 142f1f2f0).
const VOID_DEPTH: f32 = 16.0;
/// A player in a bush that moved no more than this across this tick is not pricked (BDS 1471f0d30).
const STIRS: f32 = 0.003;

/// Whether the player's box touches a cactus, and whether it is in a bush with berries on it.
pub(super) fn thorns(level: &Level, body: &Body) -> [bool; 2] {
    [touches(level, body, CACTUS), in_ripe_bush(level, body)]
}

/// Whether the player is pricked by the thorns it is among, having moved `moved` across.
pub(super) fn pricks([cactus, bush]: [bool; 2], moved: [f32; 2]) -> bool {
    cactus || bush && (moved[0].abs() > STIRS || moved[1].abs() > STIRS)
}

fn in_ripe_bush(level: &Level, body: &Body) -> bool {
    let registry = level.registry();
    let (low, high) = body.bounds();
    let cells = |axis: usize| low[axis].floor() as i32..=high[axis].floor() as i32;
    let ripe = |[x, y, z]: [i32; 3]| {
        let state = level.in_height(y).then(|| registry.get(level.block(x, y, z))).flatten();
        state.is_some_and(|state| state.name == BUSH && state.property(GROWTH).is_some_and(|growth| growth != "0"))
    };
    cells(0).any(|x| cells(1).any(|y| cells(2).any(|z| ripe([x, y, z]))))
}

/// Whether feet at height `y` are in the void.
pub(super) fn in_void(level: &Level, y: f32) -> bool {
    y < level.heights().start as f32 - VOID_DEPTH
}
