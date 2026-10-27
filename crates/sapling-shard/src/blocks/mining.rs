//! How long a survival break takes, and whether a client that says it finished one took that
//! long. The time is vanilla's, as acacia-bot computes it for its own breaks
//! (acacia-bot `interact/break_time.rs` has the sources): the held tool's speed on the block's
//! material, hardness x 30 ticks when the tool harvests the block and x 100 when it does not.
// TODO: no haste, mining fatigue, water or mid-air slowdown: the server has no effects and only a
// rough on-ground flag, and each would only make a break slower than what is asked for here.

use acacia_proto::packets::{LevelEvent, LevelEventEvent};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_data::enchant::{self, Enchant};
use sapling_world::{Material, Mining, Tool, ToolKind, ToolTier};

use super::loot::Breaker;
use super::send;
use crate::components::{InLayer, Layer, Tick};
use crate::inventory::Inventory;

/// Ticks a break may arrive early: an input can be a tick or two ahead of the shard's count.
const LEEWAY: u64 = 3;
/// `BlockStartBreak` carries this over the break's ticks: the crack's growth per tick.
const CRACK_SCALE: u32 = 65535;
const HITS_EVERY: u64 = 5;

/// The break a player has started and not finished.
#[derive(Component, Default)]
pub(crate) struct Breaking(Option<Started>);

struct Started {
    pos: BlockPos,
    tick: u64,
    needed: u32,
}

/// The speed of `tool` on `material`; `None` when it breaks the block at once.
fn tool_speed(material: Material, tool: Option<Tool>) -> Option<f32> {
    use {Material as M, ToolKind as K};
    let Some(tool) = tool else { return Some(1.0) };
    let tier = tool.tier.map_or(1.0, ToolTier::speed);
    Some(match (material, tool.kind) {
        (M::SwordInstant, K::Sword) => return None,
        (M::Cobweb, K::Sword | K::Shears) | (M::Leaves, K::Shears) => 15.0,
        (M::Wool, K::Shears) => 5.0,
        (M::Vine, K::Shears) => 2.0,
        (M::Leaves | M::Plant | M::Gourd | M::Vine, K::Sword) => 1.5,
        (M::Pickaxe, K::Pickaxe) | (M::Shovel, K::Shovel) | (M::Axe | M::Plant | M::Gourd | M::Vine, K::Axe) | (M::Hoe | M::Leaves, K::Hoe) => tier,
        _ => 1.0,
    })
}

/// Ticks from StartBreak until the block breaks (1: at once); `None` for a block that cannot be
/// broken or whose hardness is not known.
pub(crate) fn break_ticks(mining: &Mining, tool: Option<Tool>, efficiency: u8) -> Option<u32> {
    if mining.hardness.is_nan() || mining.is_unbreakable() {
        return None;
    }
    let harvests = mining.can_harvest(tool);
    let Some(mut speed) = tool_speed(mining.material, tool) else { return Some(1) };
    if !harvests {
        speed = 1.0;
    }
    if speed > 1.0 {
        speed += enchant::efficiency(efficiency);
    }
    let ticks = f64::from(mining.hardness) * if harvests { 30.0 } else { 100.0 } / f64::from(speed);
    // Hardnesses like 0.6 are not exact: without the epsilon 18.0000007 would round up to 19.
    Some((ticks - 1e-4).ceil().max(1.0) as u32)
}

/// The tool in a player's hand and its levels of `enchants`.
fn held_tool<const N: usize>(world: &World, player: Entity, enchants: [Enchant; N]) -> (Option<Tool>, [u8; N]) {
    let Some(held) = world.get::<Inventory>(player).and_then(|inventory| inventory.in_hand().as_ref()) else { return (None, [0; N]) };
    (Tool::from_identifier(&held.stack.name), enchants.map(|enchant| enchant::level(held.stack.tag.as_ref(), enchant)))
}

/// What the tool in a player's hand means for the drop of a block of `mining`.
pub(crate) fn breaker(world: &World, player: Entity, mining: &Mining) -> Breaker {
    let (tool, [silk_touch, fortune]) = held_tool(world, player, [Enchant::SilkTouch, Enchant::Fortune]);
    let is = |kind: ToolKind| tool.is_some_and(|tool| tool.kind == kind);
    Breaker { harvests: mining.can_harvest(tool), silk_touch: silk_touch > 0, shears: is(ToolKind::Shears), pickaxe: is(ToolKind::Pickaxe), fortune, by_entity: true }
}

/// [`breaker`] for a tool not in a hand, as `/loot ... mine` names one: an unknown item is none.
pub(crate) fn breaker_of(item: Option<&sapling_api::Item>, mining: &Mining) -> Breaker {
    let tool = item.and_then(|item| Tool::from_identifier(&item.name));
    let level = |enchant: Enchant| item.and_then(|item| item.enchantments.iter().find(|(id, _)| *id == enchant as i16)).map_or(0, |(_, level)| (*level).clamp(0, 255) as u8);
    let is = |kind: ToolKind| tool.is_some_and(|tool| tool.kind == kind);
    Breaker { harvests: mining.can_harvest(tool), silk_touch: level(Enchant::SilkTouch) > 0, shears: is(ToolKind::Shears), pickaxe: is(ToolKind::Pickaxe), fortune: level(Enchant::Fortune), by_entity: true }
}

