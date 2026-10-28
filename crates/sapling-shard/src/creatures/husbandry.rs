//! Raising animals, as BDS's vanilla packs describe each kind (`sapling_data::husbandry`, docs/research/husbandry.md):
//! breeding and babies growing up (breeding.rs, ageing.rs), taming, sitting, following and
//! defending an owner (owners.rs, guarding.rs), leads and their knots (leads.rs, knots.rs), and
//! what animals give (products.rs). What a player's click on a mob does is clicks.rs; what a mob
//! keeps of all this is its [`Raised`], written back with its compound (`sapling_world::Raising`).

mod ageing;
mod breeding;
mod clicks;
mod guarding;
mod knots;
mod leads;
mod nesting;
mod owners;
mod products;
mod shown;
mod sniffing;
mod state;
mod tadpoles;

use bevy_ecs::prelude::*;
use sapling_world::Actor;

use crate::components::Tick;
use crate::entities::Stored;

pub(super) use ageing::be_born;
pub(super) use tadpoles::frog_variant;
pub(crate) use clicks::interact;
pub(crate) use guarding::{owner_hit, owner_hurt};
pub(crate) use knots::{KNOT, from_storage as knot_from_storage, tie_to_fence, undo as undo_knot};
pub(crate) use owners::{is_owner, tame_by_riding};
pub(crate) use products::shear_in;
pub(crate) use shown::links_packet;
pub(crate) use state::Raised;

/// Gives a new creature its raising: as the world folder kept it (`stored`, its look and
/// compound), or a newcomer's. A stored baby's box is a baby's.
pub(super) fn adopt(world: &mut World, entity: Entity, kind: &str, stored: Option<(&Actor, &[u8])>) {
    let grows = sapling_data::husbandry(kind).and_then(|raised| raised.grows).unwrap_or(ageing::GROWS);
    let mut raised = match stored {
        Some((look, compound)) => Raised::stored(look, sapling_world::raising(compound).unwrap_or_default(), grows),
        None => Raised::newcomer(kind),
    };
    if stored.is_none() {
        tadpoles::newcomer(&mut raised, kind, grows);
    }
    let baby = raised.is_baby();
    world.entity_mut(entity).insert(raised);
    if let Some((_, compound)) = stored {
        nesting::adopt(world, entity, kind, compound);
    }
    if baby {
        ageing::fit_body(world, entity, kind);
    }
}

/// Before the mob's move, what its raising has it do: sit, be pulled on its lead, court, follow
/// its owner, fight for it or follow food. False when none of these steers it.
pub(super) fn steer(world: &mut World, entity: Entity, layer: Entity, kind: &str) -> bool {
    if world.get::<Raised>(entity).is_none() {
        return false;
    }
    owners::stays(world, entity)
        || leads::pull(world, entity, layer)
        || nesting::steer(world, entity, layer, kind)
        || sniffing::steer(world, entity, layer, kind)
        || guarding::chase(world, entity, layer, kind)
        || breeding::court(world, entity, layer, kind)
        || owners::follow(world, entity, layer, kind)
        || breeding::tempted(world, entity, layer, kind)
}

/// A tick of every raised mob: babies grow, love and cooldowns wear off, partners breed, hens
/// lay, sniffers sniff and dig, leads hold or break and knots without a fence go.
pub(crate) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let raised: Vec<(Entity, String)> = world.query_filtered::<(Entity, &crate::entities::Kind), With<Raised>>().iter(world).map(|(entity, kind)| (entity, kind.0.clone())).collect();
    for (entity, kind) in raised {
        ageing::tick(world, entity, &kind);
        breeding::tick(world, entity, &kind, now);
        products::lay(world, entity, &kind, now);
        products::graze(world, entity, &kind);
        sniffing::tick(world, entity, &kind, now);
        nesting::tick(world, entity, &kind, now);
        leads::hold(world, entity, now);
    }
    knots::tick(world);
}

/// The compound the mob is kept as, with its raising now.
pub(crate) fn kept(compound: &[u8], raised: &Raised) -> Option<Vec<u8>> {
    sapling_world::raised_actor(compound, &raised.kept).ok()
}

/// The mob's kept `definitions` lose and gain the component groups `swap` names, as BDS's event would.
fn regroup(world: &mut World, entity: Entity, swap: &sapling_data::Swap) {
    let Some(mut stored) = world.get_mut::<Stored>(entity) else { return };
    match sapling_world::regrouped_actor(&stored.compound, &swap.removed, &swap.added) {
        Ok(compound) => stored.compound = compound,
        Err(error) => tracing::warn!(%error, "a mob's compound is unreadable: its component groups are not kept"),
    }
}

/// A sheared mooshroom becomes a cow where it stood, keeping its name and its health.
fn become_cow(world: &mut World, entity: Entity) {
    let Some((layer, _, _, body)) = shown::of(world, entity) else { return };
    let Some(cow) = super::spawn_entity(world, layer, "minecraft:cow", body.at, sapling_api::SpawnReason::Transformed) else { return };
    let name = world.get::<Raised>(entity).map(|raised| raised.kept.name.clone()).unwrap_or_default();
    if let Some(left) = world.get::<super::Health>(entity).map(super::Health::left) {
        if let Some(mut health) = world.get_mut::<super::Health>(cow) {
            health.end();
            health.mend(left);
        }
    }
    if !name.is_empty() {
        clicks::name(world, cow, name);
    }
    crate::mobs::remove(world, entity);
}

/// The mob stays: a name tag, a lead or an owner keeps it from despawning.
fn keep_for_good(world: &mut World, entity: Entity) {
    if let Some(mut raised) = world.get_mut::<Raised>(entity) {
        raised.kept.persistent = true;
    }
    world.entity_mut(entity).remove::<crate::spawning::Natural>();
}

/// Euclidean distance.
fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    (0..3).map(|axis| (a[axis] - b[axis]).powi(2)).sum::<f32>().sqrt()
}
