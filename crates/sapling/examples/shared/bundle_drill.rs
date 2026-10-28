//! The bundle drill of bds_command_probe: items put into a bundle and taken out by hand-made
//! ItemStackRequests naming the bundle's dynamic container, with what the server answers.

use std::time::Duration;

use acacia_bot::proto::packets::ItemStackRequest as RequestPacket;
use acacia_bot::proto::types::{
    ContainerSlotType, FullContainerName, ItemStackRequest, ItemStackRequestActionsItem as Action, ItemStackRequestActionsItemContent as Content,
    ItemStackRequestActionsItemContentPlace as Place, ItemStackRequestActionsItemContentTake as Take, ItemStackRequestActionsItemTypeId as TypeId,
    ItemStackRequestCause, StackRequestSlotInfo,
};
use acacia_bot::proto::nbt::Value;
use acacia_bot::items::SlotRef;
use acacia_bot::{Bot, BotEvent};

pub const BUNDLE_SHOWN: &[&str] = &["InventorySlot", "InventoryContent", "PlayerHotbar", "ItemStackResponse", "LevelSoundEvent", "MobEquipment", "ContainerOpen", "ContainerClose"];

/// The next request id: the client counts down by twos from -1.
static NEXT: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(-1001);

fn hotbar(actor: &Bot, slot: u8) -> StackRequestSlotInfo {
    let stack_id = SlotRef::Main(slot).stack(actor.state()).and_then(|stack| stack.stack_network_id).unwrap_or(0);
    StackRequestSlotInfo { slot_type: FullContainerName { container_id: ContainerSlotType::Hotbar, dynamic_container_id: None }, slot, stack_id }
}

/// The int fields of the item in hotbar `slot`'s tag.
fn tag_ints(actor: &Bot, slot: u8) -> Vec<(String, i64)> {
    let Some(Value::Compound(fields)) = SlotRef::Main(slot).stack(actor.state()).and_then(|stack| stack.nbt.as_ref()).map(|nbt| &nbt.value) else { return Vec::new() };
    fields.iter().filter_map(|(key, value)| match value {
        Value::Int(v) => Some((key.to_string(), i64::from(*v))),
        Value::Long(v) => Some((key.to_string(), *v)),
        _ => None,
    }).collect()
}

/// The bundle `id`'s slot.
fn dynamic(id: u32, slot: u8, stack_id: i32) -> StackRequestSlotInfo {
    StackRequestSlotInfo { slot_type: FullContainerName { container_id: ContainerSlotType::Dynamic, dynamic_container_id: Some(id) }, slot, stack_id }
}

/// Sends one request and prints what comes back for a second; the stack ids the answer gives
/// the dynamic container's slots.
async fn request(actor: &mut Bot, type_id: TypeId, content: Content) -> Vec<(u8, i32)> {
    let request_id = NEXT.fetch_sub(2, std::sync::atomic::Ordering::Relaxed);
    let legacy = type_id.to_raw() as u8;
    let packet = RequestPacket {
        requests: vec![ItemStackRequest { request_id, actions: vec![Action { type_id, legacy_type_id: legacy, content }], custom_names: Vec::new(), cause: ItemStackRequestCause::Unknown(-1) }],
    };
    println!("## request {request_id}: {packet:?}");
    actor.client().send(&packet);
    let mut slots = Vec::new();
    let end = tokio::time::Instant::now() + Duration::from_millis(1000);
    while let Ok(Some(BotEvent::Packet(packet))) = tokio::time::timeout_at(end, actor.next()).await {
        let Some(name) = acacia_bot::proto::packet_name(packet.id).filter(|name| BUNDLE_SHOWN.contains(name)) else { continue };
        println!("## got {name} {}", super::dump::dump(&packet).unwrap_or_default());
        if let Ok(response) = packet.decode::<acacia_bot::proto::packets::ItemStackResponse>() {
            for container in response.responses.iter().filter_map(|answer| answer.containers.as_ref()).flatten() {
                if container.slot_type.container_id == ContainerSlotType::Dynamic {
                    slots.extend(container.slots.iter().map(|slot| (slot.slot, slot.item_stack_id.unwrap_or(0))));
                }
            }
        }
    }
    slots
}

