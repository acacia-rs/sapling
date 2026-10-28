//! The ctx calls for the server around the shard and for each player's own spawn point.

use acacia_proto::packets::{SetDefaultGameType, SetSpawnPosition, SetSpawnPositionSpawnType, Transfer};
use acacia_proto::types::BlockCoordinates;
use std::sync::Arc;

use sapling_api::{BlockPos, Commands, GameMode, PlayerId, SaveState, ScriptEvent, ServerBackend, SpawnBackend};

use super::Ctx;
use crate::components::{InLayer, Layer, Outgoing, Settings};
use crate::plugins::{CommandRegistry, ScriptEvents};
use crate::sleep::BedSpawn;
use crate::{chat, join};

/// What BDS sends as the bed's place of a spawn point without one.
const NO_BLOCK: BlockCoordinates = BlockCoordinates { x: i32::MIN, y: i32::MIN, z: i32::MIN };

impl ServerBackend for Ctx<'_> {
    fn seed(&self) -> i64 {
        self.layer().world.level().seed
    }

    fn default_game_mode(&self) -> GameMode {
        self.world.resource::<Settings>().0.game_mode
    }

    fn set_default_game_mode(&mut self, mode: GameMode) {
        self.world.resource_mut::<Settings>().0.game_mode = mode;
        chat::send_to_all(self.world, &sapling_net::encode(&SetDefaultGameType { gamemode: join::wire_game_mode(mode) }));
    }

    fn send_to_server(&mut self, player: PlayerId, address: &str, port: u16) -> bool {
        let Some(mut out) = self.out(player) else { return false };
        out.push(&Transfer { server_address: address.to_owned(), port, reload_world: false, gatherings_configuration: None });
        true
    }

    fn commands(&self) -> Arc<Commands> {
        self.world.resource::<CommandRegistry>().0.clone()
    }

    fn send_script_event(&mut self, event: ScriptEvent) {
        self.world.resource_mut::<ScriptEvents>().0.push(event);
    }

    fn hold_saves(&mut self) -> bool {
        crate::saving::hold(self.world)
    }

    fn save_state(&mut self) -> SaveState {
        crate::saving::query(self.world)
    }

    fn resume_saves(&mut self) -> bool {
        crate::saving::resume(self.world)
    }
}

impl SpawnBackend for Ctx<'_> {
    fn spawn_point(&self, player: PlayerId) -> Option<BlockPos> {
        let entity = self.entity(player)?;
        let layer = self.world.get::<InLayer>(entity)?.0;
        self.world.get::<BedSpawn>(entity).filter(|spawn| spawn.layer == layer).map(|spawn| spawn.spot)
    }

    fn set_spawn_point(&mut self, player: PlayerId, at: Option<BlockPos>) -> bool {
        let Some(entity) = self.entity(player) else { return false };
        let Some(layer) = self.world.get::<InLayer>(entity).map(|layer| layer.0) else { return false };
        if let Some(mut inventory) = self.world.get_mut::<crate::inventory::Inventory>(entity) {
            inventory.unsaved = true;
        }
        // BDS tells nothing of a spawn point taken away (research/captures/bds-worldcmds.txt).
        let Some([x, y, z]) = at else {
            self.world.entity_mut(entity).remove::<BedSpawn>();
            return true;
        };
        let dimension = self.world.get::<Layer>(layer).map_or(0, |layer| layer.world.dimension());
        let packet = SetSpawnPosition { spawn_type: SetSpawnPositionSpawnType::Player, player_position: BlockCoordinates { x, y, z }, dimension, world_position: NO_BLOCK };
        self.world.entity_mut(entity).insert(BedSpawn::set_by_command([x, y, z], layer));
        if let Some(mut out) = self.world.get_mut::<Outgoing>(entity) {
            out.push(&packet);
        }
        true
    }
}
