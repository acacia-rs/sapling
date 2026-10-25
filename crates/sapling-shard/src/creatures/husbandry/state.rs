//! What a mob's raising is now ([`Raised`]): what a world folder keeps of it (`sapling_world::Raising`)
//! and what lives only while it is loaded, its lead holder, partner, foe and next egg; and how it
//! shows, on top of how its kind or its stored self does.

use acacia_proto::types::MetadataFlags1 as Flags;
use bevy_ecs::prelude::*;
use sapling_world::{Actor, ActorState, Raising};

/// A mob's raising. Every creature has one; a name tag is anyone's.
#[derive(Component, Clone, Debug, Default)]
pub(crate) struct Raised {
    pub kept: Raising,
    /// Read from storage; riding/mounts.rs saddles a mob.
    pub saddled: bool,
    pub chested: bool,
    /// The entity holding its lead, once the kept `leasher` is found, and the tick it was looked for since.
    pub holder: Option<Entity>,
    pub looked_since: u64,
    /// The mob it courts and the ticks they spent near.
    pub partner: Option<(Entity, u32)>,
    /// What it fights for its owner, and the tick it may next bite.
    pub foe: Option<(Entity, u64)>,
    /// The tick it next lays its egg or scute in, once one was rolled.
    pub lays_at: Option<u64>,
    /// It is on its way to its owner.
    pub following: bool,
}

impl Raised {
    /// As a world folder kept it (`look`, `kept`); a baby BDS kept no age of grows the whole way.
    pub fn stored(look: &Actor, mut kept: Raising, grows: u32) -> Self {
        kept.baby = kept.baby.map(|left| if left == 0 { grows as i32 } else { left });
        let has = |state: ActorState| look.states.contains(&state);
        Self { kept, saddled: has(ActorState::Saddled), chested: has(ActorState::Chested), ..Self::default() }
    }

    /// A new mob of `kind`'s: a mooshroom has eaten no flower (its pack's `mark_variant` -1).
    pub fn newcomer(kind: &str) -> Self {
        let mark_variant = if kind == "minecraft:mooshroom" { super::products::UNFED } else { 0 };
        Self { kept: Raising { mark_variant, ..Raising::default() }, ..Self::default() }
    }

    pub fn is_baby(&self) -> bool {
        self.kept.baby.is_some()
    }

    pub fn is_tamed(&self) -> bool {
        self.kept.tamed
    }

    /// The flags its raising shows, beyond its kind's.
    pub fn flags(&self) -> Flags {
        let kept = &self.kept;
        let bits = [
            (self.is_baby(), Flags::BABY),
            (kept.tamed, Flags::TAMED),
            (kept.sitting, Flags::SITTING),
            (kept.sheared, Flags::SHEARED),
            (kept.in_love > 0, Flags::INLOVE),
            (kept.leasher.is_some(), Flags::LEASHED),
            (self.saddled, Flags::SADDLED),
            (self.chested, Flags::CHESTED),
        ];
        Flags(bits.iter().filter(|(on, _)| *on).fold(0, |flags, (_, flag)| flags | flag.0))
    }

    /// `look` as it is now raised: its age, wool, name, markings and stored flags.
    pub fn dress(&self, look: &Actor) -> Actor {
        let kept = &self.kept;
        let states = [(kept.tamed, ActorState::Tamed), (kept.sitting, ActorState::Sitting), (kept.sheared, ActorState::Sheared), (self.saddled, ActorState::Saddled), (self.chested, ActorState::Chested)];
        Actor {
            baby: self.is_baby(),
            color: kept.colour,
            name: kept.name.clone(),
            mark_variant: kept.mark_variant,
            states: states.iter().filter(|(on, _)| *on).map(|(_, state)| *state).collect(),
            ..look.clone()
        }
    }

    /// The flags [`Raised::dress`] cannot show through stored states: love and the lead.
    pub fn unstored_flags(&self) -> Flags {
        Flags(self.flags().0 & (Flags::INLOVE.0 | Flags::LEASHED.0))
    }
}
