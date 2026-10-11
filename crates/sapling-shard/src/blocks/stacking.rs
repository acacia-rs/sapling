//! Things that go into one cell several at a time: put onto its like, a candle, sea pickle, snow
//! layer, petal, wildflower, leaf litter or turtle egg makes the block there one more, and a
//! slab a double slab (docs/research/block-uses.md, "Stacking").
// TODO: a top slab takes none from below.

use acacia_proto::types::Vec3f;
use sapling_api::{BlockPos, Face, GameMode};
use sapling_world::BlockRegistry;

use super::{Request, Site, level, send, with_property};
use crate::inventory;

const EGGS: [&str; 4] = ["one_egg", "two_egg", "three_egg", "four_egg"];

/// The property that counts what is in the block `name`, and its last value; with whether BDS
/// told the new one by `item.use.on` from the cell over it (else by the block's `place`).
fn counted(name: &str) -> Option<(&'static str, u8, bool)> {
    Some(match name.strip_prefix("minecraft:")? {
        candle if candle.ends_with("candle") => ("candles", 3, true),
        "sea_pickle" => ("cluster_count", 3, true),
        "snow_layer" => ("height", 7, false),
        "pink_petals" | "wildflowers" | "leaf_litter" => ("growth", 3, false),
        _ => return None,
    })
}

/// The block `id` with one more of itself in it; `None` where it takes none, or none on `face`.
fn one_more(registry: &BlockRegistry, id: u32, face: Face) -> Option<(u32, bool)> {
    let state = registry.get(id)?;
    let get = |key: &str| state.properties.split(',').filter_map(|pair| pair.split_once('=')).find(|(name, _)| *name == key).map(|(_, value)| value);
    if let Some((key, last, by_use)) = counted(state.name) {
        let now: u8 = get(key)?.parse().ok()?;
        return Some((with_property(registry, id, key, &(now + 1).to_string()).filter(|_| now < last)?, by_use));
    }
    if state.name == "minecraft:turtle_egg" {
        let now = EGGS.iter().position(|eggs| Some(*eggs) == get("turtle_egg_count"))?;
        return Some((with_property(registry, id, "turtle_egg_count", EGGS.get(now + 1)?)?, false));
    }
    let lower = state.name.ends_with("_slab") && !state.name.ends_with("double_slab") && get("minecraft:vertical_half") == Some("bottom");
    let double = registry.find(&state.name.replacen("_slab", "_double_slab", 1), state.properties).filter(|_| lower && face == Face::Up)?;
    Some((double, true))
}

impl Request<'_> {
    /// A click on `face` of the block at `pos` holding `held`: where the block is `held`'s own and
    /// takes one more, it does. False for anything else.
    pub(super) fn stack(&mut self, site: &Site, pos: BlockPos, face: Face, held: &str) -> bool {
        let Some(level) = level(self.world, site.layer) else { return false };
        let (registry, id) = (level.registry(), level.block(pos[0], pos[1], pos[2]));
        let Some(state) = registry.get(id).filter(|state| state.name == held) else { return false };
        let Some((more, by_use)) = one_more(registry, id, face) else { return false };
        let is_slab = state.name.ends_with("_slab");
        // Candles and pickles were heard from the cell over them, a slab from its own.
        let [x, y, z] = pos.map(|c| c as f32 + 0.5);
        let heard = match (by_use, is_slab) {
            (true, _) => send::noise("item.use.on", Vec3f { x, y: if is_slab { y } else { y + 1.0 }, z }, registry.network_hash(more) as i32),
            (false, _) => send::sound("place", registry, pos, more),
        };
        send::set(self.world, site.layer, pos, more);
        send::to_viewers(self.world, site.layer, pos, &[heard]);
        if site.mode != GameMode::Creative {
            inventory::change(self.world, self.entity, |inventory| inventory.take(inventory::Place::Main(inventory.selected.into()), 1).is_ok());
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn more(block: &str, properties: &str, face: Face) -> Option<String> {
        let registry = BlockRegistry::vanilla();
        let id = registry.find(&format!("minecraft:{block}"), properties).or_else(|| registry.states_of(&format!("minecraft:{block}")).next().map(|(id, _)| id))?;
        let state = registry.get(one_more(&registry, id, face)?.0)?;
        Some(format!("{} {}", state.name.replace("minecraft:", ""), state.properties))
    }

    // research/captures/bds-stacks.txt
    #[test]
    fn what_goes_several_to_a_cell_takes_one_more_until_it_is_full() {
        assert_eq!(more("candle", "candles=0,lit=0", Face::Up).as_deref(), Some("candle candles=1,lit=0"));
        assert_eq!([more("candle", "candles=3,lit=0", Face::Up), more("stone", "", Face::Up)], [None, None]);
        assert_eq!(more("snow_layer", "covered_bit=0,height=2", Face::Up).as_deref(), Some("snow_layer covered_bit=0,height=3"));
        assert!(more("turtle_egg", "cracked_state=no_cracks,turtle_egg_count=one_egg", Face::Up).is_some_and(|eggs| eggs.contains("two_egg")));
        assert!(more("turtle_egg", "cracked_state=no_cracks,turtle_egg_count=four_egg", Face::Up).is_none());
        assert!(more("oak_slab", "minecraft:vertical_half=bottom", Face::Up).is_some_and(|double| double.starts_with("oak_double_slab")));
        assert!(more("oak_slab", "minecraft:vertical_half=bottom", Face::North).is_none() && more("oak_slab", "minecraft:vertical_half=top", Face::Up).is_none());
    }
}
