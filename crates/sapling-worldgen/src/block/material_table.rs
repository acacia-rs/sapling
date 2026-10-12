//! From BDS's block registry (scripts/data/block_types.tsv, column `blocksMotion`: the material's byte +3), listed by
//! scripts/dbg_blocks.py's dump; every other type's material blocks motion.

/// The block types whose material does not block motion.
pub(super) const OPEN_MATERIAL: &str =
    "acacia_sapling activator_rail air allium azure_bluet bamboo_sapling beetroot big_dripleaf birch_sapling \
     black_candle black_carpet blue_candle blue_carpet blue_orchid brown_candle brown_carpet brown_mushroom \
     bubble_column bush cactus_flower campfire candle carrots cave_vines cave_vines_body_with_berries \
     cave_vines_head_with_berries cherry_sapling client_request_placeholder_block closed_eyeblossom cocoa \
     colored_torch_blue colored_torch_green colored_torch_purple colored_torch_red conduit copper_torch cornflower \
     creeper_head crimson_fungus crimson_roots cyan_candle cyan_carpet dandelion dark_oak_sapling deadbush \
     decorated_pot detector_rail dragon_head end_gateway end_portal end_rod fern fire firefly_bush flower_pot \
     flowing_lava flowing_water frame frog_spawn glow_frame glow_lichen golden_dandelion golden_rail gray_candle \
     gray_carpet green_candle green_carpet hanging_roots jungle_sapling kelp ladder large_fern lava leaf_litter \
     lever light_block_0 light_block_1 light_block_10 light_block_11 light_block_12 light_block_13 light_block_14 \
     light_block_15 light_block_2 light_block_3 light_block_4 light_block_5 light_block_6 light_block_7 \
     light_block_8 light_block_9 light_blue_candle light_blue_carpet light_gray_candle light_gray_carpet lilac \
     lily_of_the_valley lime_candle lime_carpet magenta_candle magenta_carpet mangrove_propagule melon_stem \
     moss_carpet nether_sprouts nether_wart oak_sapling open_eyeblossom orange_candle orange_carpet orange_tulip \
     oxeye_daisy pale_hanging_moss pale_moss_carpet pale_oak_sapling peony piglin_head pink_candle pink_carpet \
     pink_petals pink_tulip pitcher_crop pitcher_plant player_head poplar_sapling poppy portal potatoes \
     powered_comparator powered_repeater pumpkin_stem purple_candle purple_carpet rail red_candle red_carpet \
     red_mushroom red_shrub red_tulip redstone_torch redstone_wire reeds resin_clump rose_bush scaffolding \
     sculk_vein sea_pickle seagrass short_dry_grass short_grass skeleton_skull small_dripleaf_block snow_layer \
     soul_campfire soul_fire soul_torch spore_blossom spruce_sapling structure_void sunflower sweet_berry_bush \
     tall_dry_grass tall_grass torch torchflower torchflower_crop trip_wire tripwire_hook twisting_vines \
     underwater_torch unlit_redstone_torch unpowered_comparator unpowered_repeater vine warped_fungus warped_roots \
     water web weeping_vines wheat white_candle white_carpet white_tulip wildflowers wither_rose \
     wither_skeleton_skull yellow_candle yellow_carpet zombie_head";
