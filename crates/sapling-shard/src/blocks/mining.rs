//! How long a survival break takes, and whether a client that says it finished one took that
//! long. The time is vanilla's, as acacia-bot computes it for its own breaks
//! (acacia-bot `interact/break_time.rs` has the sources): the held tool's speed on the block's
//! material, hardness x 30 ticks when the tool harvests the block and x 100 when it does not.
// TODO: no haste, mining fatigue, water or mid-air slowdown: the server has no effects and only a
// rough on-ground flag, and each would only make a break slower than what is asked for here.

use acacia_proto::nbt::Value;
use acacia_proto::packets::{LevelEvent, LevelEventEvent};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_world::{Material, Mining, Tool, ToolKind, ToolTier};

use super::loot::Breaker;
use super::send;
use crate::components::Tick;
use crate::inventory::Inventory;

/// Ticks a break may arrive early: an input can be a tick or two ahead of the shard's count.
const LEEWAY: u64 = 3;
/// `ench` ids.
const EFFICIENCY: i16 = 15;
const SILK_TOUCH: i16 = 16;
const FORTUNE: i16 = 18;
/// `BlockStartBreak` carries this over the break's ticks: the crack's growth per tick.
const CRACK_SCALE: u32 = 65535;

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
        (M::Pickaxe, K::Pickaxe)
        | (M::Shovel, K::Shovel)
        | (M::Axe | M::Plant | M::Gourd | M::Vine, K::Axe)
        | (M::Hoe | M::Leaves, K::Hoe) => tier,
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
    let Some(mut speed) = tool_speed(mining.material, tool) else {
        return Some(1);
    };
    if !harvests {
        speed = 1.0;
    }
    if speed > 1.0 && efficiency > 0 {
        speed += f32::from(efficiency).powi(2) + 1.0;
    }
    let ticks = f64::from(mining.hardness) * if harvests { 30.0 } else { 100.0 } / f64::from(speed);
    // Hardnesses like 0.6 are not exact: without the epsilon 18.0000007 would round up to 19.
    Some((ticks - 1e-4).ceil().max(1.0) as u32)
}

/// The tool in a player's hand and its levels of the enchantments `ids` name.
fn held_tool<const N: usize>(
    world: &World,
    player: Entity,
    ids: [i16; N],
) -> (Option<Tool>, [u8; N]) {
    let Some(held) = world
        .get::<Inventory>(player)
        .and_then(|inventory| inventory.in_hand().as_ref())
    else {
        return (None, [0; N]);
    };
    let level = |id: i16| match held.stack.tag.as_ref().and_then(|tag| tag.get("ench")) {
        Some(Value::List(list)) => list
            .items
            .iter()
            .find(|enchantment| enchantment.get("id") == Some(&Value::Short(id)))
            .and_then(|enchantment| enchantment.get("lvl")),
        _ => None,
    };
    let levels = ids.map(|id| {
        if let Some(Value::Short(level)) = level(id) {
            *level as u8
        } else {
            0
        }
    });
    (Tool::from_identifier(&held.stack.name), levels)
}

/// What the tool in a player's hand means for the drop of a block of `mining`.
pub(crate) fn breaker(world: &World, player: Entity, mining: &Mining) -> Breaker {
    let (tool, [silk_touch, fortune]) = held_tool(world, player, [SILK_TOUCH, FORTUNE]);
    let is = |kind: ToolKind| tool.is_some_and(|tool| tool.kind == kind);
    Breaker {
        harvests: mining.can_harvest(tool),
        silk_touch: silk_touch > 0,
        shears: is(ToolKind::Shears),
        pickaxe: is(ToolKind::Pickaxe),
        fortune,
        by_entity: true,
    }
}

fn crack(event: LevelEventEvent, pos: BlockPos, data: i32) -> bytes::Bytes {
    let [x, y, z] = pos.map(|c| c as f32);
    sapling_net::encode(&LevelEvent {
        event,
        position: Vec3f { x, y, z },
        data,
    })
}

/// A survival player started on the block at `pos`: the break is timed from now, and those who
/// see the block watch it crack, as from BDS.
pub(crate) fn start(
    world: &mut World,
    player: Entity,
    layer: Entity,
    pos: BlockPos,
    mining: &Mining,
) {
    let (tool, [efficiency]) = held_tool(world, player, [EFFICIENCY]);
    // The sword and shears speeds are the Java game's, never faster than Bedrock's but perhaps
    // slower: a break with one of them is taken as soon as it is claimed.
    let is_unsure =
        tool.is_some_and(|tool| matches!(tool.kind, ToolKind::Sword | ToolKind::Shears));
    let needed =
        break_ticks(mining, tool, efficiency).map(|ticks| if is_unsure { 1 } else { ticks });
    let tick = world.resource::<Tick>().0;
    if let Some(mut breaking) = world.get_mut::<Breaking>(player) {
        breaking.0 = needed.map(|needed| Started { pos, tick, needed });
    }
    if let Some(needed) = needed {
        send::to_viewers(
            world,
            layer,
            pos,
            &[crack(
                LevelEventEvent::BlockStartBreak,
                pos,
                (CRACK_SCALE / needed) as i32,
            )],
        );
    }
}

/// The player gave its break up.
pub(crate) fn abort(world: &mut World, player: Entity, layer: Entity) {
    let Some(started) = world
        .get_mut::<Breaking>(player)
        .and_then(|mut breaking| breaking.0.take())
    else {
        return;
    };
    send::to_viewers(
        world,
        layer,
        started.pos,
        &[crack(LevelEventEvent::BlockStopBreak, started.pos, 0)],
    );
}

/// Whether the player started on the block at `pos` long enough ago to have broken it now. The
/// started break ends either way.
pub(crate) fn finish(world: &mut World, player: Entity, pos: BlockPos) -> bool {
    let tick = world.resource::<Tick>().0;
    let started = world
        .get_mut::<Breaking>(player)
        .and_then(|mut breaking| breaking.0.take());
    started.is_some_and(|started| {
        started.pos == pos && tick - started.tick + LEEWAY >= u64::from(started.needed)
    })
}

#[cfg(test)]
mod tests {
    use sapling_world::BlockRegistry;

    use super::*;

    fn ticks(block: &str, tool: &str, efficiency: u8) -> Option<u32> {
        let registry = BlockRegistry::vanilla();
        let mining = registry
            .states_of(block)
            .next()
            .expect("a vanilla block")
            .1
            .mining;
        break_ticks(&mining, Tool::from_identifier(tool), efficiency)
    }

    #[test]
    fn a_break_takes_as_long_as_the_block_and_the_tool_make_it() {
        // Stone, hardness 1.5: 7.5 s by hand without a drop, 1.15 s with a wooden pickaxe.
        assert_eq!(
            [
                ticks("minecraft:stone", "minecraft:stick", 0),
                ticks("minecraft:stone", "minecraft:wooden_pickaxe", 0)
            ],
            [Some(150), Some(23)]
        );
        assert_eq!(
            ticks("minecraft:stone", "minecraft:diamond_pickaxe", 0),
            Some(6)
        );
        assert!(ticks("minecraft:stone", "minecraft:diamond_pickaxe", 5) < Some(3));
        assert_eq!(
            [
                ticks("minecraft:dirt", "minecraft:stick", 0),
                ticks("minecraft:dirt", "minecraft:iron_shovel", 0)
            ],
            [Some(15), Some(3)]
        );
        assert_eq!(
            [
                ticks("minecraft:bedrock", "minecraft:diamond_pickaxe", 0),
                ticks("minecraft:torch", "", 0)
            ],
            [None, Some(1)]
        );
    }
}
