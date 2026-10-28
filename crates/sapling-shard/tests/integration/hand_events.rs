//! The hand and world notices of docs/research/ui-events.md: each scene with a plugin that logs
//! every event and, as `Log::cancelling`, cancels each that can be.

use crate::common;

use std::sync::{Arc, Mutex, mpsc};

use acacia_proto::packets::{
    BlockPickRequest, BookEdit, BookEditContent, BookEditContentReplacePage, BookEditType, ContainerOpen, InventoryContent, LecternUpdate, LevelEvent, LevelEventEvent, LevelSoundEvent,
    PlayerHotbar, ServerboundDiagnostics,
};
use acacia_proto::types::{Action as BlockAction, ContainerSlotType as Container, ItemStackResponsesItemStatus as Status, Vec3i};
use common::requests::{answer, place, request, slot};
use common::shooting::{ALEX, draw, ready};
use common::{Harness, all, block_actions, click};
use sapling_api::{
    Block, BlockPickEvent, ContainerSlotEvent, DiagnosticsEvent, EntityDespawnEvent, FireExtinguishEvent, GameMode, Item, ItemDamageEvent, ItemReleaseEvent, LecternPageTurnEvent,
    LiquidDecayEvent, Plugin, PluginBuilder, Priority, SetOnFireEvent, ShardCtx, SoundEvent, StartBreakEvent, WorldCloseEvent,
};

#[derive(Clone, Default)]
struct Log {
    lines: Arc<Mutex<Vec<String>>>,
    cancel: bool,
}

impl Log {
    fn cancelling() -> Self {
        Self { cancel: true, ..Self::default() }
    }

    fn heard(&self, what: &str) -> bool {
        self.lines.lock().unwrap().iter().any(|line| line.starts_with(what))
    }
}

macro_rules! log {
    ($plugin:expr, $log:expr, cancel: $($event:ty => $say:expr),*; notice: $($notice:ty => $told:expr),*) => {
        $(
            let (lines, cancel) = ($log.lines.clone(), $log.cancel);
            $plugin.on::<$event>(Priority::Normal, move |_, event| {
                lines.lock().unwrap().push($say(&*event));
                if cancel {
                    event.cancel();
                }
            });
        )*
        $(
            let lines = $log.lines.clone();
            $plugin.on::<$notice>(Priority::Normal, move |_, event| lines.lock().unwrap().push($told(&*event)));
        )*
    };
}

impl Plugin for Log {
    fn build(&self, plugin: &mut PluginBuilder<'_>) {
        log!(plugin, self,
            cancel:
            StartBreakEvent => |e: &StartBreakEvent| format!("start {:?}", e.pos()),
            BlockPickEvent => |e: &BlockPickEvent| format!("pick {}", e.block().name),
            ItemReleaseEvent => |e: &ItemReleaseEvent| format!("release {}", e.item().name),
            ItemDamageEvent => |e: &ItemDamageEvent| format!("damage {} {}", e.item().name, e.damage),
            ContainerSlotEvent => |e: &ContainerSlotEvent| format!("slot {} {:?} {}", e.slot(), e.change(), e.item().count),
            LecternPageTurnEvent => |e: &LecternPageTurnEvent| format!("page {} {}", e.from(), e.to),
            SetOnFireEvent => |_: &SetOnFireEvent| "fire".to_owned(),
            FireExtinguishEvent => |e: &FireExtinguishEvent| format!("extinguish {:?}", e.pos()),
            LiquidDecayEvent => |e: &LiquidDecayEvent| format!("decay {:?} {}", e.pos(), e.after().name),
            SoundEvent => |e: &SoundEvent| format!("sound {}", e.name());
            notice:
            DiagnosticsEvent => |e: &DiagnosticsEvent| format!("fps {}", e.diagnostics().frames_per_second),
            EntityDespawnEvent => |e: &EntityDespawnEvent| format!("gone {}", e.kind()),
            WorldCloseEvent => |e: &WorldCloseEvent| format!("close {}", e.name())
        );
    }
}

fn harness(log: &Log) -> Harness {
    let mut h = Harness::new();
    h.shard.add_plugin(log);
    h.enter(1, "Alex");
    h
}

fn ask<T: Send + 'static>(h: &mut Harness, ask: impl FnOnce(&mut ShardCtx<'_>) -> T + Send + 'static) -> T {
    let (tx, rx) = mpsc::channel();
    h.remote.run(move |ctx| tx.send(ask(ctx)).expect("asked"));
    h.tick_all();
    rx.recv().expect("answered")
}

