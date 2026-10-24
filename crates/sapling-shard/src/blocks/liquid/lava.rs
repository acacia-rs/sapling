//! Which blocks keep lava out, BDS's `isLavaBlocking` (docs/research/assumptions-checked.md 4): the
//! same rule as sapling-worldgen's `feature/liquid/flow.rs` `lava_blocking`, by name. The base
//! asks whether the material blocks motion or is solid (collision stands in for it); 46 types keep
//! lava out whatever their material and 48 let it in.

/// Whether the block `name` (with collision where `solid`) keeps lava out.
pub(crate) fn keeps_out(name: &str, solid: bool) -> bool {
    let name = name.strip_prefix("minecraft:").unwrap_or(name);
    let coral = name.contains("coral") && !name.ends_with("coral_block");
    let yields = coral
        || name.ends_with("rail")
        || name.starts_with("chorus")
        || matches!(name, "snow_layer" | "azalea" | "flowering_azalea" | "big_dripleaf" | "small_dripleaf_block" | "waterlily" | "powder_snow" | "shelf_mushroom");
    // Neither solid nor blocking motion, on the base rule.
    let yields = yields || name.ends_with("torch") || matches!(name, "web" | "lever" | "tripwire_hook" | "trip_wire" | "redstone_wire");
    let holds = name.contains("candle")
        || matches!(name, "glow_lichen" | "sculk_vein" | "bamboo" | "campfire" | "soul_campfire" | "ladder" | "scaffolding" | "frame" | "glow_frame" | "portal" | "end_portal" | "resin_clump");
    !yields && (holds || solid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lava_takes_rails_and_torches_but_not_ladders_or_candles() {
        let kept = |name: &str, solid: bool| keeps_out(name, solid);
        assert_eq!([kept("minecraft:rail", true), kept("minecraft:torch", false), kept("minecraft:snow_layer", true)], [false; 3]);
        assert_eq!([kept("minecraft:ladder", false), kept("minecraft:candle", false), kept("minecraft:stone", true)], [true; 3]);
        assert!(!kept("minecraft:short_grass", false), "no collision, no override: burnt");
    }
}
