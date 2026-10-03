//! Chunk sending in sub-chunk request mode: columns are announced as they enter a player's view,
//! and their sections are answered from the layer's encode cache when the client asks.

use acacia_proto::packets::{NetworkChunkPublisherUpdate, PlayStatus, PlayStatusStatus, PlayerAuthInput, Subchunk, SubchunkRequest};
use acacia_proto::types::BlockCoordinates;
use bevy_ecs::prelude::*;
use sapling_data::EYE_HEIGHT;
use sapling_world::ChunkPos;

use crate::components::{InLayer, Layer, Outgoing, PacketQueues, Position, Stage};
use crate::view::View;

/// Sections answered per request; vanilla asks for a few columns (under 100 sections) at a time.
const MAX_SECTIONS_PER_REQUEST: usize = 1024;

/// Where the client says it is. Not validated yet: movement is the client's word until physics
/// checks arrive with milestone step 3.
pub(crate) fn player_input(mut packets: ResMut<PacketQueues>, mut players: Query<(&Stage, &mut Position)>) {
    for (entity, input) in packets.take::<PlayerAuthInput>() {
        let Ok((stage, mut position)) = players.get_mut(entity) else { continue };
        let eyes = [input.position.x, input.position.y, input.position.z];
        if *stage < Stage::Loading || eyes.iter().any(|c| !c.is_finite()) {
            continue;
        }
        *position = Position { feet: [eyes[0], eyes[1] - EYE_HEIGHT, eyes[2]], yaw: input.yaw, pitch: input.pitch };
    }
}

/// Announces the columns that entered each player's view; a joining player then spawns.
pub(crate) fn send_chunks(layers: Query<&Layer>, mut players: Query<(&Position, &InLayer, &mut Stage, &mut View, &mut Outgoing)>) {
    for (position, in_layer, mut stage, mut view, mut out) in &mut players {
        if *stage < Stage::Chunks {
            continue;
        }
        let Ok(layer) = layers.get(in_layer.0) else { continue };
        let [x, y, z] = position.feet.map(|c| c.floor() as i32);
        if let Some(entered) = view.recenter(ChunkPos::of_block(x, z)) {
            let radius = (view.radius() * 16) as u32;
            out.push(&NetworkChunkPublisherUpdate { coordinates: BlockCoordinates { x, y, z }, radius, saved_chunks: Vec::new() });
            for pos in entered {
                out.push(&layer.world.level_chunk(pos));
            }
        }
        if *stage == Stage::Chunks {
            out.push(&PlayStatus { status: PlayStatusStatus::PlayerSpawn });
            *stage = Stage::Loading;
        }
    }
}

pub(crate) fn sub_chunk_requests(mut packets: ResMut<PacketQueues>, layers: Query<&Layer>, mut players: Query<(&InLayer, &mut Outgoing)>) {
    for (entity, request) in packets.take::<SubchunkRequest>() {
        let Ok((in_layer, mut out)) = players.get_mut(entity) else { continue };
        let Ok(layer) = layers.get(in_layer.0) else { continue };
        if request.dimension != layer.world.dimension() {
            continue;
        }
        let origin = &request.origin;
        let entry = |offset: &acacia_proto::types::Vec3i8| {
            let pos = ChunkPos::new(origin.x + i32::from(offset.x), origin.z + i32::from(offset.z));
            layer.world.sub_chunk(pos, origin.y + i32::from(offset.y), [offset.x, offset.y, offset.z])
        };
        let entries = request.requests.iter().take(MAX_SECTIONS_PER_REQUEST).map(entry).collect();
        out.push(&Subchunk { cache_enabled: false, dimension: request.dimension, origin: request.origin.clone(), entries });
    }
}