fn crack(event: LevelEventEvent, pos: BlockPos, data: i32) -> bytes::Bytes {
    let [x, y, z] = pos.map(|c| c as f32);
    sapling_net::encode(&LevelEvent { event, position: Vec3f { x, y, z }, data })
}

/// A survival player started on the block at `pos`: the break is timed from now, and those who
/// see the block watch it crack, as from BDS.
pub(crate) fn start(world: &mut World, player: Entity, layer: Entity, pos: BlockPos, mining: &Mining) {
    let (tool, [efficiency]) = held_tool(world, player, [Enchant::Efficiency]);
    // The sword and shears speeds are the Java game's, never faster than Bedrock's but perhaps
    // slower: a break with one of them is taken as soon as it is claimed.
    let is_unsure = tool.is_some_and(|tool| matches!(tool.kind, ToolKind::Sword | ToolKind::Shears));
    let needed = break_ticks(mining, tool, efficiency).map(|ticks| if is_unsure { 1 } else { ticks });
    let tick = world.resource::<Tick>().0;
    if let Some(mut breaking) = world.get_mut::<Breaking>(player) {
        breaking.0 = needed.map(|needed| Started { pos, tick, needed });
    }
    // The first blow lights redstone ore (research/captures/bds-oresteps.txt).
    super::light_ore(world, layer, pos);
    if let Some(needed) = needed {
        send::to_viewers(world, layer, pos, &[crack(LevelEventEvent::BlockStartBreak, pos, (CRACK_SCALE / needed) as i32)]);
        hit(world, player, layer, pos);
    }
}

/// The breaker's blow on the block at `pos`, heard by those who see it but not the breaker.
fn hit(world: &mut World, player: Entity, layer: Entity, pos: BlockPos) {
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return };
    let sound = send::sound("hit", level.registry(), pos, level.block(pos[0], pos[1], pos[2]));
    crate::view::to_viewers_but(world, layer, pos, Some(player), &[sound]);
}

/// Each break under way is heard hit on every [`HITS_EVERY`]th tick from its start, as BDS's were
/// (bds-oresteps.txt, bds-toil.txt: one at the start, then one every fifth input).
pub(crate) fn sound_hits(world: &mut World) {
    let tick = world.resource::<Tick>().0;
    let mut breakers = world.query::<(Entity, &Breaking, &InLayer)>();
    let due: Vec<_> = breakers.iter(world).filter_map(|(player, breaking, layer)| breaking.0.as_ref().filter(|started| tick > started.tick && (tick - started.tick) % HITS_EVERY == 0).map(|started| (player, layer.0, started.pos))).collect();
    for (player, layer, pos) in due {
        hit(world, player, layer, pos);
    }
}

/// The player gave its break up.
pub(crate) fn abort(world: &mut World, player: Entity, layer: Entity) {
    let Some(started) = world.get_mut::<Breaking>(player).and_then(|mut breaking| breaking.0.take()) else { return };
    send::to_viewers(world, layer, started.pos, &[crack(LevelEventEvent::BlockStopBreak, started.pos, 0)]);
}

/// Whether the player started on the block at `pos` long enough ago to have broken it now. The
/// started break ends either way.
pub(crate) fn finish(world: &mut World, player: Entity, pos: BlockPos) -> bool {
    let tick = world.resource::<Tick>().0;
    let started = world.get_mut::<Breaking>(player).and_then(|mut breaking| breaking.0.take());
    started.is_some_and(|started| started.pos == pos && tick - started.tick + LEEWAY >= u64::from(started.needed))
}

#[cfg(test)]
mod tests {
    use sapling_world::BlockRegistry;

    use super::*;

    fn ticks(block: &str, tool: &str, efficiency: u8) -> Option<u32> {
        let registry = BlockRegistry::vanilla();
        let mining = registry.states_of(block).next().expect("a vanilla block").1.mining;
        break_ticks(&mining, Tool::from_identifier(tool), efficiency)
    }

    #[test]
    fn a_break_takes_as_long_as_the_block_and_the_tool_make_it() {
        // Stone, hardness 1.5: 7.5 s by hand without a drop, 1.15 s with a wooden pickaxe.
        assert_eq!([ticks("minecraft:stone", "minecraft:stick", 0), ticks("minecraft:stone", "minecraft:wooden_pickaxe", 0)], [Some(150), Some(23)]);
        assert_eq!(ticks("minecraft:stone", "minecraft:diamond_pickaxe", 0), Some(6));
        assert!(ticks("minecraft:stone", "minecraft:diamond_pickaxe", 5) < Some(3));
        assert_eq!([ticks("minecraft:dirt", "minecraft:stick", 0), ticks("minecraft:dirt", "minecraft:iron_shovel", 0)], [Some(15), Some(3)]);
        assert_eq!([ticks("minecraft:bedrock", "minecraft:diamond_pickaxe", 0), ticks("minecraft:torch", "", 0)], [None, Some(1)]);
    }
}
