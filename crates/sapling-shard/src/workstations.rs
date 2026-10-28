//! The workstations whose screens craft by the server's own reckoning: the enchanting table, the
//! anvil, the grindstone, the smithing table, the loom, a beacon's payment and the cartography
//! table (whose zoomed and locked maps maps.rs makes). The craft itself is a request's
//! (inventory/forge.rs); here is what the table offers (enchanting.rs) and what a done craft
//! leaves the world to do: a new enchanting seed, an anvil's wear (anvil.rs), a grindstone's
//! experience. BDS's rules and numbers: docs/research/workstations.md.

mod anvil;
mod enchanting;

use std::hash::{BuildHasher, RandomState};

use acacia_proto::types::WindowType;
use bevy_ecs::prelude::*;
use sapling_api::GameMode;

use crate::components::{InLayer, Mode, NetId, Outgoing, Tick};
use crate::inventory::{Bench, Forged, Inventory};
use crate::{containers, experience, orbs};

pub(crate) use anvil::dispatch_damage as dispatch_anvil_damage;
pub(crate) use enchanting::Offers;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Workstation {
    Enchanting,
    Anvil,
    Grindstone,
    Smithing,
    /// Only read at: its book turns by LecternUpdate (blocks/lectern.rs).
    Lectern,
    Loom,
    /// Paid at (beacons.rs).
    Beacon,
    Cartography,
}

/// The workstation the block `block` is, and the screen it opens.
pub(crate) fn of_block(block: &str) -> Option<(Workstation, WindowType)> {
    Some(match block {
        "minecraft:enchanting_table" => (Workstation::Enchanting, WindowType::Enchantment),
        _ if anvil::is_anvil(block) => (Workstation::Anvil, WindowType::Anvil),
        "minecraft:grindstone" => (Workstation::Grindstone, WindowType::Grindstone),
        "minecraft:smithing_table" => (Workstation::Smithing, WindowType::SmithingTable),
        "minecraft:lectern" => (Workstation::Lectern, WindowType::Lectern),
        "minecraft:loom" => (Workstation::Loom, WindowType::Loom),
        "minecraft:beacon" => (Workstation::Beacon, WindowType::Beacon),
        "minecraft:cartography_table" => (Workstation::Cartography, WindowType::Cartography),
        _ => return None,
    })
}

/// A seed for a player's enchanting table offers: never negative, as BDS's `nextInt()`.
pub(crate) fn new_seed() -> i32 {
    (RandomState::new().hash_one(std::time::Instant::now()) >> 33) as i32
}

/// What follows a player opening a workstation's screen: an enchanting table offers nothing yet.
pub(crate) fn opened(world: &mut World, player: Entity, workstation: Workstation) {
    if workstation == Workstation::Enchanting {
        enchanting::reset(world, player);
    }
}

/// The workstation a player's request is made at, with what it offers: a trader's screen too.
pub(crate) fn bench(world: &World, player: Entity) -> Option<Bench> {
    let Some((workstation, layer, pos)) = containers::workstation(world, player) else { return crate::creatures::bargains(world, player).map(Bench::Trade) };
    Some(match workstation {
        Workstation::Enchanting => Bench::Enchanting(world.get::<Offers>(player).map(Offers::listed).unwrap_or_default()),
        Workstation::Anvil => Bench::Anvil,
        Workstation::Grindstone => Bench::Grindstone,
        Workstation::Smithing => Bench::Smithing,
        Workstation::Loom => Bench::Loom,
        Workstation::Beacon => Bench::Beacon(crate::beacons::level(world, layer, pos)),
        Workstation::Cartography => Bench::Cartography(crate::maps::chart(world, player)),
        Workstation::Lectern => return None,
    })
}

/// What follows a request a player made, done: what its craft (`forged`) leaves the world to do,
/// and the enchanting table's offers for what now lies in it.
pub(crate) fn answered(world: &mut World, player: Entity, forged: Option<Forged>) {
    if let Some(Forged::Traded(index)) = forged {
        return crate::creatures::traded(world, player, index);
    }
    let Some((workstation, layer, pos)) = containers::workstation(world, player) else {
        // A map extended in a crafting grid.
        if let (Some(Forged::Charted { from, to, zoomed }), Some(layer)) = (forged, world.get::<InLayer>(player).map(|layer| layer.0)) {
            crate::maps::charted(world, layer, from, to, zoomed);
        }
        return;
    };
    let creative = world.get::<Mode>(player).is_some_and(|mode| mode.0 == GameMode::Creative);
    match forged {
        Some(Forged::Enchanted) => {
            if let Some(mut inventory) = world.get_mut::<Inventory>(player) {
                (inventory.enchant_seed, inventory.unsaved) = (new_seed(), true);
            }
        }
        Some(Forged::Repaired) => anvil::wear(world, layer, pos, creative),
        Some(Forged::Beaconed(effects)) => crate::beacons::pay(world, layer, pos, effects),
        Some(Forged::Charted { from, to, zoomed }) => crate::maps::charted(world, layer, from, to, zoomed),
        Some(Forged::Ground([least, most])) => {
            let points = least + world.get_resource_or_insert_with(crate::blocks::Dice::default).roll(most - least + 1);
            orbs::pay_player(world, player, points as i32);
        }
        _ => {}
    }
    if forged.is_some() {
        show_level(world, player);
    }
    if workstation == Workstation::Enchanting {
        enchanting::offer(world, player, layer, pos);
    }
}

/// Shows a player its level, which a craft may have cost.
fn show_level(world: &mut World, player: Entity) {
    let tick = world.resource::<Tick>().0;
    let mut query = world.query::<(&NetId, &Inventory, &mut Outgoing)>();
    if let Ok((id, inventory, mut out)) = query.get_mut(world, player) {
        out.push(&experience::attributes(id, &inventory.experience, true, tick));
    }
}
