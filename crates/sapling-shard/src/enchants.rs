//! Where a player's enchantments act: what it wears and holds is read here, and the rules are
//! `sapling_data::enchant`'s. Weapon hits: creatures/attacks.rs and duels.rs; protection and the
//! rest of what armour does: vitals.rs; the stacks' own (Mending, the curses): inventory/enchanted.rs.

use bevy_ecs::prelude::*;
use sapling_data::enchant::{self, Enchant, Harm, Warding};

use crate::blocks::Dice;
use crate::components::Outgoing;
use crate::creatures::Cause;
use crate::inventory::{self, Inventory};

/// The NBT of what a player holds, for the rules that weigh several of its enchantments.
pub(crate) fn weapon(world: &World, player: Entity) -> Option<acacia_proto::nbt::Value> {
    world.get::<Inventory>(player)?.in_hand().as_ref()?.stack.tag.clone()
}

/// What the armour a player wears wards off.
pub(crate) fn warding(world: &World, player: Entity) -> Warding {
    world.get::<Inventory>(player).map_or_else(Warding::default, |inventory| Warding::of(inventory.worn_tags()))
}

/// What the protection enchantments count a hurt of `cause` as; `None` for one none of them wards
/// (BDS `ProtectionEnchant::getDamageProtection`; a sonic boom is never reduced).
pub(crate) fn harm_of(cause: Cause) -> Option<Harm> {
    Some(match cause {
        Cause::Fire | Cause::FireTick | Cause::Lava | Cause::Magma => Harm::Fire,
        Cause::BlockExplosion | Cause::EntityExplosion => Harm::Blast,
        Cause::Projectile => Harm::Projectile,
        Cause::Fall | Cause::Stalagmite => Harm::Fall,
        Cause::Void | Cause::Starving | Cause::SonicBoom => return None,
        _ => Harm::Other,
    })
}

/// A player takes an orb worth `value`: one thing it has on with Mending is mended by it first,
/// and the client shown; what is left of the value.
pub(crate) fn mend(world: &mut World, player: Entity, value: i32) -> i32 {
    let pick = world.get_resource_or_insert_with(Dice::default).roll(u32::MAX);
    let mut query = world.query::<(&mut Inventory, &mut Outgoing)>();
    let Ok((mut inventory, mut out)) = query.get_mut(world, player) else { return value };
    let left = inventory.mend(value, |count| pick % count.max(1));
    if left != value && inventory.sent {
        inventory.send_all(&mut out, false);
    }
    left
}

/// A player wearing armour with Thorns was hit by `attacker`, a player or a mob: each such piece
/// may hurt it back, each hurt its own, and wears by the hit (BDS `ProtectionEnchant::doPostHurt`).
pub(crate) fn thorns(world: &mut World, wearer: Entity, attacker: Entity) {
    let levels = world.get::<Inventory>(wearer).map_or([0; 4], |inventory| inventory.worn_tags().map(|tag| enchant::level(tag, Enchant::Thorns)));
    if levels == [0; 4] || wearer == attacker {
        return;
    }
    let mut pricked = [None; 4];
    for (piece, level) in levels.into_iter().enumerate().filter(|(_, level)| *level > 0) {
        let back = {
            let mut dice = world.get_resource_or_insert_with(Dice::default);
            enchant::thorns(level, &mut || dice.fraction())
        };
        pricked[piece] = Some(back.is_some());
        if let Some(back) = back
            && !crate::vitals::thorned(world, attacker, back)
        {
            crate::creatures::thorned(world, attacker, back);
        }
    }
    inventory::wear_thorny(world, wearer, pricked);
}
