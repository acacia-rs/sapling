//! NPC dialogues a player has open, as dragonfly's `session/handler_npc_request.go` runs them:
//! the speaker is marked an NPC (InteractiveTag, NpcSkinId), NpcDialogue opens the screen, and
//! NpcRequest brings each button press (ExecuteAction, the button's index) and the close
//! (ExecuteClosingCommands).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};

use acacia_proto::packets::{NpcDialogue, NpcDialogueActionType, NpcRequest, NpcRequestRequestType, SetEntityData};
use acacia_proto::types::{
    EntityProperties, MetadataDictionaryItem as Item, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value,
    MetadataDictionaryItemValueDefault as Plain,
};
use bevy_ecs::prelude::*;
use sapling_api::command::{Executed, Selected};
use sapling_api::{Dialogue, DialogueCallback, EntityId, Permission, Player, Sender, ShardCtx};

use crate::components::{Identity, Outgoing, PacketQueues, Position, in_world_identity};
use crate::handlers::Ctx;
use crate::plugins::CommandRegistry;

/// A player's eyes above its feet: dragonfly moves a player speaker's portrait by them.
const EYES: f32 = 1.62;

static SERIALS: AtomicU64 = AtomicU64::new(0);

#[derive(Component)]
pub(crate) struct Talking {
    npc: u64,
    /// Tells this dialogue from one opened after it to the same player.
    serial: u64,
    dialogue: Dialogue,
    // The mutex only makes the callback `Sync` for the ECS; it is never contended.
    on_press: Mutex<DialogueCallback>,
}

fn npc_marks(id: u64, eyes: f32) -> SetEntityData {
    let portrait = serde_json::json!({"portrait_offsets": {"translate": [0.0, eyes * -32.0, 0.0], "rotate": [0.0, -32.0, 0.0], "scale": [1.0, 1.0, 1.0]}});
    let metadata = vec![
        Item { key: Key::InteractiveTag, r#type: Type::Byte, legacy_type: 0, value: Value::Default(Plain::Byte(1)) },
        Item { key: Key::NpcSkinId, r#type: Type::String, legacy_type: 4, value: Value::Default(Plain::String(portrait.to_string())) },
    ];
    SetEntityData { runtime_entity_id: id, metadata, properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 }
}

fn screen(npc: u64, action_type: NpcDialogueActionType, dialogue: Option<&Dialogue>) -> NpcDialogue {
    let (body, title, actions) = dialogue.map_or_else(Default::default, |d| (d.body.clone(), d.title.clone(), d.actions_json()));
    NpcDialogue { entity_id: npc, action_type, dialogue: body, screen_name: "default".into(), npc_name: title, action_json: actions }
}

/// Opens `dialogue` on `player`'s screen spoken by `npc`; one open before is closed first.
pub(crate) fn open(world: &mut World, player: Entity, npc: Entity, dialogue: &Dialogue, on_press: DialogueCallback) -> bool {
    let Some(id) = crate::ui::net_id(world, npc) else { return false };
    close(world, player);
    let eyes = if world.get::<Identity>(npc).is_some() { EYES } else { 0.0 };
    let Some(mut out) = world.get_mut::<Outgoing>(player) else { return false };
    out.push(&npc_marks(id, eyes));
    out.push(&screen(id, NpcDialogueActionType::Open, Some(dialogue)));
    world.entity_mut(player).insert(Talking { npc: id, serial: SERIALS.fetch_add(1, Ordering::Relaxed), dialogue: dialogue.clone(), on_press: Mutex::new(on_press) });
    true
}

/// Closes what `player` has open on its screen and tells the callback.
pub(crate) fn close(world: &mut World, player: Entity) {
    let Some(talking) = take(world, player) else { return };
    if let Some(mut out) = world.get_mut::<Outgoing>(player) {
        out.push(&screen(talking.npc, NpcDialogueActionType::Close, None));
    }
    closed(world, player, talking);
}

/// The dialogue `player` has open, taken off it.
fn take(world: &mut World, player: Entity) -> Option<Talking> {
    world.get_entity_mut(player).ok().and_then(|mut player| player.take::<Talking>())
}

fn closed(world: &mut World, player: Entity, talking: Talking) {
    let Some(who) = in_world_identity(world, player) else { return };
    let mut on_press = talking.on_press.into_inner().unwrap_or_else(PoisonError::into_inner);
    on_press(&mut ShardCtx::new(&mut Ctx { world }), &who, None);
}

/// Hands this tick's presses and closes to the callbacks of the dialogues they answer.
pub(crate) fn dispatch(world: &mut World) {
    for (player, request) in world.resource_mut::<PacketQueues>().take::<NpcRequest>() {
        if world.get::<Talking>(player).is_none_or(|talking| talking.npc != request.runtime_entity_id) {
            continue;
        }
        crate::worlds::focus_on(world, player);
        match request.request_type {
            NpcRequestRequestType::ExecuteAction => press(world, player, request.action_type.to_raw() as usize),
            NpcRequestRequestType::ExecuteClosingCommands => {
                if let Some(talking) = take(world, player) {
                    closed(world, player, talking);
                }
            }
            _ => {}
        }
    }
}

/// Runs button `index`'s commands as its NPC with the presser as `@initiator`, then the callback.
fn press(world: &mut World, player: Entity, index: usize) {
    let Some(who) = in_world_identity(world, player) else { return };
    let Some((npc, commands)) = world.get::<Talking>(player).and_then(|talking| Some((talking.npc, talking.dialogue.buttons.get(index)?.commands.clone()))) else { return };
    let sender = speaker(world, npc, &who);
    let registry = world.resource::<CommandRegistry>().0.clone();
    for line in &commands {
        registry.run(&mut ShardCtx::new(&mut Ctx { world }), &sender, Permission::Operator, line);
    }
    crate::scoreboard::send(world);
    // The callback is lent out while it runs, so that it may open or close dialogues itself.
    let Some(mut talking) = world.get_mut::<Talking>(player) else { return };
    let serial = talking.serial;
    let mut on_press = std::mem::replace(talking.on_press.get_mut().unwrap_or_else(PoisonError::into_inner), Box::new(|_, _, _| {}));
    on_press(&mut ShardCtx::new(&mut Ctx { world }), &who, Some(index));
    match world.get_mut::<Talking>(player).filter(|talking| talking.serial == serial) {
        Some(mut talking) => *talking.on_press.get_mut().unwrap_or_else(PoisonError::into_inner) = on_press,
        None => on_press(&mut ShardCtx::new(&mut Ctx { world }), &who, None),
    }
}

/// The NPC `npc` as the sender of its buttons' commands, `presser` its `@initiator`.
fn speaker(world: &World, npc: u64, presser: &Player) -> Sender {
    let player = world.iter_entities().find(|entity| entity.get::<crate::components::NetId>().is_some_and(|id| id.0 == npc) && entity.contains::<Identity>());
    let (executor, feet) = match player {
        Some(player) => (Some(Selected::Player(player.get::<Identity>().expect("found by it").0.clone())), player.get::<Position>().map_or([0.0; 3], |p| p.feet)),
        None => match crate::entities::describe(world, EntityId(npc)) {
            Some(info) => (Some(Selected::Entity(info.clone())), info.feet),
            None => (None, [0.0; 3]),
        },
    };
    Sender::Executed(Box::new(Executed::new(Sender::Console, executor, feet, [0.0, 0.0], 0).with_initiator(presser.clone())))
}
