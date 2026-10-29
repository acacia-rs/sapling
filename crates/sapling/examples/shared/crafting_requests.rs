//! Hand-made ItemStackRequests for the crafting drills: items laid into a workstation's UI slots,
//! special recipes crafted in a crafting table's grid, anvil crafts with chosen name lists, and
//! results dropped at once so the watcher sees their whole tag in AddItemEntity. The bot's own
//! view of the slots is not updated by these; each drill step starts from `clear` and `replaceitem`.

use std::cell::RefCell;
use std::time::Duration;

use acacia_bot::items::SlotRef;
use acacia_bot::proto::packets::{ItemStackRequest as RequestPacket, ItemStackResponse};
use acacia_bot::proto::types::{
    ContainerSlotType as Kind, FullContainerName, ItemStackRequest, ItemStackRequestActionsItem as Action, ItemStackRequestActionsItemContent as Content,
    ItemStackRequestActionsItemContentConsume as Consume, ItemStackRequestActionsItemContentCraftRecipe as CraftRecipe,
    ItemStackRequestActionsItemContentCreate as Create, ItemStackRequestActionsItemContentDrop as Drop, ItemStackRequestActionsItemContentOptional as Optional,
    ItemStackRequestActionsItemContentPlace as Place, ItemStackRequestActionsItemContentResultsDeprecated as Results, ItemStackRequestActionsItemTypeId as TypeId, ItemStackRequestCause, StackRequestSlotInfo,
};
use acacia_bot::{Bot, BotEvent};

/// The multi recipes `!grid` names (docs/research/crafting-storage.md).
const RECIPES: [(&str, &str); 4] = [
    ("dup", "b5c5d105-75a2-4076-af2b-923ea2bf4bf0"),
    ("shield", "00000000-0000-0000-0000-0000000000c8"),
    ("book", "d1ca6b84-338e-4f2f-9c6b-76cc8b4bd98d"),
    ("map", "85939755-ba10-4d9d-a4cc-efb7a8e943c4"),
];
const TABLE_GRID: u8 = 32;
const CREATED: u8 = 50;
const HOTBAR: u8 = 9;

static NEXT: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(-3001);

thread_local! {
    /// Stack id and count of each UI slot as the last answer naming it gave them.
    static UI: RefCell<Vec<(u8, i32, u8)>> = const { RefCell::new(Vec::new()) };
    /// Hotbar slots these requests changed: stack id and count from the answer, and the id the
    /// bot believed then; a slot update since (the bot's id changed) makes the entry stale.
    static HOT: RefCell<Vec<(u8, i32, u8, i32)>> = const { RefCell::new(Vec::new()) };
}

fn believed(actor: &Bot, slot: u8) -> (i32, u8) {
    let stack = SlotRef::Main(slot).stack(actor.state()).filter(|stack| stack.count > 0);
    (stack.and_then(|stack| stack.stack_network_id).unwrap_or(0), stack.map_or(0, |stack| u8::try_from(stack.count).unwrap_or(u8::MAX)))
}

/// The request container of UI slot `slot` (acacia-bot's `items::ui` numbering).
fn ui_kind(slot: u8) -> Kind {
    match slot {
        1 => Kind::AnvilInput,
        2 => Kind::AnvilMaterial,
        9 => Kind::LoomInput,
        10 => Kind::LoomDye,
        14 => Kind::EnchantingInput,
        15 => Kind::EnchantingLapis,
        28..=40 => Kind::CraftingInput,
        _ => Kind::CreativeOutput,
    }
}

fn at(kind: Kind, slot: u8, stack_id: i32) -> StackRequestSlotInfo {
    StackRequestSlotInfo { slot_type: FullContainerName { container_id: kind, dynamic_container_id: None }, slot, stack_id }
}

fn hotbar(actor: &Bot, slot: u8) -> (StackRequestSlotInfo, u8) {
    let (bot_id, bot_count) = believed(actor, slot);
    let changed = HOT.with(|hot| hot.borrow().iter().find(|(at, .., then)| *at == slot && *then == bot_id).map(|&(_, id, count, _)| (id, count)));
    let (id, count) = changed.unwrap_or((bot_id, bot_count));
    (at(Kind::Hotbar, slot, id), count)
}

