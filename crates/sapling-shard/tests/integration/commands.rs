//! Commands, permissions, teleports and game modes on a shard (harness: common/mod.rs).

use crate::common;

use acacia_proto::manual::Uuid;
use acacia_proto::packets::{AvailableCommands, CommandOutput, CommandRequest, MovePlayer, MovePlayerMode, MovePlayerTeleportCause, Text, TextContent, UpdatePlayerGameType};
use acacia_proto::types::{CommandOrigin, GameMode as WireGameMode, InputData};
use common::{Harness, all, input_at};
use sapling_api::command::Target;
use sapling_api::{Command, GameMode, Output, Permission, PlayerId, Sender, ShardCtx};
use sapling_data::EYE_HEIGHT;

#[derive(Command)]
#[command(crate = "sapling_api", aliases = ["lobby"])]
struct Hub;

impl Command for Hub {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let Some(player) = sender.player() else { return out.error("players only") };
        ctx.teleport(player.id, ctx.spawn());
        out.print("Back at the hub");
    }
}

#[derive(Command)]
#[command(crate = "sapling_api", name = "gamemode", permission = "operator")]
struct SetMode {
    mode: GameMode,
    player: Option<Target>,
}

impl Command for SetMode {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let Some(target) = self.player.map(|t| t.0).or_else(|| sender.player().cloned()) else { return out.error("whose?") };
        ctx.set_game_mode(target.id, self.mode);
        out.print(format!("{} is now in {:?}", target.name, ctx.game_mode(target.id).unwrap()));
    }
}

/// Lets a test promote a player from inside: `/promote`.
#[derive(Command)]
#[command(crate = "sapling_api")]
struct Promote;

impl Command for Promote {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, _: &mut Output) {
        let id = sender.player().expect("a player").id;
        let next = if ctx.permission(id) == Some(Permission::Member) { Permission::Operator } else { Permission::Member };
        ctx.set_permission(id, next);
    }
}

fn harness() -> Harness {
    let mut h = Harness::new();
    h.shard.add_command::<Hub>();
    h.shard.add_command::<SetMode>();
    h.shard.add_command::<Promote>();
    h
}

