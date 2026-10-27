//! Firework rockets, as BDS's `FireworksRocketActor` flew (docs/research/wither-lightning-fireworks.md,
//! "Fireworks"): launched off a block, by a dispenser (`dispense`) or from a crossbow
//! (shots/crossbows.rs) along a way, each starts at 0.03 a tick that way, goes 1.05 times faster
//! and 0.03 more each tick, and bursts (burst.rs) after 10 ticks a unit of `Flight` and one more,
//! and up to 5 and 6 more. One used while gliding rides along with its glider until it bursts
//! (gliding.rs). Their recipes: recipes.rs; their item tags: item.rs.

mod burst;
mod item;
mod recipes;

use acacia_proto::nbt::{self, Nbt, Network, Raw, Value};
use acacia_proto::packets::LevelSoundEvent;
use acacia_proto::types::{MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as MetaValue, MetadataDictionaryItemValueDefault as Plain, MetadataFlags1, Vec3f};
use bevy_ecs::prelude::*;
use bytes::BytesMut;
use sapling_api::{BlockPos, Face};
use sapling_data::AddEntityOptions;
use sapling_world::ItemStack;

use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{InLayer, Layer, NetId, Tick};
use crate::sync::Told;
use crate::{entities, inventory};

pub(crate) use item::ROCKET;
pub(crate) use recipes::{crafted, dye};

pub(crate) const KIND: &str = "minecraft:fireworks_rocket";
const SIZE: [f32; 2] = [0.25, 0.25];
/// A rocket's life: this many ticks a unit of `Flight` and one more, and a roll of each of these more.
const LIFE_PER_FLIGHT: u64 = 10;
const LIFE_SPREAD: [u32; 2] = [6, 7];
/// Each tick it goes this many times faster, and this much more along its way.
const SPEEDS_UP: f32 = 1.05;
const PUSH: f32 = 0.03;
/// It starts off its way by up to about this much each way.
const WOBBLE: f32 = 0.001;

/// A flying rocket: what it was made as, the tick it bursts, the way it flies, and the glider
/// it boosts, whose place it keeps.
#[derive(Component)]
struct Rocket {
    item: ItemStack,
    born: u64,
    bursts: u64,
    way: [f32; 3],
    glider: Option<Entity>,
}

