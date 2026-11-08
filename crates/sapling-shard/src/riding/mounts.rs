//! Mobs that take a rider (docs/research/vehicles.md "Rideable mobs"): a saddle or horse armour
//! put on by hand, getting on by a click, and how a ridden pig, strider or camel is steered. A
//! tamed and saddled horse, donkey or mule is moved by its rider's client (drive.rs). Tamed and
//! saddled are the mob's raising (`creatures::Raised`); a wild mount ridden a while tries to be
//! tamed (breaking.rs).
// A tamed mount's inventory, and a chest put on one, are packs.rs's.

use acacia_proto::types::MetadataFlags1;
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_world::ItemStack;

use super::{Reins, Seats, harness, packets};
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
pub(super) const TAMED_SADDLED: &[&str] = &["minecraft:horse", "minecraft:donkey", "minecraft:mule", "minecraft:skeleton_horse", "minecraft:zombie_horse"];
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
/// Fit to BDS's ridden grown camel, 0.198 blocks a tick walking (research/captures/bds-ak1camel.txt).
const CAMEL_SPEED: f32 = 0.303;

/// A mob's armour or carpet in its inventory's slot 1, what its body slot shows, and the tick its
/// boost ends. Putting on armour fills both; `slot.armor.body` the body alone (riding/body.rs).
#[derive(Component, Default, Clone)]
pub(crate) struct Tack {
    pub armor: Option<ItemStack>,
    pub body: Option<ItemStack>,
    boosted_until: u64,
}

/// The flags a saddle shows on `kind`: with the horse family's `input_ground_controlled` and
/// `can_power_jump`, a camel's `dash_action` (research/captures/bds-mountflags.txt).
pub(crate) fn saddle_flags(kind: &str) -> MetadataFlags1 {
    if kind == harness::KIND {
        return MetadataFlags1(0);
    }
    let controls = match kind {
        CAMEL => MetadataFlags1::WASD_CONTROLLED.0 | MetadataFlags1::CAN_DASH.0,
        _ if TAMED_SADDLED.contains(&kind) => MetadataFlags1::WASD_CONTROLLED.0 | MetadataFlags1::CAN_POWER_JUMP.0,
        _ => 0,
    };
    MetadataFlags1(MetadataFlags1::SADDLED.0 | controls)
}

/// Whether the mob has a saddle on, or a happy ghast its harness.
pub(crate) fn saddled(world: &World, mob: Entity) -> bool {
    world.get::<Raised>(mob).is_some_and(|raised| raised.saddled) || harness::harnessed(world, mob)
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
    if super::inventory::unrideable(world, target, &kind) || super::seat(world, player, target).is_err() {
        return false;
    }
    if wild {
        super::breaking::start(world, target);
    }
    true
}

/// Puts the saddle, the horse armour or the llama's carpet a player holds on `mob`; false when
/// it takes none of them now.
fn put_on(world: &mut World, player: Entity, mob: Entity, kind: &str) -> bool {
    let Some(stack) = held(world, player) else { return false };
    if kind == harness::KIND {
        return harness::take_off(world, player, mob, &stack) || harness::put_on(world, player, mob, &stack);
    }
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
        tack.body = tack.armor.clone();
        "armor.equip_generic"
    } else {
        return false;
    };
    blocks::spend_held(world, player);
    world.entity_mut(mob).insert(tack);
    let (Some(id), Some(body)) = (world.get::<NetId>(mob).map(|id| id.0), world.get::<Body>(mob).copied()) else { return true };
    let mut told = worn(world, mob);
    // A carpet put on is heard as `armor` too, first (research/captures/bds-tack.txt).
    if sound == "armor.equip_generic" {
        told.push(packets::sound("armor", id, kind, body.at));
    }
    told.push(packets::sound(sound, id, kind, body.at));
    super::tell(world, mob, mob, told);
    super::packs::tack_changed(world, mob);
    super::retell(world, mob);
    true
}

/// What shows a mob's tack: its flags, with `SADDLED`, and what its body slot shows.
pub(super) fn worn(world: &mut World, mob: Entity) -> Vec<Bytes> {
    let Some(tack) = world.get::<Tack>(mob).cloned() else { return Vec::new() };
    let (Some(id), Some(kind)) = (world.get::<NetId>(mob).map(|id| id.0), world.get::<Kind>(mob).map(|kind| kind.0.clone())) else { return Vec::new() };
    let word = entities::mob_flags(world, mob, &kind, saddle_flags(&kind), saddled(world, mob));
    let mut shown = vec![packets::flags(id, word)];
    shown.extend(tack.body.as_ref().map(|body| body_shown(id, Some(body))));
    shown
}

/// The `MobArmorEquipment` showing `body` in the mob `id`'s body slot.
pub(super) fn body_shown(id: u64, body: Option<&ItemStack>) -> Bytes {
    let slots = [None, None, None, None, body.map(crate::inventory::stack)];
    sapling_net::encode(&sapling_data::mob_armor(id, &slots))
}

/// How a ridden mob moves this tick.
pub(crate) enum Rein {
    /// As it would unridden.
    Free,
    /// It stands.
    Held,
    /// It walks towards this place at this speed.
    Walk([f32; 2], f32),
    /// It flies, pushed by this.
    Fly([f32; 3]),
}

/// How the rider of `mob` steers it.
pub(crate) fn steers(world: &World, mob: Entity, kind: &str, body: &Body) -> Rein {
    if super::stilled::is_stilled(world, mob) {
        return Rein::Fly([0.0; 3]);
    }
    let Some(driver) = world.get::<Seats>(mob).and_then(|seats| seats.0.first().copied()) else { return Rein::Free };
    if let Some((to, speed)) = creatures::jockey_rein(world, mob, kind, driver) {
        return Rein::Walk(to, speed);
    }
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
    if kind == harness::KIND && saddled(world, mob) {
        return Rein::Fly(harness::flight(reins, at));
    }
    if kind == CAMEL && saddled(world, mob) {
        return if reins.keys == [0.0; 2] { Rein::Held } else { toward(reins.keys, CAMEL_SPEED) };
    }
    if TAMED_SADDLED.contains(&kind) { Rein::Held } else { Rein::Free }
}

/// A rider using the stick that steers what it drives boosts it and wears the stick, on the ground
/// or in the air, a worn-out stick leaving a fishing rod (its pack's `boostable`, as BDS did:
/// research/captures/bds-n3stick.txt); false when it drives nothing that stick steers.
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
    if let (Some(id), Some(body)) = (world.get::<NetId>(mob).map(|id| id.0), world.get::<Body>(mob).copied()) {
        let heard = packets::sound("boost", id, &kind, creatures::head_of(&kind, &body));
        super::tell(world, mob, mob, vec![heard]);
    }
    crate::inventory::wear_hand_leaving(world, player, steered.boost_wear, steered.lasts, "minecraft:fishing_rod");
    true
}
