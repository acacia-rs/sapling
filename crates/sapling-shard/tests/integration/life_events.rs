//! A player's body and hands as plugin events, each cancelled: pickups, meals, experience, hunger,
//! healing, sleep, signs and container screens.

use crate::common;

use std::sync::{Arc, Mutex, mpsc};

use acacia_proto::RawPacket;
use acacia_proto::manual::Uuid;
use acacia_proto::nbt::{Nbt, Value};
use acacia_proto::packets::{BlockEntityData, CommandRequest, CompletedUsingItem, ContainerClose, ContainerOpen, SetEntityData, UpdateAttributes};
use acacia_proto::types::{BlockCoordinates, CommandOrigin, MetadataDictionaryItemKey as Key, TransactionTransactionData, TransactionUseItemActionType, WindowType};
use common::{Harness, all, click};
use sapling_api::{
    Block, Command, ContainerCloseEvent, ContainerOpenEvent, DamageCause, Effect, ExperienceGainEvent, FoodLossEvent, HealEvent, Item, ItemConsumeEvent, ItemPickupEvent, Output,
    Plugin, PluginBuilder, PlayerId, Priority, Sender, ShardCtx, SignEditEvent, SleepEvent, Slot,
};

const ALEX: PlayerId = PlayerId(1);

/// Notes every event, and cancels every one that can be.
#[derive(Clone, Default)]
struct Veto(Arc<Mutex<Vec<String>>>);

impl Veto {
    fn heard(&self, what: &str) -> bool {
        self.0.lock().unwrap().iter().any(|line| line.starts_with(what))
    }

    fn log(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
}

macro_rules! veto {
    ($plugin:expr, $log:expr, $($event:ty => $say:expr),*) => {$(
        let log = $log.clone();
        $plugin.on::<$event>(Priority::Normal, move |_, event| {
            log.lock().unwrap().push($say(&*event));
            event.cancel();
        });
    )*};
}

impl Plugin for Veto {
    fn build(&self, plugin: &mut PluginBuilder<'_>) {
        veto!(plugin, self.0,
            ItemPickupEvent => |e: &ItemPickupEvent| format!("pickup {}", e.item().name),
            ItemConsumeEvent => |e: &ItemConsumeEvent| format!("consume {:?}", e.item().map(|item| item.name.clone())),
            ExperienceGainEvent => |e: &ExperienceGainEvent| format!("experience {}", e.amount),
            FoodLossEvent => |e: &FoodLossEvent| format!("food {} -> {}", e.from(), e.to()),
            HealEvent => |e: &HealEvent| format!("heal {:?} {}", e.source(), e.amount),
            SleepEvent => |e: &SleepEvent| format!("sleep {:?}", e.bed()),
            SignEditEvent => |e: &SignEditEvent| format!("sign {} {:?} -> {:?}", e.front(), e.old_text(), e.new_text()),
            ContainerOpenEvent => |e: &ContainerOpenEvent| format!("open {}", e.block())
        );
        let log = self.0.clone();
        plugin.on::<ContainerCloseEvent>(Priority::Normal, move |_, event| log.lock().unwrap().push(format!("close {:?}", event.pos())));
    }
}

/// `/grant`: gives the sender 7 experience points.
#[derive(Command)]
#[command(crate = "sapling_api")]
struct Grant;

impl Command for Grant {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, _: &mut Output) {
        assert!(ctx.give_experience(sender.player().expect("a player").id, 7));
    }
}

fn harness(veto: Option<&Veto>) -> Harness {
    let mut h = Harness::new();
    if let Some(veto) = veto {
        h.shard.add_plugin(veto);
    }
    h.shard.add_command::<Grant>();
    h.enter(1, "Alex");
    h
}

fn ask<T: Send + 'static>(h: &mut Harness, ask: impl FnOnce(&mut ShardCtx<'_>) -> T + Send + 'static) -> T {
    let (tx, rx) = mpsc::channel();
    h.remote.run(move |ctx| tx.send(ask(ctx)).expect("asked"));
    h.tick_all();
    rx.recv().expect("answered")
}

fn ticks(h: &mut Harness, n: usize) -> Vec<RawPacket> {
    (0..n).flat_map(|_| h.tick(1)).collect()
}

fn attribute(sent: &[RawPacket], name: &str) -> Vec<f32> {
    all::<UpdateAttributes>(sent).iter().flat_map(|update| &update.attributes).filter(|attribute| attribute.name == name).map(|attribute| attribute.current).collect()
}

#[test]
fn a_cancelled_pickup_leaves_the_item_lying() {
    let veto = Veto::default();
    for veto in [None, Some(&veto)] {
        let mut h = harness(veto);
        ask(&mut h, |ctx| assert!(ctx.drop_item(&Item::new("minecraft:dirt", 3), [0.5, 65.0, 0.5])));
        ticks(&mut h, 60);
        let held = ask(&mut h, |ctx| ctx.item(ALEX, Slot::Main(0)).map(|item| item.count));
        assert_eq!(held, if veto.is_some() { None } else { Some(3) });
    }
    assert!(veto.heard("pickup minecraft:dirt"));
}

