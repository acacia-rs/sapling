//! The mobs a pool element's data markers name, "minecraft:villager_v2:minecraft:spawn_from_village"
//! (`StructurePoolElement` marker handler 145e1e2a0, spawned by `postProcessMobsAt` 141ef28c0): kind
//! and event, made at the corner of the marker's cell. An abandoned village's actor rules (1480bbe90)
//! turn its villagers into zombie villagers.

use super::super::canvas::Canvas;
use super::super::template::Template;
use crate::block::Block;

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