fn name(h: &mut Harness, pos: [i32; 3]) -> String {
    ask(h, move |ctx| ctx.block(pos).map(|block| block.name).unwrap_or_default())
}

fn sounds(sent: &[acacia_proto::RawPacket]) -> Vec<String> {
    all::<LevelSoundEvent>(sent).into_iter().map(|sound| sound.sound_id).collect()
}

#[test]
fn a_cancelled_start_break_stops_the_crack_on_the_breakers_screen() {
    let stops = |log: &Log| {
        let mut h = harness(log);
        h.receive(1, &block_actions(&[(BlockAction::StartBreak, [1, 64, 0])]));
        all::<LevelEvent>(&h.tick(1)).iter().filter(|event| event.event == LevelEventEvent::BlockStopBreak).count()
    };
    let (cancelling, listening) = (Log::cancelling(), Log::default());
    assert_eq!((stops(&cancelling), stops(&listening)), (1, 0));
    assert!(cancelling.heard("start [1, 64, 0]") && listening.heard("start [1, 64, 0]"));
}

#[test]
fn a_punch_puts_out_the_fire_on_the_face_unless_cancelled_and_its_sound_is_offered() {
    let punch = |log: &Log| {
        let mut h = harness(log);
        ask(&mut h, |ctx| assert!(ctx.set_block([1, 65, 0], &Block::new("minecraft:fire"))));
        h.receive(1, &block_actions(&[(BlockAction::StartBreak, [1, 64, 0])]));
        let heard = sounds(&h.tick(1));
        (name(&mut h, [1, 65, 0]), heard)
    };
    let cancelling = Log::cancelling();
    assert_eq!(punch(&cancelling).0, "minecraft:fire");
    assert!(cancelling.heard("extinguish [1, 65, 0]") && !cancelling.heard("start"));
    let listening = Log::default();
    assert_eq!(punch(&listening), ("minecraft:air".to_owned(), vec!["extinguish.fire".to_owned()]));
    assert!(listening.heard("sound extinguish.fire"));
}

#[test]
fn a_cancelled_pick_puts_nothing_in_the_hand() {
    let log = Log::cancelling();
    let mut h = harness(&log);
    ask(&mut h, |ctx| {
        ctx.set_game_mode(ALEX, GameMode::Creative);
        assert!(ctx.set_block([2, 65, 0], &Block::new("minecraft:oak_log")));
    });
    h.receive(1, &BlockPickRequest { x: 2, y: 65, z: 0, add_user_data: false, selected_slot: 0 });
    assert!(all::<PlayerHotbar>(&h.tick(1)).is_empty());
    assert!(log.heard("pick minecraft:oak_log"));
}

#[test]
fn a_cancelled_release_looses_nothing_and_wear_is_offered() {
    let arrows = |log: &Log| {
        let mut h = ready(GameMode::Survival, &[("minecraft:bow", 1, 0), ("minecraft:arrow", 8, 0)], 0.0);
        h.shard.add_plugin(log);
        let sent = draw(&mut h, 25, 3);
        all::<acacia_proto::packets::AddEntity>(&sent).iter().filter(|added| added.entity_type == "minecraft:arrow").count()
    };
    let (cancelling, listening) = (Log::cancelling(), Log::default());
    assert_eq!((arrows(&cancelling), arrows(&listening)), (0, 1));
    assert!(cancelling.heard("release minecraft:bow") && !cancelling.heard("damage"));
    assert!(listening.heard("damage minecraft:bow 1"), "{:?}", listening.lines);
}

#[test]
fn a_cancelled_slot_change_refuses_the_request() {
    let status = |log: &Log| {
        let mut h = harness(log);
        ask(&mut h, |ctx| {
            assert!(ctx.set_block([1, 65, 0], &Block::new("chest")));
            assert_eq!(ctx.give(ALEX, &Item::new("minecraft:stone", 10)), 0);
        });
        let stone = all::<InventoryContent>(&h.tick(1)).first().and_then(|main| main.input[0].stack_id);
        h.receive(1, &click([1, 65, 0], 1, None));
        assert_eq!(all::<ContainerOpen>(&h.tick(1)).len(), 1);
        h.receive(1, &request(-3, vec![place(4, slot(Container::Hotbar, 0, stone.unwrap_or(0)), slot(Container::Container, 0, 0))]));
        answer(&h.tick(1)).0
    };
    let (cancelling, listening) = (Log::cancelling(), Log::default());
    assert_eq!((status(&cancelling), status(&listening)), (Status::Error, Status::Ok));
    assert!(cancelling.heard("slot 0 Place 4"), "{:?}", cancelling.lines);
}