fn occupied(actor: &Bot, slot: u8) -> bool {
    hotbar(actor, slot).1 > 0
}

fn ui(slot: u8) -> (StackRequestSlotInfo, u8) {
    let (id, count) = UI.with(|ui| ui.borrow().iter().find(|(at, ..)| *at == slot).map_or((0, 0), |(_, id, count)| (*id, *count)));
    (at(ui_kind(slot), slot, id), count)
}

/// Legacy id = variant + 2 from 7 on, as acacia-bot's items/request.rs; BDS kicks a mismatch.
fn act_of(type_id: TypeId, content: Content) -> Action {
    let raw = type_id.to_raw() as u8;
    Action { legacy_type_id: if raw >= 7 { raw + 2 } else { raw }, type_id, content }
}

/// Sends one request, prints the actor's answer for a second and returns it: status Ok, and
/// `(container, slot, stack id, count)` per slot it names. UI slots' ids are remembered.
async fn request(actor: &mut Bot, actions: Vec<Action>, names: Vec<String>) -> (bool, Vec<(Kind, u8, i32, u8)>, i32) {
    let request_id = NEXT.fetch_sub(2, std::sync::atomic::Ordering::Relaxed);
    let cause = if names.is_empty() { ItemStackRequestCause::Unknown(-1) } else { ItemStackRequestCause::AnvilText };
    let packet = RequestPacket { requests: vec![ItemStackRequest { request_id, actions, custom_names: names, cause }] };
    println!("## request {request_id}: {packet:?}");
    actor.client().send(&packet);
    let (mut ok, mut slots) = (false, Vec::new());
    let end = tokio::time::Instant::now() + Duration::from_millis(1000);
    while let Ok(Some(BotEvent::Packet(packet))) = tokio::time::timeout_at(end, actor.next()).await {
        let Some(name) = acacia_bot::proto::packet_name(packet.id).filter(|name| super::SHOWN.contains(name)) else { continue };
        println!("## got {name} {}", crate::dump::dump(&packet).unwrap_or_default());
        let Ok(response) = packet.decode::<ItemStackResponse>() else { continue };
        for answer in response.responses.iter().filter(|answer| answer.request_id == request_id) {
            ok = format!("{:?}", answer.status) == "Ok";
            for container in answer.containers.iter().flatten() {
                slots.extend(container.slots.iter().map(|slot| (container.slot_type.container_id, slot.slot, slot.item_stack_id.unwrap_or(0), slot.count)));
            }
        }
    }
    for &(_, slot, id, count) in slots.iter().filter(|(kind, slot, ..)| *kind == Kind::Hotbar && *slot < HOTBAR) {
        let then = believed(actor, slot).0;
        HOT.with(|hot| {
            let mut hot = hot.borrow_mut();
            hot.retain(|(at, ..)| *at != slot);
            hot.push((slot, id, count, then));
        });
    }
    UI.with(|ui| {
        let mut ui = ui.borrow_mut();
        for &(kind, slot, id, count) in slots.iter().filter(|(kind, ..)| !matches!(kind, Kind::Hotbar | Kind::Inventory | Kind::HotbarAndInventory)) {
            let slot = if kind == Kind::CraftingInput && slot < 28 { slot + TABLE_GRID } else { slot };
            ui.retain(|(at, ..)| *at != slot);
            ui.push((slot, id, count));
        }
    });
    (ok, slots, request_id)
}

/// The hotbar slot a `!grid` cell word names: a number, or the first slot holding that item.
fn cell_source(actor: &Bot, word: &str, used: &[u8]) -> Option<u8> {
    if let Ok(slot) = word.parse() {
        return Some(slot);
    }
    let name = format!("minecraft:{word}");
    (0..HOTBAR).filter(|slot| !used.contains(slot)).find(|&slot| {
        SlotRef::Main(slot).stack(actor.state()).is_some_and(|stack| stack.count > 0 && actor.state().items.name(stack.network_id) == Some(name.as_str()))
    })
}

fn empty_hotbar(actor: &Bot) -> Vec<u8> {
    (0..HOTBAR).filter(|&slot| !occupied(actor, slot)).collect()
}

