//! The decorated pot from sherds (`685a742a-…`, assemble 142c7de60): a brick or a sherd in each of
//! a table grid's four edge-middle cells, the corners and centre empty. The pot's `sherds` are
//! the names in cells 1, 3, 5 and 7 in that order; four bricks make a plain pot, no tag
//! (1457edc20 compares each face with the brick).

use acacia_proto::nbt::{List, Value};
use sapling_data::CreativeItem;

use super::{Grid, Made};

const POT: &str = "minecraft:decorated_pot";
const BRICK: &str = "minecraft:brick";
const SHERDS: &str = "sherds";
const SHERD_TAG: &str = "minecraft:decorated_pot_sherds";
const FACES: [usize; 4] = [1, 3, 5, 7];
const STRING_TAG: u8 = 8;

fn is_face(name: &str) -> bool {
    name == BRICK || name.ends_with("_pottery_sherd") || sapling_data::has_tag(name, SHERD_TAG)
}

/// The pot item's block, as the plain pot's recipe gives it.
fn pot_block() -> Option<u32> {
    let brick = Some((BRICK, 0));
    let grid = [None, brick, None, brick, None, brick, None, brick, None];
    sapling_data::laid_out_in(&grid)?.outputs.iter().find(|(item, _)| item.name == POT).and_then(|(item, _)| item.block)
}

pub(super) fn crafted(grid: &Grid) -> Option<Made> {
    if grid.width != 3 || grid.cells.len() != 9 {
        return None;
    }
    let laid = grid.cells.iter().enumerate().all(|(cell, stack)| stack.is_some_and(|stack| is_face(&stack.name)) == FACES.contains(&cell));
    if !laid {
        return None;
    }
    let faces: Vec<&str> = FACES.iter().filter_map(|cell| grid.cells[*cell]).map(|stack| stack.name.as_str()).collect();
    let tag = faces.iter().any(|face| *face != BRICK).then(|| {
        let list = Value::List(List { tag: STRING_TAG, items: faces.iter().map(|face| Value::String((*face).into())).collect() });
        Value::Compound(vec![(SHERDS.into(), list)])
    });
    Some(vec![(CreativeItem { name: POT, damage: 0, block: pot_block(), tag }, 1)])
}

#[cfg(test)]
mod tests {
    use super::*;
    use sapling_world::ItemStack;

    fn stack(name: &str) -> ItemStack {
        ItemStack { name: name.into(), count: 1, damage: 0, block: None, tag: None }
    }

    fn craft(cells: [Option<&str>; 9]) -> Option<Made> {
        let stacks: Vec<Option<ItemStack>> = cells.iter().map(|name| name.map(stack)).collect();
        crafted(&Grid { cells: stacks.iter().map(Option::as_ref).collect(), width: 3 })
    }

    #[test]
    fn sherds_go_on_the_pot_in_grid_order_and_bricks_alone_make_a_plain_one() {
        let (angler, arms) = (Some("minecraft:angler_pottery_sherd"), Some("minecraft:arms_up_pottery_sherd"));
        let made = craft([None, angler, None, Some(BRICK), None, arms, None, Some(BRICK), None]).expect("a pot");
        let Some(Value::List(faces)) = made[0].0.tag.as_ref().and_then(|tag| tag.get(SHERDS)) else { panic!("sherds") };
        let names: Vec<&Value> = faces.items.iter().collect();
        assert_eq!(names, [&Value::String(angler.unwrap().into()), &Value::String(BRICK.into()), &Value::String(arms.unwrap().into()), &Value::String(BRICK.into())]);
        assert_eq!((made[0].0.name, made[0].1), (POT, 1));
        let plain = craft([None, Some(BRICK), None, Some(BRICK), None, Some(BRICK), None, Some(BRICK), None]).expect("a plain pot");
        assert!(plain[0].0.tag.is_none() && plain[0].0.block.is_some());
        assert!(craft([Some(BRICK), angler, None, Some(BRICK), None, arms, None, Some(BRICK), None]).is_none(), "a corner filled");
        assert!(craft([None, angler, None, Some("minecraft:stone"), None, arms, None, Some(BRICK), None]).is_none());
    }
}
