//! A skin changed in game (the dressing room): the PlayerSkin a client sends is held to a login's
//! limits (`skin::checked`), offered to plugins, then shown to everyone on the shard and kept in
//! the player's list entry; refused, the player is shown its old skin again.
// TODO: players on other shards keep the old skin until the player's entry is sent anew.

use acacia_proto::packets::PlayerSkin;
use bevy_ecs::prelude::*;
use sapling_api::SkinChangeEvent;

use crate::components::{Outgoing, PacketQueues, in_world_identity};
use crate::roster::Profile;
use crate::{chat, plugins, skin, worlds};

pub(crate) fn dispatch(world: &mut World) {
    for (entity, sent) in world.resource_mut::<PacketQueues>().take::<PlayerSkin>() {
        let (Some(player), Some(uuid)) = (in_world_identity(world, entity), world.get::<Profile>(entity).map(|profile| profile.uuid)) else { continue };
        let Some(skin) = skin::checked(sent.skin) else { continue };
        worlds::focus_on(world, entity);
        let allowed = !plugins::offer(world, SkinChangeEvent::new(player, skin.skin_id.clone())).is_cancelled();
        let Some(mut profile) = world.get_mut::<Profile>(entity) else { continue };
        let packet = |skin| PlayerSkin { uuid, skin, skin_name: sent.skin_name.clone(), old_skin_name: sent.old_skin_name.clone() };
        if allowed {
            profile.reskin(skin.clone());
            chat::send_to_all(world, &sapling_net::encode(&packet(skin)));
        } else if let Some(old) = profile.skin() {
            world.get_mut::<Outgoing>(entity).expect("players have an outbox").push(&packet(old));
        }
    }
}
