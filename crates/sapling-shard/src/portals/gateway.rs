//! End gateways, as BDS sent a player through the one worldgen placed (research/captures/
//! bds-end-gateway.txt; block actor tick 1487fcae0): in the End, a body in the gateway block is
//! teleported at once to its exit, a block up and in the middle of the cell, told twice; the
//! next tick the gateway is shown again.
// TODO: the beam BDS shows every 200 ticks or so; gateways whose block entity has no exit (those a
// dragon's death makes) find one first.

use acacia_proto::packets::{MovePlayerTeleportCause, UpdateBlock};
use acacia_proto::types::{BlockCoordinates, UpdateBlockFlags};
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_world::END;

use crate::components::{InLayer, Layer, Position};
use crate::movement;

const GATEWAY: &str = "minecraft:end_gateway";
/// The exit every gateway the End's generation places keeps in its block entity.
const WORLDGEN_EXIT: BlockPos = [100, 50, 0];
/// The source entity types of BDS's two MovePlayer for one trip.
const TOLD_AS: [i64; 2] = [1, 0];
/// What BDS shows the gateway again with: neighbours, network and its sixteen bit.
const SHOWN_AGAIN: UpdateBlockFlags = UpdateBlockFlags(19);

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

/// Where a gateway sends: its block entity's `ExitPortal`, else the generated gateways' exit.
fn exit(level: &sapling_world::World, pos: BlockPos) -> BlockPos {
    let kept = level.block_entity(pos).and_then(|entity| match entity.nbt.get("ExitPortal") {
        Some(acacia_proto::nbt::Value::List(list)) => match list.items.as_slice() {
            [acacia_proto::nbt::Value::Int(x), acacia_proto::nbt::Value::Int(y), acacia_proto::nbt::Value::Int(z)] => Some([*x, *y, *z]),
            _ => None,
        },
        _ => None,
    });
    kept.unwrap_or(WORLDGEN_EXIT)
}

/// Shows last tick's gateways again, then sends the players in a gateway through it.
pub(crate) fn tick(world: &mut World, players: &[Entity]) {
    let used = world.get_resource_mut::<Used>().map(|mut used| std::mem::take(&mut used.0)).unwrap_or_default();
    let now = world.resource::<crate::components::Tick>().0;
    for (layer, [x, y, z], traveller, _) in used.iter().filter(|(.., since)| *since + 1 == now) {
        let Some(level) = world.get::<Layer>(*layer).map(|layer| &layer.world) else { continue };
        let shown = sapling_net::encode(&UpdateBlock { position: BlockCoordinates { x: *x, y: *y, z: *z }, block_runtime_id: level.registry().network_hash(level.block(*x, *y, *z)), flags: SHOWN_AGAIN, layer: 0 });
        crate::view::to_viewers(world, *layer, [*x, *y, *z], std::slice::from_ref(&shown));
        // The traveller's client still holds the gateway's column: BDS tells it too.
        let column = sapling_world::ChunkPos::of_block(*x, *z);
        if let Ok((view, mut out)) = world.query::<(&crate::view::View, &mut crate::components::Outgoing)>().get_mut(world, *traveller) {
            if !view.covers(column) {
                out.0.push(shown);
            }
        }
    }
    let mut cooling: Vec<_> = used.into_iter().filter(|(.., since)| now < since + COOLDOWN).collect();
    for &player in players {
        let Some((layer, pos)) = gateway_at(world, player).filter(|(layer, pos)| !cooling.iter().any(|(l, p, ..)| l == layer && p == pos)) else { continue };
        let Some([x, y, z]) = world.get::<Layer>(layer).filter(|level| level.world.dimension() == END).map(|level| exit(&level.world, pos)) else { continue };
        movement::teleport_as(world, player, [x as f32 + 0.5, (y + 1) as f32, z as f32 + 0.5], MovePlayerTeleportCause::UnknownValue, &TOLD_AS);
        cooling.push((layer, pos, player, now));
    }
    world.get_resource_or_insert_with(Used::default).0 = cooling;
}
