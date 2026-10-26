//! Inventory requests as a vanilla client sends them, and the server's answers.

use acacia_proto::RawPacket;
use acacia_proto::packets::{ItemStackRequest, ItemStackResponse};
use acacia_proto::types::{
    ContainerSlotType as Container, FullContainerName, ItemStackRequest as Request, ItemStackRequestActionsItem as ActionItem,
    ItemStackRequestActionsItemContent as Action, ItemStackRequestActionsItemContentConsume as Consume, ItemStackRequestActionsItemContentPlace as Place, ItemStackRequestActionsItemContentTake as Take,
    ItemStackRequestActionsItemTypeId as Kind, ItemStackRequestCause, ItemStackResponsesItemStatus as Status, StackRequestSlotInfo,
};

use super::all;

pub fn slot(container_id: Container, slot: u8, stack_id: i32) -> StackRequestSlotInfo {
    StackRequestSlotInfo { slot_type: FullContainerName { container_id, dynamic_container_id: None }, slot, stack_id }
}

/// One request as a vanilla client sends it: alone in its packet.
pub fn request(request_id: i32, actions: Vec<Action>) -> ItemStackRequest {
    let kind = |action: &Action| match action {
        Action::Take(_) => (Kind::Take, 0),
        Action::Place(_) => (Kind::Place, 1),
        Action::Swap(_) => (Kind::Swap, 2),
        Action::Drop(_) => (Kind::Drop, 3),
        Action::Destroy(_) => (Kind::Destroy, 4),
        Action::Consume(_) => (Kind::Consume, 5),
        Action::CraftRecipe(_) => (Kind::CraftRecipe, 12),
        Action::CraftCreative(_) => (Kind::CraftCreative, 12),
        Action::Optional(_) => (Kind::Optional, 15),
        Action::CraftGrindstoneRequest(_) => (Kind::CraftGrindstoneRequest, 16),
        other => panic!("{other:?}"),
    };
    let actions = actions.into_iter().map(|content| ActionItem { type_id: kind(&content).0, legacy_type_id: kind(&content).1, content }).collect();
    ItemStackRequest { requests: vec![Request { request_id, actions, custom_names: Vec::new(), cause: ItemStackRequestCause::Unknown(-1) }] }
}

/// As [`request`], renaming at an anvil to `name`.
pub fn renaming(request_id: i32, actions: Vec<Action>, name: &str) -> ItemStackRequest {
    let mut named = request(request_id, actions);
    (named.requests[0].custom_names, named.requests[0].cause) = (vec![name.to_owned()], ItemStackRequestCause::AnvilText);
    named
}

pub fn consume(count: u8, source: StackRequestSlotInfo) -> Action {
    Action::Consume(Consume { count, source })
}

pub fn take(count: u8, source: StackRequestSlotInfo, destination: StackRequestSlotInfo) -> Action {
    Action::Take(Take { count, source, destination })
}

pub fn place(count: u8, source: StackRequestSlotInfo, destination: StackRequestSlotInfo) -> Action {
    Action::Place(Place { count, source, destination })
}

/// The answer among `sent`: whether it was done, and the slots it names as (container, slot, count, stack id).
pub fn answer(sent: &[RawPacket]) -> (Status, Vec<(Container, u8, u8, i32)>) {
    let answers = all::<ItemStackResponse>(sent);
    let answer = &answers[0].responses[0];
    let slots = answer.containers.iter().flatten().flat_map(|container| container.slots.iter().map(move |slot| (container.slot_type.container_id, slot.slot, slot.count, slot.item_stack_id.unwrap_or(0))));
    (answer.status, slots.collect())
}