/// Drops each hotbar slot the answer `slots` filled, so its item shows whole.
async fn drop_results(actor: &mut Bot, slots: &[(Kind, u8, i32, u8)], dests: &[u8]) {
    let drops: Vec<Action> = slots
        .iter()
        .filter(|(kind, slot, _, count)| *kind == Kind::Hotbar && dests.contains(slot) && *count > 0)
        .map(|&(_, slot, id, count)| act_of(TypeId::Drop, Content::Drop(Drop { count, source: at(Kind::Hotbar, slot, id), randomly: false })))
        .collect();
    if !drops.is_empty() {
        request(actor, drops, Vec::new()).await;
    }
}

/// `!grid[!] recipe counts cells...`: the cells (`-` empty, a hotbar slot or an item name) laid
/// into the open table's grid, then the multi recipe crafted with one result per count (`2,1`),
/// each placed in an empty hotbar slot and dropped (kept with `!grid!`). `3/2/1` tries those
/// counts in turn while the server refuses.
async fn grid(actor: &mut Bot, words: &[&str], keep: bool) {
    let Some(uuid) = RECIPES.iter().find(|(key, _)| Some(key) == words.first()).map(|(_, uuid)| *uuid) else { return println!("## no recipe {:?}", words.first()) };
    let Some(network_id) = actor.state().recipes.book().multi_recipe(uuid) else { return println!("## no multi recipe {uuid}") };
    let tries: Vec<Vec<u8>> = words.get(1).map_or(vec![vec![1]], |counts| counts.split('/').map(|counts| counts.split(',').filter_map(|count| count.parse().ok()).collect()).collect());
    let (mut places, mut used) = (Vec::new(), Vec::new());
    for (cell, word) in words.iter().skip(2).enumerate().filter(|(_, word)| **word != "-") {
        let Some(slot) = cell_source(actor, word, &used) else { return println!("## no {word} in the hotbar") };
        used.push(slot);
        let source = hotbar(actor, slot).0;
        places.push(act_of(TypeId::Place, Content::Place(Place { count: 1, source, destination: at(Kind::CraftingInput, TABLE_GRID + cell as u8, 0) })));
    }
    let cells: Vec<u8> = words.iter().skip(2).enumerate().filter(|(_, word)| **word != "-").map(|(cell, _)| TABLE_GRID + cell as u8).collect();
    request(actor, places, Vec::new()).await;
    for counts in tries {
        // Vanilla follows the craft with its results, which BDS ignores at the grid.
        let results = Content::ResultsDeprecated(Results { result_items: Vec::new(), times_crafted: 1 });
        let mut actions = vec![act_of(TypeId::CraftRecipe, Content::CraftRecipe(CraftRecipe { recipe_network_id: network_id, times_crafted: 1 })), act_of(TypeId::ResultsDeprecated, results)];
        actions.extend(cells.iter().map(|&cell| act_of(TypeId::Consume, Content::Consume(Consume { count: 1, source: ui(cell).0 }))));
        let dests: Vec<u8> = empty_hotbar(actor).into_iter().take(counts.len()).collect();
        let id = NEXT.load(std::sync::atomic::Ordering::Relaxed);
        for (index, (&count, &dest)) in counts.iter().zip(&dests).enumerate() {
            if counts.len() > 1 {
                actions.push(act_of(TypeId::Create, Content::Create(Create { result_slot_id: index as u8 })));
            }
            let source = at(Kind::CreativeOutput, CREATED, id);
            actions.push(act_of(TypeId::Place, Content::Place(Place { count, source, destination: at(Kind::Hotbar, dest, 0) })));
        }
        let (ok, slots, _) = request(actor, actions, Vec::new()).await;
        println!("## crafted {counts:?}: {ok}");
        if ok {
            if !keep {
                drop_results(actor, &slots, &dests).await;
            }
            return;
        }
    }
}

