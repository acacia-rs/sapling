//! Items that place a block without being block items: a stack of them names no block (its
//! `block_runtime_id` is 0), and the game knows which block each one builds
//! (docs/research/block-rules.md, "Items whose block has another name";
//! docs/research/block-entities.md, section 1).

use sapling_api::Face;
use sapling_world::BlockRegistry;

use super::doors;

/// Items whose block carries their own name: every door, and these.
const SAME_NAME: &[&str] =
    &["minecraft:bed", "minecraft:hopper", "minecraft:cake", "minecraft:flower_pot", "minecraft:cauldron", "minecraft:brewing_stand", "minecraft:campfire", "minecraft:soul_campfire"];

/// Items whose block has another name: seeds and the like, and redstone's
/// (docs/research/standing-and-shapes.md, A1).
const RENAMED: &[(&str, &str)] = &[
    ("minecraft:wheat_seeds", "minecraft:wheat"),
    ("minecraft:carrot", "minecraft:carrots"),
    ("minecraft:potato", "minecraft:potatoes"),
    ("minecraft:beetroot_seeds", "minecraft:beetroot"),
    ("minecraft:torchflower_seeds", "minecraft:torchflower_crop"),
    ("minecraft:pitcher_pod", "minecraft:pitcher_crop"),
    ("minecraft:melon_seeds", "minecraft:melon_stem"),
    ("minecraft:pumpkin_seeds", "minecraft:pumpkin_stem"),
    ("minecraft:nether_wart", "minecraft:nether_wart"),
    ("minecraft:sweet_berries", "minecraft:sweet_berry_bush"),
    ("minecraft:cocoa_beans", "minecraft:cocoa"),
    ("minecraft:glow_berries", "minecraft:cave_vines"),
    ("minecraft:sugar_cane", "minecraft:reeds"),
    ("minecraft:redstone", "minecraft:redstone_wire"),
    ("minecraft:repeater", "minecraft:unpowered_repeater"),
    ("minecraft:comparator", "minecraft:unpowered_comparator"),
    ("minecraft:string", "minecraft:trip_wire"),
];

/// The block a sign or banner item builds on `face`: standing on a top, on the wall on a side,
/// none under a ceiling.
fn standing_or_wall(name: &str, face: Face) -> Option<String> {
    let (standing, wall) = match name {
        "minecraft:banner" => ("minecraft:standing_banner".to_owned(), "minecraft:wall_banner".to_owned()),
        "minecraft:oak_sign" => ("minecraft:standing_sign".to_owned(), "minecraft:wall_sign".to_owned()),
        _ => {
            let wood = name.strip_suffix("_sign").filter(|_| !name.ends_with("_hanging_sign"))?;
            let wood = if wood == "minecraft:dark_oak" { "minecraft:darkoak" } else { wood };
            (format!("{wood}_standing_sign"), format!("{wood}_wall_sign"))
        }
    };
    match face {
        Face::Up => Some(standing),
        Face::Down => None,
        _ => Some(wall),
    }
}

/// The block the item `name` places on `face`, in the state the item gives it before it is
/// turned.
pub(super) fn block_of(registry: &BlockRegistry, name: &str, face: Face) -> Option<u32> {
    let first = |block: &str| registry.states_of(block).next().map(|(id, _)| id);
    if name.ends_with("_hanging_sign") {
        // Under a ceiling it hangs, on a wall it sticks out; it does not stand.
        let hanging = match face {
            Face::Up => return None,
            Face::Down => "1",
            _ => "0",
        };
        return doors::with(registry, first(name)?, "hanging", hanging);
    }
    if let Some(block) = standing_or_wall(name, face) {
        return first(&block);
    }
    if let Some((_, block)) = RENAMED.iter().find(|(item, _)| *item == name) {
        return first(block);
    }
    let places = name.ends_with("_door") && !name.ends_with("trapdoor") || SAME_NAME.contains(&name);
    first(name).filter(|_| places)
}

