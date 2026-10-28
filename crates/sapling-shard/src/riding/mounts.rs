//! Mobs that take a rider (docs/research/vehicles.md "Rideable mobs"): a saddle or horse armour
//! put on by hand, getting on by a click, and how a ridden pig, strider or camel is steered. A
//! tamed and saddled horse, donkey or mule is moved by its rider's client (drive.rs). Tamed and
//! saddled are the mob's raising (`creatures::Raised`); getting on a wild mount tries to tame it.
// A tamed mount's inventory, and a chest put on one, are packs.rs's.
// TODO: a wild mount that is not tamed throws its rider off at once; BDS lets it ride a while first.

use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_world::ItemStack;

use super::{Reins, Seats, packets};
use crate::body::Body;
use crate::components::{NetId, Position, Tick};
use crate::creatures::Raised;
use crate::entities::{self, Kind};
use crate::inventory::Inventory;
use crate::movement::Motion;
use crate::{blocks, creatures};

const SADDLE: &str = "minecraft:saddle";
/// A player reaches a mob to get on it from this far, eyes to feet (acacia's riding notes).
const REACH: f32 = 6.7;
const EYES: f32 = 1.62;
/// Saddled whether tamed or not; the horse family wants taming first.
const ALWAYS_SADDLED: &[&str] = &["minecraft:pig", "minecraft:strider", "minecraft:camel"];
const TAMED_SADDLED: &[&str] = &["minecraft:horse", "minecraft:donkey", "minecraft:mule", "minecraft:skeleton_horse", "minecraft:zombie_horse"];
/// A sneaking player's click does not get it on these (`crouching_skip_interact`).
const SNEAK_SKIPS: &[&str] = &["minecraft:strider", "minecraft:camel"];

/// The steering item, `minecraft:movement` while ridden, a boost's ticks and the uses it costs,
/// and how long the item lasts.
struct Steered {
    kind: &'static str,
    stick: &'static str,
    speed: f32,
    boost_ticks: u64,
    boost_wear: i32,
    lasts: u16,
}

const BOOST: f32 = 1.35;
const STEERED: [Steered; 2] = [
    Steered { kind: "minecraft:pig", stick: "minecraft:carrot_on_a_stick", speed: 0.25, boost_ticks: 60, boost_wear: 2, lasts: 25 },
    Steered { kind: "minecraft:strider", stick: "minecraft:warped_fungus_on_a_stick", speed: 0.16 * 1.45, boost_ticks: 320, boost_wear: 1, lasts: 100 },
];
const CAMEL: &str = "minecraft:camel";
const CAMEL_SPEED: f32 = 0.09;

/// A mob's armour once a player put it on, and the tick its boost ends.
#[derive(Component, Default, Clone)]
pub(crate) struct Tack {
    pub armor: Option<ItemStack>,
    boosted_until: u64,
}

/// Whether the mob has a saddle on.
pub(crate) fn saddled(world: &World, mob: Entity) -> bool {
    world.get::<Raised>(mob).is_some_and(|raised| raised.saddled)
}

fn tamed(world: &World, mob: Entity) -> bool {
    world.get::<Raised>(mob).is_some_and(Raised::is_tamed)
}

/// Whether a tamed, saddled horse's rider's client moves it (acacia's riding notes).
pub(crate) fn predicted(world: &World, mob: Entity, kind: &str) -> bool {
    TAMED_SADDLED.contains(&kind) && tamed(world, mob) && saddled(world, mob)
}

fn held(world: &World, player: Entity) -> Option<ItemStack> {
    world.get::<Inventory>(player)?.in_hand().as_ref().map(|held| held.stack.clone())
}

/// Whether `target` is within a player's reach for a click.
pub(crate) fn reaches(world: &World, player: Entity, target: Entity) -> bool {
    let (Some(at), Some(body)) = (world.get::<Position>(player), world.get::<Body>(target)) else { return false };
    let eyes = [at.feet[0], at.feet[1] + EYES, at.feet[2]];
    eyes.iter().zip(body.at).map(|(a, b)| (a - b).powi(2)).sum::<f32>() <= REACH * REACH
}

/// A player's click on `target`, once plugins let it through and nothing else took it
/// (vehicles.rs: a chest opens): tack goes on, or the player gets on.
pub(crate) fn used_on(world: &mut World, player: Entity, target: Entity) -> bool {
    let Some(kind) = world.get::<Kind>(target).map(|kind| kind.0.clone()) else { return false };
    if !reaches(world, player, target) {
        return false;
    }
    if put_on(world, player, target, &kind) {
        return true;
    }
    let sneaking = world.get::<Motion>(player).is_some_and(|motion| motion.pose.sneaking);
    if super::packs::clicked(world, player, target, &kind, sneaking) {
        return true;
    }
    if sneaking && (SNEAK_SKIPS.contains(&kind.as_str()) || (TAMED_SADDLED.contains(&kind.as_str()) && tamed(world, target))) {
        return false;
    }
    let wild = !tamed(world, target) && sapling_data::husbandry(&kind).is_some_and(|rules| rules.temper.is_some());
    if super::seat(world, player, target).is_err() {
        return false;
    }
    if wild && !creatures::tame_by_riding(world, target, player) {
        super::unseat(world, player, true);
    }
    true
}

