//! A player's own inputs and state changes as plugin events, and a cancel undoing each on the
//! client: sprint and sneak, the held slot, teleports, the game mode; jumps and swings at nothing
//! are heard.

use crate::common;

use std::sync::{Arc, Mutex, mpsc};

use acacia_proto::manual::Uuid;
use acacia_proto::packets::{CommandRequest, MobEquipment, PlayerHotbar, SetEntityData, UpdatePlayerGameType};
use acacia_proto::types::{CommandOrigin, GameMode as WireGameMode, InputData, MetadataDictionaryItemValue as Value, MetadataFlags1, WindowID};
use common::{Harness, all, input_at};
use sapling_api::{
    Command, GameMode, GameModeChangeEvent, HeldSlotChangeEvent, JumpEvent, Output, Plugin, PluginBuilder, Priority, PunchAirEvent, Sender, ShardCtx, TeleportEvent,
    ToggleSneakEvent, ToggleSprintEvent,
};

const SPAWN: [f32; 3] = [0.5, 65.0, 0.5];

/// Notes every event, and cancels them all.
#[derive(Clone, Default)]
struct Veto(Arc<Mutex<Vec<String>>>);

impl Veto {
    fn heard(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
}

impl Plugin for Veto {
    fn build(&self, plugin: &mut PluginBuilder<'_>) {
        let log = self.0.clone();
        plugin.on::<ToggleSneakEvent>(Priority::Normal, move |_, event| {
            log.lock().unwrap().push(format!("sneak {}", event.sneaking()));
            event.cancel();
        });
        let log = self.0.clone();
        plugin.on::<ToggleSprintEvent>(Priority::Normal, move |_, event| {
            log.lock().unwrap().push(format!("sprint {}", event.sprinting()));
            event.cancel();
        });
        let log = self.0.clone();
        plugin.on::<JumpEvent>(Priority::Normal, move |_, _| log.lock().unwrap().push("jump".into()));
        let log = self.0.clone();
        plugin.on::<PunchAirEvent>(Priority::Normal, move |_, _| log.lock().unwrap().push("punch air".into()));
        let log = self.0.clone();
        plugin.on::<HeldSlotChangeEvent>(Priority::Normal, move |_, event| {
            log.lock().unwrap().push(format!("slot {} -> {}", event.from(), event.to()));
            event.cancel();
        });
        let log = self.0.clone();
        plugin.on::<TeleportEvent>(Priority::Normal, move |_, event| {
            log.lock().unwrap().push(format!("teleport {:?}", event.to()));
            event.cancel();
        });
        let log = self.0.clone();
        plugin.on::<GameModeChangeEvent>(Priority::Normal, move |_, event| {
            log.lock().unwrap().push(format!("mode {:?} -> {:?}", event.from(), event.to()));
            event.cancel();
        });
    }
}

/// `/hop`: moves the sender three blocks along x, as `/tp` would.
#[derive(Command)]
#[command(crate = "sapling_api")]
struct Hop;

impl Command for Hop {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, _: &mut Output) {
        let player = sender.player().expect("a player sent it").id;
        ctx.teleport(player, [3.5, 65.0, 0.5]);
    }
}

/// `/creative`: puts the sender in creative.
#[derive(Command)]
#[command(crate = "sapling_api")]
struct Creative;

impl Command for Creative {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, _: &mut Output) {
        let player = sender.player().expect("a player sent it").id;
        ctx.set_game_mode(player, GameMode::Creative);
    }
}

fn harness(veto: &Veto) -> Harness {
    let mut h = Harness::new();
    h.shard.add_plugin(veto);
    h.shard.add_command::<Hop>();
    h.shard.add_command::<Creative>();
    h.enter(1, "Steve");
    h
}

fn ask<T: Send + 'static>(h: &mut Harness, ask: impl FnOnce(&mut ShardCtx<'_>) -> T + Send + 'static) -> T {
    let (tx, rx) = mpsc::channel();
    h.remote.run(move |ctx| tx.send(ask(ctx)).expect("asked"));
    h.tick_all();
    rx.recv().expect("answered")
}

fn command(h: &mut Harness, line: &str) {
    let origin = CommandOrigin { r#type: "player".into(), uuid: Uuid([1; 16]), request_id: "req".into(), player_entity_id: 0 };
    h.receive(1, &CommandRequest { command: line.into(), origin, internal: false, version: "latest".into() });
}

fn own_flags(data: &[SetEntityData]) -> Vec<MetadataFlags1> {
    data.iter().flat_map(|d| &d.metadata).filter_map(|item| if let Value::Flags(flags) = &item.value { Some(*flags) } else { None }).collect()
}

#[test]
fn a_cancelled_sneak_or_sprint_is_undone_on_the_client_and_jumps_are_heard() {
    let veto = Veto::default();
    let mut h = harness(&veto);
    let mut input = input_at(SPAWN);
    input.input_data = vec![InputData::StartSneaking, InputData::StartSprinting, InputData::StartJumping, InputData::MissedSwing];
    h.receive(1, &input);
    let flags = own_flags(&all::<SetEntityData>(h.tick_all().to(1)));
    let last = flags.last().expect("the player is told its pose again");
    assert!(!last.contains(MetadataFlags1::SNEAKING) && !last.contains(MetadataFlags1::SPRINTING), "{flags:?}");
    assert_eq!(veto.heard(), ["sprint true", "sneak true", "jump", "punch air"]);
}

#[test]
fn a_cancelled_slot_change_puts_the_hand_back() {
    let veto = Veto::default();
    let mut h = harness(&veto);
    h.receive(1, &MobEquipment { runtime_entity_id: 1, item: sapling_data::no_item(), slot: 4, selected_slot: 4, window_id: WindowID::Inventory });
    let hotbar = all::<PlayerHotbar>(h.tick_all().to(1));
    assert_eq!(hotbar.iter().map(|h| (h.selected_slot, h.select_slot)).collect::<Vec<_>>(), [(0, true)]);
    let held = ask(&mut h, |ctx| {
        let player = ctx.players().next().unwrap().id;
        ctx.held_slot(player)
    });
    assert_eq!((held, veto.heard()), (Some(0), vec!["slot 0 -> 4".to_string()]));
}

#[test]
fn a_cancelled_teleport_or_mode_change_leaves_the_player_as_it_was() {
    let veto = Veto::default();
    let mut h = harness(&veto);
    command(&mut h, "/hop");
    h.tick_all();
    command(&mut h, "/creative");
    let modes = all::<UpdatePlayerGameType>(h.tick_all().to(1));
    assert_eq!(modes.iter().map(|m| m.gamemode).collect::<Vec<_>>(), [WireGameMode::Survival], "the client is told its mode again");
    let (feet, mode) = ask(&mut h, |ctx| {
        let player = ctx.players().next().unwrap().id;
        (ctx.position(player).unwrap().feet, ctx.game_mode(player))
    });
    assert_eq!((feet, mode), (SPAWN, Some(GameMode::Survival)));
    assert_eq!(veto.heard(), ["teleport [3.5, 65.0, 0.5]", "mode Survival -> Creative"]);
}
