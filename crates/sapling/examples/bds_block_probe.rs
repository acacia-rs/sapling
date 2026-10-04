//! Shows what a vanilla server sends when blocks are broken, placed and clicked: an actor bot does
//! it in creative, survival and adventure, also what a client should not be able to do (a break
//! finished at once, a click far away), while a watcher bot stands by. Both print every block,
//! sound, inventory and correction packet they get. The ground truth for sapling's step 6
//! (docs/DESIGN.md "Step 6 as built").
//! `cargo run -p sapling --example bds_block_probe -- <server> > probe.txt` (offline mode, operators by default)
use std::future::Future;
use std::time::{Duration, Instant};

use acacia_bot::client::{Client, PacketFilter};
use acacia_bot::interact::{Face, to_wire};
use acacia_bot::proto::packets::{self, InventoryTransaction, PlayerAuthInput, PlayerAuthInputBlockActionItem};
use acacia_bot::proto::types::{
    Action, BlockCoordinates, InputData, Transaction, TransactionLegacy, TransactionTransactionData, TransactionTransactionType, TransactionUseItem,
    TransactionUseItemActionType, TransactionUseItemClientCooldownState, TransactionUseItemClientPrediction, TransactionUseItemHand, TransactionUseItemTriggerType,
    Vec2f, Vec3f, Vec3i,
};
use acacia_bot::proto::{Packet, RawPacket};
use acacia_bot::{Bot, BotConfig, BotEvent};
use acacia_world::BlockAccess;

/// Characters of a decoded packet shown.
const SHOWN: usize = 420;
const SETTLE: Duration = Duration::from_millis(1200);

macro_rules! shown {
    ($($t:ident),* $(,)?) => {
        /// The packets worth printing, decoded; `None` for the rest.
        fn decoded(p: &RawPacket) -> Option<String> {
            $(if p.id == <packets::$t>::ID {
                return Some(match p.decode::<packets::$t>() {
                    Ok(v) => format!("{} {v:?}", stringify!($t)),
                    Err(e) => format!("{} <decode error: {e}>", stringify!($t)),
                });
            })*
            None
        }
    };
}
shown!(
    UpdateBlock, UpdateBlockSynced, UpdateSubchunkBlocks, LevelEvent, LevelEventGeneric, LevelSoundEvent, InventoryTransaction, InventorySlot, InventoryContent,
    ItemStackResponse, PlayerAction, Animate, BlockEvent, BlockEntityData, MobEquipment, UpdatePlayerGameType, AddItemEntity, TakeItemEntity, PlayerHotbar,
    ContainerOpen, Disconnect,
);

struct Probe {
    start: Instant,
    actor: Bot,
    watcher: Bot,
}

fn show(start: Instant, who: &str, event: Option<BotEvent>) -> Result<(), String> {
    match event {
        Some(BotEvent::Packet(packet)) => {
            if let Some(text) = decoded(&packet) {
                let text: String = text.chars().take(SHOWN).collect();
                println!("{:>7} {who:<7} {text}", start.elapsed().as_millis());
            }
            Ok(())
        }
        Some(BotEvent::Disconnected(reason)) => Err(format!("{who} disconnected: {reason:?}")),
        Some(_) => Ok(()),
        None => Err(format!("{who} is gone")),
    }
}

/// Runs an actor's action while the watcher keeps printing.
async fn during<T>(start: Instant, watcher: &mut Bot, action: impl Future<Output = T>) -> Result<T, String> {
    tokio::pin!(action);
    loop {
        tokio::select! {
            result = &mut action => return Ok(result),
            event = watcher.next() => show(start, "watcher", event)?,
        }
    }
}

impl Probe {
    async fn settle(&mut self, wait: Duration) -> Result<(), String> {
        let end = tokio::time::Instant::now() + wait;
        loop {
            tokio::select! {
                event = self.actor.next() => show(self.start, "actor", event)?,
                event = self.watcher.next() => show(self.start, "watcher", event)?,
                _ = tokio::time::sleep_until(end) => return Ok(()),
            }
        }
    }

    async fn command(&mut self, line: &str) -> Result<(), String> {
        println!("## /{line}");
        self.actor.client().command(line);
        self.settle(SETTLE).await
    }

    fn name(&self, [x, y, z]: [i32; 3]) -> String {
        let world = self.actor.world().expect("a world tracker");
        let (Some(view), Some(registry)) = (world.view(), world.registry()) else { return "?".into() };
        registry.get(view.block(x, y, z)).map_or("?".into(), |state| state.name.to_string())
    }

