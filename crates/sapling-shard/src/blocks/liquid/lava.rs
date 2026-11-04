//! Which blocks keep lava out, BDS's `isLavaBlocking` (docs/research/assumptions-checked.md 4, drilled in
//! fire.md "What lava takes"): the base (141a28450) asks whether the block's material blocks motion or is
//! solid (`sapling_worldgen::material_blocks_motion`, BDS's registry walk); 46 types keep lava out whatever
//! their material and 48 let it in. sapling-worldgen's `feature/liquid/flow.rs` `lava_blocking` is the
//! same rule with collision for the material, which worldgen's replays were judged with.

/// Open-material types whose material is solid: the base keeps lava out of them.
const SOLID_OPEN: [&str; 4] = ["conduit", "decorated_pot", "structure_void", "client_request_placeholder_block"];

/// Whether the block `name` keeps lava out.
pub(crate) fn keeps_out(name: &str) -> bool {
    let name = name.strip_prefix("minecraft:").unwrap_or(name);
    let coral = name.contains("coral") && !name.ends_with("coral_block");
    let yields = coral
        || name.ends_with("rail")
        || name.starts_with("chorus")
        || matches!(name, "snow_layer" | "azalea" | "flowering_azalea" | "big_dripleaf" | "small_dripleaf_block" | "waterlily" | "powder_snow" | "shelf_mushroom");
    // Sea pickles held lava back in the lavaeats drill, though their material is open.
    let holds = name.contains("candle")
        || matches!(
            name,
            "glow_lichen" | "sculk_vein" | "bamboo" | "campfire" | "soul_campfire" | "ladder" | "scaffolding" | "frame" | "glow_frame" | "portal" | "end_portal" | "resin_clump" | "sea_pickle"
        );
    !yields && (holds || sapling_worldgen::material_blocks_motion(name) || SOLID_OPEN.contains(&name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lava_takes_what_bds_let_it_take_in_the_lavaeats_drill() {
        let taken = ["white_carpet", "moss_carpet", "unpowered_repeater", "skeleton_skull", "flower_pot", "end_rod", "snow_layer", "torch", "rail", "web", "short_grass", "lever", "redstone_wire", "leaf_litter", "pink_petals"];
        let kept = [
            "stone_button", "stone_pressure_plate", "standing_sign", "standing_banner", "sea_pickle", "decorated_pot", "conduit", "candle", "ladder", "oak_fence", "glass_pane", "iron_bars", "cake", "heavy_core",
            "pointed_dripstone", "small_amethyst_bud", "lantern", "lightning_rod", "wooden_pressure_plate", "trapdoor", "brewing_stand",
        ];
        assert_eq!(taken.iter().filter(|name| keeps_out(&format!("minecraft:{name}"))).collect::<Vec<_>>(), Vec::<&&str>::new());
        assert_eq!(kept.iter().filter(|name| !keeps_out(&format!("minecraft:{name}"))).collect::<Vec<_>>(), Vec::<&&str>::new());
    }
}
