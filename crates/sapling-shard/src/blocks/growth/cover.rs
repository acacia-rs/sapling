//! What a random tick does by light to what covers the ground: grass and mycelium spreading to
//! open dirt and dying under a block, ice and snow melting by a lamp (docs/research/growth.md
//! "Light"). Of grass and mycelium only those under a block are kept track of, and of ice and
//! snow only those lit enough to melt: what a tick would leave as it is costs nothing.

use sapling_api::{Block, BlockPos, Face};

use super::super::{send, with_property};
use super::Picks;
use super::rules::{Change, How, is_air, is_lit, name_at, shown};

pub(super) const DIRT: &str = "minecraft:dirt";
const GRASS: &str = "minecraft:grass_block";
const MYCELIUM: &str = "minecraft:mycelium";
const ICE: &str = "minecraft:ice";
const SNOW: &str = "minecraft:snow_layer";
/// Of the 45 cells a grass block's random tick may pick, it tries this many.
const SPREAD_CELLS: u32 = 45;
const SPREAD_TRIES: u32 = 4;
/// Over dirt with less light than this mycelium does not spread, and mycelium under a block dies.
const DIM: u8 = 4;
/// Block light in its cell from which ice melts and snow loses a layer for each level more.
const MELTING_LIGHT: u8 = 12;

type Level = sapling_world::World;

/// Whether a random tick may change the block `name` here.
pub(super) fn is_ticked(name: &str) -> bool {
    matches!(name, DIRT | GRASS | MYCELIUM | ICE | SNOW)
}

/// Whether the block `name` at `pos` is worth its random ticks: dirt open to the air, grass and
/// mycelium that die, ice and snow by a lamp.
pub(super) fn is_kept(level: &Level, pos: BlockPos, name: &str) -> bool {
    match name {
        DIRT => is_air(level, Face::Up.neighbour(pos)),
        GRASS | MYCELIUM => dies(level, pos, name),
        _ => melting(level, pos) > 0,
    }
}

fn brightness(level: &Level, [x, y, z]: BlockPos, darken: u8) -> u8 {
    level.brightness_at(x, y, z, darken).unwrap_or(0)
}

/// Whether the grass or mycelium `name` at `pos` dies (`GrassBlockBase::_canBeGrass`,
/// `MyceliumBlock::randomTick`): grass under a block that stops light or under water, mycelium
/// under a solid block in the dark.
fn dies(level: &Level, pos: BlockPos, name: &str) -> bool {
    let over @ [x, y, z] = Face::Up.neighbour(pos);
    let registry = level.registry();
    if !level.in_height(y) {
        return false;
    }
    let states = [level.block(x, y, z), level.liquid(x, y, z)].map(|id| registry.get(id));
    match name {
        GRASS => states.iter().flatten().any(|state| state.light_filter >= sapling_world::FULL_LIGHT || state.is_liquid()),
        _ => brightness(level, over, 0) < DIM && states[0].is_some_and(|state| state.is_solid()),
    }
}

/// The block light in the cell of `pos`, or one less than beside it: sapling's light table has
/// ice filter 2, where BDS's `IceBlock` gives it none, and BDS's ice by a torch melted.
fn melting_light(level: &Level, pos: BlockPos) -> u8 {
    let block = |[x, y, z]: BlockPos| level.light_at(x, y, z).map_or(0, |(_, block)| block);
    Face::ALL.iter().map(|face| block(face.neighbour(pos)).saturating_sub(1)).fold(block(pos), u8::max)
}

/// How many layers the ice or snow at `pos` loses to block light (`TopSnowBlock::randomTick`: one
/// at 12 and one more for each level over, up to 8); ice melts at any.
fn melting(level: &Level, pos: BlockPos) -> u8 {
    (melting_light(level, pos) + 1).saturating_sub(MELTING_LIGHT).min(8)
}

/// The ice and snow the block at `at` may melt: its light reaches 12 in their cells.
pub(super) fn melted_by(level: &Level, [x, y, z]: BlockPos) -> Vec<BlockPos> {
    let registry = level.registry();
    let shines = registry.get(level.block(x, y, z)).map_or(0, |state| i32::from(state.light_emission));
    let reach = shines - i32::from(MELTING_LIGHT);
    if reach <= 0 {
        return Vec::new();
    }
    let cube = (-reach..=reach).flat_map(|dx| (-reach..=reach).flat_map(move |dy| (-reach..=reach).map(move |dz| [dx, dy, dz])));
    let near = cube.filter(|offset| offset.iter().map(|d| d.abs()).sum::<i32>() <= reach).map(|[dx, dy, dz]| [x + dx, y + dy, z + dz]);
    near.filter(|cell| name_at(level, *cell).is_some_and(|name| name == ICE || name == SNOW)).collect()
}

/// The blocks of `name` whose random tick may pick the dirt at `pos` and that may spread, light 9
/// over them: grass picks from one under to three over itself, mycelium two either way, and
/// mycelium reads its light with no night in it.
fn spreading_near(level: &Level, [x, y, z]: BlockPos, name: &str, darken: u8) -> u32 {
    let (rows, darken) = if name == GRASS { (-1..=3, darken) } else { (-2..=2, 0) };
    let cells = (-1..=1).flat_map(|dx| rows.clone().flat_map(move |dy| (-1..=1).map(move |dz| [x + dx, y + dy, z + dz])));
    cells.filter(|[x, y, z]| name_at(level, [*x, *y, *z]) == Some(name) && is_lit(level, [*x, y + 1, *z], darken)).count() as u32
}

/// What becomes of melting ice: water, or air where water boils.
fn melted_ice(level: &Level, pos: BlockPos) -> Option<Change> {
    if level.dimension() == sapling_world::NETHER {
        return Some(shown(pos, level.registry().air_id()));
    }
    send::resolve(level.registry(), &Block::new("minecraft:water")).map(|id| Change { pos, id, how: How::Melts })
}

/// Snow at `pos` with `id` that loses `layers`: air once it has no more.
fn melted_snow(level: &Level, pos: BlockPos, id: u32, layers: u8) -> Option<Change> {
    let registry = level.registry();
    let height = registry.get(id)?.property("height")?.parse::<u8>().ok()?;
    let left = if height + 1 > layers { with_property(registry, id, "height", &(height - layers).to_string())? } else { registry.air_id() };
    Some(shown(pos, left))
}

/// What a random tick makes of the block `name` at `pos`.
pub(super) fn ticked(level: &Level, pos: BlockPos, name: &str, darken: u8, picks: &mut Picks) -> Vec<Change> {
    let registry = level.registry();
    let to = |name: &str| send::resolve(registry, &Block::new(name));
    let over = Face::Up.neighbour(pos);
    let change = match name {
        // Seen from the dirt: each grass block near has its tries at it, then each mycelium,
        // which also needs some light over the dirt.
        DIRT if is_air(level, over) => {
            let spreads = |spreader: &&str| (*spreader == GRASS || brightness(level, over, 0) >= DIM) && picks.below(SPREAD_CELLS) < SPREAD_TRIES * spreading_near(level, pos, spreader, darken);
            [GRASS, MYCELIUM].into_iter().find(spreads).and_then(to).map(|id| shown(pos, id))
        }
        GRASS | MYCELIUM if dies(level, pos, name) => to(DIRT).map(|id| shown(pos, id)),
        ICE if melting(level, pos) > 0 => melted_ice(level, pos),
        SNOW => match melting(level, pos) {
            0 => None,
            layers => melted_snow(level, pos, level.block(pos[0], pos[1], pos[2]), layers),
        },
        _ => None,
    };
    change.into_iter().collect()
}