    fn wire_id(&self, [x, y, z]: [i32; 3]) -> u32 {
        let view = self.actor.world().and_then(|w| w.view()).expect("a chunk view");
        view.world().wire_id(view.block(x, y, z))
    }

    async fn break_block(&mut self, label: &str, pos: [i32; 3]) -> Result<(), String> {
        println!("## {label}: break {pos:?} ({})", self.name(pos));
        let began = Instant::now();
        let result = during(self.start, &mut self.watcher, self.actor.break_block(pos)).await?;
        self.settle(SETTLE).await?;
        println!("## -> {result:?} after {} ms, now {}", began.elapsed().as_millis(), self.name(pos));
        Ok(())
    }

    async fn place_block(&mut self, label: &str, against: [i32; 3], face: Face) -> Result<(), String> {
        let target = face.adjacent(against);
        println!("## {label}: place against {against:?} ({}) face {face:?}, holding {:?}", self.name(against), self.actor.state().inventory.held());
        let result = during(self.start, &mut self.watcher, self.actor.place_block(against, face)).await?;
        self.settle(SETTLE).await?;
        println!("## -> {result:?}, {target:?} is now {}", self.name(target));
        Ok(())
    }

    async fn click(&mut self, label: &str, pos: [i32; 3], face: Face) -> Result<(), String> {
        println!("## {label}: click {pos:?} ({}) face {face:?}", self.name(pos));
        let result = during(self.start, &mut self.watcher, self.actor.use_item_on_block(pos, face)).await?;
        self.settle(SETTLE).await?;
        println!("## -> {result:?}, now {}", self.name(pos));
        Ok(())
    }

    /// A ClickBlock the bot's own reach check would refuse: sent as it is, from where the bot stands.
    async fn far_click(&mut self, label: &str, pos: [i32; 3], face: Face) -> Result<(), String> {
        let (eye, target) = (self.actor.eye_position(), face.adjacent(pos));
        let distance = (0..3).map(|i| (pos[i] as f32 + 0.5 - eye[i]).powi(2)).sum::<f32>().sqrt();
        println!("## {label}: click {pos:?} ({}) {distance:.1} blocks from the eye", self.name(pos));
        let inventory = &self.actor.state().inventory;
        let [cx, cy, cz] = face.click_offset();
        let data = TransactionUseItem {
            action_type: TransactionUseItemActionType::ClickBlock,
            trigger_type: TransactionUseItemTriggerType::PlayerInput,
            block_position: BlockCoordinates { x: pos[0], y: pos[1], z: pos[2] },
            face: face as u8,
            hotbar_slot: i32::from(inventory.selected_hotbar_slot),
            hand: TransactionUseItemHand::MainHand,
            held_item: to_wire(inventory.held()),
            player_pos: Vec3f { x: eye[0], y: eye[1], z: eye[2] },
            click_pos: Vec3f { x: cx, y: cy, z: cz },
            block_runtime_id: self.wire_id(pos),
            client_prediction: TransactionUseItemClientPrediction::Success,
            client_cooldown_state: TransactionUseItemClientCooldownState::Off,
        };
        let transaction = Transaction {
            legacy: TransactionLegacy { legacy_request_id: 0, legacy_transactions: None },
            transaction_type: TransactionTransactionType::ItemUse,
            actions: Vec::new(),
            transaction_data: TransactionTransactionData::ItemUse(data),
        };
        let aim = [pos[0] as f32 + 0.5, pos[1] as f32 + 1.0, pos[2] as f32 + 0.5];
        during(self.start, &mut self.watcher, self.actor.look_at(aim)).await?.map_err(|e| e.to_string())?;
        self.actor.client().send(&InventoryTransaction { transaction });
        self.settle(SETTLE).await?;
        println!("## -> {target:?} is now {}", self.name(target));
        Ok(())
    }

    /// Block actions in an input of the probe's own, as the bot queues only whole breaks.
    async fn raw_actions(&mut self, label: &str, pos: [i32; 3], actions: &[Action]) -> Result<(), String> {
        println!("## {label}: {actions:?} at {pos:?} ({})", self.name(pos));
        let eye = self.actor.eye_position();
        let zero = || Vec2f { x: 0.0, z: 0.0 };
        let item = |action: &Action| PlayerAuthInputBlockActionItem { action: *action, position: Vec3i { x: pos[0], y: pos[1], z: pos[2] }, face: 1 };
        let input = PlayerAuthInput {
            pitch: 0.0,
            yaw: 0.0,
            position: Vec3f { x: eye[0], y: eye[1], z: eye[2] },
            move_vector: zero(),
            head_yaw: 0.0,
            input_data: vec![InputData::BlockAction],
            input_mode: packets::PlayerAuthInputInputMode::Mouse,
            play_mode: packets::PlayerAuthInputPlayMode::Normal,
            interaction_model: packets::PlayerAuthInputInteractionModel::Crosshair,
            interact_rotation: zero(),
            tick: 0,
            delta: Vec3f { x: 0.0, y: 0.0, z: 0.0 },
            transaction: None,
            item_stack_request: None,
            block_action: Some(actions.iter().map(item).collect()),
            vehicle_rotation: None,
            predicted_vehicle: None,
            analogue_move_vector: zero(),
            camera_orientation: Vec3f { x: 0.0, y: 0.0, z: 0.0 },
            raw_move_vector: zero(),
        };
        self.actor.client().send(&input);
        self.settle(SETTLE).await?;
        println!("## -> now {}", self.name(pos));
        Ok(())
    }
}

