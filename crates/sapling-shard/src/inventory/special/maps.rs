//! Extending (`d392b075-…`, matches 142c806e0, assemble 142c80c10) and upgrading (`aecd2294-…`,
//! matches 142c81f60, assemble 142c82530) a map in the grid. Extending takes the filled map in a
//! table's middle cell and paper in all eight others; the map must be known, unlocked, below
//! scale 4 and of damage below 3, and the result is the map under its next zoom level's id, made
//! at craft time as the cartography table's zoom makes it. Upgrading takes a map and a compass and
//! nothing else: a known unlocked filled map not yet showing players gets `map_display_players`; an
//! empty map not already a locator (damage 2) becomes a bare one.

use acacia_proto::nbt::Value;
use sapling_data::CreativeItem;

use super::{Grid, Made, with_fields};
use crate::inventory::forge::remade;
use crate::maps::FILLED;

const PAPER: &str = "minecraft:paper";
const EMPTY_MAP: &str = "minecraft:empty_map";
const COMPASS: &str = "minecraft:compass";
const DISPLAY_PLAYERS: &str = "map_display_players";
const MIDDLE: usize = 4;
const PAPER_NEEDED: usize = 8;
const MAX_SCALE: u8 = 4;
const LOCATOR: i16 = 2;
/// Explorer maps (3 to 5) and locked maps (6) are not extended.
const EXTENDED_DAMAGE: i16 = 3;

/// The map ids extending the grid's map makes it from and into, when the grid extends it.
pub(super) fn extending(grid: &Grid) -> Option<(i64, i64)> {
    let map = grid.cells.get(MIDDLE).copied().flatten().filter(|map| map.name == FILLED && map.damage < EXTENDED_DAMAGE)?;
    let paper = grid.filled().iter().filter(|stack| stack.name == PAPER).count();
    if grid.width != 3 || paper < PAPER_NEEDED || paper + 1 != grid.filled().len() {
        return None;
    }
    let (id, ..) = grid.map.filter(|(id, scale, locked)| crate::maps::map_id(map) == Some(*id) && *scale < MAX_SCALE && !locked)?;
    Some((id, id + 1))
}

pub(super) fn extended(grid: &Grid) -> Option<Made> {
    let (_, to) = extending(grid)?;
    let map = grid.cells[MIDDLE]?;
    Some(vec![(CreativeItem { name: FILLED, damage: map.damage, block: None, tag: remade(map.tag.as_ref(), to) }, 1)])
}

fn shows_players(tag: Option<&Value>) -> bool {
    matches!(tag.and_then(|tag| tag.get(DISPLAY_PLAYERS)), Some(Value::Byte(shown)) if *shown != 0)
}

pub(super) fn upgraded(grid: &Grid) -> Option<Made> {
    let filled = grid.filled();
    let [first, second] = filled.as_slice() else { return None };
    let map = [first, second].into_iter().find(|stack| stack.name != COMPASS)?;
    if [first, second].iter().filter(|stack| stack.name == COMPASS).count() != 1 || shows_players(map.tag.as_ref()) {
        return None;
    }
    if map.name == FILLED {
        grid.map.filter(|(id, _, locked)| crate::maps::map_id(map) == Some(*id) && !locked)?;
        let tag = with_fields(map.tag.as_ref(), vec![(DISPLAY_PLAYERS, Value::Byte(1))]);
        return Some(vec![(CreativeItem { name: FILLED, damage: map.damage, block: None, tag: Some(tag) }, 1)]);
    }
    (map.name == EMPTY_MAP && map.damage != LOCATOR).then(|| vec![(CreativeItem { name: EMPTY_MAP, damage: LOCATOR, block: None, tag: None }, 1)])
}

#[cfg(test)]
mod tests {
    use super::*;
    use sapling_world::ItemStack;

    fn stack(name: &str, damage: i16, tag: Option<Value>) -> ItemStack {
        ItemStack { name: name.into(), count: 1, damage, block: None, tag }
    }

    fn uuid(id: i64) -> Option<Value> {
        Some(Value::Compound(vec![("map_uuid".into(), Value::Long(id))]))
    }

    #[test]
    fn eight_paper_round_a_known_map_extend_it_to_its_next_zoom_id() {
        let mut stacks: Vec<ItemStack> = (0..9).map(|_| stack(PAPER, 0, None)).collect();
        stacks[MIDDLE] = stack(FILLED, 0, uuid(-10));
        let grid = |stacks: &[ItemStack], map| extended(&Grid { cells: stacks.iter().map(Some).collect(), width: 3, map });
        let made = grid(&stacks, Some((-10, 0, false))).expect("extends");
        assert_eq!((made[0].1, made[0].0.tag.as_ref().and_then(|tag| tag.get("map_uuid"))), (1, Some(&Value::Long(-9))));
        assert_eq!(made[0].0.tag.as_ref().and_then(|tag| tag.get("map_is_init")), Some(&Value::Byte(1)));
        assert!(grid(&stacks, Some((-10, 4, false))).is_none(), "at the widest scale");
        assert!(grid(&stacks, Some((-10, 0, true))).is_none(), "locked");
        assert!(grid(&stacks, None).is_none(), "unknown");
        stacks.swap(MIDDLE, 0);
        assert!(grid(&stacks, Some((-10, 0, false))).is_none(), "off the middle");
    }

    #[test]
    fn a_compass_makes_a_map_a_locator() {
        let grid = |stacks: &[ItemStack], map| upgraded(&Grid { cells: stacks.iter().map(Some).collect(), width: 2, map });
        let made = grid(&[stack(EMPTY_MAP, 0, None), stack(COMPASS, 0, None)], None).expect("a locator");
        assert_eq!((made[0].0.name, made[0].0.damage), (EMPTY_MAP, LOCATOR));
        assert!(grid(&[stack(EMPTY_MAP, LOCATOR, None), stack(COMPASS, 0, None)], None).is_none());
        let filled = [stack(COMPASS, 0, None), stack(FILLED, 0, uuid(-5))];
        let made = grid(&filled, Some((-5, 1, false))).expect("shows players");
        assert_eq!(made[0].0.tag.as_ref().and_then(|tag| tag.get(DISPLAY_PLAYERS)), Some(&Value::Byte(1)));
        assert!(grid(&filled, Some((-5, 1, true))).is_none(), "locked");
        assert!(grid(&[stack(COMPASS, 0, None), stack(COMPASS, 0, None)], None).is_none());
    }
}
