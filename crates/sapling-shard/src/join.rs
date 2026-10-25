//! The join sequence after login, in BDS's order with BDS's gates (sapling-data README): each
//! [`Stage`] sends its packets and waits for the client packet that opens the next one.

use acacia_proto::packets::{
    ChunkRadiusUpdate, RequestChunkRadius, ResourcePackClientResponse, ResourcePackClientResponseResponseStatus as PackStatus, SetDifficulty,
    SetLocalPlayerAsInitialized, UpdatePlayerGameType,
};
use acacia_proto::types::GameMode as WireGameMode;
use bevy_ecs::prelude::*;
use sapling_api::GameMode;
use sapling_data::StartGameOptions;

use crate::clock;
use crate::components::{Arrivals, Conn, Identity, InLayer, Layer, Mode, Net, NetId, Outgoing, PacketQueues, Position, Settings, Stage, Tick};
use crate::inventory::Inventory;
use crate::view::View;

/// Below this the client's own minimum render distance shows holes.
const MIN_VIEW_RADIUS: i32 = 2;

pub(crate) fn wire_game_mode(mode: GameMode) -> WireGameMode {
    match mode {
        GameMode::Survival => WireGameMode::Survival,
        GameMode::Creative => WireGameMode::Creative,
        GameMode::Adventure => WireGameMode::Adventure,
        GameMode::Spectator => WireGameMode::Spectator,
    }
}

/// Puts a player in `mode`. BDS's `/gamemode` sends the change to the player itself; the others
/// learn whom to show how.
pub(crate) fn set_game_mode(world: &mut World, player: Entity, mode: GameMode) {
    let (tick, Some(id)) = (world.resource::<Tick>().0, world.get::<NetId>(player).copied()) else { return };
    let Some(mut current) = world.get_mut::<Mode>(player) else { return };
    current.0 = mode;
    let packet = UpdatePlayerGameType { gamemode: wire_game_mode(mode), player_unique_id: id.0 as i64, tick };
    crate::chat::send_to_all(world, &sapling_net::encode(&packet));
}

type Joining<'a> = (&'a Conn, &'a Identity, &'a NetId, &'a Position, &'a InLayer, &'a Inventory, &'a Mode, &'a mut Stage, &'a mut Outgoing);

/// Pack negotiation, then, once storage has answered for the player (it may start in another
/// dimension: dimensions::start_in), StartGame with the registries, the player's own state and
/// the world's: spawn, time, difficulty and rules.
pub(crate) fn resource_packs(
    mut packets: ResMut<PacketQueues>,
    settings: Res<Settings>,
    tick: Res<Tick>,
    net: Res<Net>,
    layers: Query<&Layer>,
    mut players: Query<Joining>,
) {
    for (entity, response) in packets.take::<ResourcePackClientResponse>() {
        let Ok((conn, identity, .., mut stage, mut out)) = players.get_mut(entity) else { continue };
        let packs = &settings.0.resource_packs;
        match (*stage, response.response_status) {
            (Stage::PacksInfo, PackStatus::SendPacks) => {
                let infos = packs.data_infos(response.resourcepackids.as_deref().unwrap_or_default());
                infos.iter().for_each(|info| out.push(info));
            }
            (Stage::PacksInfo, PackStatus::HaveAllPacks) => {
                out.push(&packs.stack(true));
                *stage = Stage::PackStack;
            }
            (Stage::PackStack, PackStatus::Completed) => *stage = Stage::Packed,
            (_, PackStatus::Refused) if packs.required => net.0.kick(conn.0, "disconnectionScreen.resourcePack"),
            // Optional packs turned down: the world without them.
            (Stage::PacksInfo, PackStatus::Refused) => {
                out.push(&packs.stack(false));
                *stage = Stage::PackStack;
            }
            (stage, status) => tracing::debug!(player = %identity.0.name, ?stage, ?status, "unexpected resource pack response"),
        }
    }
    for (_, identity, id, position, in_layer, inventory, mode, mut stage, mut out) in &mut players {
        if *stage != Stage::Packed || !inventory.loaded {
            continue;
        }
        let Ok(layer) = layers.get(in_layer.0) else { continue };
        let level = layer.world.level();
        let options = StartGameOptions {
            entity_id: id.0 as i64,
            runtime_entity_id: id.0,
            game_mode: wire_game_mode(mode.0),
            world_game_mode: wire_game_mode(settings.0.game_mode),
            position: position.feet,
            yaw: position.yaw,
            pitch: position.pitch,
            world_spawn: layer.spawn.map(|c| c.floor() as i32),
            world_name: settings.0.world_name.clone(),
            tick: tick.0 as i64,
            difficulty: settings.0.difficulty as i32,
            rules: level.rules.clone(),
            block_network_ids_are_hashes: sapling_world::NETWORK_IDS_ARE_HASHES,
            dimension: layer.world.dimension(),
            hardcore: level.hardcore,
        };
        out.0.extend_from_slice(sapling_data::pre_start_packets());
        out.push(&sapling_data::start_game(&options));
        out.0.extend_from_slice(sapling_data::registry_packets());
        out.push(&sapling_data::update_abilities(id.0 as i64));
        out.push(&sapling_data::entity_data(id.0, &identity.0.name));
        out.push(&sapling_data::update_attributes(id.0));
        out.push(&sapling_data::no_player_spawn());
        out.push(&clock::registry(level));
        out.push(&SetDifficulty { difficulty: settings.0.difficulty as u32 });
        out.push(&level.rules.packet());
        *stage = Stage::StartGame;
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
