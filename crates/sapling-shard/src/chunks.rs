//! Chunk sending in sub-chunk request mode: columns are announced as they enter a player's view
//! and have been loaded, a few per tick, and their sections are answered from the layer's encode
//! cache when the client asks.

use std::collections::HashMap;

use acacia_proto::packets::{NetworkChunkPublisherUpdate, PlayStatus, PlayStatusStatus, Subchunk, SubchunkRequest};
use acacia_proto::types::{BlockCoordinates, Vec3i8};
use bevy_ecs::prelude::*;
use sapling_world::ChunkPos;

use crate::blobs::BlobCache;
use crate::components::{Conn, InLayer, Layer, Net, NetId, Outgoing, PacketQueues, Position, Settings, Stage, Tick};
use crate::dimensions::{self, Arriving};
use crate::kept::Kept;
use crate::view::View;

/// Sections answered per request; vanilla asks for a few columns (under 100 sections) at a time.
const MAX_SECTIONS_PER_REQUEST: usize = 1024;
/// Sections a player may have waiting, per column of its view's square: more than any dimension
/// has, so only a client asking for sections over again reaches it. Vanilla never asks twice, so
/// a section dropped below this would stay a hole.
const MAX_WAITING_PER_COLUMN: usize = 32;
/// A joining player spawns once the columns this close are sent; BDS has sent about as many
/// (26 in acacia's capture) when it sends PlayStatus(PlayerSpawn).
const SPAWN_RADIUS: i32 = 3;
/// Ticks between sweeps for columns to save and columns nobody sees any more.
pub(crate) const UNLOAD_INTERVAL: u64 = 100;
/// Ticks between drops of queued loads that left every view, so the columns a flight or teleport
/// passed are not generated.
const CANCEL_INTERVAL: u64 = 10;

/// Takes the columns the I/O threads loaded since the last tick and lights each world anew.
pub(crate) fn poll_and_relight_worlds(mut layers: Query<&mut Layer>) {
    for mut layer in &mut layers {
        layer.world.poll();
        layer.world.relight();
    }
}

type Viewer<'a> = (&'a NetId, &'a Conn, &'a Position, &'a InLayer, &'a mut Stage, &'a mut View, &'a mut BlobCache, &'a mut Outgoing, Option<&'a mut Arriving>);

/// Follows each player with its view and announces up to a tick's budget of the columns that
/// entered it, nearest first, as they finish loading; a joining player then spawns, and one
/// changing dimension is told it may leave its loading screen (dimensions.rs). A congested
/// connection or a full blob cache gets none until it drains: announcements are what can wait,
/// section answers (`sub_chunk_requests`) are not.
pub(crate) fn send_chunks(settings: Res<Settings>, net: Res<Net>, mut layers: Query<&mut Layer>, mut players: Query<Viewer>) {
    for (id, conn, position, in_layer, mut stage, mut view, mut blobs, mut out, mut arriving) in &mut players {
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
        let held = blobs.is_backlogged() || net.0.is_congested(conn.0);
        let budget = if held { 0 } else { settings.0.columns_per_tick };
        // A joining or arriving player waits on its spawn columns: those go ahead of other players' loads.
        let waits = *stage == Stage::Chunks || arriving.as_ref().is_some_and(|arriving| !arriving.told_ready);
        let spawn = waits.then(|| view.center()).flatten();
        let spawn_radius = SPAWN_RADIUS.min(view.radius());
        let urgent = |pos: ChunkPos| spawn.is_some_and(|c| (pos.x - c.x).pow(2) + (pos.z - c.z).pow(2) <= spawn_radius * spawn_radius);
        for pos in view.take_ready(budget, |pos| if urgent(pos) { world.request_urgent(pos) } else { world.request(pos) }) {
            let Some(column) = world.level_chunk(pos, blobs.is_enabled()) else { continue };
            column.blobs.iter().for_each(|blob| blobs.advertise(blob));
            out.push(&column.packet(pos.x, pos.z, world.dimension()));
        }
        if !spawn_columns_sent(&view) {
            continue;
        }
        if *stage == Stage::Chunks {
            out.push(&PlayStatus { status: PlayStatusStatus::PlayerSpawn });
            *stage = Stage::Loading;
        }
        if let Some(arriving) = arriving.as_mut().filter(|arriving| !arriving.told_ready && arriving.trip.is_none() && !arriving.to_platform) {
            out.push(&dimensions::ready(id.0));
            arriving.told_ready = true;
        }
    }
}

