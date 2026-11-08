use super::*;

const HAND: Breaker = Breaker { by_entity: true, ..Breaker::UNAIDED };

/// What a block leaves when every throw comes up `thrown` (capped below the sides).
fn left_of(block: &str, properties: &str, by: Breaker, thrown: u32) -> Vec<(String, u8)> {
    of(&format!("minecraft:{block}"), properties, &by, &mut |sides| thrown.min(sides - 1)).into_iter().map(|item| (item.name, item.count)).collect()
}

fn left(block: &str, by: Breaker, thrown: u32) -> Vec<(String, u8)> {
    left_of(block, "", by, thrown)
}

fn named(item: &str, count: u8) -> Vec<(String, u8)> {
    vec![(format!("minecraft:{item}"), count)]
}

#[test]
fn a_block_leaves_itself_or_what_its_table_names() {
    assert_eq!([left("dirt", HAND, 0), left("stone", HAND, 0), left("oak_planks", HAND, 0)], [named("dirt", 1), named("cobblestone", 1), named("oak_planks", 1)]);
    assert_eq!([left("deepslate_diamond_ore", HAND, 0), left("lit_redstone_ore", HAND, u32::MAX), left("reeds", HAND, 0)], [named("diamond", 1), named("redstone", 5), named("sugar_cane", 1)]);
    assert_eq!([left("glass", HAND, 0), left("blue_ice", HAND, 0), left("mob_spawner", HAND, 0)], [vec![], vec![], vec![]]);
    assert_eq!([left("oak_slab", HAND, 0), left("oak_double_slab", HAND, 0)], [named("oak_slab", 1), named("oak_slab", 2)]);
    assert!(left("stone", Breaker { harvests: false, ..HAND }, 0).is_empty(), "nothing from a block the tool does not harvest");
    assert!(of("mod:lamp", "", &HAND, &mut |_| 0).is_empty(), "no item stands for it");
    let signs = [left("standing_sign", HAND, 0), left("wall_sign", HAND, 0), left("spruce_standing_sign", HAND, 0), left("darkoak_wall_sign", HAND, 0)];
    assert_eq!(signs, [named("oak_sign", 1), named("oak_sign", 1), named("spruce_sign", 1), named("dark_oak_sign", 1)]);
    assert_eq!([left("standing_banner", HAND, 0), left("wall_banner", HAND, 0)], [named("banner", 1), named("banner", 1)]);
    assert_eq!(left("lit_furnace", HAND, 0), named("furnace", 1));
}

#[test]
fn chance_enchantments_and_shears_change_what_is_left() {
    assert_eq!([left("gravel", HAND, 0), left("gravel", HAND, u32::MAX)], [named("flint", 1), named("gravel", 1)]);
    assert_eq!([left("short_grass", HAND, 0), left("short_grass", HAND, u32::MAX), left("oak_leaves", HAND, u32::MAX)], [named("wheat_seeds", 1), vec![], vec![]]);
    assert_eq!(left("oak_leaves", HAND, 0), [named("apple", 1), named("oak_sapling", 1), named("stick", 1)].concat());
    let silk = Breaker { silk_touch: true, ..HAND };
    assert_eq!([left("stone", silk, 0), left("glass", silk, 0), left("diamond_ore", silk, 0)], [named("stone", 1), named("glass", 1), named("diamond_ore", 1)]);
    assert_eq!(left("oak_leaves", Breaker { shears: true, ..HAND }, u32::MAX), named("oak_leaves", 1));
    let fortune = Breaker { fortune: 3, ..HAND };
    assert_eq!([left("diamond_ore", fortune, u32::MAX), left("diamond_ore", fortune, 0), left("stone", fortune, u32::MAX)], [named("diamond", 4), named("diamond", 1), named("cobblestone", 1)]);
}

#[test]
fn the_tables_states_are_read_off_bedrock_blocks() {
    assert_eq!(left_of("wheat", "growth=3", HAND, 0), named("wheat_seeds", 1));
    assert_eq!(left_of("wheat", "growth=7", HAND, 0), [named("wheat", 1), named("wheat_seeds", 3)].concat(), "every one of the crop's three throws succeeds");
    assert_eq!([left_of("beetroot", "growth=7", HAND, u32::MAX), left_of("beetroot", "growth=4", HAND, 0)], [named("beetroot", 1), named("beetroot_seeds", 1)], "a ripe crop may leave no seed");
    let carrots = |thrown| left_of("carrots", "growth=7", HAND, thrown).iter().map(|(_, count)| *count).sum::<u8>();
    assert_eq!([carrots(0), carrots(u32::MAX)], [4, 1], "bds-at1drops.txt: 1 to 4");
    assert_eq!(left_of("sweet_berry_bush", "growth=3", HAND, 0), named("sweet_berries", 2), "a bush's growth is its age");
    assert_eq!([left_of("snow_layer", "covered_bit=0,height=2", HAND, 0), left_of("snow_layer", "covered_bit=0,height=2", Breaker::UNAIDED, 0)], [named("snowball", 3), vec![]]);
}

/// research/captures/bds-n2pistons.txt: a piston broke a cluster into 2 shards and a pot into itself.
#[test]
fn a_cluster_leaves_two_shards_but_to_a_pickaxe_and_a_pot_leaves_itself() {
    let pickaxe = Breaker { pickaxe: true, ..HAND };
    assert_eq!([left("amethyst_cluster", Breaker::UNAIDED, 0), left("amethyst_cluster", HAND, 0), left("amethyst_cluster", pickaxe, 0)], [named("amethyst_shard", 2), named("amethyst_shard", 2), named("amethyst_shard", 4)]);
    assert_eq!(left_of("decorated_pot", "direction=0", Breaker::UNAIDED, 0), named("decorated_pot", 1));
}

#[test]
fn a_decorated_pot_broken_by_hand_leaves_itself() {
    assert_eq!(left_of("decorated_pot", "direction=0", HAND, 0), named("decorated_pot", 1));
}
