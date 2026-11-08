//! A happy ghast's harness (its pack's `adult_harnessed`), as BDS did it (research/captures/
//! bds-as3ghast.txt): put on by a click with one in hand, it shows in the body slot, is heard
//! (`armor.equip_generic`) and adds three extended flags; shears take it off a ghast no one rides,
//! tossing it 5 blocks up. A harnessed ghast seats four (seats.rs) and flies where its driver looks
//! (`free_camera_controlled`); its first rider is heard `attach`ing, its last `detach`ing. A player on its
//! top stills it (stilled.rs). Its extended flags are in its announcement for those who see it later.

use acacia_proto::types::{
    EntityProperties, MetadataDictionaryItem as Item, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value,
    MetadataDictionaryItemValueDefault as Plain, MetadataFlags1, MetadataFlags2,
};
use acacia_proto::packets::SetEntityData;
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_world::ItemStack;

use super::{Reins, Seats, packets};
use crate::body::Body;
use crate::components::{InLayer, NetId, Position};
use crate::creatures;
use crate::movement::Motion;

pub(super) const KIND: &str = "minecraft:happy_ghast";
const SHEARS: &str = "minecraft:shears";
/// Shears last this many uses.
const SHEARS_LAST: u16 = 238;
/// The extended flags BDS showed it with unharnessed, harnessed, and after shears (one of the
/// harness's three stayed).
const UNHARNESSED: i64 = 1 << 25 | 1 << 55 | 1 << 60;
const HARNESSED: i64 = UNHARNESSED | 1 << 58 | 1 << 59 | 1 << 61;
const UNHARNESSED_AGAIN: i64 = UNHARNESSED | 1 << 59;
/// Its `flying_speed` (`adult_mobile`), and how much of it going backwards gets.
const FLYING_SPEED: f32 = 0.016;
const BACKWARDS: f32 = 0.5;
/// Its pack's `vertical_movement_action` `vertical_velocity`, added to the driver's intent.
const VERTICAL_VELOCITY: f32 = 0.5;
/// How far over its feet shears toss the harness (its pack's `drop_item_y_offset`) and the item
/// box's half height.
const TOSSED_UP: f32 = 5.0 + 0.125;
/// Its ambient sound interval with riders and without (`adult_with_passengers`, `adult_without_passengers`).
const RIDDEN_INTERVAL: f32 = 30.0;
const ALONE_INTERVAL: f32 = 5.0;

fn is_harness(name: &str) -> bool {
    name.starts_with("minecraft:") && name.ends_with("_harness")
}

/// A happy ghast shears took a harness off: one of the harness's flags stays.
#[derive(Component)]
struct Sheared;

/// Whether the mob is a happy ghast with a harness on.
pub(super) fn harnessed(world: &World, mob: Entity) -> bool {
    super::body::worn(world, mob).is_some_and(|body| is_harness(&body.name))
}

/// The happy ghast's extended flags now: harnessed or not, and still or not.
pub(super) fn extended(world: &World, mob: Entity) -> i64 {
    let worn = match (harnessed(world, mob), world.get::<Sheared>(mob).is_some()) {
        (true, _) => HARNESSED,
        (false, true) => UNHARNESSED_AGAIN,
        (false, false) => UNHARNESSED,
    };
    worn | if super::stilled::is_stilled(world, mob) { super::stilled::STILL } else { 0 }
}