async fn connect(server: &str, name: &str) -> Result<Bot, Box<dyn std::error::Error>> {
    let config = BotConfig { physics: true, subscribe: PacketFilter::all(), ..BotConfig::default() };
    Ok(Bot::connect(Client::builder(server).offline(name), config).await?)
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = std::env::args().nth(1).ok_or("usage: bds_block_probe <server>")?;
    let (mut actor, mut watcher) = (connect(&server, "BlockActor").await?, connect(&server, "BlockWatcher").await?);
    watcher.wait_ticks(60).await?;
    actor.wait_ticks(60).await?;
    for line in ["gamemode creative", "clear @s", "fill ~-4 ~-1 ~-14 ~4 ~-1 ~14 dirt", "fill ~-4 ~ ~-14 ~4 ~3 ~14 air", "give @s stone 16"] {
        actor.client().command(line);
        actor.wait_ticks(20).await?;
    }
    // Setup noise is not what the probe is about.
    for bot in [&mut actor, &mut watcher] {
        while let Ok(Some(_)) = tokio::time::timeout(Duration::from_millis(5), bot.next()).await {}
    }
    let feet = actor.movement().and_then(|m| m.position()).ok_or("the actor has no position")?.map(|c| c.floor() as i32);
    let at = |dx: i32, dy: i32, dz: i32| [feet[0] + dx, feet[1] + dy, feet[2] + dz];
    println!("## actor feet {feet:?}, watcher {:?}", watcher.movement().and_then(|m| m.position()));
    let mut p = Probe { start: Instant::now(), actor, watcher };

    p.break_block("creative", at(1, -1, 0)).await?;
    p.raw_actions("creative, start only", at(1, -1, 1), &[Action::StartBreak]).await?;
    p.raw_actions("creative, start and predict", at(2, -1, 1), &[Action::StartBreak, Action::PredictBreak]).await?;
    p.place_block("creative", at(2, -1, 0), Face::Up).await?;
    p.break_block("creative, the placed stone", at(2, 0, 0)).await?;
    for distance in [9, 10, 11, 12, 13] {
        p.far_click("creative, far", at(0, -1, distance), Face::Up).await?;
    }
    let trapdoor = at(-1, 0, 1);
    p.command(&format!("setblock {} {} {} trapdoor", trapdoor[0], trapdoor[1], trapdoor[2])).await?;
    p.click("creative, a trapdoor", trapdoor, Face::Up).await?;

    p.command("gamemode survival").await?;
    p.break_block("survival", at(-1, -1, 0)).await?;
    p.place_block("survival", at(-2, -1, 0), Face::Up).await?;
    p.raw_actions("survival, finished at once", at(-1, -1, -1), &[Action::StartBreak, Action::PredictBreak]).await?;
    for distance in [5, 6, 7] {
        p.far_click("survival, far", at(0, -1, -distance), Face::Up).await?;
    }
    p.click("survival, a trapdoor", trapdoor, Face::Up).await?;

    p.command("gamemode adventure").await?;
    p.break_block("adventure", at(1, -1, 2)).await?;
    p.place_block("adventure", at(-2, -1, 1), Face::Up).await?;
    p.click("adventure, a trapdoor", trapdoor, Face::Up).await?;
    // Last: BDS closes the connection on an action that is not about breaking.
    p.command("gamemode creative").await?;
    p.raw_actions("creative, stop only", at(1, -1, -1), &[Action::StopBreak]).await?;
    p.raw_actions("creative, abort only", at(1, -1, -1), &[Action::AbortBreak]).await?;
    p.raw_actions("creative, creative destroy only", at(1, -1, -1), &[Action::CreativePlayerDestroyBlock]).await?;
    p.command("gamemode survival").await?;
    p.actor.disconnect().await;
    p.watcher.disconnect().await;
    Ok(())
}
