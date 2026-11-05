//! End gateways, as BDS sent a player through the one worldgen placed (research/captures/
//! bds-end-gateway.txt; block actor tick 1487fcae0): in the End, a body in the gateway block is
//! teleported at once to its exit, a block up and in the middle of the cell, told twice with the
//! gateway's block event; the next tick the gateway and its entity are shown again. While its
//! column ticks a gateway's `Age` goes up by one a tick, uncapped, and it is shown again at Age 1
//! and 200 (bds-u1gate.txt), in its first tick since its column came from storage
//! (gateway_load.rs) and as a trip's cooldown ends (docs/research/end-dimension.md "Gateways").

use acacia_proto::nbt::Value;
use acacia_proto::packets::{BlockEvent, BlockEventType, MovePlayerTeleportCause};
use acacia_proto::types::BlockCoordinates;
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_world::END;

use crate::components::{InLayer, Layer, Position};
use crate::movement;

const GATEWAY: &str = "minecraft:end_gateway";
const ENTITY: &str = "EndGateway";
/// The exit every gateway the End's generation places keeps in its block entity.
const WORLDGEN_EXIT: BlockPos = [100, 50, 0];
/// The source entity types of BDS's two MovePlayer for one trip.
const TOLD_AS: [i64; 2] = [1, 0];
/// The ages a gateway is shown again at: a generated one counts as changed until its first tick.
const SHOWN_AT: [i32; 2] = [1, 200];

/// Ticks a gateway waits after a trip (BDS's block actor sets 20 where Java sets 40).
const COOLDOWN: u64 = 20;

/// Gateways used lately: the layer, the cell, the traveller and the tick of the trip. Each is
/// shown again the tick after and takes no one until its cooldown is over.
#[derive(Resource, Default)]
pub(crate) struct Used(Vec<(Entity, BlockPos, Entity, u64)>);

/// The gateway cell a player's body is in.
fn gateway_at(world: &World, player: Entity) -> Option<(Entity, BlockPos)> {
    let layer = world.get::<InLayer>(player)?.0;
    let (feet, level) = (world.get::<Position>(player)?.feet, &world.get::<Layer>(layer)?.world);
    let range = |low: f32, high: f32| (low.floor() as i32)..=(high.floor() as i32);
    let [x, y, z] = feet;
    let cells = range(x - super::HALF_WIDTH, x + super::HALF_WIDTH).flat_map(|cx| range(z - super::HALF_WIDTH, z + super::HALF_WIDTH).flat_map(move |cz| range(y, y + super::HEIGHT - 0.01).map(move |cy| [cx, cy, cz])));
    let found = cells.into_iter().find(|&[cx, cy, cz]| level.in_height(cy) && level.registry().get(level.block(cx, cy, cz)).is_some_and(|state| state.name == GATEWAY));
    found.map(|pos| (layer, pos))
}

/// Where a gateway sends: its block entity's `ExitPortal` ((0, 0, 0) without one, as BDS's), else
/// the generated gateways' exit.
fn exit(level: &sapling_world::World, pos: BlockPos) -> BlockPos {
    level.block_entity(pos).map_or(WORLDGEN_EXIT, |entity| crate::dragon::gateway_exit(&entity.nbt).unwrap_or([0; 3]))
}

/// Sends `packets` about the gateway at `pos` to its viewers, and to `traveller` too, whose client
/// still holds the gateway's column after its trip (BDS tells it).
fn tell(world: &mut World, layer: Entity, pos: BlockPos, traveller: Option<Entity>, packets: &[bytes::Bytes]) {
    crate::view::to_viewers(world, layer, pos, packets);
    let column = sapling_world::ChunkPos::of_block(pos[0], pos[2]);
    let Some(traveller) = traveller else { return };
    if let Ok((view, mut out)) = world.query::<(&crate::view::View, &mut crate::components::Outgoing)>().get_mut(world, traveller) {
        if !view.covers(column) {
            out.0.extend_from_slice(packets);
        }
    }
}

