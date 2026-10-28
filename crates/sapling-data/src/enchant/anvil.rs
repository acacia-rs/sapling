//! What an anvil makes of its input and material, and the levels it costs (BDS's rules:
//! docs/research/workstations.md, "Anvil"): a rename, a repair with the item's material, two
//! of an item put together, or a book's enchantments laid on.

use acacia_proto::nbt::Value;

use super::kinds::{self, Kind};
use super::list::{all, with_all};
use crate::gear::{BOOK, ENCHANTED_BOOK, Made, damage, gear, with_damage, with_field};
use crate::items::Stack;

/// The cost from which a craft is too expensive out of creative.
pub const TOO_EXPENSIVE: i32 = 40;
/// One craft in `WEARS` wears the anvil, `WEAR_CHANCE` of them.
pub const WEARS: u32 = 100;
pub const WEAR_CHANCE: u32 = 12;
const REPAIR_COST: &str = "RepairCost";
const DISPLAY: &str = "display";
const NAME: &str = "Name";
/// A unit of material mends a quarter of what the item lasts.
const UNIT_SHARE: i32 = 4;
/// Two of an item put together mend a twelfth more.
const BONUS_PERCENT: i32 = 12;

/// What an anvil craft makes, what it costs and how much material it takes.
#[derive(Debug, Clone, PartialEq)]
pub struct Repair {
    pub made: Made,
    pub cost: i32,
    pub material: u8,
}

fn int(tag: Option<&Value>, key: &str) -> i32 {
    match tag.and_then(|tag| tag.get(key)) {
        Some(Value::Int(value)) => *value,
        _ => 0,
    }
}

/// A stack's prior work penalty: what each anvil craft before added.
pub fn prior_work(tag: Option<&Value>) -> i32 {
    int(tag, REPAIR_COST)
}

/// The penalty an item carries after one more anvil craft.
pub fn more_work(penalty: i32) -> i32 {
    penalty * 2 + 1
}

/// The name an anvil gave a stack.
pub fn custom_name(tag: Option<&Value>) -> Option<&str> {
    match tag?.get(DISPLAY)?.get(NAME)? {
        Value::String(name) => Some(name.as_str()),
        _ => None,
    }
}

/// `tag` with `display.Name` set to `name`, or taken off for `None`.
pub fn with_name(tag: Option<Value>, name: Option<&str>) -> Option<Value> {
    let display = tag.as_ref().and_then(|tag| tag.get(DISPLAY)).cloned();
    let display = with_field(display, NAME, name.map(|name| Value::String(name.into())));
    with_field(tag, DISPLAY, display)
}

/// The levels a level of an enchantment is worth on an anvil, by how rare it is; halved off a book.
fn multiplier(kind: &Kind, from_book: bool) -> i32 {
    let each = match kind.frequency {
        30 => 1,
        10 => 2,
        3 => 4,
        _ => 8,
    };
    if from_book { (each / 2).max(1) } else { each }
}

/// What the enchantments `levels` are worth on an anvil.
fn worth(levels: &[(i16, u8)], from_book: bool) -> i32 {
    levels.iter().filter_map(|(id, level)| Some(multiplier(kinds::kind_of(*id)?, from_book) * i32::from(*level))).sum()
}

/// Whether a craft that costs `cost` may be done, by a creative player or not.
pub fn allowed(cost: i32, creative: bool) -> bool {
    creative || cost < TOO_EXPENSIVE
}

/// The anvil's craft of `input` with `material` (none for a rename alone), renamed to `name` if
/// one is given (an empty one takes the name off); `None` when the anvil makes nothing of them.
pub fn repair(input: &Stack<'_>, material: Option<&Stack<'_>>, name: Option<&str>) -> Option<Repair> {
    let mut tag = input.tag.cloned();
    let (mut work, mut used, mut too_many) = (0, 0, false);
    let lasts = gear(input.name).map_or(0, |gear| i32::from(gear.lasts));
    let worn = damage(input.tag);
    if let Some(material) = material {
        if gear(input.name).is_some_and(|gear| gear.is_repaired_by(material.name)) && lasts > 0 {
            let mut left = worn;
            let unit = |left: i32| left.min(lasts / UNIT_SHARE);
            if unit(left) < 1 {
                return None;
            }
            while unit(left) > 0 && used < material.count {
                (left, used, work) = (left - unit(left), used + 1, work + 1);
            }
            tag = with_damage(tag, left);
        } else if material.name == input.name || material.name == ENCHANTED_BOOK {
            let from_book = material.name == ENCHANTED_BOOK;
            if !from_book && lasts > 0 {
                let left = (worn + damage(material.tag) - lasts - lasts * BONUS_PERCENT / 100).max(0);
                if left < worn {
                    (tag, work) = (with_damage(tag, left), work + 2);
                }
            }
            let levels = merged(input, material)?;
            let gained = worth(&levels, from_book) - worth(&all(input.tag), from_book);
            too_many = gained > 0 && input.count > 1;
            (tag, work, used) = (with_all(tag, &levels), work + gained.max(0), 1);
        } else {
            return None;
        }
    }
    let renamed = match (name, custom_name(input.tag)) {
        (Some(""), Some(_)) => Some(None),
        (Some(name), had) if !name.is_empty() && Some(name) != had => Some(Some(name)),
        _ => None,
    };
    if let Some(name) = renamed {
        (tag, work) = (with_name(tag, name), work + 1);
    }
    if work == 0 {
        return None;
    }
    let only_renamed = renamed.is_some() && work == 1;
    let penalty = prior_work(input.tag) + material.map_or(0, |material| prior_work(material.tag));
    let cost = match (too_many, only_renamed) {
        (true, _) => TOO_EXPENSIVE,
        (false, true) => (penalty + work).min(TOO_EXPENSIVE - 1),
        (false, false) => penalty + work,
    };
    if !only_renamed {
        let most = prior_work(input.tag).max(material.map_or(0, |material| prior_work(material.tag)));
        tag = with_field(tag, REPAIR_COST, Some(Value::Int(more_work(most))));
    }
    let enchanted = input.name == BOOK && !all(tag.as_ref()).is_empty();
    let made = Made { name: if enchanted { ENCHANTED_BOOK.into() } else { input.name.to_owned() }, tag };
    Some(Repair { made, cost, material: used })
}

/// The enchantments of `input` with those of `material` laid on: one that goes on the item and
/// with what it has, one level up when it has it at that level, up to its most; `None` when
/// `input` takes no enchantments.
fn merged(input: &Stack<'_>, material: &Stack<'_>) -> Option<Vec<(i16, u8)>> {
    let slots = if input.name == ENCHANTED_BOOK || input.name == BOOK { u32::MAX } else { gear(input.name)?.slots };
    let mut levels = all(input.tag);
    for (id, level) in all(material.tag) {
        let Some(kind) = kinds::kind_of(id).filter(|kind| kind.goes_on(slots)) else { continue };
        match levels.iter().position(|(have, _)| *have == id) {
            Some(at) if levels[at].1 < kind.max => {
                let now = levels[at].1;
                levels[at].1 = if now == level { now + 1 } else { now.max(level) };
            }
            Some(_) => {}
            None if levels.iter().all(|(have, _)| kinds::go_together(*have, id)) => levels.push((id, level)),
            None => {}
        }
    }
    Some(levels)
}