fn origin(byte: u8) -> CommandOrigin {
    CommandOrigin { r#type: "player".into(), uuid: Uuid([byte; 16]), request_id: "req".into(), player_entity_id: 0 }
}

fn request(line: &str, byte: u8) -> CommandRequest {
    CommandRequest { command: line.into(), origin: origin(byte), internal: false, version: "latest".into() }
}

/// (message, parameters, success) of the one answer among `packets`.
fn answer(packets: &[acacia_proto::RawPacket]) -> (String, Vec<String>, bool) {
    let outputs = all::<CommandOutput>(packets);
    assert_eq!((outputs.len(), outputs[0].output.len()), (1, 1), "{outputs:?}");
    let line = &outputs[0].output[0];
    assert_eq!(outputs[0].success_count, u32::from(line.success));
    (line.message_id.clone(), line.parameters.clone(), line.success)
}

fn listed(packets: &[acacia_proto::RawPacket]) -> Vec<Vec<String>> {
    all::<AvailableCommands>(packets).into_iter().map(|list| list.command_data.into_iter().map(|c| c.name).collect()).collect()
}

#[test]
fn a_player_gets_the_commands_it_may_use_and_a_new_list_when_that_changes() {
    let mut h = harness();
    assert_eq!(listed(h.enter(1, "Alex").to(1)), [["hub", "promote"]]);
    assert!(listed(&h.tick(1)).is_empty());

    h.receive(1, &request("/gamemode creative", 1));
    assert_eq!(answer(&h.tick(1)), ("commands.generic.unknown".into(), vec!["gamemode".into()], false));
    h.receive(1, &request("/promote", 2));
    assert_eq!(listed(&h.tick(1)), [["hub", "gamemode", "promote"]]);
    h.receive(1, &request("/gamemode creative", 3));
    assert_eq!(answer(&h.tick(1)), ("Alex is now in Creative".into(), vec![], true));
    h.receive(1, &request("/promote", 4));
    assert_eq!(listed(&h.tick(1)), [["hub", "promote"]]);
    // Someone joining later is told on its own; the others are not told again.
    let sent = h.enter(2, "Steve");
    assert_eq!((listed(sent.to(2)), listed(sent.to(1))), (vec![vec!["hub".to_owned(), "promote".to_owned()]], vec![]));
}

#[test]
fn commands_are_answered_as_bds_answers_them() {
    let mut h = harness();
    h.enter(1, "Alex");
    h.receive(1, &request("/hub", 7));
    let sent = h.tick(1);
    let output = &all::<CommandOutput>(&sent)[0];
    assert_eq!((&output.origin, output.output_type.as_str(), output.success_count, output.has_data, &output.data), (&origin(7), "alloutput", 1, false, &None));
    assert_eq!(answer(&sent), ("Back at the hub".into(), vec![], true));

    h.receive(1, &request("/LOBBY", 8));
    assert_eq!(answer(&h.tick(1)).0, "Back at the hub");
    h.receive(1, &request("/nonexistent foo", 9));
    assert_eq!(answer(&h.tick(1)), ("commands.generic.unknown".into(), vec!["nonexistent".into()], false));
    h.receive(1, &request("/hub now", 9));
    assert_eq!(answer(&h.tick(1)), ("commands.generic.syntax".into(), vec!["/hub ".into(), "now".into(), "".into()], false));

    // A command that prints nothing gets no answer; an origin with an oversized id loses it.
    h.receive(1, &request("/promote", 1));
    assert!(all::<CommandOutput>(&h.tick(1)).is_empty());
    let mut long = request("/gamemode banana", 2);
    long.origin.request_id = "x".repeat(65);
    h.receive(1, &long);
    let sent = h.tick(1);
    assert_eq!(answer(&sent), ("commands.generic.syntax".into(), vec!["/gamemode ".into(), "banana".into(), "".into()], false));
    assert_eq!(all::<CommandOutput>(&sent)[0].origin.request_id, "");
    h.receive(1, &request("/gamemode creative Nobody", 3));
    assert_eq!(answer(&h.tick(1)).0, "commands.generic.noTargetMatch");
}

fn json_lines(packets: &[acacia_proto::RawPacket]) -> Vec<String> {
    all::<Text>(packets).into_iter().filter_map(|text| if let TextContent::Json(json) = text.content { Some(json.message) } else { None }).collect()
}

#[test]
fn an_operators_command_is_echoed_to_the_other_operators_as_bds_echoes_it() {
    let mut h = harness();
    for (conn, name) in [(1, "Alex"), (2, "Steve"), (3, "Bob")] {
        h.enter(conn, name);
    }
    h.receive(1, &request("/promote", 1));
    h.receive(2, &request("/promote", 1));
    h.tick_all();

    h.receive(1, &request("/gamemode creative", 2));
    let sent = h.tick_all();
    let echo = r#"{"rawtext":[{"text":"§7§o["},{"translate":"Alex"},{"text":": "},{"translate":"Alex is now in Creative","with":{"rawtext":null}},{"text":"]"}]}
"#;
    assert_eq!((json_lines(sent.to(1)), json_lines(sent.to(2)), json_lines(sent.to(3))), (vec![], vec![echo.to_owned()], vec![]));
    // Not a failure, and not a command members may run.
    h.receive(1, &request("/gamemode creative Nobody", 3));
    h.receive(1, &request("/hub", 4));
    assert!(json_lines(h.tick_all().to(2)).is_empty());
}

#[test]
fn a_translated_line_is_filled_in_by_the_client() {
    let mut h = harness();
    h.enter(1, "Alex");
    h.remote.run(|ctx| ctx.send_translation(PlayerId(1), "commands.tp.successVictim", &["1.00, 2.00, 3.00".into()]));
    let line = r#"{"rawtext":[{"translate":"commands.tp.successVictim","with":{"rawtext":[{"text":"1.00, 2.00, 3.00"}]}}]}
"#;
    assert_eq!(json_lines(&h.tick(1)), [line]);
}

#[test]
fn only_players_in_the_world_run_commands_and_only_a_few_a_tick() {
    let mut h = harness();
    h.enter(1, "Alex");
    h.join(2, "Early");
    h.receive(2, &request("/hub", 1));
    (0..4).for_each(|n| h.receive(1, &request("/hub", n)));
    h.receive(1, &request("/", 9));
    let sent = h.tick_all();
    assert!(all::<CommandOutput>(sent.to(2)).is_empty());
    assert_eq!(all::<CommandOutput>(sent.to(1)).iter().map(|o| o.origin.uuid.0[0]).collect::<Vec<_>>(), [0, 1, 2]);
}

#[test]
fn a_teleport_moves_the_player_for_itself_and_its_viewers() {
    let mut h = harness();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    h.receive(1, &input_at([5.5, 65.0, 5.5]));
    h.tick_all();
    h.remote.run(|ctx| ctx.teleport(PlayerId(1), [-3.5, 65.0, 2.5]));
    let sent = h.tick_all();
    let own = &all::<MovePlayer>(sent.to(1))[0];
    assert_eq!((own.mode, own.position.x, own.position.y, own.position.z, own.on_ground), (MovePlayerMode::Teleport, -3.5, 65.0 + EYE_HEIGHT, 2.5, true));
    assert_eq!(own.teleport.as_ref().map(|t| t.cause), Some(MovePlayerTeleportCause::Command));
    let seen = &all::<MovePlayer>(sent.to(2))[0];
    assert_eq!((seen.mode, seen.position.x, seen.runtime_id), (MovePlayerMode::Normal, -3.5, own.runtime_id));

    // Inputs sent before the client handled the teleport are from the old place: they neither
    // move the player back nor earn a second teleport, until the client has been silent too long.
    h.receive(1, &input_at([5.5, 65.0, 5.5]));
    let sent = h.tick_all();
    assert!(all::<MovePlayer>(sent.to(1)).is_empty() && all::<MovePlayer>(sent.to(2)).is_empty());
    (0..19).for_each(|_| drop(h.tick_all()));
    h.receive(1, &input_at([5.5, 65.0, 5.5]));
    let sent = h.tick_all();
    assert_eq!((all::<MovePlayer>(sent.to(1))[0].mode, all::<MovePlayer>(sent.to(2)).len()), (MovePlayerMode::Teleport, 0));
    let mut handled = input_at([-3.0, 65.0, 2.5]);
    handled.input_data = vec![InputData::HandledTeleport];
    h.receive(1, &handled);
    assert_eq!(all::<MovePlayer>(h.tick_all().to(2))[0].position.x, -3.0);
    h.receive(1, &input_at([-2.5, 65.0, 2.5]));
    assert_eq!(all::<MovePlayer>(h.tick_all().to(2))[0].position.x, -2.5);

    // Not for those who left, and not to nowhere.
    h.remote.run(|ctx| {
        ctx.teleport(PlayerId(1), [f32::NAN, 0.0, 0.0]);
        ctx.teleport(PlayerId(9), [0.0, 0.0, 0.0]);
        let at = ctx.position(PlayerId(1)).unwrap().feet;
        ctx.send_message(PlayerId(1), &format!("{at:?} {:?}", ctx.spawn()));
    });
    let sent = h.tick_all();
    assert!(all::<MovePlayer>(sent.to(1)).is_empty());
    assert!(format!("{:?}", all::<Text>(sent.to(1))[0].content).contains("[-2.5, 65.0, 2.5] [0.5, 65.0, 0.5]"));
}

#[test]
fn a_game_mode_change_is_told_to_everyone() {
    let mut h = harness();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    h.remote.run(|ctx| {
        assert_eq!(ctx.game_mode(PlayerId(2)), Some(GameMode::Survival));
        ctx.set_game_mode(PlayerId(2), GameMode::Spectator);
        assert_eq!((ctx.game_mode(PlayerId(2)), ctx.game_mode(PlayerId(9))), (Some(GameMode::Spectator), None));
    });
    let sent = h.tick_all();
    for conn in [1, 2] {
        let update = &all::<UpdatePlayerGameType>(sent.to(conn))[0];
        assert_eq!((update.gamemode, update.player_unique_id), (WireGameMode::Spectator, 2));
    }
}
