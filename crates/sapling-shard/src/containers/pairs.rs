//! Double chests: two chests side by side that name each other in their block entities
//! (`pairx`, `pairz`, and `pairlead` on the half whose slots come first), as the game stores
//! them and as clients need them to draw one large chest. A half gone other than by a player's
//! break leaves the other alone a tick later, shown again; one put between two singles pairs
//! with the one west or north of it (research/captures/bds-t3chests.txt).

use acacia_proto::nbt::Value;
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::screens::shut_out;
use super::{CHEST, keep_entity, kind};
use crate::components::Layer;

const LEAD: &str = "pairlead";
const X: &str = "pairx";
const Z: &str = "pairz";
const FACING: &str = "minecraft:cardinal_direction=";

fn entity(world: &World, layer: Entity, pos: BlockPos) -> Option<Value> {
    world.get::<Layer>(layer)?.world.block_entity(pos).map(|entity| entity.nbt.clone())
}

/// The column of the half a chest's entity names, and whether this half leads.
fn partner(entity: &Value) -> Option<([i32; 2], bool)> {
    let (Some(Value::Int(x)), Some(Value::Int(z))) = (entity.get(X), entity.get(Z)) else { return None };
    Some(([*x, *z], entity.get(LEAD) == Some(&Value::Byte(1))))
}

/// The entity paired with the half at `with` (leading it or not), or with nothing.
fn paired(entity: &Value, with: Option<(BlockPos, bool)>) -> Value {
    let mut fields = if let Value::Compound(fields) = entity { fields.clone() } else { Vec::new() };
    fields.retain(|(key, _)| ![LEAD, X, Z].contains(&key.as_str()));
    if let Some(([x, _, z], leads)) = with {
        fields.extend([(LEAD.into(), Value::Byte(leads.into())), (X.into(), Value::Int(x)), (Z.into(), Value::Int(z))]);
    }
    Value::Compound(fields)
}

/// The cells of the chest at `pos`, the lead half first: one cell, or two that name each other.
pub(crate) fn halves(world: &World, layer: Entity, pos: BlockPos) -> Vec<BlockPos> {
    world.get::<Layer>(layer).map_or_else(|| vec![pos], |layer| halves_in(&layer.world, pos))
}

/// [`halves`] in a world read directly.
pub(crate) fn halves_in(level: &sapling_world::World, pos: BlockPos) -> Vec<BlockPos> {
    let entity = |at: BlockPos| level.block_entity(at).map(|entity| entity.nbt.clone());
    let Some(([x, z], leads)) = entity(pos).as_ref().and_then(partner) else { return vec![pos] };
    let other = [x, pos[1], z];
    let is_mutual = entity(other).as_ref().and_then(partner).is_some_and(|(back, _)| back == [pos[0], pos[2]]);
    match (is_mutual, leads) {
        (false, _) => vec![pos],
        (true, true) => vec![pos, other],
        (true, false) => vec![other, pos],
    }
}

/// The block and the way it faces, of a chest that can still pair.
fn single_chest(world: &World, layer: Entity, pos: BlockPos) -> Option<(&'static str, &'static str)> {
    single_chest_in(&world.get::<Layer>(layer)?.world, pos)
}

fn single_chest_in(level: &sapling_world::World, pos: BlockPos) -> Option<(&'static str, &'static str)> {
    let state = level.registry().get(level.block(pos[0], pos[1], pos[2]))?;
    let facing = state.properties.split(',').find_map(|pair| pair.strip_prefix(FACING))?;
    let is_free = kind(state.name).is_some_and(|kind| kind.entity == CHEST.entity) && level.block_entity(pos).is_some_and(|entity| partner(&entity.nbt).is_none());
    is_free.then_some((state.name, facing))
}

/// The cells a chest facing `facing` at `pos` may pair with: beside it, not in front or behind.
fn beside(pos: BlockPos, facing: &str) -> [BlockPos; 2] {
    let along_x = matches!(facing, "north" | "south");
    [-1, 1].map(|step| if along_x { [pos[0] + step, pos[1], pos[2]] } else { [pos[0], pos[1], pos[2] + step] })
}

/// The state a copper chest `id` put at `pos` takes: that of a single copper chest of another
/// stage beside it facing the same way, which it then pairs with; BDS's exposed one put by an
/// unweathered one was shown unweathered in its placing (bds-r3copper.txt).
pub(crate) fn staged(level: &sapling_world::World, pos: BlockPos, id: u32) -> u32 {
    let is_copper = |name: &str| name.ends_with("copper_chest");
    let Some(state) = level.registry().get(id).filter(|state| is_copper(state.name)) else { return id };
    let Some(facing) = state.properties.split(',').find_map(|pair| pair.strip_prefix(FACING)) else { return id };
    let single = |at: BlockPos| single_chest_in(level, at).filter(|(_, other)| *other == facing);
    let cells = beside(pos, facing);
    if cells.iter().any(|at| single(*at).is_some_and(|(name, _)| name == state.name)) {
        return id;
    }
    cells.into_iter().find(|at| single(*at).is_some_and(|(name, _)| is_copper(name))).map_or(id, |at| level.block(at[0], at[1], at[2]))
}

