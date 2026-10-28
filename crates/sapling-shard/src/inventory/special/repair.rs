//! Item repair (`…-000000000001`, matches 141b56330, assemble 141b55170): two of a damageable
//! item, one each, make one new of it worn `d1 + d2 - lasts - lasts / 20` (at least 0). Only the
//! curses (binding, vanishing) of either are kept, each once; every other field is left behind.

use sapling_data::CreativeItem;
use sapling_data::enchant::{all, kind_of, with_all};
use sapling_data::gear::{damage, gear, with_damage};

use super::{Grid, Made};

/// Two of an item put together mend a twentieth more.
const BONUS_SHARE: i32 = 20;

pub(super) fn repaired(grid: &Grid) -> Option<Made> {
    let filled = grid.filled();
    let [first, second] = filled.as_slice() else { return None };
    let lasts = i32::from(gear(&first.name)?.lasts);
    if first.name != second.name || first.count != 1 || second.count != 1 || lasts == 0 {
        return None;
    }
    let worn = (damage(first.tag.as_ref()) + damage(second.tag.as_ref()) - lasts - lasts / BONUS_SHARE).max(0);
    let mut curses: Vec<(i16, u8)> = Vec::new();
    for curse in [first, second].into_iter().flat_map(|stack| all(stack.tag.as_ref())).filter(|(id, _)| kind_of(*id).is_some_and(|kind| kind.curse)) {
        if !curses.contains(&curse) {
            curses.push(curse);
        }
    }
    let name = sapling_data::static_item_name(&first.name)?;
    Some(vec![(CreativeItem { name, damage: 0, block: None, tag: with_all(with_damage(None, worn), &curses) }, 1)])
}

#[cfg(test)]
mod tests {
    use super::*;
    use acacia_proto::nbt::Value;
    use sapling_world::ItemStack;

    fn sword(worn: i32, enchants: &[(i16, u8)]) -> ItemStack {
        let tag = with_damage(with_all(Some(Value::Compound(Vec::new())), enchants), worn);
        ItemStack { name: "minecraft:iron_sword".into(), count: 1, damage: 0, block: None, tag }
    }

    fn grid(stacks: &[ItemStack]) -> Grid<'_> {
        Grid { cells: stacks.iter().map(Some).collect(), width: 2, map: None }
    }

    #[test]
    fn two_worn_swords_make_one_with_their_uses_left_and_a_twentieth_more() {
        // An iron sword lasts 250: 100 + 120 left, plus 12, leaves 18 worn.
        let made = repaired(&grid(&[sword(150, &[(9, 3), (28, 1)]), sword(130, &[(28, 1)])])).expect("repairs");
        let (item, count) = &made[0];
        assert_eq!((item.name, count, damage(item.tag.as_ref())), ("minecraft:iron_sword", &1, 18));
        assert_eq!(all(item.tag.as_ref()), [(28, 1)], "only the curse, once");
        assert_eq!(damage(repaired(&grid(&[sword(10, &[]), sword(5, &[])])).expect("repairs")[0].0.tag.as_ref()), 0);
    }

    #[test]
    fn only_two_of_one_damageable_item_repair() {
        let other = ItemStack { name: "minecraft:golden_sword".into(), ..sword(5, &[]) };
        assert!(repaired(&grid(&[sword(5, &[]), other])).is_none());
        assert!(repaired(&grid(&[sword(5, &[]), sword(5, &[]), sword(5, &[])])).is_none());
        let paper = ItemStack { name: "minecraft:paper".into(), count: 1, damage: 0, block: None, tag: None };
        assert!(repaired(&grid(&[paper.clone(), paper])).is_none());
    }
}
