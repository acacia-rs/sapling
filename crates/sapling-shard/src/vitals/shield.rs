//! A shield raised by sneaking with it in either hand, as BDS's was (research/captures/
//! bds-shield.txt; BDS `Player::_hurt` 1402b5fe0, `isBlocking` 1402ab9e0): five ticks after it
//! goes up it turns away a hit, a shot or a blast from an entity in front, wearing by one more
//! than the hit; a mob whose hit it turned is thrown back, and an axe's hit puts it down.
// TODO: BDS writes the tick the shield went up into the item (its `blocking_tick`) and shows the
// item anew as it goes up and down; a riding player's is up too; a blocked mob's hit or shot also
// stood the blocker up from its sneak on BDS; Thorns and fireworks are turned away there too.

use acacia_proto::packets::{LevelSoundEvent, SetEntityData};
use acacia_proto::types::{EntityProperties, MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataFlags2, Vec3f, WindowIDVarint};
use bevy_ecs::prelude::*;
use sapling_data::EYE_HEIGHT;

use super::{is_dead, is_mortal, tell};
use crate::components::{NetId, Outgoing, Position, Tick, in_world};
use crate::creatures::Cause;
use crate::inventory::{HOTBAR, Inventory, Place};
use crate::movement::Motion;

const SHIELD: &str = "minecraft:shield";
/// Bits of `FlagsExtended` BDS set: raised, going up or down, and the moment of a block (acacia-proto
/// names them one bit too low: `TRANSITION_BLOCKING` for the first).
const RAISED: i64 = 1 << 8;
const MOVING: i64 = 1 << 9;
const STRUCK: i64 = 1 << 10 | 1 << 11;
/// Ticks the going up or down shows; a shield up no longer than this turns nothing away.
const MOVES: u64 = 2;
const READIES: u64 = 4;
/// Ticks an axe's hit puts the shield down for.
const DISABLED: u64 = 100;
/// Half a sneaking player's height: where the shield's breaking is heard from.
const SNEAKING_MIDDLE: f32 = 0.745;
/// Mobs that hit with an axe.
const AXE_WIELDERS: [&str; 2] = ["minecraft:vindicator", "minecraft:piglin_brute"];
/// A mob whose hit was turned is thrown back as `Mob::knockback` throws: across and up, the cap up,
/// what of its motion it keeps (bds-shield.txt: a zombie's 0.556 across and 0.4 up).
const THROWN_BACK: [f32; 3] = [0.6, 0.6, 0.4];
const KEEPS: f32 = 0.5;
const UP_BY: f32 = 1.2;

/// A player's shield: whether it is up and since when, until when the going up or down shows,
/// until when an axe keeps it down.
#[derive(Component, Default)]
pub(crate) struct Guard {
    raised: Option<u64>,
    moving_until: u64,
    down_until: u64,
}

/// Where a player holds a shield: the off hand first.
fn shield_at(inventory: &Inventory) -> Option<Place> {
    let is_shield = |place: Place| inventory.slot(place).and_then(Option::as_ref).is_some_and(|held| held.stack.name == SHIELD);
    [Place::Offhand, Place::Main(usize::from(inventory.selected) % HOTBAR)].into_iter().find(|place| is_shield(*place))
}

fn extended(id: u64, bits: i64, tick: u64) -> SetEntityData {
    let flags = MetadataFlags2(sapling_data::player_flags_extended().0 | bits);
    let item = MetadataDictionaryItem { key: Key::FlagsExtended, r#type: Type::Long, legacy_type: 7, value: Value::FlagsExtended(flags) };
    SetEntityData { runtime_entity_id: id, metadata: vec![item], properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick }
}

/// Raises and lowers the shields of players who sneak with one, telling them and their viewers.
pub(super) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    for player in in_world(world) {
        let sneaking = world.get::<Motion>(player).is_some_and(|motion| motion.pose.sneaking);
        let holds = world.get::<Inventory>(player).and_then(shield_at).is_some();
        let Some(id) = world.get::<NetId>(player).map(|id| id.0) else { continue };
        if world.get::<Guard>(player).is_none() {
            world.entity_mut(player).insert(Guard::default());
        }
        let alive = !is_dead(world, player);
        let mut guard = world.get_mut::<Guard>(player).expect("put above");
        let raised = sneaking && holds && alive && now >= guard.down_until;
        let told = if raised != guard.raised.is_some() {
            (guard.raised, guard.moving_until) = (raised.then_some(now), now + MOVES);
            Some(MOVING)
        } else if guard.moving_until == now {
            Some(0)
        } else {
            None
        };
        let raised_bit = if raised { RAISED } else { 0 };
        if let Some(moving) = told {
            tell(world, player, &extended(id, raised_bit | moving, now));
        }
    }
}

