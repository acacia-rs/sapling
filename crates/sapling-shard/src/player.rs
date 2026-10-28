//! What a player entity is made of: the part that goes with the player to another shard, and
//! the part each shard makes anew. A component missing from both is missing after a transfer.

use acacia_proto::packets::ResourcePacksInfo;
use bevy_ecs::prelude::*;
use sapling_api::{Player, PlayerId};
use sapling_net::{ConnId, VerifiedLogin};

use crate::blobs::BlobCache;
use crate::dimensions::ClientDimension;
use crate::blocks::Breaking;
use crate::inventory::{Inventory, RecipeBook};
use crate::chunks::SectionBacklog;
use crate::commands::Access;
use crate::components::{Conn, Identity, InLayer, Mode, NetId, Outgoing, Position, Stage};
use crate::forms::Forms;
use crate::hud::Hud;
use crate::mobs::SeesMobs;
use crate::movement::Motion;
use crate::roster::Profile;
use crate::simulation::Simulated;
use crate::tags::Marks;
use crate::view::View;
use crate::vitals::Vitals;
use crate::visibility::Sees;

#[derive(Bundle)]
pub(crate) struct Carried {
    pub conn: Conn,
    pub identity: Identity,
    pub net_id: NetId,
    pub position: Position,
    pub stage: Stage,
    pub view: View,
    pub blobs: BlobCache,
    pub motion: Motion,
    pub profile: Profile,
    /// Unsent packets go along, so the old shard's last words precede the new shard's first.
    pub out: Outgoing,
    pub access: Access,
    pub mode: Mode,
    /// Open forms keep their callbacks, which then run on the shard the answer reaches.
    pub forms: Forms,
    /// What the client shows; it keeps showing it, so nothing is sent again.
    pub hud: Hud,
    pub marks: Marks,
    /// The player's items, under the stack ids its client knows them by.
    pub inventory: Inventory,
    pub recipes: RecipeBook,
    pub vitals: Vitals,
    pub dimension: ClientDimension,
}

impl Carried {
    /// A player who logged in, with `first`, the first packet of its join, queued.
    pub fn joining(conn: ConnId, login: &VerifiedLogin, net_id: u64, spawn: [f32; 3], mode: Mode, dimension: i32, first: &ResourcePacksInfo) -> Self {
        let identity = &login.identity;
        let player = Player::new(PlayerId(conn.get()), identity.display_name.clone(), identity.uuid, identity.xuid.clone());
        let mut out = Outgoing::default();
        out.push(first);
        Self {
            conn: Conn(conn),
            identity: Identity(player),
            net_id: NetId(net_id),
            position: Position { feet: spawn, yaw: 0.0, pitch: 0.0, head_yaw: 0.0, on_ground: true },
            stage: Stage::PacksInfo,
            view: View::default(),
            blobs: BlobCache::default(),
            motion: Motion::default(),
            profile: Profile::new(login, net_id),
            out,
            access: Access::default(),
            mode,
            forms: Forms::default(),
            hud: Hud::default(),
            marks: Marks::new(identity.uuid.as_bytes()),
            inventory: Inventory::default(),
            recipes: RecipeBook::default(),
            vitals: Vitals::default(),
            dimension: ClientDimension::new(dimension),
        }
    }
}

/// What a shard knows of a player only while it is there.
#[derive(Bundle)]
pub(crate) struct Local {
    in_layer: InLayer,
    backlog: SectionBacklog,
    sees: Sees,
    sees_mobs: SeesMobs,
    breaking: Breaking,
    /// Starts from the client's place on the shard's first input.
    simulated: Simulated,
}

impl Local {
    pub fn in_layer(layer: Entity) -> Self {
        let (backlog, sees, sees_mobs, breaking, simulated) = Default::default();
        Self { in_layer: InLayer(layer), backlog, sees, sees_mobs, breaking, simulated }
    }
}