/// Shows the gateway at `pos` again, block and entity.
fn show(world: &mut World, layer: Entity, pos: BlockPos, traveller: Option<Entity>) {
    let Some(packets) = world.get::<Layer>(layer).map(|layer| crate::dragon::gateway_shown(&layer.world, pos)) else { return };
    tell(world, layer, pos, traveller, &packets);
}

/// The gateway entity `nbt` a tick older, and its new age; none for another entity.
fn older(nbt: &Value) -> Option<(Value, i32)> {
    let Some(Value::String(id)) = nbt.get("id") else { return None };
    if id.as_str() != ENTITY {
        return None;
    }
    let age = match nbt.get("Age") {
        Some(Value::Int(age)) => age.saturating_add(1),
        _ => 1,
    };
    let mut aged = nbt.clone();
    if let Value::Compound(fields) = &mut aged {
        fields.retain(|(key, _)| key != "Age");
        fields.push(("Age".into(), Value::Int(age)));
        fields.sort_by(|a, b| a.0.cmp(&b.0));
    }
    Some((aged, age))
}

/// Ages every gateway in a ticking End column, showing those at an age BDS shows.
fn age(world: &mut World) {
    let mut due = Vec::new();
    for (layer, column) in crate::ticking::ticked_columns(world) {
        let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world).filter(|level| level.dimension() == END) else { continue };
        let aged: Vec<(BlockPos, Value, i32)> = level.block_entities_in(column).into_iter().filter_map(|entity| older(&entity.nbt).map(|(nbt, age)| (entity.pos, nbt, age))).collect();
        for (pos, nbt, age) in aged {
            if let Some(mut level) = world.get_mut::<Layer>(layer) {
                level.world.set_block_entity(pos, &nbt);
            }
            if super::gateway_load::first_tick(world, layer, pos, &nbt) || SHOWN_AT.contains(&age) {
                due.push((layer, pos));
            }
        }
    }
    for (layer, pos) in due {
        show(world, layer, pos, None);
    }
}

/// Ages the gateways, shows last tick's again and those whose cooldown ends, then sends the
/// players in a gateway through it.
pub(crate) fn tick(world: &mut World, players: &[Entity]) {
    super::gateway_load::forget_gone(world);
    age(world);
    let used = world.get_resource_mut::<Used>().map(|mut used| std::mem::take(&mut used.0)).unwrap_or_default();
    let now = world.resource::<crate::components::Tick>().0;
    for &(layer, pos, traveller, since) in &used {
        // [U] the cooldown's end shown in the tick it ends (no capture sees it: the traveller is away).
        if since + 1 == now || since + COOLDOWN == now {
            show(world, layer, pos, Some(traveller).filter(|_| since + 1 == now));
        }
    }
    let mut cooling: Vec<_> = used.into_iter().filter(|(.., since)| now < since + COOLDOWN).collect();
    for &player in players {
        let Some((layer, pos)) = gateway_at(world, player).filter(|(layer, pos)| !cooling.iter().any(|(l, p, ..)| l == layer && p == pos)) else { continue };
        let Some([x, y, z]) = world.get::<Layer>(layer).filter(|level| level.world.dimension() == END).map(|level| exit(&level.world, pos)) else { continue };
        movement::teleport_as(world, player, [x as f32 + 0.5, (y + 1) as f32, z as f32 + 0.5], MovePlayerTeleportCause::UnknownValue, &TOLD_AS);
        let [bx, by, bz] = pos;
        let event = sapling_net::encode(&BlockEvent { position: BlockCoordinates { x: bx, y: by, z: bz }, r#type: BlockEventType::ChangeState, data: 0 });
        tell(world, layer, pos, Some(player), &[event]);
        cooling.push((layer, pos, player, now));
    }
    world.get_resource_or_insert_with(Used::default).0 = cooling;
}