/// `!anvilcraft[!] index names dest`: the anvil's input (and material, if laid) crafted with
/// `names` (`|`-separated, `~` a space, `''` empty, `_` for none) and `index` as the filtered
/// string index, the result placed in hotbar `dest` and dropped (kept with `!anvilcraft!`).
async fn anvil(actor: &mut Bot, words: &[&str], keep: bool) {
    let index = words.first().and_then(|word| word.parse().ok()).unwrap_or(0);
    let names: Vec<String> = words.get(1).filter(|names| **names != "_").map_or(Vec::new(), |names| names.split('|').map(|name| if name == "''" { String::new() } else { name.replace('~', " ") }).collect());
    let dest = words.get(2).and_then(|word| word.parse().ok()).unwrap_or(8);
    let mut actions = vec![act_of(TypeId::Optional, Content::Optional(Optional { recipe_network_id: 0, filtered_string_index: index }))];
    for slot in [2, 1] {
        let (source, count) = ui(slot);
        if count > 0 {
            actions.push(act_of(TypeId::Consume, Content::Consume(Consume { count, source })));
        }
    }
    let id = NEXT.load(std::sync::atomic::Ordering::Relaxed);
    actions.push(act_of(TypeId::Place, Content::Place(Place { count: 1, source: at(Kind::CreativeOutput, CREATED, id), destination: at(Kind::Hotbar, dest, 0) })));
    let (ok, slots, _) = request(actor, actions, names).await;
    println!("## anvil: {ok}");
    if ok && !keep {
        drop_results(actor, &slots, &[dest]).await;
    }
}

/// Does a crafting drill's `!` line: `!ui from slot [count]` lays hotbar `from` into UI `slot`,
/// `!uiback slot to` takes it back into hotbar `to`, `!grid` and `!anvilcraft` craft (above),
/// `!move`, `!ominous` and `!forget` (below), `!inv` prints the hotbar as the bot believes it.
/// False else.
pub async fn act(actor: &mut Bot, line: &str) -> bool {
    let words: Vec<&str> = line.split(' ').collect();
    let number = |at: usize, default: u8| words.get(at).and_then(|word| word.parse().ok()).unwrap_or(default);
    match words[0] {
        "!ui" => {
            let (source, count) = hotbar(actor, number(1, 0));
            let (slot, count) = (number(2, 14), number(3, count));
            request(actor, vec![act_of(TypeId::Place, Content::Place(Place { count, source, destination: at(ui_kind(slot), slot, 0) }))], Vec::new()).await;
        }
        "!uiback" => {
            let ((source, count), to) = (ui(number(1, 14)), number(2, 0));
            request(actor, vec![act_of(TypeId::Place, Content::Place(Place { count, source, destination: at(Kind::Hotbar, to, 0) }))], Vec::new()).await;
            UI.with(|ui| ui.borrow_mut().retain(|(at, ..)| *at != number(1, 14)));
        }
        "!grid" | "!grid!" => grid(actor, &words[1..], words[0].ends_with('!')).await,
        "!anvilcraft" | "!anvilcraft!" => anvil(actor, &words[1..], words[0].ends_with('!')).await,
        // `!move from to`: a hotbar stack moved whole by the bot, which keeps its view of both slots.
        "!move" => {
            let (from, to) = (number(1, 0), number(2, 8));
            println!("## moved: {:?}", actor.move_item(SlotRef::Main(from), SlotRef::Main(to), hotbar(actor, from).1).await);
        }
        // `!ominous`: the creative inventory's ominous banner (a banner with `Type`) taken.
        "!ominous" => {
            let entry = actor.state().creative.items().iter().find(|item| actor.state().items.name(item.stack.network_id) == Some("minecraft:banner") && format!("{:?}", item.stack.nbt).contains("Type")).map(|item| item.entry_id);
            match entry {
                Some(entry) => println!("## took: {:?}", actor.take_creative(entry, 1).await),
                None => println!("## no ominous banner in the creative inventory"),
            }
        }
        "!forget" => {
            HOT.with(|hot| hot.borrow_mut().clear());
            UI.with(|ui| ui.borrow_mut().clear());
        }
        "!inv" => {
            for slot in 0..HOTBAR {
                if let Some(stack) = SlotRef::Main(slot).stack(actor.state()).filter(|stack| stack.count > 0) {
                    let name = actor.state().items.name(stack.network_id).unwrap_or("?");
                    println!("## hotbar {slot}: {name} x{} aux {} id {:?} tag {:?}", stack.count, stack.metadata, stack.stack_network_id, stack.nbt);
                }
            }
        }
        _ => return false,
    }
    true
}
