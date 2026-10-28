//! Structure blocks, as BDS kept them (docs/research/structure-blocks.md): the entity a put block
//! gets, its mode settled onto the block a tick on; an operator's editor in creative mode and the
//! StructureBlockUpdate it sends, which keeps the settings and may save or load at once; power,
//! which saves or loads on a rise. Saving, loading and the editor's preview: act.rs.

mod act;

use acacia_proto::nbt::Value;
use acacia_proto::packets::StructureBlockUpdate;
use acacia_proto::types::WindowType;
use bevy_ecs::prelude::*;
use sapling_api::{Block, BlockPos};

use super::entity_fields::Fields;
use super::{level, send, shown_again};
use crate::components::{InLayer, PacketQueues};

pub(crate) use act::dispatch_exports;

const BLOCK: &str = "minecraft:structure_block";
const ENTITY: &str = "StructureBlock";
/// `structure_block_type` by the entity's `data`, as BDS numbers the modes.
const MODES: [&str; 6] = ["data", "save", "load", "corner", "invalid", "export"];
const SAVE: i64 = 1;
const LOAD: i64 = 2;

fn read(world: &World, layer: Entity, pos: BlockPos) -> Option<Fields> {
    let is_one = send::block_at(world, layer, pos).is_some_and(|block| block.name == BLOCK);
    is_one.then(|| Fields::read(world, layer, pos, ENTITY)).flatten()
}

/// The entity BDS gave a structure block put where none was: a save block of a 5-cell box
/// starting under it, whatever mode the block was put in (research/captures/bds-structblocks.txt).
fn fresh(pos: BlockPos) -> Fields {
    let Value::Compound(mut fields) = sapling_world::new_block_entity(ENTITY, pos) else { unreachable!("a new entity is a compound") };
    let (byte, int) = (|n: i8| Value::Byte(n), |n: i32| Value::Int(n));
    fields.extend(
        [
            ("BlockEntityVersion", int(0)),
            ("animationMode", byte(0)),
            ("animationSeconds", Value::Float(0.0)),
            ("data", int(SAVE as i32)),
            ("dataField", Value::String("".into())),
            ("ignoreEntities", byte(0)),
            ("includePlayers", byte(0)),
            ("integrity", Value::Float(100.0)),
            ("isPowered", byte(0)),
            ("lastTouchedPlayerId", Value::Long(-1)),
            ("mirror", byte(0)),
            ("redstoneSaveMode", int(0)),
            ("removeBlocks", byte(0)),
            ("rotation", byte(0)),
            ("seed", Value::Long(0)),
            ("showBoundingBox", byte(1)),
            ("structureName", Value::String("".into())),
            ("xStructureOffset", int(0)),
            ("xStructureSize", int(5)),
            ("yStructureOffset", int(-1)),
            ("yStructureSize", int(5)),
            ("zStructureOffset", int(0)),
            ("zStructureSize", int(5)),
        ]
        .map(|(key, value)| (key.into(), value)),
    );
    Fields(fields)
}

/// Gives a structure block just put its entity: the one the cell held where that was a
/// structure block's, else a fresh one, shown now and again a tick on with its mode.
pub(super) fn furnish(world: &mut World, layer: Entity, pos: BlockPos, before: Option<&Value>) {
    if !send::block_at(world, layer, pos).is_some_and(|block| block.name == BLOCK) {
        return;
    }
    match before.and_then(|before| Fields::of(before, ENTITY)) {
        Some(kept) => kept.show(world, layer, pos),
        None => {
            fresh(pos).show(world, layer, pos);
            shown_again::next_tick(world, layer, pos);
        }
    }
}

/// Puts the block in its entity's mode, without showing it: BDS showed a block put as `load`
/// turned `save` a tick later, with the entity (bds-structblocks.txt).
pub(super) fn settle(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(fields) = read(world, layer, pos) else { return };
    if let Some(id) = mode_block(world, layer, fields.int("data")) {
        send::set_quietly(world, layer, pos, id);
    }
}

fn mode_block(world: &World, layer: Entity, mode: i64) -> Option<u32> {
    let mode = MODES.get(usize::try_from(mode).ok()?)?;
    send::resolve(level(world, layer)?.registry(), &Block::new(BLOCK).with(format!("structure_block_type={mode}")))
}