/// The hanging sign `id`, about to hang under the block `above`, hung from one chain where that
/// block has no whole bottom face to take two, or its placer sneaks (Java `HangingSignItem`,
/// `CeilingHangingSignBlock`); any other block as it is.
pub(super) fn attached(registry: &BlockRegistry, id: u32, above: u32, sneaking: bool) -> u32 {
    let hangs = registry.get(id).is_some_and(|state| state.name.ends_with("_hanging_sign") && state.property("hanging") == Some("1"));
    let whole_bottom = registry.get(above).is_some_and(|state| state.boxes.iter().any(|shape| shape.min[1] <= 0.0 && shape.min[0] <= 0.0 && shape.max[0] >= 1.0 && shape.min[2] <= 0.0 && shape.max[2] >= 1.0));
    match hangs && (sneaking || !whole_bottom) {
        true => doors::with(registry, id, "attached_bit", "1").unwrap_or(id),
        false => id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn placed(item: &str, face: Face) -> Option<&'static str> {
        let registry = BlockRegistry::vanilla();
        assert!(sapling_data::is_item(item), "{item}");
        block_of(&registry, item, face).and_then(|id| registry.get(id)).map(|state| state.name)
    }

    #[test]
    fn doors_beds_and_hoppers_place_the_block_of_their_name() {
        let items = ["minecraft:wooden_door", "minecraft:spruce_door", "minecraft:iron_door", "minecraft:copper_door", "minecraft:bed", "minecraft:hopper", "minecraft:cake"];
        for item in items {
            assert_eq!(placed(item, Face::Up), Some(item), "{item}");
        }
        assert_eq!([placed("minecraft:diamond", Face::Up), placed("minecraft:stone", Face::Up), placed("minecraft:spruce_trapdoor", Face::Up)], [None; 3]);
    }

    #[test]
    fn signs_and_banners_stand_on_a_top_and_hang_on_a_side() {
        assert_eq!([placed("minecraft:oak_sign", Face::Up), placed("minecraft:oak_sign", Face::North)], [Some("minecraft:standing_sign"), Some("minecraft:wall_sign")]);
        assert_eq!(placed("minecraft:dark_oak_sign", Face::East), Some("minecraft:darkoak_wall_sign"));
        assert_eq!(placed("minecraft:spruce_sign", Face::Up), Some("minecraft:spruce_standing_sign"));
        assert_eq!([placed("minecraft:banner", Face::Up), placed("minecraft:banner", Face::West)], [Some("minecraft:standing_banner"), Some("minecraft:wall_banner")]);
        assert_eq!([placed("minecraft:oak_sign", Face::Down), placed("minecraft:oak_hanging_sign", Face::Up)], [None, None]);
        assert_eq!(placed("minecraft:oak_hanging_sign", Face::Down), Some("minecraft:oak_hanging_sign"));
        for item in ["minecraft:flower_pot", "minecraft:cauldron", "minecraft:brewing_stand", "minecraft:campfire", "minecraft:soul_campfire"] {
            assert_eq!(placed(item, Face::South), Some(item));
        }
        for (item, block) in RENAMED {
            assert_eq!(placed(item, Face::Up), Some(*block));
        }
    }

    #[test]
    fn a_hanging_sign_hangs_from_one_chain_under_what_is_not_a_whole_bottom() {
        let registry = BlockRegistry::vanilla();
        let sign = block_of(&registry, "minecraft:oak_hanging_sign", Face::Down).expect("a hanging sign");
        let first = |name: &str| registry.states_of(name).next().expect("a block").0;
        let chained = |above: &str, sneaking: bool| registry.get(attached(&registry, sign, first(above), sneaking)).and_then(|state| state.property("attached_bit"));
        assert_eq!([chained("minecraft:stone", false), chained("minecraft:oak_fence", false), chained("minecraft:stone", true)], [Some("0"), Some("1"), Some("1")]);
    }
}
