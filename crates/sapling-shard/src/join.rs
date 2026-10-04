//! The join sequence after login, in BDS's order with BDS's gates (sapling-data README): each
//! [`Stage`] sends its packets and waits for the client packet that opens the next one.

use acacia_proto::manual::Uuid;
use acacia_proto::packets::{
    ChunkRadiusUpdate, RequestChunkRadius, ResourcePackClientResponse, ResourcePackClientResponseResponseStatus as PackStatus, ResourcePackStack,
    ResourcePacksInfo, ResourcePacksInfoWorldTemplate, SetLocalPlayerAsInitialized,
};
use acacia_proto::types::GameMode as WireGameMode;
use bevy_ecs::prelude::*;
use sapling_api::GameMode;
use sapling_data::StartGameOptions;

use crate::components::{Arrivals, Conn, Identity, Net, NetId, Outgoing, PacketQueues, Position, Settings, Stage, Tick};
use crate::view::View;

/// Below this the client's own minimum render distance shows holes.
const MIN_VIEW_RADIUS: i32 = 2;

/// The first packet after login: no packs to download.
pub(crate) fn resource_packs_info() -> ResourcePacksInfo {
    ResourcePacksInfo {
        must_accept: false,
        has_addons: false,
        has_scripts: false,
        disable_vibrant_visuals: false,
        world_template: ResourcePacksInfoWorldTemplate { uuid: Uuid::default(), version: "0.0.0".into() },
        texture_packs: Vec::new(),
    }
}

fn resource_pack_stack() -> ResourcePackStack {
    ResourcePackStack {
        must_accept: false,
        resource_packs: Vec::new(),
        game_version: "*".into(),
        experiments: Vec::new(),
        experiments_previously_used: false,
        has_editor_packs: false,
    }
}

pub(crate) fn wire_game_mode(mode: GameMode) -> WireGameMode {
    match mode {
        GameMode::Survival => WireGameMode::Survival,
        GameMode::Creative => WireGameMode::Creative,
        GameMode::Adventure => WireGameMode::Adventure,
        GameMode::Spectator => WireGameMode::Spectator,
    }
}

type Joining<'a> = (&'a Conn, &'a Identity, &'a NetId, &'a Position, &'a mut Stage, &'a mut Outgoing);

/// Pack negotiation, then StartGame with the registries and the player's own state.
pub(crate) fn resource_packs(mut packets: ResMut<PacketQueues>, settings: Res<Settings>, tick: Res<Tick>, net: Res<Net>, mut players: Query<Joining>) {
    for (entity, response) in packets.take::<ResourcePackClientResponse>() {
        let Ok((conn, identity, id, position, mut stage, mut out)) = players.get_mut(entity) else { continue };
        match (*stage, response.response_status) {
            (Stage::PacksInfo, PackStatus::HaveAllPacks) => {
                out.push(&resource_pack_stack());
                *stage = Stage::PackStack;
            }
            (Stage::PackStack, PackStatus::Completed) => {
                let options = StartGameOptions {
                    entity_id: id.0 as i64,
                    runtime_entity_id: id.0,
                    game_mode: wire_game_mode(settings.0.game_mode),
                    position: position.feet,
                    yaw: position.yaw,
                    pitch: position.pitch,
                    world_spawn: position.feet.map(|c| c.floor() as i32),
                    world_name: settings.0.world_name.clone(),
                    tick: tick.0 as i64,
                    block_network_ids_are_hashes: sapling_world::NETWORK_IDS_ARE_HASHES,
                };
                out.0.extend_from_slice(sapling_data::pre_start_packets());
                out.push(&sapling_data::start_game(&options));
                out.0.extend_from_slice(sapling_data::registry_packets());
                out.push(&sapling_data::update_abilities(id.0 as i64));
                out.push(&sapling_data::entity_data(id.0, &identity.0.name));
                out.push(&sapling_data::update_attributes(id.0));
                *stage = Stage::StartGame;
            }
            (_, PackStatus::Refused) => net.0.kick(conn.0, "disconnectionScreen.resourcePack"),
            (stage, status) => tracing::debug!(player = %identity.0.name, ?stage, ?status, "unexpected resource pack response"),
        }
    }
}

/// The client's view distance, capped; the first request also opens the chunk stage.
pub(crate) fn chunk_radius(mut packets: ResMut<PacketQueues>, settings: Res<Settings>, mut players: Query<(&mut Stage, &mut View, &mut Outgoing)>) {
    for (entity, request) in packets.take::<RequestChunkRadius>() {
        let Ok((mut stage, mut view, mut out)) = players.get_mut(entity) else { continue };
        if *stage < Stage::StartGame {
            continue;
        }
        let radius = request.chunk_radius.clamp(MIN_VIEW_RADIUS, settings.0.max_view_radius.max(MIN_VIEW_RADIUS));
        view.set_radius(radius);
        out.push(&ChunkRadiusUpdate { chunk_radius: radius });
        if *stage == Stage::StartGame {
            *stage = Stage::Chunks;
        }
    }
}

/// The client left its loading screen: the player is in the world.
pub(crate) fn initialized(mut packets: ResMut<PacketQueues>, mut arrivals: ResMut<Arrivals>, mut players: Query<&mut Stage>) {
    for (entity, _) in packets.take::<SetLocalPlayerAsInitialized>() {
        let Ok(mut stage) = players.get_mut(entity) else { continue };
        if *stage == Stage::Loading {
            *stage = Stage::InWorld;
            arrivals.joined.push(entity);
        }
    }
}
