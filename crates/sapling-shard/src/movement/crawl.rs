//! Crawling: a player whose box meets a block over it goes down to 0.6 high, and stands again once
//! there is room, told to it and its viewers as BDS told it (research/captures/bds-f1crawl.txt: the
//! tick after a teleport under a one-high ceiling and out again; the eyes told where they were).

use acacia_physics::{Aabb, WorldView};
use acacia_proto::packets::SetEntityData;
use acacia_proto::types::{EntityProperties, MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain, MetadataFlags2};
use bevy_ecs::prelude::*;
use sapling_api::GameMode;

use super::Motion;
use crate::components::{InLayer, Layer, Mode, NetId, Outgoing, Position, Stage};
use crate::riding::Riding;
use crate::sleep::Sleeping;
use crate::visibility::ToViewers;

const CRAWLING_HEIGHT: f32 = 0.6;
const WIDTH: f32 = 0.6;
/// The `FlagsExtended` bit BDS set on a crawler.
const CRAWLING: i64 = 1 << 50;
/// Feet on a floor touch it; a box this far inside its own is what counts.
const SLACK: f32 = 1e-3;

#[derive(Component)]
pub(crate) struct Crawling;

type Crawler<'a> = (Entity, &'a NetId, &'a Stage, &'a Position, &'a Motion, &'a InLayer, &'a Mode, &'a mut Outgoing, Has<Crawling>, Has<Riding>, Has<Sleeping>);

/// Lays down each player whose box of its pose meets a block, and stands up each crawler with room.
pub(crate) fn crawl(mut commands: Commands, mut to_viewers: ResMut<ToViewers>, layers: Query<&Layer>, mut players: Query<Crawler>) {
    for (entity, id, stage, position, motion, in_layer, mode, mut out, crawling, riding, sleeping) in &mut players {
        let Ok(layer) = layers.get(in_layer.0) else { continue };
        let upright = *stage == Stage::InWorld && !riding && !sleeping && mode.0 != GameMode::Spectator && !motion.pose.gliding && !motion.pose.swimming;
        let low = upright && !fits(&layer.world, position.feet, motion.pose.height());
        if low == crawling {
            continue;
        }
        if low {
            commands.entity(entity).insert(Crawling);
        } else {
            commands.entity(entity).remove::<Crawling>();
        }
        let shown = shown(id.0, if low { CRAWLING_HEIGHT } else { motion.pose.height() }, low, motion.input_tick);
        out.push(&shown);
        to_viewers.push(entity, &shown);
    }
}

fn fits(level: &sapling_world::World, [x, y, z]: [f32; 3], height: f32) -> bool {
    let half = WIDTH / 2.0;
    let standing = Aabb { min: [x - half, y, z - half], max: [x + half, y + height, z + half] }.grow(-SLACK);
    let mut boxes = Vec::new();
    crate::terrain::physics_view(level, |view| view.collisions(&standing, &mut boxes));
    boxes.is_empty()
}

fn shown(id: u64, height: f32, crawling: bool, tick: u64) -> SetEntityData {
    let flags = sapling_data::player_flags_extended().0 | if crawling { CRAWLING } else { 0 };
    let metadata = vec![
        MetadataDictionaryItem { key: Key::BoundingboxHeight, r#type: Type::Float, legacy_type: 3, value: Value::Default(Plain::Float(height)) },
        MetadataDictionaryItem { key: Key::FlagsExtended, r#type: Type::Long, legacy_type: 7, value: Value::FlagsExtended(MetadataFlags2(flags)) },
    ];
    SetEntityData { runtime_entity_id: id, metadata, properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick }
}
