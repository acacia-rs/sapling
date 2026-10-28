//! Debug shapes as ServerScriptDebugDrawer: one shape per packet under the plugin's id; a shape
//! with no type takes away what the id showed (gophertunnel's `DebugDrawerShape`).

use acacia_proto::packets::{ServerScriptDebugDrawer, ServerScriptDebugDrawerShapesItem as Item, ServerScriptDebugDrawerShapesItemShapeType as Type};
use acacia_proto::types::Vec3f;
use sapling_api::{DebugShape, ShapeKind};

fn vec3([x, y, z]: [f32; 3]) -> Vec3f {
    Vec3f { x, y, z }
}

fn erased(id: u64) -> Item {
    Item {
        network_id: id,
        shape_type: None,
        location: None,
        scale: None,
        rotation: None,
        time_left: None,
        color: None,
        text: None,
        box_bound: None,
        line_end_location: None,
        arrow_head_length: None,
        arrow_head_radius: None,
        segment_count: None,
    }
}

fn drawn(id: u64, shape: &DebugShape) -> Item {
    let mut item = Item {
        location: Some(vec3(shape.at)),
        scale: Some(shape.scale),
        rotation: Some(vec3(shape.rotation)),
        time_left: shape.seconds,
        color: Some(shape.argb()),
        ..erased(id)
    };
    item.shape_type = Some(match &shape.kind {
        ShapeKind::Line { to } => {
            item.line_end_location = Some(vec3(*to));
            Type::Line
        }
        ShapeKind::Box { bounds } => {
            item.box_bound = Some(vec3(*bounds));
            Type::Box
        }
        ShapeKind::Sphere { segments } => {
            item.segment_count = Some(*segments);
            Type::Sphere
        }
        ShapeKind::Circle { segments } => {
            item.segment_count = Some(*segments);
            Type::Circle
        }
        ShapeKind::Text(text) => {
            item.text = Some(text.clone());
            Type::Text
        }
        ShapeKind::Arrow { to, head_length, head_radius, segments } => {
            (item.line_end_location, item.arrow_head_length, item.arrow_head_radius, item.segment_count) = (Some(vec3(*to)), Some(*head_length), Some(*head_radius), Some(*segments));
            Type::Arrow
        }
        _ => return erased(id),
    });
    item
}

pub(crate) fn packet(id: u64, shape: Option<&DebugShape>) -> ServerScriptDebugDrawer {
    ServerScriptDebugDrawer { shapes: vec![shape.map_or_else(|| erased(id), |shape| drawn(id, shape))] }
}