/// Puts the saddle, the horse armour or the llama's carpet a player holds on `mob`; false when
/// it takes none of them now.
fn put_on(world: &mut World, player: Entity, mob: Entity, kind: &str) -> bool {
    let Some(stack) = held(world, player) else { return false };
    let grown = !creatures::is_baby(world, mob);
    let mut tack = world.get::<Tack>(mob).cloned().unwrap_or_default();
    let saddles = ALWAYS_SADDLED.contains(&kind) || (TAMED_SADDLED.contains(&kind) && tamed(world, mob));
    let sound = if stack.name == SADDLE && saddles && grown && !saddled(world, mob) {
        if let Some(mut raised) = world.get_mut::<Raised>(mob) {
            raised.saddled = true;
        }
        "saddle"
    } else if super::packs::takes(kind, 1, &stack.name) && grown && tamed(world, mob) && tack.armor.is_none() {
        tack.armor = Some(ItemStack { count: 1, ..stack });
        "armor.equip_generic"
    } else {
        return false;
    };
    blocks::spend_held(world, player);
    world.entity_mut(mob).insert(tack);
    let (Some(id), Some(body)) = (world.get::<NetId>(mob).map(|id| id.0), world.get::<Body>(mob).copied()) else { return true };
    let mut told = worn(world, mob);
    told.push(packets::sound(sound, id, kind, body.at));
    super::tell(world, mob, mob, told);
    super::packs::tack_changed(world, mob);
    super::retell(world, mob);
    true
}

/// What shows a mob's tack: its flags, with `SADDLED`, and the armour or carpet on its body.
pub(super) fn worn(world: &mut World, mob: Entity) -> Vec<Bytes> {
    let Some(tack) = world.get::<Tack>(mob).cloned() else { return Vec::new() };
    let (Some(id), Some(kind)) = (world.get::<NetId>(mob).map(|id| id.0), world.get::<Kind>(mob).map(|kind| kind.0.clone())) else { return Vec::new() };
    let word = entities::mob_flags(world, mob, &kind, acacia_proto::types::MetadataFlags1::SADDLED, saddled(world, mob));
    let mut shown = vec![packets::flags(id, word)];
    if let Some(armor) = &tack.armor {
        let slots = [None, None, None, None, Some(crate::inventory::stack(armor))];
        shown.push(sapling_net::encode(&sapling_data::mob_armor(id, &slots)));
    }
    shown
}

/// How a ridden mob moves this tick.
pub(crate) enum Rein {
    /// As it would unridden.
    Free,
    /// It stands.
    Held,
    /// It walks towards this place at this speed.
    Walk([f32; 2], f32),
}

/// How the rider of `mob` steers it.
pub(crate) fn steers(world: &World, mob: Entity, kind: &str, body: &Body) -> Rein {
    let Some(driver) = world.get::<Seats>(mob).and_then(|seats| seats.0.first().copied()) else { return Rein::Free };
    let (Some(reins), Some(at)) = (world.get::<Reins>(driver), world.get::<Position>(driver)) else { return Rein::Free };
    let toward = |[across, forward]: [f32; 2], speed: f32| {
        let (sin, cos) = at.yaw.to_radians().sin_cos();
        let [dx, dz] = [across * cos - forward * sin, across * sin + forward * cos];
        Rein::Walk([body.at[0] + dx * 4.0, body.at[2] + dz * 4.0], speed)
    };
    if let Some(steered) = STEERED.iter().find(|steered| steered.kind == kind) {
        if !held(world, driver).is_some_and(|stack| stack.name == steered.stick) {
            return Rein::Free;
        }
        let boosted = world.get::<Tack>(mob).is_some_and(|tack| tack.boosted_until > world.resource::<Tick>().0);
        return toward([0.0, 1.0], steered.speed * if boosted { BOOST } else { 1.0 });
    }
    if kind == CAMEL && saddled(world, mob) {
        return if reins.keys == [0.0; 2] { Rein::Held } else { toward(reins.keys, CAMEL_SPEED) };
    }
    if TAMED_SADDLED.contains(&kind) { Rein::Held } else { Rein::Free }
}

/// A rider using the stick that steers what it drives boosts it and wears the stick; false when
/// it drives nothing that stick steers.
// TODO: unverified that BDS boosts on the stick's use in the air; a worn-out stick is not left as a fishing rod.
pub(crate) fn boost(world: &mut World, player: Entity) -> bool {
    let Some(mob) = super::driven_by(world, player) else { return false };
    let Some(kind) = world.get::<Kind>(mob).map(|kind| kind.0.clone()) else { return false };
    let Some(steered) = STEERED.iter().find(|steered| steered.kind == kind && held(world, player).is_some_and(|stack| stack.name == steered.stick)) else { return false };
    let now = world.resource::<Tick>().0;
    let mut tack = world.get::<Tack>(mob).cloned().unwrap_or_default();
    if tack.boosted_until > now {
        return true;
    }
    tack.boosted_until = now + steered.boost_ticks;
    world.entity_mut(mob).insert(tack);
    crate::inventory::wear_hand_by(world, player, steered.boost_wear, steered.lasts);
    true
}
