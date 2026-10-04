//! Forms a player has open: sent with an id of its own, answered once, forgotten when it leaves.

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

use acacia_proto::packets::{ModalFormRequest, ModalFormResponse};
use bevy_ecs::prelude::*;
use sapling_api::{FormCallback, ShardCtx};

use crate::components::{Outgoing, PacketQueues, in_world_identity};
use crate::handlers::Ctx;

/// Forms one player may leave unanswered; further ones are not sent.
const MAX_PENDING: usize = 16;
/// Bytes of answer read; a custom form of a few inputs stays far below.
const MAX_ANSWER: usize = 32 * 1024;

#[derive(Component, Default)]
pub(crate) struct Forms {
    next_id: u32,
    // The mutex only makes the callback `Sync` for the ECS; it is never contended.
    pending: HashMap<u32, Mutex<FormCallback>>,
}

/// Opens a form on `entity`'s screen and keeps `on_answer` for its response.
pub(crate) fn send(world: &mut World, entity: Entity, json: String, on_answer: FormCallback) {
    let Some(mut forms) = world.get_mut::<Forms>(entity) else { return };
    if forms.pending.len() >= MAX_PENDING {
        tracing::debug!(?entity, "too many unanswered forms; not sending another");
        return;
    }
    let form_id = forms.next_id;
    forms.next_id = form_id.wrapping_add(1);
    forms.pending.insert(form_id, Mutex::new(on_answer));
    if let Some(mut out) = world.get_mut::<Outgoing>(entity) {
        out.push(&ModalFormRequest { form_id, data: json });
    }
}

/// Hands this tick's form answers to their callbacks. An id that is not open (never sent, or
/// answered before) is ignored.
pub(crate) fn dispatch(world: &mut World) {
    for (entity, response) in world.resource_mut::<PacketQueues>().take::<ModalFormResponse>() {
        let Some(player) = in_world_identity(world, entity) else { continue };
        let Some(callback) = world.get_mut::<Forms>(entity).and_then(|mut forms| forms.pending.remove(&response.form_id)) else {
            tracing::debug!(player = %player.name, id = response.form_id, "answer to a form that is not open");
            continue;
        };
        let callback = callback.into_inner().unwrap_or_else(PoisonError::into_inner);
        tracing::debug!(player = %player.name, id = response.form_id, data = ?response.data, "form answer");
        let data = response.data.filter(|data| data.len() <= MAX_ANSWER);
        let mut backend = Ctx { world };
        callback(&mut ShardCtx::new(&mut backend), &player, data.as_deref());
    }
}
