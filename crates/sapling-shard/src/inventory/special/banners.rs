//! A banner copied (`b5c5d105-…`, assemble 142c6a1e0) and a banner put on a shield
//! (`…-00c8`, assemble 141b57e60). A patterned banner and a blank one of its colour make two of
//! the patterned one. A shield and a banner make the shield with the banner's colour as `Base`
//! and its `Patterns`.

use acacia_proto::nbt::Value;
use sapling_data::CreativeItem;

use super::{Grid, Made, with_fields};

const BANNER: &str = "minecraft:banner";
const SHIELD: &str = "minecraft:shield";
const PATTERNS: &str = "Patterns";
const BASE: &str = "Base";
const COPIES: u8 = 2;

fn patterns(tag: Option<&Value>) -> Option<&Value> {
    tag?.get(PATTERNS).filter(|patterns| matches!(patterns, Value::List(list) if !list.items.is_empty()))
}

pub(super) fn duplicated(grid: &Grid) -> Option<Made> {
    let [a, b] = <[_; 2]>::try_from(grid.filled()).ok()?;
    let (patterned, blank) = if patterns(a.tag.as_ref()).is_some() { (a, b) } else { (b, a) };
    let fits = [a, b].iter().all(|stack| stack.name == BANNER) && a.damage == b.damage && patterns(patterned.tag.as_ref()).is_some() && patterns(blank.tag.as_ref()).is_none();
    fits.then(|| vec![(CreativeItem { name: BANNER, damage: patterned.damage, block: None, tag: patterned.tag.clone() }, COPIES)])
}

pub(super) fn shield(grid: &Grid) -> Option<Made> {
    let (shield, banner) = (grid.only(SHIELD)?, grid.only(BANNER)?);
    if grid.filled().len() != 2 || patterns(shield.tag.as_ref()).is_some() {
        return None;
    }
    let mut added = vec![(BASE, Value::Int(banner.damage.into()))];
    added.extend(patterns(banner.tag.as_ref()).map(|patterns| (PATTERNS, patterns.clone())));
    let tag = with_fields(shield.tag.as_ref(), added);
    Some(vec![(CreativeItem { name: SHIELD, damage: shield.damage, block: None, tag: Some(tag) }, 1)])
}

#[cfg(test)]
mod tests {
    use super::*;
    use acacia_proto::nbt::List;
    use sapling_world::ItemStack;

    fn stack(name: &str, damage: i16, tag: Option<Value>) -> ItemStack {
        ItemStack { name: name.into(), count: 1, damage, block: None, tag }
    }

    fn grid(stacks: &[ItemStack]) -> Grid<'_> {
        Grid { cells: stacks.iter().map(Some).collect(), width: 2 }
    }

    fn patterned() -> Option<Value> {
        let layer = Value::Compound(vec![("Color".into(), Value::Int(1)), ("Pattern".into(), Value::String("bs".into()))]);
        Some(Value::Compound(vec![(PATTERNS.into(), Value::List(List { tag: 10, items: vec![layer] }))]))
    }

    #[test]
    fn a_patterned_banner_copies_onto_a_blank_one_of_its_colour() {
        let made = duplicated(&grid(&[stack(BANNER, 4, None), stack(BANNER, 4, patterned())])).expect("copies");
        assert_eq!((made[0].1, made[0].0.damage, &made[0].0.tag), (2, 4, &patterned()));
        assert!(duplicated(&grid(&[stack(BANNER, 3, None), stack(BANNER, 4, patterned())])).is_none(), "another colour");
        assert!(duplicated(&grid(&[stack(BANNER, 4, patterned()), stack(BANNER, 4, patterned())])).is_none());
    }

    #[test]
    fn a_shield_takes_the_banners_colour_and_patterns() {
        let made = shield(&grid(&[stack(SHIELD, 7, None), stack(BANNER, 4, patterned())])).expect("a shield");
        let tag = made[0].0.tag.as_ref().unwrap();
        assert_eq!((tag.get(BASE), tag.get(PATTERNS)), (Some(&Value::Int(4)), patterned().as_ref().and_then(|tag| tag.get(PATTERNS))));
        assert_eq!((made[0].0.name, made[0].0.damage), (SHIELD, 7));
        let decorated = ItemStack { name: SHIELD.into(), count: 1, damage: 0, block: None, tag: made[0].0.tag.clone() };
        assert!(shield(&grid(&[decorated, stack(BANNER, 4, None)])).is_none(), "already decorated");
    }
}
