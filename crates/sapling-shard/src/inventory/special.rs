//! The special recipes: `CraftingData`'s multi recipes, which have no cells; what lies in the grid
//! makes the result. A request names one by its network id, and only that one's rule is asked.
//! Which uuid is which (`Recipes::init` 141aff700 pairs each `*_recipeId` with its uuid), and
//! what each makes: docs/research/crafting-storage.md.
// TODO: map extending (`d392b075-…`), map upgrading (`aecd2294-…`) and item repair (`…-0001`) in the grid.

mod banners;
mod copies;
mod fireworks;
mod pot;

use sapling_data::CreativeItem;
use sapling_world::ItemStack;

/// A crafting grid's cells row by row, `width` to a row; `None` is an empty cell.
pub(super) struct Grid<'a> {
    pub cells: Vec<Option<&'a ItemStack>>,
    pub width: usize,
}

impl<'a> Grid<'a> {
    pub fn filled(&self) -> Vec<&'a ItemStack> {
        self.cells.iter().flatten().copied().collect()
    }

    /// The one filled cell named `name`, if exactly one is.
    pub fn only(&self, name: &str) -> Option<&'a ItemStack> {
        let mut named = self.filled().into_iter().filter(|stack| stack.name == name);
        named.next().filter(|_| named.next().is_none())
    }
}

/// What one craft makes, each result with its count; the grid's every filled cell is used up once.
pub(super) type Made = Vec<(CreativeItem, u8)>;

const RECIPES: [(&str, fn(&Grid) -> Option<Made>); 6] = [
    ("00000000-0000-0000-0000-000000000002", |grid| fireworks::crafted(&grid.filled()).map(|made| vec![made])),
    ("685a742a-c42e-4a4e-88ea-5eb83fc98e5b", pot::crafted),
    ("d1ca6b84-338e-4f2f-9c6b-76cc8b4bd98d", copies::book),
    ("85939755-ba10-4d9d-a4cc-efb7a8e943c4", copies::map),
    ("b5c5d105-75a2-4076-af2b-923ea2bf4bf0", banners::duplicated),
    ("00000000-0000-0000-0000-0000000000c8", banners::shield),
];

/// What the special recipe of `network_id` makes of `grid`, if it takes what lies there.
pub(super) fn crafted(network_id: u32, grid: &Grid) -> Option<Made> {
    let (_, rule) = RECIPES.iter().find(|(uuid, _)| sapling_data::multi_recipe(uuid) == Some(network_id))?;
    rule(grid)
}

/// `tag`'s fields with `added` put in, keys in byte order as BDS's `CompoundTag` keeps them.
fn with_fields(tag: Option<&acacia_proto::nbt::Value>, added: Vec<(&str, acacia_proto::nbt::Value)>) -> acacia_proto::nbt::Value {
    use acacia_proto::nbt::Value;
    let mut fields = match tag {
        Some(Value::Compound(fields)) => fields.clone(),
        _ => Vec::new(),
    };
    for (key, value) in added {
        fields.retain(|(kept, _)| kept != key);
        fields.push((key.into(), value));
    }
    fields.sort_by(|a, b| a.0.cmp(&b.0));
    Value::Compound(fields)
}
