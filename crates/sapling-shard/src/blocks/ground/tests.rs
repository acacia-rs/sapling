use super::*;

#[test]
fn every_block_named_is_in_the_registry() {
    let registry = BlockRegistry::vanilla();
    let lists = [CROPS, VEGETATION, BUSHES, CANE_GROUND, LIT_FROM_BELOW, DIODES, RAILS, VINES, BAMBOO_GROUND, LOW_CENTRE, &[REEDS, "minecraft:farmland", "minecraft:soul_sand", "minecraft:frosted_ice"]];
    for block in lists.concat() {
        assert!(registry.states_of(block).next().is_some(), "{block}");
    }
}

fn on(under: &str, block: &str) -> bool {
    let mut level = World::void();
    let (under, block) = (level.block_id(under).unwrap(), level.block_id(block).unwrap());
    level.set_block(0, 64, 0, under);
    may_stand(&level, [0, 65, 0], block)
}

/// research/captures/bds-p2place.txt, bds-p2fall.txt.
#[test]
fn a_hopper_holds_rails_dust_and_repeaters_and_flowers_keep_to_soil() {
    let held = ["minecraft:rail", "minecraft:redstone_wire", "minecraft:unpowered_repeater"].map(|block| on("minecraft:hopper", block));
    assert_eq!(held, [true; 3]);
    assert_eq!([on("minecraft:stone", "minecraft:poppy"), on("minecraft:stone", "minecraft:short_grass"), on("minecraft:grass_block", "minecraft:poppy")], [false, false, true]);
}

/// research/captures/bds-p3ground.txt: dust wants centre and edge, a rail edge, a diode any.
#[test]
fn dust_rails_and_diodes_ask_their_own_kind_of_top() {
    let tops = ["minecraft:oak_fence", "minecraft:cauldron", "minecraft:composter", "minecraft:iron_bars", "minecraft:oak_leaves"];
    let held = |block| tops.map(|top| on(top, block));
    assert_eq!(held("minecraft:redstone_wire"), [false; 5]);
    assert_eq!(held("minecraft:rail"), [false, true, true, false, false]);
    assert_eq!(held("minecraft:unpowered_repeater"), [true, true, true, true, false]);
    assert_eq!([on("minecraft:air", "minecraft:white_carpet"), on("minecraft:oak_fence", "minecraft:white_carpet")], [false, true]);
}

/// research/captures/bds-u3plants.txt: which of eleven blocks hold each plant, standing on it
/// (`hangs` false) or hanging under it.
#[test]
fn nether_vines_kelp_and_dripstone_ask_their_own_support() {
    const SUPPORTERS: [(&str, &str, &str); 11] = [
        ("minecraft:smooth_stone_slab", "minecraft:vertical_half", "top"),
        ("minecraft:smooth_stone_slab", "minecraft:vertical_half", "bottom"),
        ("minecraft:oak_leaves", "", ""),
        ("minecraft:glass", "", ""),
        ("minecraft:oak_fence", "", ""),
        ("minecraft:cobblestone_wall", "", ""),
        ("minecraft:cauldron", "", ""),
        ("minecraft:composter", "", ""),
        ("minecraft:oak_stairs", "upside_down_bit", "1"),
        ("minecraft:hopper", "", ""),
        ("minecraft:iron_bars", "", ""),
    ];
    let held = |plant: &str, hangs: bool| {
        SUPPORTERS.map(|(name, key, value)| {
            let mut level = World::void();
            let mut support = level.block_id(name).unwrap();
            if !key.is_empty() {
                support = super::super::with_property(level.registry(), support, key, value).unwrap();
            }
            let mut plant = level.block_id(plant).unwrap();
            plant = super::super::with_property(level.registry(), plant, "hanging", "0").unwrap_or(plant);
            level.set_block(0, 64, 0, support);
            may_stand(&level, [0, if hangs { 63 } else { 65 }, 0], plant)
        })
    };
    let (t, f) = (true, false);
    assert_eq!(held(TWISTING, false), [t, f, t, t, f, f, f, f, f, f, f]);
    assert_eq!(held(KELP, false), [t, f, f, f, f, f, f, f, f, f, f]);
    assert_eq!(held(DRIPSTONE, false), [t, f, f, t, f, f, f, f, t, t, f]);
    assert_eq!(held(WEEPING, true), [f, t, f, t, f, f, f, t, f, f, f]);
}