/// The item a rocket is shown carrying: its stack's compound, as the network writes NBT.
fn display_item(world: &World, layer: Entity, stack: &ItemStack) -> Option<MetadataDictionaryItem> {
    let registry = world.get::<Layer>(layer)?.world.registry();
    let mut out = BytesMut::new();
    nbt::write::<Network>(&mut out, &Nbt { name: String::new(), value: Value::Compound(stack.fields(registry)) });
    let raw = Raw::<Network>::read(&mut &out[..]).ok()?;
    Some(MetadataDictionaryItem { key: Key::MinecartDisplayBlock, r#type: Type::Compound, legacy_type: 5, value: MetaValue::Default(Plain::Compound(raw)) })
}

/// Launches a rocket made as `stack` from `at` along the unit `way` in `layer`; the rocket.
pub(crate) fn launch(world: &mut World, layer: Entity, at: [f32; 3], way: [f32; 3], stack: &ItemStack) -> Option<Entity> {
    launch_with(world, layer, at, way, stack, None).map(|(rocket, _)| rocket)
}

/// Sets off a rocket made as `stack` at the feet of `glider`, in `layer`, to ride along with it;
/// the ticks it lasts.
pub(crate) fn boost(world: &mut World, layer: Entity, glider: Entity, stack: &ItemStack) -> Option<u64> {
    let at = world.get::<crate::components::Position>(glider)?.feet;
    launch_with(world, layer, at, [0.0; 3], stack, Some(glider)).map(|(_, lasts)| lasts)
}

/// The rocket and the ticks it lasts.
fn launch_with(world: &mut World, layer: Entity, at: [f32; 3], way: [f32; 3], stack: &ItemStack, glider: Option<Entity>) -> Option<(Entity, u64)> {
    let shown = display_item(world, layer, stack);
    let motion: [f32; 3] = {
        let mut dice = world.get_resource_or_insert_with(Dice::default);
        std::array::from_fn(|axis| (dice.fraction() + dice.fraction() - 1.0) * WOBBLE + way[axis] * PUSH)
    };
    let [yaw, pitch] = crate::creatures::facing(motion);
    let announce = |id: u64| {
        let options = AddEntityOptions { id, identifier: KIND, feet: at, yaw, pitch, variant: 0, mark_variant: 0, color: 0, baby: false, flags: MetadataFlags1(0), health: None, name: "" };
        let mut added = sapling_data::add_entity(&options);
        added.velocity = Vec3f { x: motion[0], y: motion[1], z: motion[2] };
        if let Some(item) = shown {
            added.metadata.retain(|kept| kept.key != item.key);
            added.metadata.push(item);
        }
        Some(vec![sapling_net::encode(&added)])
    };
    let rocket = entities::spawn(world, layer, KIND, Body::new(at, motion, SIZE), at, announce)?;
    let now = world.resource::<Tick>().0;
    let spread = {
        let mut dice = world.get_resource_or_insert_with(Dice::default);
        u64::from(dice.roll(LIFE_SPREAD[0]) + dice.roll(LIFE_SPREAD[1]))
    };
    let lasts = LIFE_PER_FLIGHT * (u64::from(item::flight(stack)) + 1) + spread;
    world.entity_mut(rocket).insert(Rocket { item: ItemStack { count: 1, ..stack.clone() }, born: now, bursts: now + lasts, way, glider });
    let [x, y, z] = at;
    let sound = LevelSoundEvent { sound_id: "launch".into(), position: Vec3f { x, y, z }, extra_data: -1, entity_type: KIND.into(), is_baby_mob: false, is_global: false, entity_unique_id: -1, fire_at_position: None };
    entities::after_announcing(world, layer, at.map(|c| c.floor() as i32), vec![sapling_net::encode(&sound)]);
    Some((rocket, lasts))
}

/// A player's click on `face` of the block at `pos` launches the rocket in its hand up from the
/// face; false where it holds none or none could go.
pub(crate) fn launch_held(world: &mut World, player: Entity, pos: BlockPos, face: Face) -> bool {
    let held = world.get::<inventory::Inventory>(player).and_then(|inventory| inventory.in_hand().as_ref()).map(|held| held.stack.clone());
    let (Some(stack), Some(layer)) = (held.filter(|stack| stack.name == ROCKET), world.get::<InLayer>(player).map(|layer| layer.0)) else { return false };
    let normal = face.offset().map(|c| c as f32);
    let at: [f32; 3] = std::array::from_fn(|axis| pos[axis] as f32 + 0.5 + normal[axis] * 0.5);
    launch(world, layer, at, [0.0, 1.0, 0.0], &stack).is_some()
}

/// A dispenser at `pos` facing `facing` shoots `stack` out of its face, along it.
pub(crate) fn dispense(world: &mut World, layer: Entity, pos: BlockPos, facing: Face, stack: &ItemStack) -> Option<Entity> {
    let way = facing.offset().map(|c| c as f32);
    let mut at: [f32; 3] = std::array::from_fn(|axis| pos[axis] as f32 + 0.5 + way[axis] * (0.5 + SIZE[0] / 2.0));
    if way[1] != 0.0 {
        at[1] -= SIZE[1] / 2.0;
    }
    launch(world, layer, at, way, stack)
}

/// A tick of one rocket: it bursts when its time is up, else speeds on.
fn fly(world: &mut World, rocket: Entity, layer: Entity, id: u64, now: u64) -> Option<()> {
    let mut body = *world.get::<Body>(rocket)?;
    let (bursts, way, glider) = world.get::<Rocket>(rocket).map(|kept| (kept.bursts, kept.way, kept.glider))?;
    if now > bursts {
        return burst::burst(world, rocket, layer);
    }
    match glider.and_then(|glider| Some((world.get::<crate::components::Position>(glider)?.feet, world.get::<crate::movement::Motion>(glider)?.delta))) {
        Some((feet, delta)) => (body.at, body.motion) = (feet, delta),
        None if glider.is_some() => return burst::burst(world, rocket, layer),
        None => {
            body.motion = std::array::from_fn(|axis| body.motion[axis] * SPEEDS_UP + way[axis] * PUSH);
            body.shove(&world.get::<Layer>(layer)?.world, body.motion);
        }
    }
    *world.get_mut::<Body>(rocket)? = body;
    let packets = world.get_mut::<Told>(rocket)?.update(id, body.at, body.motion, false);
    entities::tell_viewers(world, rocket, layer, body.at.map(|c| c.floor() as i32), packets);
    Some(())
}

/// Moves every rocket a tick.
pub(crate) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let rockets: Vec<(Entity, Entity, u64)> = world.query_filtered::<(Entity, &InLayer, &NetId), With<Rocket>>().iter(world).map(|(rocket, layer, id)| (rocket, layer.0, id.0)).collect();
    for (rocket, layer, id) in rockets {
        if world.get::<Rocket>(rocket).is_some_and(|kept| kept.born != now) {
            fly(world, rocket, layer, id, now);
        }
    }
}
