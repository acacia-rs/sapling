//! What the blocks a mob touches or stands on do to it. Fire burns as BDS's cows were burnt
//! (research/captures/bds-burn2.txt): soul fire and a lit soul campfire 2 a hit, fire, a lit campfire
//! and lava 1 (lava's own hurt besides: harm.rs); magma 1 to one standing on it. Hay, slime and a bed
//! soften a fall onto them (bds-harm2.txt).

use sapling_world::World as Level;

use crate::body::Body;

const MAGMA: &str = "minecraft:magma";

/// What a hit of the fire the mob at `body` touches costs, if it touches any, and whether it sets
/// the mob alight: a campfire does not.
pub(super) fn fire_harm(level: &Level, body: &Body) -> Option<(f32, bool)> {
    const EDGE: f32 = 1e-4;
    let (low, high) = body.bounds();
    let cells = |axis: usize| low[axis].floor() as i32..=(high[axis] - EDGE).floor() as i32;
    let registry = level.registry();
    let harm = |x: i32, y: i32, z: i32| -> Option<(f32, bool)> {
        let state = registry.get(level.block(x, y, z))?;
        let lit = !matches!(state.property("extinguished"), Some("1" | "true"));
        match state.name {
            "minecraft:soul_fire" => Some((2.0, true)),
            "minecraft:soul_campfire" if lit => Some((2.0, false)),
            "minecraft:campfire" if lit => Some((1.0, false)),
            "minecraft:fire" | "minecraft:lava" | "minecraft:flowing_lava" => Some((1.0, true)),
            _ => registry.get(level.liquid(x, y, z)).filter(|liquid| liquid.name.ends_with("lava")).map(|_| (1.0, true)),
        }
    };
    let within = |y: i32| level.in_height(y);
    cells(0)
        .flat_map(|x| cells(1).filter(|y| within(*y)).flat_map(move |y| cells(2).map(move |z| [x, y, z])))
        .filter(|[x, _, z]| level.is_loaded(sapling_world::ChunkPos::of_block(*x, *z)))
        .filter_map(|[x, y, z]| harm(x, y, z))
        .reduce(|a, b| (a.0.max(b.0), a.1 || b.1))
}

/// Whether the mob at `body`, on the ground, stands on magma.
pub(super) fn on_magma(level: &Level, body: &Body) -> bool {
    under(level, body.at) == Some(MAGMA)
}

/// What a landing with its feet at `feet` counts of the fall, by the block under them ([`softens`]).
pub(crate) fn softening(level: &Level, feet: [f32; 3]) -> f32 {
    under(level, feet).map_or(1.0, softens)
}

/// What a landing on the block `name` counts of the fall's height: 8 blocks cost a cow 5 on stone and
/// none on hay or slime, 8.44 onto a bed cost it 1 (bds-harm2.txt; a player on hay the same). The
/// bed's share is fitted to that one drop: half, as the Java game's, would cost 2.
pub(super) fn softens(name: &str) -> f32 {
    match name {
        "minecraft:hay_block" => 0.2,
        "minecraft:bed" => 0.4,
        "minecraft:slime" => 0.0,
        _ => 1.0,
    }
}

/// The block under feet at `feet`; a bed's top is 0.5625 up its cell, so the cell is 0.2 below them.
fn under(level: &Level, [x, y, z]: [f32; 3]) -> Option<&'static str> {
    let [x, y, z] = [x.floor() as i32, (y - 0.2).floor() as i32, z.floor() as i32];
    (level.in_height(y) && level.is_loaded(sapling_world::ChunkPos::of_block(x, z))).then(|| level.registry().get(level.block(x, y, z)).map(|state| state.name)).flatten()
}
