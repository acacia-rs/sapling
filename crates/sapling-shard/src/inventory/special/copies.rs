//! Copying a written book (`d1ca6b84-…`, assemble 142c6d3a0) and a filled map (`85939755-…`)
//! in the grid. A written book and writable books make that many copies, a generation on, and
//! the book itself back as a second result; a copy of a copy (generation 2) is not copied. A
//! filled map and empty maps make that many and one more of the map.

use acacia_proto::nbt::Value;
use sapling_data::CreativeItem;
use sapling_world::ItemStack;

use super::{Grid, Made, with_fields};
use crate::maps::FILLED;

const WRITTEN: &str = "minecraft:written_book";
const WRITABLE: &str = "minecraft:writable_book";
const EMPTY_MAP: &str = "minecraft:empty_map";
const GENERATION: &str = "generation";
const LAST_COPIED: i32 = 1;

/// How many of the grid's items are `name` when all but `one` of them are, else `None`.
fn copies_of(grid: &Grid, one: &str, name: &str) -> Option<(ItemStack, u8)> {
    let original = grid.only(one)?.clone();
    let blanks = grid.filled().iter().filter(|stack| stack.name == name).count();
    (blanks > 0 && blanks + 1 == grid.filled().len()).then_some((original, blanks as u8))
}

fn made(name: &'static str, stack: &ItemStack, tag: Option<Value>, count: u8) -> (CreativeItem, u8) {
    (CreativeItem { name, damage: stack.damage, block: None, tag }, count)
}

pub(super) fn book(grid: &Grid) -> Option<Made> {
    let (original, blanks) = copies_of(grid, WRITTEN, WRITABLE)?;
    let generation = match original.tag.as_ref().and_then(|tag| tag.get(GENERATION)) {
        Some(Value::Int(generation)) => *generation,
        _ => 0,
    };
    if generation > LAST_COPIED {
        return None;
    }
    let copy = with_fields(original.tag.as_ref(), vec![(GENERATION, Value::Int(generation + 1))]);
    Some(vec![made(WRITTEN, &original, Some(copy), blanks), made(WRITTEN, &original, original.tag.clone(), 1)])
}

pub(super) fn map(grid: &Grid) -> Option<Made> {
    let (original, blanks) = copies_of(grid, FILLED, EMPTY_MAP)?;
    Some(vec![made(FILLED, &original, original.tag.clone(), blanks + 1)])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stack(name: &str, tag: Option<Value>) -> ItemStack {
        ItemStack { name: name.into(), count: 1, damage: 0, block: None, tag }
    }

    fn grid(stacks: &[ItemStack]) -> Grid<'_> {
        Grid { cells: stacks.iter().map(Some).collect(), width: 2 }
    }

    #[test]
    fn a_written_book_is_copied_a_generation_on_and_given_back() {
        let tag = |generation: i32| Some(Value::Compound(vec![("author".into(), Value::String("Steve".into())), (GENERATION.into(), Value::Int(generation))]));
        let stacks = [stack(WRITABLE, None), stack(WRITTEN, tag(0)), stack(WRITABLE, None)];
        let made = book(&grid(&stacks)).expect("copies");
        assert_eq!((made[0].1, made[0].0.tag.as_ref().and_then(|tag| tag.get(GENERATION))), (2, Some(&Value::Int(1))));
        assert_eq!((made[1].1, &made[1].0.tag), (1, &tag(0)));
        assert!(book(&grid(&[stack(WRITTEN, tag(2)), stack(WRITABLE, None)])).is_none(), "a copy of a copy");
        assert!(book(&grid(&[stack(WRITTEN, tag(0)), stack(WRITTEN, tag(0)), stack(WRITABLE, None)])).is_none());
        assert!(book(&grid(&[stack(WRITTEN, tag(0))])).is_none());
    }

    #[test]
    fn a_filled_map_and_empty_maps_make_that_many_and_one_more() {
        let tag = Some(Value::Compound(vec![("map_uuid".into(), Value::Long(-5))]));
        let made = map(&grid(&[stack(FILLED, tag.clone()), stack(EMPTY_MAP, None), stack(EMPTY_MAP, None)])).expect("copies");
        assert_eq!((made[0].0.name, made[0].1, &made[0].0.tag), (FILLED, 3, &tag));
        assert!(map(&grid(&[stack(FILLED, tag.clone()), stack("minecraft:paper", None)])).is_none());
    }
}