#[test]
fn cancelled_experience_is_not_gained() {
    let veto = Veto::default();
    let mut h = harness(Some(&veto));
    let origin = CommandOrigin { r#type: "player".into(), uuid: Uuid([1; 16]), request_id: "req".into(), player_entity_id: 0 };
    h.receive(1, &CommandRequest { command: "/grant".into(), origin, internal: false, version: "latest".into() });
    h.tick_all();
    assert_eq!(ask(&mut h, |ctx| ctx.experience(ALEX)), Some((0, 0.0)));
    assert_eq!(veto.log(), ["experience 7"]);
}

#[test]
fn cancelled_healing_and_food_loss_keep_health_and_hunger() {
    let veto = Veto::default();
    let mut h = harness(Some(&veto));
    ask(&mut h, |ctx| {
        assert!(ctx.hurt_player(ALEX, 14.0, DamageCause::Generic));
        assert!(ctx.give_effect(ALEX, Effect::Hunger, 400, 40, false));
    });
    let sent = ticks(&mut h, 200);
    assert!(attribute(&sent, "minecraft:health").iter().all(|health| *health == 6.0), "{:?}", attribute(&sent, "minecraft:health"));
    assert_eq!(attribute(&sent, "minecraft:player.saturation").last(), Some(&0.0), "the saturation goes");
    assert!(attribute(&sent, "minecraft:player.hunger").iter().all(|food| *food == 20.0), "the hunger stays");
    assert!(veto.heard("heal Natural 1") && veto.heard("food 20 -> 19"), "{:?}", veto.log());
}

#[test]
fn a_cancelled_meal_is_not_eaten() {
    let veto = Veto::default();
    let mut h = harness(Some(&veto));
    ask(&mut h, |ctx| {
        // Eaten whatever the hunger.
        assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new("minecraft:golden_apple", 2))));
    });
    let mut eat = click([0, 0, 0], 255, None);
    if let TransactionTransactionData::ItemUse(used) = &mut eat.transaction.transaction_data {
        used.action_type = TransactionUseItemActionType::ClickAir;
    }
    h.receive(1, &eat);
    let sent = ticks(&mut h, 40);
    assert!(all::<CompletedUsingItem>(&sent).is_empty());
    assert_eq!(ask(&mut h, |ctx| ctx.item(ALEX, Slot::Main(0)).map(|item| item.count)), Some(2));
    assert!(veto.heard("consume Some(\"minecraft:golden_apple\")"), "{:?}", veto.log());
}

#[test]
fn a_cancelled_sleep_keeps_the_player_up() {
    let veto = Veto::default();
    let mut h = harness(Some(&veto));
    ask(&mut h, |ctx| {
        assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new("minecraft:bed", 1))));
        ctx.set_time(13_000);
    });
    h.receive(1, &click([1, 64, 0], 1, None));
    h.tick(1);
    h.receive(1, &click([1, 65, 1], 1, None));
    let sent = h.tick(1);
    assert!(!all::<SetEntityData>(&sent).iter().any(|data| data.metadata.iter().any(|item| item.key == Key::PlayerBedPosition)));
    assert_eq!(veto.log(), ["sleep [1, 65, 1]"]);
}

#[test]
fn a_cancelled_sign_edit_keeps_the_text_and_shows_it_again() {
    let veto = Veto::default();
    let mut h = harness(Some(&veto));
    ask(&mut h, |ctx| assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new("minecraft:oak_sign", 1)))));
    h.receive(1, &click([1, 64, 0], 1, None));
    h.tick(1);
    let side = Value::Compound(vec![("Text".into(), Value::String("Hello".into()))]);
    let nbt = Nbt { name: String::new(), value: Value::Compound(vec![("FrontText".into(), side)]) };
    h.receive(1, &BlockEntityData { position: BlockCoordinates { x: 1, y: 65, z: 0 }, nbt: nbt.into() });
    let shown = all::<BlockEntityData>(&h.tick(1));
    let text = shown.iter().map(|data| data.nbt.decode().unwrap().value.get("FrontText").and_then(|side| side.get("Text")).cloned()).collect::<Vec<_>>();
    assert_eq!(text, [Some(Value::String(Default::default()))], "the editor is shown the sign as it was");
    assert_eq!(veto.log(), ["sign true \"\" -> \"Hello\""]);
}

#[test]
fn a_cancelled_container_opens_no_screen_and_a_close_is_heard() {
    let veto = Veto::default();
    let mut h = harness(Some(&veto));
    ask(&mut h, |ctx| assert!(ctx.set_block([1, 65, 0], &Block::new("chest"))));
    h.receive(1, &click([1, 65, 0], 1, None));
    assert!(all::<ContainerOpen>(&h.tick(1)).is_empty());
    assert_eq!(veto.log(), ["open minecraft:chest"]);

    let closes = Closes::default();
    let mut h = harness(None);
    h.shard.add_plugin(&closes);
    ask(&mut h, |ctx| assert!(ctx.set_block([1, 65, 0], &Block::new("chest"))));
    h.receive(1, &click([1, 65, 0], 1, None));
    let window = all::<ContainerOpen>(&h.tick(1))[0].window_id;
    h.receive(1, &ContainerClose { window_id: window, window_type: WindowType::Container, server: false });
    h.tick(1);
    assert_eq!(*closes.0.lock().unwrap(), ["close [1, 65, 0]"]);
}

#[derive(Clone, Default)]
struct Closes(Arc<Mutex<Vec<String>>>);

impl Plugin for Closes {
    fn build(&self, plugin: &mut PluginBuilder<'_>) {
        let log = self.0.clone();
        plugin.on::<ContainerCloseEvent>(Priority::Normal, move |_, event| log.lock().unwrap().push(format!("close {:?}", event.pos())));
    }
}
