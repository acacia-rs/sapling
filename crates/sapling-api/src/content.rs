//! Custom items and blocks a plugin adds to the game ([`crate::Plugin::content`]). The server gives
//! them network ids, lists them in what joining clients are sent, and builds the resource pack
//! that draws them (docs/research/custom-content.md).

pub use crate::custom_block::{BlockBox, BlockDrop, CustomBlock, Geometry, Material, Permutation, RenderMethod, StateValue, StateValues};

/// Where a custom item is listed in the creative inventory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreativeCategory {
    Construction,
    Nature,
    Equipment,
    Items,
    /// Not listed: only commands and plugins hand it out.
    None,
}

impl CreativeCategory {
    /// The number the ItemRegistry's `creative_category` and CreativeContent's groups give it.
    pub fn id(self) -> i32 {
        match self {
            Self::Construction => 1,
            Self::Nature => 2,
            Self::Equipment => 3,
            Self::Items => 4,
            Self::None => 5,
        }
    }
}

/// The armour slot a wearable custom item goes in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArmourSlot {
    Head,
    Chest,
    Legs,
    Feet,
}

impl ArmourSlot {
    /// `minecraft:wearable`'s slot name.
    pub fn slot_name(self) -> &'static str {
        match self {
            Self::Head => "slot.armor.head",
            Self::Chest => "slot.armor.chest",
            Self::Legs => "slot.armor.legs",
            Self::Feet => "slot.armor.feet",
        }
    }

    /// 0 head to 3 feet, as [`crate::Slot::Armor`] numbers them.
    pub fn index(self) -> u8 {
        self as u8
    }
}

/// What eating a custom item gives.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CustomFood {
    pub nutrition: u8,
    /// `minecraft:food`'s `saturation_modifier`: saturation is nutrition × modifier × 2.
    pub saturation_modifier: f32,
    /// Eaten with a full hunger bar too.
    pub can_always_eat: bool,
    /// Ticks the use is held before it is eaten (32 for most foods).
    pub use_ticks: u32,
}

impl CustomFood {
    pub fn new(nutrition: u8, saturation_modifier: f32) -> Self {
        Self { nutrition, saturation_modifier, can_always_eat: false, use_ticks: 32 }
    }
}

/// A custom item: its look (a PNG texture, a display name) and the components clients and the
/// server act on. Built with [`CustomItem::new`] and the setters.
///
/// ```
/// let ruby = sapling_api::CustomItem::new("example:ruby", "Ruby", Vec::new()).max_stack(16);
/// assert_eq!(ruby.identifier, "example:ruby");
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct CustomItem {
    /// Namespaced, never `minecraft:`.
    pub identifier: String,
    pub display_name: String,
    /// A PNG, 16×16 like the vanilla items'.
    pub texture: Vec<u8>,
    pub max_stack: u8,
    pub category: CreativeCategory,
    /// A creative group to list it in (`itemGroup.name.sword`); empty for none.
    pub group: String,
    pub food: Option<CustomFood>,
    /// Uses it lasts before it breaks.
    pub durability: Option<u16>,
    /// Where it is worn, and the armour points it gives there.
    pub armour: Option<(ArmourSlot, u8)>,
    /// Health a hit with it takes (a bare hand's 1 without).
    pub attack_damage: Option<u8>,
    /// Held like a tool, upright.
    pub hand_equipped: bool,
    /// A cooldown category and the ticks it lasts after a use.
    pub cooldown: Option<(String, u32)>,
    pub allow_off_hand: bool,
    /// Shown with the enchantment glint.
    pub glint: bool,
}

impl CustomItem {
    pub fn new(identifier: impl Into<String>, display_name: impl Into<String>, texture: Vec<u8>) -> Self {
        Self {
            identifier: identifier.into(),
            display_name: display_name.into(),
            texture,
            max_stack: 64,
            category: CreativeCategory::Items,
            group: String::new(),
            food: None,
            durability: None,
            armour: None,
            attack_damage: None,
            hand_equipped: false,
            cooldown: None,
            allow_off_hand: false,
            glint: false,
        }
    }

    pub fn max_stack(mut self, max: u8) -> Self {
        self.max_stack = max.clamp(1, 64);
        self
    }

    pub fn category(mut self, category: CreativeCategory) -> Self {
        self.category = category;
        self
    }

    pub fn group(mut self, group: impl Into<String>) -> Self {
        self.group = group.into();
        self
    }

    pub fn food(mut self, food: CustomFood) -> Self {
        self.food = Some(food);
        self
    }

    /// Wears out after `uses`; one to a slot.
    pub fn durability(mut self, uses: u16) -> Self {
        self.durability = Some(uses);
        self.max_stack(1)
    }

    /// Worn in `slot` for `points` of armour; one to a slot.
    pub fn armour(mut self, slot: ArmourSlot, points: u8) -> Self {
        self.armour = Some((slot, points));
        self.max_stack(1)
    }

    pub fn attack_damage(mut self, damage: u8) -> Self {
        self.attack_damage = Some(damage);
        self
    }

    pub fn hand_equipped(mut self, held_like_a_tool: bool) -> Self {
        self.hand_equipped = held_like_a_tool;
        self
    }

    pub fn cooldown(mut self, category: impl Into<String>, ticks: u32) -> Self {
        self.cooldown = Some((category.into(), ticks));
        self
    }

    pub fn allow_off_hand(mut self, allowed: bool) -> Self {
        self.allow_off_hand = allowed;
        self
    }

    pub fn glint(mut self, shown: bool) -> Self {
        self.glint = shown;
        self
    }

    /// The part of the identifier after the namespace: `ruby` of `example:ruby`.
    pub fn short_name(&self) -> &str {
        self.identifier.rsplit(':').next().unwrap_or(&self.identifier)
    }
}

/// What [`crate::Plugin::content`] adds to: the custom items and blocks of the whole server,
/// registered once, before any shard starts.
#[derive(Debug, Default, Clone)]
pub struct Content {
    pub items: Vec<CustomItem>,
    pub blocks: Vec<CustomBlock>,
}

impl Content {
    pub fn item(&mut self, item: CustomItem) -> &mut Self {
        self.items.push(item);
        self
    }

    pub fn block(&mut self, block: CustomBlock) -> &mut Self {
        self.blocks.push(block);
        self
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty() && self.blocks.is_empty()
    }
}
