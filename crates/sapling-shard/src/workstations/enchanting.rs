//! What an enchanting table offers a player for the item in it: three enchantments, drawn from
//! the player's enchanting seed and the bookshelves around the table, sent as
//! `PlayerEnchantOptions` with the answer to the request that laid the item in, and nothing for
//! an empty slot or an item it cannot enchant. Each offer is named by a recipe id, counted up
//! across all players, that the request that takes it names (inventory/forge.rs).

use acacia_proto::packets::PlayerEnchantOptions;
use acacia_proto::types::{Enchant as WireEnchant, EnchantOption};
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_data::enchant::{Draws, Offer, table};
use sapling_world::ItemStack;
use sapling_worldgen::random::Mt19937;

use crate::components::{Layer, Outgoing};
use crate::inventory::{Inventory, enchanting_input, stack};

/// The offers made for the item last in a player's enchanting table.
#[derive(Component, Default, Clone)]
pub(crate) struct Offers {
    item: Option<ItemStack>,
    list: Vec<(u32, Offer)>,
}

impl Offers {
    pub fn listed(&self) -> Vec<(u32, Offer)> {
        self.list.clone()
    }
}

/// The id the last offer made on the server was named by; BDS counts them for all players.
#[derive(Resource, Default)]
struct OptionIds(u32);

impl OptionIds {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_add(1).max(1);
        self.0
    }
}

/// BDS's `Random`, as the table draws from it.
struct Bedrock(Mt19937);

impl Draws for Bedrock {
    fn int(&mut self, bound: i32) -> i32 {
        self.0.next_int(bound)
    }

    fn float(&mut self) -> f32 {
        self.0.next_float()
    }
}

/// Offers nothing to a player who just opened a table.
pub(super) fn reset(world: &mut World, player: Entity) {
    world.entity_mut(player).insert(Offers::default());
    if let Some(mut out) = world.get_mut::<Outgoing>(player) {
        out.push(&PlayerEnchantOptions { options: Vec::new() });
    }
}

/// How many bookshelves around the table at `pos` count.
fn shelves(world: &World, layer: Entity, [x, y, z]: BlockPos) -> u32 {
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return 0 };
    let name = |[dx, dy, dz]: [i32; 3]| level.registry().get(level.block(x + dx, y + dy, z + dz)).map(|state| state.name);
    let counted = table::shelf_spots().filter(|(shelf, gap)| name(*shelf).is_some_and(table::is_shelf) && gap.iter().all(|gap| name(*gap).is_some_and(table::lets_through)));
    counted.count() as u32
}

fn wire(id: u32, offer: &Offer) -> EnchantOption {
    let [equip_enchants, held_enchants, self_enchants] = table::by_activation(&offer.enchants).map(|list| list.into_iter().map(|(id, level)| WireEnchant { id: id as u8, level }).collect());
    EnchantOption { cost: offer.cost, slot_flags: offer.slots as i32, equip_enchants, held_enchants, self_enchants, name: offer.name.clone(), option_id: id }
}

/// Makes the player's offers for what now lies in its table at `pos`, and sends them if the item
/// changed.
pub(super) fn offer(world: &mut World, player: Entity, layer: Entity, pos: BlockPos) {
    let Some(inventory) = world.get::<Inventory>(player) else { return };
    let item = enchanting_input(inventory).cloned();
    if world.get::<Offers>(player).is_some_and(|offers| offers.item == item) {
        return;
    }
    let seed = inventory.enchant_seed;
    let shelves = shelves(world, layer, pos);
    let made = item.as_ref().map(|item| table::offers(shelves, &stack(item), &mut Bedrock(Mt19937::new(seed as u32)))).unwrap_or_default();
    let mut ids = world.get_resource_or_insert_with(OptionIds::default);
    let list: Vec<(u32, Offer)> = made.into_iter().map(|offer| (ids.next(), offer)).collect();
    let options = list.iter().map(|(id, offer)| wire(*id, offer)).collect();
    world.entity_mut(player).insert(Offers { item, list });
    if let Some(mut out) = world.get_mut::<Outgoing>(player) {
        out.push(&PlayerEnchantOptions { options });
    }
}