/// A click on the block `name`. False for anything but a structure block.
// TODO: BDS's answer to a click was not captured; this is the command block's.
pub(super) fn open_editor(world: &mut World, player: Entity, layer: Entity, pos: BlockPos, name: &str) -> bool {
    name == BLOCK && super::command_blocks::show_editor(world, player, layer, pos, WindowType::StructureEditor)
}

/// A structure block's signal changed: it keeps `isPowered`, and a rise saves or loads.
pub(super) fn powered(world: &mut World, layer: Entity, pos: BlockPos, on: bool) {
    let Some(mut fields) = read(world, layer, pos) else { return };
    if fields.flag("isPowered") == on {
        return;
    }
    fields.set_flag("isPowered", on);
    let mode = fields.int("data");
    fields.keep(world, layer, pos);
    if on {
        act::trigger(world, layer, pos, mode, act::By::Power);
    }
}

/// Applies the editors' StructureBlockUpdates: the entity takes the settings and the block the
/// mode, both shown back, and a trigger saves or loads at once. BDS showed a load's block a
/// tick after the blocks it put, and an animated load's before them (bds-structblocks.txt).
pub(crate) fn dispatch_updates(world: &mut World) {
    for (player, update) in world.resource_mut::<PacketQueues>().take::<StructureBlockUpdate>() {
        let pos = [update.position.x, update.position.y, update.position.z];
        let Some(layer) = world.get::<InLayer>(player).map(|layer| layer.0).filter(|_| super::command_blocks::may_edit(world, player)) else { continue };
        let Some(mut fields) = read(world, layer, pos) else { continue };
        let mode = i64::from(update.structure_block_type);
        settings(&mut fields, &update);
        let instant_load = mode == LOAD && fields.int("animationMode") == 0;
        fields.keep(world, layer, pos);
        if let Some(id) = mode_block(world, layer, mode) {
            send::set_quietly(world, layer, pos, id);
        }
        if update.should_trigger && instant_load {
            act::trigger(world, layer, pos, mode, act::By::Editor { waterlogged: update.water_logged });
            shown_again::next_tick(world, layer, pos);
            continue;
        }
        send::show_whole(world, layer, pos);
        if update.should_trigger {
            act::trigger(world, layer, pos, mode, act::By::Editor { waterlogged: update.water_logged });
        }
    }
}

/// The editor's settings, as the entity keeps them.
fn settings(fields: &mut Fields, update: &StructureBlockUpdate) {
    let settings = &update.settings;
    let byte = |n: i64| Value::Byte(n.clamp(0, 3) as i8);
    fields.set_text("structureName", &update.structure_name);
    fields.set_text("dataField", &update.data_field);
    fields.set_flag("includePlayers", update.include_players);
    fields.set_flag("showBoundingBox", update.show_bounding_box);
    fields.put("data", Value::Int(update.structure_block_type));
    fields.put("redstoneSaveMode", Value::Int(update.redstone_save_mode.into()));
    for (axis, [offset, size]) in [["xStructureOffset", "xStructureSize"], ["yStructureOffset", "yStructureSize"], ["zStructureOffset", "zStructureSize"]].into_iter().enumerate() {
        let pick = |coords: &acacia_proto::types::BlockCoordinates| [coords.x, coords.y, coords.z][axis];
        fields.put(offset, Value::Int(pick(&settings.structure_offset)));
        fields.put(size, Value::Int(pick(&settings.size)));
    }
    fields.put("rotation", byte(settings.rotation.to_raw()));
    fields.put("mirror", byte(settings.mirror.to_raw()));
    fields.put("animationMode", byte(settings.animation_mode.to_raw()));
    fields.put("animationSeconds", Value::Float(settings.animation_duration));
    fields.put("integrity", Value::Float(settings.integrity));
    fields.put("seed", Value::Long(settings.seed.into()));
    fields.set_flag("ignoreEntities", settings.ignore_entities);
    fields.set_flag("removeBlocks", settings.ignore_blocks);
    fields.put("lastTouchedPlayerId", Value::Long(settings.last_editing_player_unique_id));
}