/// Pairs the chest just placed at `pos` with a single chest of its kind beside it that faces
/// the same way; that one leads, and its open screens are shut (BDS `ChestBlockActor::onPlace`).
/// The pair is shown a tick on ([`crate::blocks::show_pair_next_tick`]).
pub(super) fn pair_placed(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(placed) = single_chest(world, layer, pos) else { return };
    let Some(old) = beside(pos, placed.1).into_iter().find(|at| single_chest(world, layer, *at) == Some(placed)) else { return };
    shut_out(world, layer, old);
    for (half, with) in [(old, (pos, true)), (pos, (old, false))] {
        if let (Some(entity), Some(mut level)) = (entity(world, layer, half), world.get_mut::<Layer>(layer)) {
            _ = level.world.set_block_entity(half, &paired(&entity, Some(with)));
        }
    }
    crate::blocks::show_pair_next_tick(world, layer, [old, pos]);
}

/// A chest's entity copied to `to` by `/clone`: a half's partner moved with it (BDS clones a half
/// only with its partner), a single chest or a shulker box marked `forceunpair` (bds-worldcmds.txt,
/// bds-t3chests.txt, bds-v2ents.txt).
pub(crate) fn carried(entity: &Value, to: BlockPos) -> Value {
    if !["Chest", "ShulkerBox"].map(|id| Value::String(id.into())).iter().any(|id| entity.get("id") == Some(id)) {
        return entity.clone();
    }
    let (Some(Value::Int(x)), Some(Value::Int(z))) = (entity.get("x"), entity.get("z")) else { return entity.clone() };
    match partner(entity) {
        Some(([px, pz], leads)) => paired(entity, Some(([px + to[0] - x, to[1], pz + to[2] - z], leads))),
        None => {
            let Value::Compound(mut fields) = entity.clone() else { return entity.clone() };
            fields.retain(|(key, _)| key != "forceunpair");
            fields.push(("forceunpair".into(), Value::Byte(1)));
            Value::Compound(fields)
        }
    }
}

/// Whether the chest at `pos` still names `gone` its other half though `gone` no longer names it.
pub(crate) fn lost_partner(world: &World, layer: Entity, pos: BlockPos, gone: BlockPos) -> bool {
    let names_gone = entity(world, layer, pos).as_ref().and_then(partner).is_some_and(|(at, _)| at == [gone[0], gone[2]] && pos[1] == gone[1]);
    names_gone && halves(world, layer, pos).len() == 1
}

/// The chest at `pos`, whose other half went, as a single chest, shown again.
pub(crate) fn stand_alone(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(entity) = entity(world, layer, pos).filter(|entity| partner(entity).is_some()) else { return };
    if halves(world, layer, pos).len() == 2 {
        return;
    }
    if let Some(mut level) = world.get_mut::<Layer>(layer) {
        _ = level.world.set_block_entity(pos, &paired(&entity, None));
    }
    crate::blocks::show_whole(world, layer, pos);
}

/// Leaves the other half of the double chest `halves` a single chest, when the half at `gone`
/// is about to go.
pub(super) fn unpair(world: &mut World, layer: Entity, halves: &[BlockPos], gone: BlockPos) {
    let [first, second] = halves else { return };
    let left = if *first == gone { *second } else { *first };
    if let Some(entity) = entity(world, layer, left) {
        keep_entity(world, layer, left, &paired(&entity, None));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chest(fields: &[(&str, Value)]) -> Value {
        let mut all = vec![("id".into(), Value::String("Chest".into())), ("x".into(), Value::Int(12)), ("y".into(), Value::Int(-60)), ("z".into(), Value::Int(10))];
        all.extend(fields.iter().map(|(key, value)| ((*key).into(), value.clone())));
        Value::Compound(all)
    }

    #[test]
    fn a_cloned_half_names_its_moved_partner_and_a_single_is_forced_apart() {
        // research/captures/bds-t3chests.txt: the pair cloned 4 east named its new cells.
        let half = chest(&[(LEAD, Value::Byte(1)), (X, Value::Int(12)), (Z, Value::Int(11))]);
        assert_eq!(partner(&carried(&half, [16, -60, 10])), Some(([16, 11], true)));
        assert_eq!(carried(&chest(&[]), [14, -60, 10]).get("forceunpair"), Some(&Value::Byte(1)));
        let barrel = Value::Compound(vec![("id".into(), Value::String("Barrel".into()))]);
        assert_eq!(carried(&barrel, [14, -60, 10]), barrel);
    }
}
