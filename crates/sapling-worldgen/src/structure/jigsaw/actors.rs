//! The mobs a pool element's data markers name, "minecraft:villager_v2:minecraft:spawn_from_village"
//! (`StructurePoolElement` marker handler 145e1e2a0, spawned by `postProcessMobsAt` 141ef28c0): kind
//! and event, made at the corner of the marker's cell. An abandoned village's actor rules (1480bbe90)
//! turn its villagers into zombie villagers.

use super::super::canvas::Canvas;
use super::super::template::{Rotation, Template};
use crate::block::Block;
use crate::chunk::{SavedCompound, TemplateActor};

const VILLAGER: &str = "minecraft:villager_v2:minecraft:spawn_from_village";
const GOLEM: &str = "minecraft:iron_golem:minecraft:from_village";

/// Files the actors of the markers among `placed` (the element's blocks inside the chunk).
pub(super) fn add(canvas: &mut Canvas, template: &'static Template, placed: &[((i32, i32, i32), Block, usize)], abandoned: bool) {
    for &(at, _, index) in placed {
        let Some((_, marker)) = template.markers.iter().find(|(block, _)| *block == index) else { continue };
        let named = match marker.as_str() {
            VILLAGER if abandoned => "minecraft:zombie_villager_v2:from_village",
            // TODO: the rule BDS's abandoned villages put on golems was not decoded; taken as removing them.
            GOLEM if abandoned => continue,
            marker => marker,
        };
        let (kind, event) = split(named);
        canvas.add_actor(kind, event, at);
    }
}

/// Files the actors a .mcstructure element saved (a bastion's mob pieces: 145e1c910, `placeInWorld`
/// 141f52300 with blocks ignored), feet and `HomePos` turned with the piece (`turned`), yaw a quarter
/// a turn; made from their compounds (`ActorFactory` 141ce8200), with no event (bds-az3bastion.txt).
pub(super) fn add_saved(canvas: &mut Canvas, template: &'static Template, origin: (i32, i32, i32), rotation: Rotation) {
    for actor in &template.actors {
        let yaw = (actor.rotation[0] + 90.0 * rotation as i32 as f32 + 180.0).rem_euclid(360.0) - 180.0;
        let saved = SavedCompound { nbt: actor.nbt, rotation: [yaw, actor.rotation[1]], home: actor.home.map(|home| turned(rotation, origin, home)) };
        canvas.file_actor(TemplateActor { kind: actor.kind, event: "minecraft:entity_spawned", feet: turned(rotation, origin, actor.pos), saved: Some(saved) });
    }
}

/// A template point at `origin` turned about the template's origin as its block cells are, the cells
/// keeping their extent: a turned piece's cell (0, 0) still spans its first block, so the point turns
/// about that block's middle (BDS's feet and homes, bds-az3bastion.txt and its world's dump).
fn turned(rotation: Rotation, origin: (i32, i32, i32), [x, y, z]: [f32; 3]) -> [f32; 3] {
    let (dx, dz) = (x - 0.5, z - 0.5);
    let (dx, dz) = match rotation {
        Rotation::None => (dx, dz),
        Rotation::Clockwise90 => (-dz, dx),
        Rotation::Clockwise180 => (-dx, -dz),
        Rotation::CounterClockwise90 => (dz, -dx),
    };
    // One lower than the template says, feet and homes alike (BDS's dump); the piece places no block to show its own y.
    [origin.0 as f32 + 0.5 + dx, origin.1 as f32 + y - 1.0, origin.2 as f32 + 0.5 + dz]
}

/// "namespace:id:event" as kind and event, the event `minecraft:entity_spawned` where there is none.
fn split(marker: &'static str) -> (&'static str, &'static str) {
    let id_end = marker.find(':').and_then(|ns| marker[ns + 1..].find(':').map(|id| ns + 1 + id));
    match id_end {
        Some(end) => (&marker[..end], &marker[end + 1..]),
        None => (marker, "minecraft:entity_spawned"),
    }
}

#[cfg(test)]
#[test]
fn a_marker_names_its_kind_then_its_event() {
    assert_eq!(split(VILLAGER), ("minecraft:villager_v2", "minecraft:spawn_from_village"));
    assert_eq!(split("minecraft:cow"), ("minecraft:cow", "minecraft:entity_spawned"));
    assert_eq!(split("minecraft:zombie_villager_v2:from_village"), ("minecraft:zombie_villager_v2", "from_village"));
}
