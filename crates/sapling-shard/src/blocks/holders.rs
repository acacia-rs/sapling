//! Blocks that hold one thing put into them by hand: an item frame takes an item and later
//! clicks turn it, a decorated pot takes a stack one at a time (docs/research/block-uses.md, "A
//! second round"). Each keeps it as BDS does, in its entity.
// TODO: a comparator reads no frame, as BDS read none lying flat; a wall frame was not drilled. A
// frame shows no map; a pot gives nothing back but by being broken.

use acacia_proto::nbt::Value;
use acacia_proto::packets::{LevelEvent, LevelEventEvent};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, GameMode};
use sapling_world::{ItemStack, new_block_entity};

use super::send;
use crate::components::{Layer, Mode};
use crate::inventory::{self, Inventory, Place};

const FRAMES: [(&str, &str); 2] = [("minecraft:frame", "ItemFrame"), ("minecraft:glow_frame", "GlowItemFrame")];
const POT: &str = "minecraft:decorated_pot";
pub(super) const FRAMED: &str = "Item";
pub(super) const POTTED: &str = "item";
const TURN: &str = "ItemRotation";
const EIGHTH: f32 = 45.0;
const STACK: u8 = 64;

/// The entity of the block at `pos`, or a new one of `kind`.
fn entity(world: &World, layer: Entity, pos: BlockPos, kind: &str) -> Value {
    let kept = world.get::<Layer>(layer).and_then(|layer| layer.world.block_entity(pos)).map(|entity| entity.nbt.clone());
    kept.unwrap_or_else(|| new_block_entity(kind, pos))
}

fn put(entity: &mut Value, key: &str, value: Value) {
    if let Value::Compound(fields) = entity {
        fields.retain(|(name, _)| name != key);
        fields.push((key.into(), value));
    }
}

fn held(world: &World, player: Entity) -> Option<ItemStack> {
    world.get::<Inventory>(player).and_then(|inventory| inventory.in_hand().as_ref()).map(|held| held.stack.clone())
}

fn spend(world: &mut World, player: Entity) {
    if world.get::<Mode>(player).is_none_or(|mode| mode.0 != GameMode::Creative) {
        inventory::change(world, player, |inventory| inventory.take(Place::Main(inventory.selected.into()), 1).is_ok());
    }
}

/// The item `stack` as a block's entity keeps it.
fn kept(world: &World, layer: Entity, stack: ItemStack) -> Option<Value> {
    world.get::<Layer>(layer).map(|layer| Value::Compound(stack.fields(layer.world.registry())))
}

/// What the decorated pot at `pos` holds, for a comparator; `None` where there is no pot.
pub(super) fn potted(world: &World, layer: Entity, pos: BlockPos) -> Option<Option<ItemStack>> {
    let level = world.get::<Layer>(layer).map(|layer| &layer.world).filter(|level| level.in_height(pos[1]))?;
    level.registry().get(level.block(pos[0], pos[1], pos[2])).filter(|state| state.name == POT)?;
    Some(level.block_entity(pos).and_then(|entity| entity.nbt.get(POTTED).and_then(ItemStack::parse)).filter(|inside| inside.count > 0))
}

/// A hit on the block at `pos`: a frame that holds an item lets it go and stands, as BDS's did
/// at the first blow. False for anything else.
pub(super) fn hit(world: &mut World, layer: Entity, pos: BlockPos) -> bool {
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return false };
    let name = level.registry().get(level.block(pos[0], pos[1], pos[2])).map(|state| state.name);
    let Some(mut fields) = level.block_entity(pos).map(|entity| entity.nbt.clone()).filter(|_| FRAMES.iter().any(|(frame, _)| Some(*frame) == name)) else { return false };
    let Some(item) = fields.get(FRAMED).and_then(ItemStack::parse) else { return false };
    if let Value::Compound(fields) = &mut fields {
        fields.retain(|(key, _)| key != FRAMED && key != TURN);
    }
    send::keep_entity(world, layer, pos, &fields);
    crate::drops::spill(world, layer, item, pos);
    let corner = Vec3f { x: pos[0] as f32, y: pos[1] as f32, z: pos[2] as f32 };
    send::to_viewers(world, layer, pos, &[sapling_net::encode(&LevelEvent { event: LevelEventEvent::SoundItemframeRemoveItem, position: corner, data: 0 })]);
    true
}

/// A click on the block `name` at `pos`: false when it is neither a frame nor a decorated pot.
pub(super) fn use_holder(world: &mut World, player: Entity, layer: Entity, pos: BlockPos, name: &str) -> bool {
    let corner = Vec3f { x: pos[0] as f32, y: pos[1] as f32, z: pos[2] as f32 };
    if let Some((_, kind)) = FRAMES.iter().find(|(frame, _)| *frame == name) {
        let mut fields = entity(world, layer, pos, kind);
        let event = match fields.get(FRAMED).map(|_| fields.get(TURN).cloned()) {
            Some(turned) => {
                let turned = if let Some(Value::Float(turned)) = turned { turned } else { 0.0 };
                put(&mut fields, TURN, Value::Float((turned + EIGHTH) % 360.0));
                LevelEventEvent::SoundItemframeRotateItem
            }
            None => {
                let Some(item) = held(world, player).and_then(|stack| kept(world, layer, ItemStack { count: 1, ..stack })) else { return true };
                put(&mut fields, FRAMED, item);
                put(&mut fields, TURN, Value::Float(0.0));
                spend(world, player);
                LevelEventEvent::SoundItemframeAddItem
            }
        };
        send::keep_entity(world, layer, pos, &fields);
        send::to_viewers(world, layer, pos, &[sapling_net::encode(&LevelEvent { event, position: corner, data: 0 })]);
        return true;
    }
    if name != POT {
        return false;
    }
    let Some(stack) = held(world, player) else { return true };
    let mut fields = entity(world, layer, pos, "DecoratedPot");
    let inside = fields.get(POTTED).and_then(ItemStack::parse).filter(|inside| inside.count > 0);
    let count = match inside {
        Some(inside) if inside.name == stack.name && inside.count < STACK => inside.count + 1,
        Some(_) => return true,
        None => 1,
    };
    let Some(item) = kept(world, layer, ItemStack { count, ..stack }) else { return true };
    put(&mut fields, POTTED, item);
    send::keep_entity(world, layer, pos, &fields);
    super::container_changed(world, layer, pos);
    spend(world, player);
    // Higher the fuller it is: BDS's first stone told 707, its second 715.
    let pitch = (700.0 + 500.0 * f32::from(count) / f32::from(STACK)) as i32;
    send::to_viewers(world, layer, pos, &[send::noise("block.decorated_pot.insert", corner, pitch)]);
    true
}