fn writing(page_number: i32) -> BookEdit {
    let content = BookEditContent::ReplacePage(BookEditContentReplacePage { page_number, text: format!("page {page_number}"), photo_name: String::new() });
    BookEdit { inventory_slot: 0, r#type: BookEditType::ReplacePage, content }
}

#[test]
fn a_cancelled_page_turn_shows_the_lectern_as_it_was() {
    let log = Log::cancelling();
    let mut h = ready(GameMode::Survival, &[("minecraft:writable_book", 1, 0)], 30.0);
    h.shard.add_plugin(&log);
    h.receive(1, &writing(0));
    h.receive(1, &writing(1));
    h.tick_all();
    ask(&mut h, |ctx| assert!(ctx.set_block([0, 65, 2], &Block::new("minecraft:lectern"))));
    h.receive(1, &click([0, 65, 2], 1, None));
    h.tick_all();
    h.receive(1, &LecternUpdate { page: 1, page_count: 2, position: Vec3i { x: 0, y: 65, z: 2 } });
    let shown = all::<acacia_proto::packets::BlockEntityData>(h.tick_all().to(1)).pop().expect("the lectern again");
    assert_eq!(shown.nbt.decode().unwrap().value.get("page"), Some(&acacia_proto::nbt::Value::Int(0)));
    assert!(log.heard("page 0 1"));
}

#[test]
fn catching_fire_is_offered_and_diagnostics_reach_plugins() {
    let log = Log::cancelling();
    let mut h = harness(&log);
    ask(&mut h, |ctx| assert!(ctx.set_block([0, 65, 0], &Block::new("minecraft:fire"))));
    (0..3).for_each(|_| drop(h.tick_all()));
    assert!(log.heard("fire"), "{:?}", log.lines);
    let diagnostics = ServerboundDiagnostics {
        average_frames_per_second: 60.0,
        average_server_sim_tick_time: 0.0,
        average_client_sim_tick_time: 0.0,
        average_begin_frame_time: 0.0,
        average_input_time: 0.0,
        average_render_time: 0.0,
        average_end_frame_time: 0.0,
        average_remainder_time_percent: 0.0,
        average_unaccounted_time_percent: 0.0,
        memory_category_values: Vec::new(),
        entity_diagnostics: Vec::new(),
        system_diagnostics: Vec::new(),
        system_categories: Vec::new(),
        whisker_scopes: Vec::new(),
    };
    h.receive(1, &diagnostics);
    h.tick(1);
    assert!(log.heard("fps 60"));
}

#[test]
fn cancelled_decay_keeps_the_stream_its_source_left() {
    let stream = |log: &Log| {
        let mut h = harness(log);
        // A plugin's set_block wakes no liquid; a break does.
        ask(&mut h, |ctx| assert!(ctx.set_block([3, 65, 3], &Block::new("minecraft:water")) && ctx.set_block([4, 65, 3], &Block::new("minecraft:stone"))));
        ask(&mut h, |ctx| assert!(ctx.break_block([4, 65, 3])));
        (0..20).for_each(|_| drop(h.tick_all()));
        assert_eq!(name(&mut h, [4, 65, 3]), "minecraft:water");
        ask(&mut h, |ctx| assert!(ctx.set_block([3, 65, 3], &Block::new("minecraft:stone"))));
        ask(&mut h, |ctx| assert!(ctx.break_block([3, 65, 3])));
        (0..30).for_each(|_| drop(h.tick_all()));
        name(&mut h, [4, 65, 3])
    };
    let (cancelling, listening) = (Log::cancelling(), Log::default());
    assert_eq!((stream(&cancelling), stream(&listening)), ("minecraft:water".to_owned(), "minecraft:air".to_owned()));
    assert!(listening.heard("decay [4, 65, 3] minecraft:air"), "{:?}", listening.lines);
}

#[test]
fn a_removed_entity_and_a_closing_world_are_told() {
    let log = Log::default();
    let mut h = harness(&log);
    let pig = ask(&mut h, |ctx| ctx.spawn_entity("minecraft:pig", [2.5, 65.0, 2.5]).expect("a pig"));
    ask(&mut h, move |ctx| assert!(ctx.remove_entity(pig)));
    h.tick_all();
    assert!(log.heard("gone minecraft:pig"), "{:?}", log.lines);
    h.shard.close();
    assert!(log.heard("close "), "{:?}", log.lines);
}