/// Whether the player's raised shield turns away a hit of `amount` by `cause` from `from`: a
/// hit, a shot or a blast from in front of it. One that does wears the shield, is heard, and
/// shows; a hit with an axe puts the shield down.
pub(crate) fn blocks(world: &mut World, player: Entity, from: [f32; 3], amount: f32, cause: Cause, axe: bool) -> bool {
    let now = world.resource::<Tick>().0;
    let ready = world.get::<Guard>(player).and_then(|guard| guard.raised).is_some_and(|since| now - since > READIES);
    let (Some(position), Some(id)) = (world.get::<Position>(player).copied(), world.get::<NetId>(player).map(|id| id.0)) else { return false };
    let Some(place) = world.get::<Inventory>(player).and_then(shield_at) else { return false };
    if !ready || !is_mortal(world, player) || !matches!(cause, Cause::Attack | Cause::Projectile | Cause::EntityExplosion) || !is_ahead(&position, from) {
        return false;
    }
    let broke = crate::inventory::wear_at(world, player, place, 1 + amount as i32);
    // A shield in the main hand was shown worn only as it went down.
    if place == Place::Offhand {
        let mut query = world.query::<(&Inventory, &mut Outgoing)>();
        if let Ok((inventory, mut out)) = query.get_mut(world, player) {
            inventory.send_window(&mut out, WindowIDVarint::Offhand);
        }
    }
    let [x, y, z] = position.feet;
    tell(world, player, &sound("item.shield.block", [x, y + EYE_HEIGHT, z], "", -1));
    let axe = axe && cause == Cause::Attack;
    if axe {
        tell(world, player, &sound("break", [x, y + SNEAKING_MIDDLE, z], "minecraft:player", id as i64));
    }
    if broke {
        crate::inventory::sound_broken(world, player);
    }
    tell(world, player, &extended(id, RAISED | STRUCK, now));
    tell(world, player, &extended(id, RAISED, now));
    if axe && let Some(mut guard) = world.get_mut::<Guard>(player) {
        guard.down_until = now + DISABLED;
    }
    true
}

/// Whether a mob of `kind` hits with an axe.
pub(crate) fn wields_axe(kind: &str) -> bool {
    AXE_WIELDERS.contains(&kind)
}

/// The motion a mob that moved at `motion` is thrown back at, the way `back`, by a shield that
/// turned its hit.
pub(crate) fn thrown_back(motion: [f32; 3], back: [f32; 2]) -> [f32; 3] {
    let [across, up, cap] = THROWN_BACK;
    [motion[0] * KEEPS + back[0] * across, (motion[1] * KEEPS + up * UP_BY).min(cap), motion[2] * KEEPS + back[1] * across]
}

/// Whether `from` lies in front of where the player looks, across: within a right angle either side.
fn is_ahead(position: &Position, from: [f32; 3]) -> bool {
    let yaw = position.yaw.to_radians();
    let [dx, dz] = [from[0] - position.feet[0], from[2] - position.feet[2]];
    -yaw.sin() * dx + yaw.cos() * dz > 0.0
}

fn sound(name: &str, [x, y, z]: [f32; 3], entity_type: &str, entity_unique_id: i64) -> LevelSoundEvent {
    LevelSoundEvent { sound_id: name.into(), position: Vec3f { x, y, z }, extra_data: -1, entity_type: entity_type.into(), is_baby_mob: false, is_global: false, entity_unique_id, fire_at_position: None }
}
