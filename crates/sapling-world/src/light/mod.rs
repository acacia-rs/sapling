//! Sky light and block light for the server's own rules (growth, spawning, melting); never sent, as
//! the client lights what it draws itself, and never saved, as BDS saves none (docs/DESIGN.md
//! "Light"). A column is lit alone on the I/O thread that loads it (own.rs), joined to its loaded
//! neighbours when it arrives, and mended around written blocks, all in [`crate::World::relight`].

mod flood;
mod grid;
mod nibbles;
mod own;
#[cfg(test)]
mod tests;
mod world_light;

use acacia_world::BlockRegistry;

pub(crate) use nibbles::ColumnLight;
pub(crate) use own::light_column;
pub(crate) use world_light::Lights;

/// Open sky, and the brightest a block shines.
pub const FULL_LIGHT: u8 = 15;

/// What each block state does to light, by runtime id: emission << 4 | filter. From acacia-world's
/// `BlockState::light_emission` and `light_filter` (minecraft-data corrected by the wiki's Bedrock
/// table, not read from BDS).
pub(crate) struct Props(Box<[u8]>);

impl Props {
    pub fn of(registry: &BlockRegistry) -> Self {
        Self((0..registry.len() as u32).map(|id| registry.get(id).map_or(FULL_LIGHT, |state| state.light_emission.min(15) << 4 | state.light_filter.min(15))).collect())
    }

    /// A cell holding `block` with `liquid` in its second layer: the stronger emission and filter
    /// of the two, so waterlogged leaves stop what leaves stop.
    pub fn cell(&self, block: u32, liquid: u32, air: u32) -> u8 {
        // Custom states follow the vanilla ids and may be registered after the world was made.
        let custom = |id| sapling_leveldb::custom_state(id).map_or(FULL_LIGHT, |state| state.light_emission.min(15) << 4 | state.light_filter.min(15));
        let of = |id: u32| self.0.get(id as usize).copied().unwrap_or_else(|| custom(id));
        let first = of(block);
        if liquid == air {
            return first;
        }
        let second = of(liquid);
        emission(first).max(emission(second)) << 4 | filter(first).max(filter(second))
    }
}

pub(crate) const fn emission(props: u8) -> u8 {
    props >> 4
}

pub(crate) const fn filter(props: u8) -> u8 {
    props & 15
}

/// The light left after stepping into a cell with `filter`: one level and the filter, except that
/// full sky light going down loses only the filter (open air keeps the sky at 15 to the ground).
pub(crate) const fn step(level: u8, filter: u8, sky_down: bool) -> u8 {
    if sky_down && level == FULL_LIGHT { level.saturating_sub(filter) } else { level.saturating_sub(1 + filter) }
}
