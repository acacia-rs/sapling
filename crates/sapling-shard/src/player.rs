//! What a player entity is made of: the part that goes with the player to another shard, and
//! the part each shard makes anew. A component missing from both is missing after a transfer.

use bevy_ecs::prelude::*;

use crate::blobs::BlobCache;
use crate::chunks::SectionBacklog;
use crate::commands::Access;
use crate::components::{Conn, Identity, InLayer, Mode, NetId, Outgoing, Position, Stage};
use crate::forms::Forms;
use crate::hud::Hud;
use crate::mobs::SeesMobs;
use crate::movement::Motion;
use crate::roster::Profile;
use crate::view::View;
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
}

/// What a shard knows of a player only while it is there.
#[derive(Bundle)]
pub(crate) struct Local {
    in_layer: InLayer,
    backlog: SectionBacklog,
    sees: Sees,
    sees_mobs: SeesMobs,
}

impl Local {
    pub fn in_layer(layer: Entity) -> Self {
        Self { in_layer: InLayer(layer), backlog: SectionBacklog::default(), sees: Sees::default(), sees_mobs: SeesMobs::default() }
    }
}
