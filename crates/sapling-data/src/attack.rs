//! What a hit with an item in hand costs a mob.
//! Every row is what BDS took from a cow (research/captures/bds-tools.txt).
// TODO: no enchantment, effect or critical hit.

const HAND: f32 = 1.0;
/// A sword of each material; an axe costs 1 less, a pickaxe or hoe 2, a shovel 3.
const SWORDS: &[(&str, f32)] = &[("wooden", 5.0), ("golden", 5.0), ("stone", 6.0), ("copper", 6.0), ("iron", 7.0), ("diamond", 8.0), ("netherite", 9.0)];
const TOOLS: &[(&str, f32)] = &[("sword", 0.0), ("axe", 1.0), ("pickaxe", 2.0), ("shovel", 3.0), ("hoe", 2.0)];
const OTHERS: &[(&str, f32)] = &[("trident", 9.0), ("mace", 6.0)];

/// The health a hit with the item `name` in hand takes; a bare hand's for `None`.
pub fn item_attack(name: Option<&str>) -> f32 {
    let Some(name) = name.map(|name| name.strip_prefix("minecraft:").unwrap_or(name)) else { return HAND };
    if let Some((_, harm)) = OTHERS.iter().find(|(other, _)| *other == name) {
        return *harm;
    }
    let tool = name.split_once('_').and_then(|(material, tool)| Some((SWORDS.iter().find(|(known, _)| *known == material)?.1, TOOLS.iter().find(|(known, _)| *known == tool)?.1)));
    tool.map_or(HAND, |(sword, less)| sword - less)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swords_cost_what_bds_took_and_other_things_a_hands_worth() {
        assert_eq!([None, Some("minecraft:wooden_sword"), Some("minecraft:diamond_sword")].map(item_attack), [1.0, 5.0, 8.0]);
        assert_eq!([Some("minecraft:iron_axe"), Some("minecraft:stone_pickaxe"), Some("minecraft:golden_hoe"), Some("minecraft:netherite_shovel"), Some("minecraft:mace"), Some("minecraft:stick")].map(item_attack), [6.0, 4.0, 3.0, 6.0, 6.0, 1.0]);
    }
}
