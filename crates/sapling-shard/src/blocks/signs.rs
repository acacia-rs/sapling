//! Writing on a sign: its placer, and anyone who clicks it later, is shown the editor, and the
//! text the client sends back is kept, as BDS does (docs/research/block-entities.md, "Sign
//! editing").
// TODO: dyes, glow ink and honeycomb on a sign are not applied; a click with them edits it.

use acacia_proto::nbt::Value;
use acacia_proto::packets::{BlockEntityData, OpenSign};
use acacia_proto::types::BlockCoordinates;
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::send;
use crate::components::{Identity, InLayer, Layer, Outgoing, PacketQueues, Position};

const SIDES: [&str; 2] = ["FrontText", "BackText"];
/// BDS's sign code sets no limit; this keeps a client from filling the world's storage.
const MAX_TEXT: usize = 512;

/// The sign a player was shown the editor of, the only one its client may write on.
#[derive(Component, Clone, Copy)]
pub(crate) struct Editing {
    layer: Entity,
    pos: BlockPos,
}

/// Shows a player who just placed the block `name` at `pos` the editor, when it is a sign.
pub(super) fn open_editor(world: &mut World, player: Entity, layer: Entity, pos: BlockPos, name: &str) {
    if !name.ends_with("_sign") {
        return;
    }
    let [x, y, z] = pos;
    let Some(mut out) = world.get_mut::<Outgoing>(player) else { return };
    out.push(&OpenSign { position: BlockCoordinates { x, y, z }, is_front: true });
    world.entity_mut(player).insert(Editing { layer, pos });
}

/// The way the front of a sign faces, as x and z: by its 16th of a turn standing (or hanging
/// from a chain on its own), else by the face it is on.
fn front(properties: &str) -> Option<[f32; 2]> {
    let get = |key: &str| properties.split(',').filter_map(|pair| pair.split_once('=')).find(|(name, _)| *name == key).map(|(_, value)| value);
    let turned = get("ground_sign_direction").filter(|_| get("facing_direction").is_none() || get("attached_bit") == Some("1"));
    if let Some(turn) = turned.and_then(|turn| turn.parse::<f32>().ok()) {
        let angle = (turn * 22.5).to_radians();
        return Some([-angle.sin(), angle.cos()]);
    }
    Some(match get("facing_direction")? {
        "2" => [0.0, -1.0],
        "3" => [0.0, 1.0],
        "4" => [-1.0, 0.0],
        _ => [1.0, 0.0],
    })
}

/// A click on the block `name` at `pos`: a sign that is not waxed shows the player the editor
/// for the side it stands on (Java `SignBlock.isFacingFrontText`). False for any other block.
pub(crate) fn edit_by_click(world: &mut World, player: Entity, layer: Entity, pos: BlockPos, name: &str) -> bool {
    if !name.ends_with("_sign") {
        return false;
    }
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return true };
    let is_waxed = level.block_entity(pos).is_some_and(|kept| matches!(kept.nbt.get("IsWaxed"), Some(Value::Byte(1))));
    let facing = level.registry().get(level.block(pos[0], pos[1], pos[2])).and_then(|state| front(state.properties));
    let (Some([fx, fz]), Some(feet)) = (facing.filter(|_| !is_waxed), world.get::<Position>(player).map(|position| position.feet)) else { return true };
    let is_front = (feet[0] - pos[0] as f32 - 0.5) * fx + (feet[2] - pos[2] as f32 - 0.5) * fz >= 0.0;
    let [x, y, z] = pos;
    let Some(mut out) = world.get_mut::<Outgoing>(player) else { return true };
    out.push(&OpenSign { position: BlockCoordinates { x, y, z }, is_front });
    world.entity_mut(player).insert(Editing { layer, pos });
    true
}

fn text(side: Option<&Value>) -> Option<&str> {
    match side?.get("Text") {
        Some(Value::String(text)) => Some(text),
        _ => None,
    }
}

/// The sign entity `kept` with a side's text replaced where `sent` changed it, each such side
/// owned by `owner`; `None` when nothing changed or the sign is waxed.
fn written(kept: &Value, sent: &Value, owner: &str) -> Option<Value> {
    let Value::Compound(mut fields) = kept.clone() else { return None };
    if matches!(kept.get("IsWaxed"), Some(Value::Byte(1))) {
        return None;
    }
    let mut changed = false;
    for (key, side) in fields.iter_mut().filter(|(key, _)| SIDES.contains(&key.as_str())) {
        let Some(new) = text(sent.get(key)).filter(|new| new.len() <= MAX_TEXT && Some(*new) != text(Some(side))) else { continue };
        let Value::Compound(side) = side else { continue };
        side.retain(|(key, _)| key != "Text" && key != "TextOwner");
        side.extend([("Text".to_owned(), Value::String(new.to_owned())), ("TextOwner".to_owned(), Value::String(owner.to_owned()))]);
        changed = true;
    }
    changed.then_some(Value::Compound(fields))
}

/// Keeps what this tick's clients wrote on the signs they were shown the editor of, for
/// everyone who sees them; anything else a client sends about a block is ignored.
pub(crate) fn dispatch_edits(world: &mut World) {
    let edits = world.resource_mut::<PacketQueues>().take::<BlockEntityData>();
    for (player, edit) in edits {
        let pos = [edit.position.x, edit.position.y, edit.position.z];
        let Some(editing) = world.get::<Editing>(player).copied().filter(|editing| editing.pos == pos) else { continue };
        world.entity_mut(player).remove::<Editing>();
        if world.get::<InLayer>(player).map(|in_layer| in_layer.0) != Some(editing.layer) {
            continue;
        }
        let owner = world.get::<Identity>(player).map(|identity| identity.0.xuid.clone()).unwrap_or_default();
        let kept = world.get::<Layer>(editing.layer).and_then(|layer| layer.world.block_entity(pos)).map(|kept| kept.nbt.clone());
        if let Some(entity) = kept.and_then(|kept| written(&kept, &edit.nbt.value, &owner)) {
            send::keep_entity(world, editing.layer, pos, &entity);
        }
    }
}
