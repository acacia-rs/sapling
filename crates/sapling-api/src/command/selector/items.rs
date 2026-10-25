//! `hasitem`: an item a candidate must carry, where, and how many of it.

use super::filters::{Bounds, Not, namespaced};
use super::lex::{self, Pair, Span};
use crate::{Item, Slot};

/// Where `hasitem` looks, as `location=` names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Location {
    MainHand,
    OffHand,
    Head,
    Chest,
    Legs,
    Feet,
    /// The four armour slots, head first.
    Armor,
    Hotbar,
    /// The 27 slots above the hotbar.
    Inventory,
    /// One sapling cannot look into: an ender chest, a saddle, a mount's chest, body armour.
    Unread,
}

const LOCATIONS: [(&str, Location); 14] = [
    ("slot.weapon.mainhand", Location::MainHand),
    ("slot.weapon.offhand", Location::OffHand),
    ("slot.armor.head", Location::Head),
    ("slot.armor.chest", Location::Chest),
    ("slot.armor.legs", Location::Legs),
    ("slot.armor.feet", Location::Feet),
    ("slot.armor", Location::Armor),
    ("slot.hotbar", Location::Hotbar),
    ("slot.inventory", Location::Inventory),
    ("slot.armor.body", Location::Unread),
    ("slot.enderchest", Location::Unread),
    ("slot.saddle", Location::Unread),
    ("slot.chest", Location::Unread),
    ("slot.equippable", Location::Unread),
];

impl Location {
    /// Its slots, each with its number within it, for a player holding hotbar slot `held`.
    fn slots(self, held: u8) -> Vec<(i32, Slot)> {
        let only = |slot| vec![(0, slot)];
        match self {
            Self::MainHand => only(Slot::Main(held)),
            Self::OffHand => only(Slot::Offhand),
            Self::Head => only(Slot::Armor(0)),
            Self::Chest => only(Slot::Armor(1)),
            Self::Legs => only(Slot::Armor(2)),
            Self::Feet => only(Slot::Armor(3)),
            Self::Armor => (0..4).map(|n| (i32::from(n), Slot::Armor(n))).collect(),
            Self::Hotbar => (0..9).map(|n| (i32::from(n), Slot::Main(n))).collect(),
            Self::Inventory => (0..27).map(|n| (i32::from(n), Slot::Main(n + 9))).collect(),
            Self::Unread => Vec::new(),
        }
    }
}

/// What a candidate carries: the stacks in its slots and the hotbar slot it holds.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Inventory {
    pub items: Vec<(Slot, Item)>,
    pub held: u8,
}

/// One `{item=...,quantity=...,location=...,slot=...,data=...}` of `hasitem`.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemTest {
    /// Namespaced.
    pub item: String,
    /// The stack's data; `None` (or `-1` typed) for any.
    pub data: Option<i16>,
    /// How many there are in all where it looks; `1..` when not given, `0` for none.
    pub quantity: Not<Bounds<i32>>,
    /// Everywhere when not given.
    pub location: Option<Location>,
    /// Which slots of `location`, numbered from 0.
    pub slot: Option<Bounds<i32>>,
}

impl ItemTest {
    /// `hasitem={...}` or `hasitem=[{...},{...}]`, every one of which must pass.
    pub(super) fn read_all(pair: &Pair<'_>) -> Result<Vec<Self>, Span> {
        if let Some((inner, at)) = lex::inside(pair.value, pair.value_at, '[') {
            return lex::split(inner, at)?.into_iter().map(|(test, at)| Self::read(test, at)).collect();
        }
        Self::read(pair.value, pair.value_at).map(|test| vec![test])
    }

    fn read(value: &str, at: usize) -> Result<Self, Span> {
        let whole = (at, at + value.len());
        let (inner, inner_at) = lex::inside(value, at, '{').ok_or(whole)?;
        let (mut item, mut test) = (None, Self { item: String::new(), data: None, quantity: Not { value: Bounds { min: Some(1), max: None }, negated: false }, location: None, slot: None });
        let mut slot_key = None;
        for pair in lex::pairs(inner, inner_at)? {
            let bad = pair.value_span();
            if pair.negated && pair.key != "quantity" {
                return Err(bad);
            }
            match pair.key {
                "item" if !pair.value.is_empty() => item = Some(namespaced(pair.value)),
                "data" => test.data = Some(pair.value.parse::<i16>().map_err(|_| bad)?).filter(|data| *data >= 0),
                "quantity" => test.quantity = Not { value: Bounds::read(pair.value).ok_or(bad)?, negated: pair.negated },
                "location" => test.location = Some(LOCATIONS.iter().find(|(name, _)| *name == pair.value).map(|(_, location)| *location).ok_or(bad)?),
                "slot" => (test.slot, slot_key) = (Some(Bounds::read(pair.value).ok_or(bad)?), Some(pair.key_span())),
                "item" => return Err(bad),
                _ => return Err(pair.key_span()),
            }
        }
        if let (Some(slot), None) = (slot_key, test.location) {
            return Err(slot);
        }
        test.item = item.ok_or(whole)?;
        Ok(test)
    }