fn extended_item(world: &World, mob: Entity) -> Item {
    Item { key: Key::FlagsExtended, r#type: Type::Long, legacy_type: 7, value: Value::FlagsExtended(MetadataFlags2(extended(world, mob))) }
}

/// Its extended flags as those who see it later are told them, for a happy ghast whose differ from its kind's.
pub(crate) fn shown_flags(world: &World, mob: Entity) -> Option<Item> {
    let ghast = world.get::<crate::entities::Kind>(mob).is_some_and(|kind| kind.0 == KIND);
    (ghast && extended(world, mob) != UNHARNESSED).then(|| extended_item(world, mob))
}

/// The sound at its middle and its extended flags, with its flag word too where `with_flags`.
fn shown(world: &mut World, mob: Entity, id: u64, sound: &str, with_flags: bool) -> Vec<Bytes> {
    let Some(body) = world.get::<Body>(mob).copied() else { return Vec::new() };
    let middle = [body.at[0], body.at[1] + body.size[1] / 2.0, body.at[2]];
    let word = crate::entities::mob_flags(world, mob, KIND, MetadataFlags1(0), false);
    let mut metadata = vec![extended_item(world, mob)];
    if with_flags {
        metadata.insert(0, Item { key: Key::Flags, r#type: Type::Long, legacy_type: 7, value: Value::Flags(word) });
    }
    crate::mobs::reannounce_item(world, mob, extended_item(world, mob));
    let data = SetEntityData { runtime_entity_id: id, metadata, properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 };
    vec![packets::sound(sound, id, KIND, middle), sapling_net::encode(&data)]
}

/// A click with `stack` in hand on a grown, unharnessed happy ghast harnesses it; false where it does not.
pub(super) fn put_on(world: &mut World, player: Entity, mob: Entity, stack: &ItemStack) -> bool {
    if !is_harness(&stack.name) || creatures::is_baby(world, mob) || harnessed(world, mob) {
        return false;
    }
    let Some(id) = world.get::<NetId>(mob).map(|id| id.0) else { return false };
    crate::blocks::spend_held(world, player);
    let harness = ItemStack { count: 1, ..stack.clone() };
    super::body::dress(world, mob, Some(harness.clone()));
    let mut told = vec![super::mounts::body_shown(id, Some(&harness))];
    told.extend(shown(world, mob, id, "armor.equip_generic", true));
    super::tell(world, mob, mob, told);
    super::retell(world, mob);
    true
}

/// A click with shears on a harnessed happy ghast no one rides, by a player not sneaking, tosses its
/// harness; false where it does not.
pub(super) fn take_off(world: &mut World, player: Entity, mob: Entity, stack: &ItemStack) -> bool {
    let sneaking = world.get::<Motion>(player).is_some_and(|motion| motion.pose.sneaking);
    if stack.name != SHEARS || sneaking || world.get::<Seats>(mob).is_some() {
        return false;
    }
    let (Some(harness), Some(id), Some(layer), Some(body)) = (super::body::worn(world, mob).filter(|body| is_harness(&body.name)), world.get::<NetId>(mob).map(|id| id.0), world.get::<InLayer>(mob).map(|layer| layer.0), world.get::<Body>(mob).copied()) else {
        return false;
    };
    super::body::dress(world, mob, None);
    world.entity_mut(mob).insert(Sheared);
    super::tell(world, mob, mob, vec![super::mounts::body_shown(id, None)]);
    // BDS showed the tossed harness before the sound and the flags (bds-as3ghast.txt).
    let tossed = crate::drops::toss(world, layer, harness, [body.at[0], body.at[1] + TOSSED_UP, body.at[2]]);
    let begun = tossed.and_then(|item| crate::mobs::at_once::show_first(world, item));
    let told = shown(world, mob, id, "armor.unequip_generic", false);
    super::tell(world, mob, mob, told);
    if let Some(begun) = begun {
        crate::mobs::at_once::finish(world, begun);
    }
    super::retell(world, mob);
    if world.get::<crate::components::Mode>(player).is_none_or(|mode| mode.0 != sapling_api::GameMode::Creative) {
        crate::inventory::wear_hand_by(world, player, 1, SHEARS_LAST);
    }
    true
}

/// When a happy ghast's riders go from none to one (`joined`) or from one to none: its `attach` or
/// `detach` sound and its new ambient sound interval (`on_passenger_mount`, `on_passenger_dismount`).
pub(super) fn riders_now(world: &World, vehicle: Entity, count: usize, joined: bool) -> Option<(Bytes, Bytes)> {
    if count != usize::from(joined) || world.get::<crate::entities::Kind>(vehicle).is_none_or(|kind| kind.0 != KIND) {
        return None;
    }
    let (Some(id), Some(at)) = (world.get::<NetId>(vehicle).map(|id| id.0), world.get::<Body>(vehicle).map(|body| body.at)) else { return None };
    let (sound, interval) = if joined { ("attach", RIDDEN_INTERVAL) } else { ("detach", ALONE_INTERVAL) };
    let item = Item { key: Key::AmbientSoundInterval, r#type: Type::Float, legacy_type: 3, value: Value::Default(Plain::Float(interval)) };
    let data = SetEntityData { runtime_entity_id: id, metadata: vec![item], properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 };
    Some((packets::sound(sound, id, KIND, at), sapling_net::encode(&data)))
}

/// [`riders_now`] after its last rider left, told next tick: BDS's `on_rider_exit_event` came a tick after
/// the link (bds-as3ghast.txt, bds-n3exit.txt).
pub(super) fn detach_next_tick(world: &mut World, vehicle: Entity, left: usize) {
    let Some((sound, interval)) = riders_now(world, vehicle, left, false) else { return };
    let (Some(layer), Some(body)) = (world.get::<InLayer>(vehicle).map(|layer| layer.0), world.get::<Body>(vehicle)) else { return };
    let at = body.at.map(|c| c.floor() as i32);
    crate::late_sounds::play_later(world, 1, layer, at, sound);
    crate::late_sounds::play_later(world, 1, layer, at, interval);
}

/// What its driver's keys push a harnessed happy ghast by this tick: its flying speed the way the
/// driver looks, pitch and all (bds-as3ghast.txt: looking straight up, it rose 0.016 more each tick
/// after 0.91 of its speed), plus `vertical_movement_action`'s 0.5 up while the driver's input says
/// `Jumping` (bds-ba2ghast.txt: 0.008 a tick alone, 0.016 with a walk up, so the sum is cut to length 1).
pub(super) fn flight(reins: &Reins, at: &Position) -> [f32; 3] {
    let [across, forward] = reins.keys;
    let forward = if forward < 0.0 { forward * BACKWARDS } else { forward };
    let (sin_yaw, cos_yaw) = at.yaw.to_radians().sin_cos();
    let (sin_pitch, cos_pitch) = reins.look[1].to_radians().sin_cos();
    let ahead = [-sin_yaw * cos_pitch, -sin_pitch, cos_yaw * cos_pitch];
    let left = [cos_yaw, 0.0, sin_yaw];
    let mut intent: [f32; 3] = std::array::from_fn(|axis| forward * ahead[axis] + across * left[axis]);
    if reins.jumping {
        intent[1] += VERTICAL_VELOCITY;
    }
    let length = intent.iter().map(|c| c * c).sum::<f32>().sqrt();
    let scale = if length > 1.0 { FLYING_SPEED / length } else { FLYING_SPEED };
    intent.map(|c| c * scale)
}
