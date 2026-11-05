//! A gateway loaded with an exit is linked again (`queueGatewayRelink` 1482743c0, run in
//! `TheEndDimension::tick` 148276910 once both ends are loaded, each end by 148274e90): the gateway
//! itself gets the end-stone spawn round its exit, then the gateway nearest the exit (within 32 on
//! each axis) the one round its own exit; an exit already standing room over a block is kept, and
//! an end island is built where no spawn is found (research/captures/bds-d2gate2.txt).

use acacia_proto::nbt::{List, Value};
use acacia_proto::packets::UpdateBlock;
use acacia_proto::types::{BlockCoordinates, UpdateBlockFlags};
use sapling_api::BlockPos;
use sapling_world::World as Level;

use super::exit_spawn::{island_at, valid_spawn};

const ENTITY: &str = "EndGateway";
const NEAREST_REACH: i32 = 32;

fn blocks(level: &Level, [x, y, z]: BlockPos) -> bool {
    level.in_height(y) && level.registry().get(level.block(x, y, z)).is_some_and(|state| state.is_solid())
}

fn free(level: &Level, at @ [_, y, _]: BlockPos) -> bool {
    !blocks(level, at) && !(level.in_height(y) && level.registry().get(level.block(at[0], y, at[2])).is_some_and(|state| state.is_liquid()))
}

fn is_gateway(level: &Level, pos: BlockPos) -> bool {
    level.block_entity(pos).is_some_and(|entity| matches!(entity.nbt.get("id"), Some(Value::String(id)) if id.as_str() == ENTITY))
}

fn kept_exit(level: &Level, pos: BlockPos) -> Option<BlockPos> {
    exit_of(&level.block_entity(pos)?.nbt)
}

/// The `ExitPortal` a gateway's entity keeps.
pub(crate) fn exit_of(nbt: &Value) -> Option<BlockPos> {
    match nbt.get("ExitPortal")? {
        Value::List(list) => match list.items.as_slice() {
            [Value::Int(x), Value::Int(y), Value::Int(z)] => Some([*x, *y, *z]),
            _ => None,
        },
        _ => None,
    }
}

/// The gateway nearest `at`'s middle within 32 on each axis, the first on a tie.
fn nearest(level: &Level, at: BlockPos) -> Option<BlockPos> {
    let low = at.map(|c| c - NEAREST_REACH);
    let high = at.map(|c| c + NEAREST_REACH);
    let columns = (low[0] >> 4..=high[0] >> 4).flat_map(|cx| (low[2] >> 4..=high[2] >> 4).map(move |cz| sapling_world::ChunkPos::new(cx, cz)));
    let inside = |pos: &BlockPos| (0..3).all(|i| (low[i]..=high[i]).contains(&pos[i]));
    let distance = |pos: BlockPos| (0..3).map(|i| (at[i] as f32 + 0.5 - pos[i] as f32).powi(2)).sum::<f32>();
    let found = columns.flat_map(|column| level.block_entities_in(column)).map(|entity| entity.pos).filter(|pos| inside(pos) && is_gateway(level, *pos));
    found.fold(None, |best: Option<(f32, BlockPos)>, pos| if best.is_none_or(|(least, _)| distance(pos) < least) { Some((distance(pos), pos)) } else { best }).map(|(_, pos)| pos)
}

/// One end of a relink: the gateway at `at` (or nearest it, `search`) and its new exit round
/// `target` (or round its own exit, when searched), building the island it needs; `None` when
/// nothing changes.
pub(super) fn link(level: &mut Level, at: BlockPos, target: BlockPos, search: bool) -> Option<(BlockPos, BlockPos)> {
    let [x, y, z] = target;
    if free(level, target) && free(level, [x, y + 1, z]) && blocks(level, [x, y - 1, z]) {
        return None;
    }
    let (gateway, target) = if search {
        let gateway = nearest(level, at)?;
        (gateway, kept_exit(level, gateway).unwrap_or(target))
    } else {
        (at, target)
    };
    if !is_gateway(level, gateway) {
        return None;
    }
    if let Some(exit) = valid_spawn(level, target, true, 1) {
        return Some((gateway, exit));
    }
    if let Some(end_stone) = level.block_id("minecraft:end_stone") {
        for [x, y, z] in island_at(target).1 {
            level.set_block(x, y, z, end_stone);
        }
    }
    // [U] BDS's second search has its fallback on: taken to answer the target where it finds none.
    Some((gateway, valid_spawn(level, target, true, 1).unwrap_or(target)))
}

/// What BDS shows a changed gateway with: the block (neighbours, network and its sixteen bit) and its entity.
pub(crate) fn shown(level: &Level, [x, y, z]: BlockPos) -> Vec<bytes::Bytes> {
    let block = UpdateBlock { position: BlockCoordinates { x, y, z }, block_runtime_id: level.registry().network_hash(level.block(x, y, z)), flags: UpdateBlockFlags(19), layer: 0 };
    let mut packets = vec![sapling_net::encode(&block)];
    packets.extend(level.block_entity([x, y, z]).map(|entity| crate::blocks::block_entity_data([x, y, z], entity)));
    packets
}

/// `entity` with `exit` as its `ExitPortal`.
pub(super) fn with_exit(entity: &Value, exit: BlockPos) -> Value {
    let mut entity = entity.clone();
    if let Value::Compound(fields) = &mut entity {
        let exit = Value::List(List { tag: 3, items: exit.iter().map(|c| Value::Int(*c)).collect() });
        fields.retain(|(key, _)| key != "ExitPortal");
        fields.push(("ExitPortal".into(), exit));
        fields.sort_by(|a, b| a.0.cmp(&b.0));
    }
    entity
}
