use std::sync::LazyLock;

use acacia_proto::packets::{GameRulesChanged, StartGame};
use acacia_proto::types::{BlockCoordinates, GameMode, GameRuleI32, GameRuleI32Value, Vec2f, Vec3f};

use crate::templates::{START_GAME, decode};

const DAYLIGHT_CYCLE: &str = "dodaylightcycle";

/// Player positions on the wire are the eyes, this far above the feet: BDS's own offset
/// (acacia-physics `DEFAULT_PLAYER_HEIGHT_OFFSET`), not the round 1.62.
pub const EYE_HEIGHT: f32 = 1.62001;

/// What a server decides per join; every other StartGame field keeps BDS's value.
#[derive(Debug, Clone, PartialEq)]
pub struct StartGameOptions {
    pub entity_id: i64,
    pub runtime_entity_id: u64,
    pub game_mode: GameMode,
    /// Feet position.
    pub position: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub world_spawn: [i32; 3],
    pub world_name: String,
    pub tick: i64,
    /// 0 peaceful to 3 hard.
    pub difficulty: i32,
    /// Whether the client advances the time of day by itself (the `dodaylightcycle` game rule).
    pub daylight_cycle: bool,
    /// Whether chunk palettes and block updates carry state hashes instead of runtime ids.
    pub block_network_ids_are_hashes: bool,
}

static TEMPLATE: LazyLock<StartGame> = LazyLock::new(|| decode(START_GAME));

fn rules(daylight_cycle: bool) -> Vec<GameRuleI32> {
    let mut rules = TEMPLATE.gamerules.clone();
    for rule in rules.iter_mut().filter(|rule| rule.name.eq_ignore_ascii_case(DAYLIGHT_CYCLE)) {
        rule.value = GameRuleI32Value::Bool(daylight_cycle);
    }
    rules
}

/// Every game rule with BDS's value but the daylight cycle's; BDS repeats them after StartGame.
pub fn game_rules(daylight_cycle: bool) -> GameRulesChanged {
    GameRulesChanged { rules: rules(daylight_cycle) }
}

pub fn start_game(o: &StartGameOptions) -> StartGame {
    let [x, y, z] = o.position;
    let [sx, sy, sz] = o.world_spawn;
    StartGame {
        difficulty: o.difficulty,
        gamerules: rules(o.daylight_cycle),
        entity_id: o.entity_id,
        runtime_entity_id: o.runtime_entity_id,
        player_gamemode: o.game_mode,
        world_gamemode: o.game_mode,
        player_position: Vec3f { x, y: y + EYE_HEIGHT, z },
        rotation: Vec2f { x: o.pitch, z: o.yaw },
        spawn_position: BlockCoordinates { x: sx, y: sy, z: sz },
        level_id: o.world_name.clone(),
        world_name: o.world_name.clone(),
        current_tick: o.tick,
        seed: 0,
        // BDS lets the client generate terrain it has the seed for; ours comes only from the server.
        client_side_generation: false,
        block_network_ids_are_hashes: o.block_network_ids_are_hashes,
        ..TEMPLATE.clone()
    }
}

#[cfg(test)]
mod tests {
    use acacia_proto::Packet;
    use bytes::BytesMut;

    use super::*;

    fn options() -> StartGameOptions {
        StartGameOptions {
            entity_id: 7,
            runtime_entity_id: 7,
            game_mode: GameMode::Adventure,
            position: [0.5, 65.0, 0.5],
            yaw: 90.0,
            pitch: 0.0,
            world_spawn: [0, 65, 0],
            world_name: "hub".into(),
            tick: 12,
            difficulty: 2,
            daylight_cycle: false,
            block_network_ids_are_hashes: true,
        }
    }

    #[test]
    fn template_reencodes_byte_exactly() {
        let template: StartGame = decode(START_GAME);
        let mut out = BytesMut::new();
        template.encode(&mut out);
        assert_eq!(&out[..], START_GAME);
        assert!(template.block_network_ids_are_hashes && template.client_side_generation);
    }

    #[test]
    fn overrides_survive_a_roundtrip() {
        let mut out = BytesMut::new();
        start_game(&options()).encode(&mut out);
        let sg = StartGame::decode(&mut &out[..]).unwrap();
        assert_eq!((sg.entity_id, sg.runtime_entity_id, sg.player_gamemode), (7, 7, GameMode::Adventure));
        assert_eq!((sg.player_position.x, sg.player_position.y, sg.player_position.z), (0.5, 65.0 + EYE_HEIGHT, 0.5));
        assert_eq!((sg.world_name.as_str(), sg.current_tick, sg.client_side_generation), ("hub", 12, false));
        assert!(!sg.block_properties.is_empty(), "data-driven vanilla blocks are kept");
        let cycle: Vec<_> = sg.gamerules.iter().filter(|rule| rule.name.eq_ignore_ascii_case(DAYLIGHT_CYCLE)).map(|rule| rule.value.clone()).collect();
        assert_eq!((sg.difficulty, cycle), (2, vec![GameRuleI32Value::Bool(false)]), "BDS lists the rule, so it can be set");
        assert_eq!(game_rules(false).rules, sg.gamerules);
    }
}