/// Whether the columns a joining or arriving player waits on have been sent to it.
pub(crate) fn spawn_columns_sent(view: &View) -> bool {
    !view.is_waiting_within(SPAWN_RADIUS.min(view.radius()))
}

/// Section requests for columns that are not loaded yet, answered once they are.
#[derive(Component, Default)]
pub(crate) struct SectionBacklog(Vec<SubchunkRequest>);

impl SectionBacklog {
    /// Drops what waits: the player left the world these requests were about.
    pub fn clear(&mut self) {
        self.0.clear();
    }
}

type Requester<'a> = (&'a InLayer, &'a View, &'a mut BlobCache, &'a mut SectionBacklog, &'a mut Outgoing);

/// Answers section requests. A vanilla client disconnects itself ("error occurred") on a
/// ChunkNotFound entry, so sections wait in the backlog until their column was announced: one
/// that is still loading, and after a world change one the client asked for in the old world,
/// which is then answered from the new one. Sections outside the view are dropped.
pub(crate) fn sub_chunk_requests(mut packets: ResMut<PacketQueues>, mut layers: Query<&mut Layer>, mut players: Query<Requester>) {
    for (entity, mut request) in packets.take::<SubchunkRequest>() {
        let Ok((_, view, _, mut backlog, _)) = players.get_mut(entity) else { continue };
        // A client may ask for no sections of a column that has none; that is not a request.
        if request.requests.is_empty() {
            continue;
        }
        request.requests.truncate(MAX_SECTIONS_PER_REQUEST);
        let waiting: usize = backlog.0.iter().map(|waiting| waiting.requests.len()).sum();
        let limit = (2 * view.radius() as usize + 1).pow(2) * MAX_WAITING_PER_COLUMN;
        if waiting + request.requests.len() > limit {
            tracing::warn!(waiting, asked = request.requests.len(), limit, "section requests over the limit dropped");
            continue;
        }
        backlog.0.push(request);
    }
    for (in_layer, view, mut blobs, mut backlog, mut out) in &mut players {
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
            request.requests.retain(|o| view.covers(column(o)));
            let (ready, waiting): (Vec<_>, Vec<_>) = request.requests.into_iter().partition(|o| view.has_sent(column(o)) && world.request(column(o)));
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

/// Every few seconds, hands the columns written since the last sweep to storage and drops the
/// loaded ones that are in no player's view.
pub(crate) fn save_and_unload_columns(tick: Res<Tick>, mut layers: Query<(Entity, &mut Layer, Option<&Kept>)>, players: Query<(&InLayer, &View)>) {
    if tick.0 % UNLOAD_INTERVAL != 0 {
        return;
    }
    let views = views_by_layer(&players);
    for (entity, mut layer, kept) in &mut layers {
        let seen = seen_in(&views, entity, kept);
        layer.world.save_modified();
        layer.world.unload(seen);
    }
}

/// Drops the queued loads of columns no view covers any more; a column already loading finishes.
pub(crate) fn cancel_unseen_loads(tick: Res<Tick>, mut layers: Query<(Entity, &mut Layer, Option<&Kept>)>, players: Query<(&InLayer, &View)>) {
    if tick.0 % CANCEL_INTERVAL != 0 {
        return;
    }
    let views = views_by_layer(&players);
    for (entity, mut layer, kept) in &mut layers {
        let cancelled = layer.world.cancel_loads(seen_in(&views, entity, kept));
        if cancelled > 0 {
            tracing::debug!(cancelled, "dropped loads nobody sees");
        }
    }
}

fn views_by_layer<'a>(players: &'a Query<(&InLayer, &View)>) -> HashMap<Entity, Vec<&'a View>> {
    let mut views: HashMap<Entity, Vec<&View>> = HashMap::new();
    for (in_layer, view) in players {
        views.entry(in_layer.0).or_default().push(view);
    }
    views
}

/// Whether any view in `layer` covers a column, or the world keeps it (kept.rs).
fn seen_in<'a>(views: &'a HashMap<Entity, Vec<&'a View>>, layer: Entity, kept: Option<&'a Kept>) -> impl Fn(ChunkPos) -> bool + 'a {
    let seen_by = views.get(&layer).map_or(&[][..], Vec::as_slice);
    move |pos| seen_by.iter().any(|view| view.covers(pos)) || kept.is_some_and(|kept| kept.holds(pos))
}