/// Does a bundle drill's `!` line: `!bundletag` prints the bundle in hotbar 0's tag, `!bundleput
/// from count [slot]` puts `count` from hotbar `from` into it, `!bundletake count to [slot]` takes
/// `count` from its slot (default 0) into hotbar `to`. False else.
pub async fn act(actor: &mut Bot, line: &str) -> bool {
    let words: Vec<&str> = line.split(' ').collect();
    let number = |at: usize, default: u8| words.get(at).and_then(|word| word.parse().ok()).unwrap_or(default);
    let id = || tag_ints(actor, 0).iter().find(|(key, _)| key == "bundle_id").map_or(0, |(_, id)| *id as u32);
    let last = |slot: u8| LAST.with(|last| last.borrow().iter().find(|(at, _)| *at == slot).map_or(0, |(_, id)| *id));
    match words[0] {
        "!bundletag" => {
            for slot in [0, 4] {
                println!("## bundle {slot}: {:?}", SlotRef::Main(slot).stack(actor.state()));
            }
        }
        "!bundleput" => {
            let (source, count, slot) = (hotbar(actor, number(1, 1)), number(2, 1), number(3, 0));
            let destination = dynamic(id(), slot, last(slot));
            remember(request(actor, TypeId::Place, Content::Place(Place { count, source, destination })).await);
        }
        "!bundletake" => {
            let (count, to, slot) = (number(1, 1), number(2, 1), number(3, 0));
            let (source, destination) = (dynamic(id(), slot, last(slot)), hotbar(actor, to));
            remember(request(actor, TypeId::Take, Content::Take(Take { count, source, destination })).await);
        }
        _ => return false,
    }
    true
}

thread_local! {
    /// The bundle's slots' stack ids as the last answer gave them.
    static LAST: std::cell::RefCell<Vec<(u8, i32)>> = const { std::cell::RefCell::new(Vec::new()) };
}

fn remember(slots: Vec<(u8, i32)>) {
    println!("## bundle slots: {slots:?}");
    LAST.with(|last| {
        let mut last = last.borrow_mut();
        for (slot, id) in slots {
            last.retain(|(at, _)| *at != slot);
            last.push((slot, id));
        }
    });
}

fn owned<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

/// Two bundles given; cobblestone and pearls put in the first, more cobblestone than fits, a
/// sword and the second bundle tried; items taken out of both slots.
pub fn bundles() -> Vec<String> {
    let mut lines = owned(["kill @e[type=!player]", "gamerule domobspawning false", "gamemode survival @s", "clear @s", "tp @a[name=CmdWatcher] @s", ""]);
    lines.extend(owned(["replaceitem entity @s slot.hotbar 0 bundle 1", "replaceitem entity @s slot.hotbar 1 cobblestone 64", "replaceitem entity @s slot.hotbar 2 ender_pearl 16"]));
    lines.extend(owned(["replaceitem entity @s slot.hotbar 3 diamond_sword 1", "replaceitem entity @s slot.hotbar 4 bundle 1", "", "!bundletag", "!bundleput 1 10 0", ""]));
    lines.extend(owned(["!bundleput 2 4 1", "", "!bundleput 1 30 0", "", "!bundleput 1 10 0", "", "!bundleput 1 1 2", "", "!bundleput 3 1 2", "", "!bundleput 4 1 2", ""]));
    lines.extend(owned(["!bundletag", "!bundletake 5 5 0", "", "!bundletake 2 6 1", "", "!bundletake 1 7 2", "", "!bundletag", "", "gamemode creative @s", "clear @s", ""]));
    lines
}

/// Two bundles given, cobblestone put in the first and taken from its empty slot 5; the items are
/// left for `bundlekeep2`, run on the same world (KEEP_WORLD=1) after a restart.
fn bundle_keep() -> Vec<String> {
    let mut lines = owned(["kill @e[type=!player]", "gamerule domobspawning false", "gamemode survival @s", "clear @s", ""]);
    lines.extend(owned(["replaceitem entity @s slot.hotbar 0 bundle 1", "replaceitem entity @s slot.hotbar 1 cobblestone 64", "replaceitem entity @s slot.hotbar 4 bundle 1", "", "", "!bundletag"]));
    lines.extend(owned(["!bundleput 1 10 0", "", "!bundletake 1 6 5", "", "!bundletag", ""]));
    lines
}

/// After `bundlekeep`'s restart: what the player kept, and the number a new bundle gets.
fn bundle_kept() -> Vec<String> {
    owned(["", "", "!bundletag", "gamemode survival @s", "replaceitem entity @s slot.hotbar 4 bundle 1", "", "", "!bundletag", "!bundletake 1 6 0", "", ""])
}

/// The lines and shown packets of the bundle drill `name`, if it is one.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    let lines = match name {
        "bundles" => bundles(),
        "bundlekeep" => bundle_keep(),
        "bundlekeep2" => bundle_kept(),
        _ => return None,
    };
    Some((lines, BUNDLE_SHOWN))
}
