//! What a structure block does: a save block keeps its box under its name as typed, to the world
//! when its editor saves and as `redstoneSaveMode` says when power does (BDS saved by power,
//! research/captures/bds-structblocks.txt); a load block puts the template so named with its
//! settings, animated as they ask. The editor's preview asks for templates with
//! StructureTemplateDataExportRequest.

use acacia_proto::packets::{
    StructureTemplateDataExportRequest, StructureTemplateDataExportRequestRequestType as Request, StructureTemplateDataExportResponse,
    StructureTemplateDataExportResponseResponseType as Response,
};
use acacia_proto::types::BlockCoordinates;
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Mirror, Rotation, StructureLoad};

use super::{LOAD, SAVE, read};
use crate::components::{InLayer, Outgoing, PacketQueues};
use crate::structures::{self, Animation};

pub(super) enum By {
    Editor { waterlogged: bool },
    Power,
}

/// Saves or loads as the block at `pos` is set to; a block of any other mode does nothing.
pub(super) fn trigger(world: &mut World, layer: Entity, pos: BlockPos, mode: i64, by: By) {
    let Some(fields) = read(world, layer, pos) else { return };
    let name = fields.text("structureName");
    if name.is_empty() {
        return;
    }
    let axis = |key: &str| ["x", "y", "z"].map(|axis| fields.int(&format!("{axis}{key}")) as i32);
    let at: BlockPos = std::array::from_fn(|i| pos[i] + axis("StructureOffset")[i]);
    match mode {
        SAVE => {
            let to_disk = match by {
                By::Editor { .. } => true,
                By::Power => fields.int("redstoneSaveMode") == 1,
            };
            structures::save_as(world, layer, &name, at, axis("StructureSize"), !fields.flag("ignoreEntities"), to_disk);
        }
        LOAD => {
            let Some(template) = structures::saved_as(world, layer, &name) else { return };
            let how = StructureLoad {
                rotation: [Rotation::None, Rotation::Clockwise90, Rotation::Clockwise180, Rotation::Clockwise270][fields.int("rotation").clamp(0, 3) as usize],
                mirror: [Mirror::None, Mirror::X, Mirror::Z, Mirror::XZ][fields.int("mirror").clamp(0, 3) as usize],
                include_entities: !fields.flag("ignoreEntities"),
                include_blocks: !fields.flag("removeBlocks"),
                waterlogged: matches!(by, By::Editor { waterlogged: true }),
                integrity: (fields.float("integrity") / 100.0).clamp(0.0, 1.0),
                seed: match fields.int("seed") {
                    0 => String::new(),
                    seed => seed.to_string(),
                },
            };
            let animation = [Animation::None, Animation::LayerByLayer, Animation::BlockByBlock][fields.int("animationMode").clamp(0, 2) as usize];
            structures::load_animated(world, layer, template, at, &how, animation, fields.float("animationSeconds"));
        }
        _ => {}
    }
}

/// Answers the editors' requests for templates: a saved one by name, or the box a save block's
/// settings take as it is now. BDS answered a name it had none of with no template
/// (bds-structblocks.txt).
pub(crate) fn dispatch_exports(world: &mut World) {
    for (player, request) in world.resource_mut::<PacketQueues>().take::<StructureTemplateDataExportRequest>() {
        let Some(layer) = world.get::<InLayer>(player).map(|layer| layer.0).filter(|_| super::super::command_blocks::may_edit(world, player)) else { continue };
        let corner = |coords: &BlockCoordinates| [coords.x, coords.y, coords.z];
        let (template, response_type) = match request.request_type {
            Request::QuerySavedStructure => (structures::saved_as(world, layer, &request.name), Response::Query),
            Request::ExportFromLoad => (structures::saved_as(world, layer, &request.name), Response::Export),
            Request::ExportFromSave => {
                let [at, offset] = [corner(&request.position), corner(&request.settings.structure_offset)];
                let low: BlockPos = std::array::from_fn(|axis| at[axis] + offset[axis]);
                (structures::capture(world, layer, low, corner(&request.settings.size), !request.settings.ignore_entities, true), Response::Export)
            }
            Request::ImportFromSave | Request::Unknown(_) => continue,
        };
        let nbt = template.as_ref().and_then(structures::network_nbt);
        let response = StructureTemplateDataExportResponse { name: request.name, success: nbt.is_some(), nbt, response_type };
        if let Some(mut out) = world.get_mut::<Outgoing>(player) {
            out.push(&response);
        }
    }
}
