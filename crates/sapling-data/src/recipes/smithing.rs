//! The smithing table's recipes, from the `CraftingData` clients are sent: a transform (a
//! netherite upgrade) turns the base into the recipe's result, a trim marks armour with the
//! template's pattern in the addition's material (`TrimData`). The result keeps the base's tag:
//! its wear, enchantments and name (docs/research/workstations.md, "Smithing table").

use std::collections::HashMap;
use std::sync::LazyLock;

use acacia_proto::nbt::Value;
use acacia_proto::packets::{CraftingData, TrimData};

use super::{Ingredient, ingredient};
use crate::gear::{Made, with_field};
use crate::items::{Stack, described};
use crate::templates::{CRAFTING_DATA, TRIM_DATA, decode};

enum Smithing {
    /// Template, base and addition, and the item the base becomes.
    Transform([Ingredient; 3], &'static str),
    /// Template, base and addition; the trim comes from the template and the addition.
    Trim([Ingredient; 3]),
}

static SMITHING: LazyLock<HashMap<u32, Smithing>> = LazyLock::new(|| {
    let data = decode::<CraftingData>(CRAFTING_DATA);
    let cells = |cells: [&_; 3]| -> Option<[Ingredient; 3]> {
        let [a, b, c] = cells.map(|cell| ingredient(cell).map(|(taken, _)| taken));
        Some([a?, b?, c?])
    };
    let transforms = data.smithing_transform_recipes.into_iter().filter_map(|recipe| {
        let result = described(recipe.result)?.name;
        Some((recipe.network_id, Smithing::Transform(cells([&recipe.template, &recipe.base, &recipe.addition])?, result)))
    });
    let trims = data.smithing_trim_recipes.into_iter().filter_map(|recipe| Some((recipe.network_id, Smithing::Trim(cells([&recipe.template, &recipe.input, &recipe.addition])?))));
    transforms.chain(trims).collect()
});

/// Trim patterns by their template item, and materials by their item.
static TRIMS: LazyLock<(HashMap<String, String>, HashMap<String, String>)> = LazyLock::new(|| {
    let data = decode::<TrimData>(TRIM_DATA);
    let patterns = data.patterns.into_iter().map(|pattern| (pattern.item_name, pattern.pattern)).collect();
    let materials = data.materials.into_iter().map(|material| (material.item_name, material.material)).collect();
    (patterns, materials)
});

/// What the smithing recipe `network_id` makes of these three, if they are what it takes.
pub fn smithed(network_id: u32, template: &Stack<'_>, base: &Stack<'_>, addition: &Stack<'_>) -> Option<Made> {
    let accepts = |cells: &[Ingredient; 3]| cells.iter().zip([template, base, addition]).all(|(cell, stack)| cell.accepts(stack.name, stack.damage));
    let tag = base.tag.cloned();
    match SMITHING.get(&network_id)? {
        Smithing::Transform(cells, result) if accepts(cells) => Some(Made { name: (*result).to_owned(), tag }),
        Smithing::Trim(cells) if accepts(cells) => {
            let (patterns, materials) = &*TRIMS;
            let (pattern, material) = (patterns.get(template.name)?, materials.get(addition.name)?);
            let trim = Value::Compound(vec![("Material".into(), Value::String(material.as_str().into())), ("Pattern".into(), Value::String(pattern.as_str().into()))]);
            Some(Made { name: base.name.to_owned(), tag: with_field(tag, "Trim", Some(trim)) })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(name: &str) -> Stack<'_> {
        Stack { name, count: 1, damage: 0, block: None, tag: None }
    }

    fn id(transform: bool) -> u32 {
        *SMITHING.iter().find(|(_, smithing)| matches!(smithing, Smithing::Transform(..)) == transform).expect("a recipe").0
    }

    #[test]
    fn a_netherite_upgrade_keeps_the_base_and_a_trim_marks_it() {
        let upgrade = (plain("minecraft:netherite_upgrade_smithing_template"), plain("minecraft:netherite_ingot"));
        let worn = Value::Compound(vec![("Damage".into(), Value::Int(9))]);
        let sword = Stack { tag: Some(&worn), ..plain("minecraft:diamond_sword") };
        let made = SMITHING.keys().find_map(|id| smithed(*id, &upgrade.0, &sword, &upgrade.1)).expect("an upgrade");
        assert_eq!(made, Made { name: "minecraft:netherite_sword".into(), tag: Some(worn) });
        assert!(smithed(id(true), &upgrade.0, &plain("minecraft:stone"), &upgrade.1).is_none());

        let trimmed = smithed(id(false), &plain("minecraft:coast_armor_trim_smithing_template"), &plain("minecraft:iron_chestplate"), &plain("minecraft:emerald")).expect("a trim");
        let trim = trimmed.tag.as_ref().and_then(|tag| tag.get("Trim")).expect("its trim");
        assert_eq!((trim.get("Pattern"), trim.get("Material")), (Some(&Value::String("coast".into())), Some(&Value::String("emerald".into()))));
        assert!(smithed(id(false), &plain("minecraft:coast_armor_trim_smithing_template"), &plain("minecraft:stone"), &plain("minecraft:emerald")).is_none());
    }
}
