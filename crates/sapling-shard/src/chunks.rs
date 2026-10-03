//! Chunk sending in sub-chunk request mode: columns are announced as they enter a player's view
//! and have been loaded, a few per tick, and their sections are answered from the layer's encode
//! cache when the client asks.

use std::collections::HashMap;

use acacia_proto::packets::{NetworkChunkPublisherUpdate, PlayStatus, PlayStatusStatus, PlayerAuthInput, Subchunk, SubchunkRequest};
use acacia_proto::types::{BlockCoordinates, Vec3i8};
use bevy_ecs::prelude::*;
use sapling_data::EYE_HEIGHT;
use sapling_world::ChunkPos;

use crate::blobs::BlobCache;
use crate::components::{InLayer, Layer, Outgoing, PacketQueues, Position, Settings, Stage, Tick};
use crate::view::View;

/// Sections answered per request; vanilla asks for a few columns (under 100 sections) at a time.
const MAX_SECTIONS_PER_REQUEST: usize = 1024;
/// Requests a player may have waiting on column loads; more are dropped.
const MAX_BACKLOG: usize = 64;
/// A joining player spawns once the columns this close are sent; BDS has sent about as many
/// (26 in acacia's capture) when it sends PlayStatus(PlayerSpawn).
const SPAWN_RADIUS: i32 = 3;
/// Ticks between sweeps for columns nobody sees any more.
const UNLOAD_INTERVAL: u64 = 100;

/// Takes the columns the I/O threads loaded since the last tick.
pub(crate) fn poll_worlds(mut layers: Query<&mut Layer>) {
    for mut layer in &mut layers {
        layer.world.poll();
    }
}

/// Where the client says it is. Not validated yet: movement is the client's word until physics
/// checks arrive with milestone step 3.
pub(crate) fn player_input(mut packets: ResMut<PacketQueues>, mut players: Query<(&Stage, &mut Position)>) {
    for (entity, input) in packets.take::<PlayerAuthInput>() {
        let Ok((stage, mut position)) = players.get_mut(entity) else { continue };
        if input.block_action.is_some() || input.transaction.is_some() || input.item_stack_request.is_some() {
            tracing::debug!(tick = input.tick, actions = ?input.block_action, transaction = ?input.transaction, request = ?input.item_stack_request, "input with an action");
        }
        let eyes = [input.position.x, input.position.y, input.position.z];
        if *stage < Stage::Loading || eyes.iter().any(|c| !c.is_finite()) {
            continue;
        }
        *position = Position { feet: [eyes[0], eyes[1] - EYE_HEIGHT, eyes[2]], yaw: input.yaw, pitch: input.pitch };
    }
}

type Viewer<'a> = (&'a Position, &'a InLayer, &'a mut Stage, &'a mut View, &'a mut BlobCache, &'a mut Outgoing);

/// Follows each player with its view and announces up to a tick's budget of the columns that
/// entered it, nearest first, as they finish loading; a joining player then spawns.
pub(crate) fn send_chunks(settings: Res<Settings>, mut layers: Query<&mut Layer>, mut players: Query<Viewer>) {
    for (position, in_layer, mut stage, mut view, mut blobs, mut out) in &mut players {
        if *stage < Stage::Chunks {
            continue;
        }
        let Ok(mut layer) = layers.get_mut(in_layer.0) else { continue };
        let world = &mut layer.world;
        let [x, y, z] = position.feet.map(|c| c.floor() as i32);
        if view.recenter(ChunkPos::of_block(x, z)) {
            let radius = (view.radius() * 16) as u32;
            out.push(&NetworkChunkPublisherUpdate { coordinates: BlockCoordinates { x, y, z }, radius, saved_chunks: Vec::new() });
        }
        let budget = if blobs.is_backlogged() { 0 } else { settings.0.columns_per_tick };
        for pos in view.take_ready(budget, |pos| world.request(pos)) {
            let Some(column) = world.level_chunk(pos, blobs.is_enabled()) else { continue };
            column.blobs.iter().for_each(|blob| blobs.advertise(blob));
            out.push(&column.packet(pos.x, pos.z, world.dimension()));
        }
        if *stage == Stage::Chunks && !view.is_waiting_within(SPAWN_RADIUS.min(view.radius())) {
            out.push(&PlayStatus { status: PlayStatusStatus::PlayerSpawn });
            *stage = Stage::Loading;
        }
    }
}

/// Section requests for columns that are not loaded yet, answered once they are.
#[derive(Component, Default)]
pub(crate) struct SectionBacklog(Vec<SubchunkRequest>);

type Requester<'a> = (&'a InLayer, &'a mut BlobCache, &'a mut SectionBacklog, &'a mut Outgoing);

/// Answers section requests. A vanilla client disconnects itself ("error occurred") on a
/// ChunkNotFound entry, so sections of unloaded columns wait in the backlog for the load.
// TODO: refuse columns far outside the player's view instead of loading whatever is asked for.
pub(crate) fn sub_chunk_requests(mut packets: ResMut<PacketQueues>, mut layers: Query<&mut Layer>, mut players: Query<Requester>) {
    for (entity, request) in packets.take::<SubchunkRequest>() {
        if let Ok((_, _, mut backlog, _)) = players.get_mut(entity) {
            if backlog.0.len() < MAX_BACKLOG {
                backlog.0.push(request);
            }
        }
    }
    for (in_layer, mut blobs, mut backlog, mut out) in &mut players {
        if backlog.0.is_empty() {
            continue;
        }
        let requests = std::mem::take(&mut backlog.0);
        let Ok(mut layer) = layers.get_mut(in_layer.0) else { continue };
        let world = &mut layer.world;
        let cached = blobs.is_enabled();
        for mut request in requests {
            if request.dimension != world.dimension() {
                continue;
            }
            let origin = request.origin.clone();
            let column = |o: &Vec3i8| ChunkPos::new(origin.x + i32::from(o.x), origin.z + i32::from(o.z));
            request.requests.truncate(MAX_SECTIONS_PER_REQUEST);
            let (ready, waiting): (Vec<_>, Vec<_>) = request.requests.into_iter().partition(|o| world.request(column(o)));
            let mut entry = |o: &Vec3i8| {
                let answer = world.sub_chunk(column(o), origin.y + i32::from(o.y), [o.x, o.y, o.z], cached);
                answer.blob.iter().for_each(|blob| blobs.advertise(blob));
                answer.entry
            };
            let entries: Vec<_> = ready.iter().map(&mut entry).collect();
            if !entries.is_empty() {
                out.push(&Subchunk { cache_enabled: cached, dimension: request.dimension, origin: origin.clone(), entries });
            }
            if !waiting.is_empty() {
                tracing::debug!(origin = ?(origin.x, origin.z), sections = waiting.len(), "sections wait for their column to load");
                backlog.0.push(SubchunkRequest { dimension: request.dimension, requests: waiting, origin });
            }
        }
    }
}

/// Every few seconds, drops the loaded columns that are in no player's view.
pub(crate) fn unload_columns(tick: Res<Tick>, mut layers: Query<(Entity, &mut Layer)>, players: Query<(&InLayer, &View)>) {
    if tick.0 % UNLOAD_INTERVAL != 0 {
        return;
    }
    let mut views: HashMap<Entity, Vec<&View>> = HashMap::new();
    for (in_layer, view) in &players {
        views.entry(in_layer.0).or_default().push(view);
    }
    for (entity, mut layer) in &mut layers {
        let seen_by = views.get(&entity).map_or(&[][..], Vec::as_slice);
        layer.world.unload(|pos| seen_by.iter().any(|view| view.covers(pos)));
    }
}
