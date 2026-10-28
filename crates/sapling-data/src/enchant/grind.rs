//! What a grindstone makes of one item or two of a kind: the enchantments ground off but the
//! curses, two put together, and the experience the ground-off ones give back (BDS's rules:
//! docs/research/workstations.md, "Grindstone").

use acacia_proto::nbt::Value;

use super::kinds;
use super::list::{all, with_all};
use crate::gear::{BOOK, ENCHANTED_BOOK, Made, damage, gear, with_damage, with_field};
use crate::items::Stack;

/// Two of an item put together mend a twentieth more.
const BONUS_SHARE: i32 = 20;
/// The prior work a ground item keeps, by how many curses it keeps.
const CURSED_WORK: [i32; 3] = [0, 1, 3];

#[derive(Debug, Clone, PartialEq)]
pub struct Ground {
    pub made: Made,
    /// The prior work it carries, which the request names as its cost.
    pub work: i32,
    /// The experience it gives back, from the first to the second.
    pub experience: [u32; 2],
}

fn is_curse(id: i16) -> bool {
    kinds::kind_of(id).is_some_and(|kind| kind.curse)
}

/// The power of the enchantments on `stack` that are not curses: what grinding them off gives
/// back.
fn ground_power(stack: &Stack<'_>) -> u32 {
    let power = |(id, level): (i16, u8)| kinds::kind_of(id).filter(|kind| !kind.curse).map_or(0, |kind| kind.min_power(level).max(0) as u32);
    all(stack.tag).into_iter().map(power).sum()
}

/// What the grindstone makes of what lies in its two slots; `None` when it makes nothing.
pub fn grind(first: Option<Stack<'_>>, second: Option<Stack<'_>>) -> Option<Ground> {
    if [&first, &second].into_iter().flatten().any(|stack| stack.count > 1) {
        return None;
    }
    let (base, other) = match (&first, &second) {
        (Some(first), second) => (first, second.as_ref()),
        (None, Some(second)) => (second, None),
        (None, None) => return None,
    };
    let mut tag = base.tag.cloned();
    if let Some(other) = other {
        let lasts = gear(base.name).map_or(0, |gear| i32::from(gear.lasts));
        if other.name != base.name || lasts == 0 {
            return None;
        }
        tag = base.tag.or(other.tag).cloned();
        tag = with_damage(tag, (damage(base.tag) + damage(other.tag) - lasts - lasts / BONUS_SHARE).max(0));
    } else if second.is_none() && base.tag.is_some_and(|tag| tag.get("Trim").is_some()) {
        // BDS's quirk: a trimmed item alone in the first slot grinds to nothing.
        return None;
    }
    let mut curses: Vec<(i16, u8)> = Vec::new();
    for curse in [Some(base), other].into_iter().flatten().flat_map(|stack| all(stack.tag)).filter(|(id, _)| is_curse(*id)) {
        if !curses.contains(&curse) {
            curses.push(curse);
        }
    }
    let work = CURSED_WORK[curses.len().min(CURSED_WORK.len() - 1)];
    tag = with_field(with_all(tag, &curses), "RepairCost", (work > 0).then_some(Value::Int(work)));
    let name = if base.name == ENCHANTED_BOOK && curses.is_empty() { BOOK } else { base.name };
    if other.is_none() && name == base.name && tag.as_ref() == base.tag {
        return None;
    }
    let power = ground_power(base) + other.map_or(0, ground_power);
    let half = power.div_ceil(2);
    let made = Made { name: name.to_owned(), tag };
    Some(Ground { made, work, experience: [half, (2 * half).saturating_sub(1).max(half)] })
}