    /// Whether `inventory` (`None`: the candidate has none) passes.
    pub(super) fn holds(&self, inventory: Option<&Inventory>) -> bool {
        let empty = Inventory::default();
        let inventory = inventory.unwrap_or(&empty);
        let looked: Vec<Slot> = match self.location {
            Some(location) => location.slots(inventory.held).into_iter().filter(|(n, _)| self.slot.is_none_or(|slot| slot.holds(*n))).map(|(_, slot)| slot).collect(),
            None => inventory.items.iter().map(|(slot, _)| *slot).collect(),
        };
        let wanted = |item: &Item| item.name == self.item && self.data.is_none_or(|data| item.damage == data);
        let count: i32 = inventory.items.iter().filter(|(slot, item)| looked.contains(slot) && wanted(item)).map(|(_, item)| i32::from(item.count)).sum();
        self.quantity.passes(self.quantity.value.holds(count))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::selector::lex::pairs;

    fn read(text: &str) -> Result<Vec<ItemTest>, Span> {
        ItemTest::read_all(&pairs(text, 0).unwrap()[0])
    }

    fn carrying(items: &[(Slot, &str, u8)]) -> Inventory {
        Inventory { items: items.iter().map(|(slot, name, count)| (*slot, Item::new(*name, *count))).collect(), held: 2 }
    }

    #[test]
    fn hasitem_reads_one_or_many_and_points_at_what_is_wrong() {
        let tests = read("hasitem=[{item=diamond,quantity=2..},{item=minecraft:stick,location=slot.hotbar,slot=0..3,data=1}]").unwrap();
        assert_eq!(tests.len(), 2);
        assert_eq!((tests[0].item.as_str(), tests[0].quantity.value, tests[0].location), ("minecraft:diamond", Bounds { min: Some(2), max: None }, None));
        assert_eq!((tests[1].location, tests[1].slot, tests[1].data), (Some(Location::Hotbar), Some(Bounds { min: Some(0), max: Some(3) }), Some(1)));
        assert_eq!(read("hasitem={item=apple,quantity=!0}").unwrap()[0].quantity, Not { value: Bounds { min: Some(0), max: Some(0) }, negated: true });
        assert_eq!(read("hasitem={item=apple,data=-1}").unwrap()[0].data, None, "-1 is any data");
        assert_eq!(read("hasitem={quantity=1}"), Err((8, 20)), "no item");
        assert_eq!(read("hasitem={item=apple,slot=1}"), Err((20, 24)), "a slot needs a location");
        assert_eq!(read("hasitem={item=apple,location=pocket}"), Err((29, 35)));
        assert_eq!(read("hasitem={item=apple,colour=red}"), Err((20, 26)));
        assert_eq!(read("hasitem=apple"), Err((8, 13)));
    }

    #[test]
    fn hasitem_counts_what_is_where_it_looks() {
        let inventory = carrying(&[(Slot::Main(2), "minecraft:apple", 3), (Slot::Main(20), "minecraft:apple", 5), (Slot::Armor(0), "minecraft:iron_helmet", 1)]);
        let holds = |text: &str| read(text).unwrap().iter().all(|test| test.holds(Some(&inventory)));
        assert!(holds("hasitem={item=apple}") && holds("hasitem={item=apple,quantity=8}") && !holds("hasitem={item=apple,quantity=9..}"));
        assert!(holds("hasitem={item=apple,location=slot.weapon.mainhand,quantity=3}"), "the held slot");
        assert!(holds("hasitem={item=apple,location=slot.inventory,slot=11,quantity=5}"), "slot 20 is the inventory's 11");
        assert!(!holds("hasitem={item=apple,location=slot.hotbar,slot=3..}"));
        assert!(holds("hasitem={item=iron_helmet,location=slot.armor.head}") && !holds("hasitem={item=iron_helmet,location=slot.armor.feet}"));
        assert!(holds("hasitem={item=bread,quantity=0}") && !holds("hasitem={item=apple,quantity=0}"));
        assert!(holds("hasitem={item=apple,data=0}") && !holds("hasitem={item=apple,data=3}"));
        assert!(holds("hasitem=[{item=apple},{item=iron_helmet}]") && !holds("hasitem=[{item=apple},{item=bread}]"), "all of them");
        assert!(read("hasitem={item=apple,quantity=0}").unwrap()[0].holds(None) && !read("hasitem={item=apple}").unwrap()[0].holds(None), "no inventory holds nothing");
    }
}
