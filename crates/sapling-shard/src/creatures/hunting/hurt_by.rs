//! A hunter hurt by a mob or a player takes it for its prey, as its pack's `behavior.hurt_by_target`
//! has it: a held skeleton, a creeper and a held pillager each took the snow golem whose snowball,
//! costing nothing, hit it, the tick after, with `mob.warning` a tick later; a player's iron golem took
//! no player that hit it (research/captures/bds-a3hurt.txt).

use bevy_ecs::prelude::*;

use super::{Hunter, take};
use crate::body::Body;
use crate::components::{InLayer, NetId, Tick};
use crate::entities::Kind;

/// Who hurt the hunter last, and the tick it did.
#[derive(Component)]
pub(super) struct HurtBy(Entity, u64);

/// The families the pack's `hurt_by_target` of `kind` does not take, or `None` for a kind without one.
fn spares(kind: &str) -> Option<&'static [&'static str]> {
    Some(match kind.strip_prefix("minecraft:")? {
        "zombie" | "husk" | "skeleton" | "stray" | "bogged" | "parched" | "spider" | "cave_spider" => &["breeze"],
        "breeze" => &["skeleton", "stray", "zombie", "husk", "spider", "cavespider", "slime"],
        "evocation_illager" | "pillager" | "vindicator" | "ravager" => &["illager"],
        "iron_golem" => &["creeper"],
        "shulker" => &["shulker"],
        "trader_llama" => &["trader_llama"],
        "bee" | "blaze" | "creeper" | "dolphin" | "drowned" | "enderman" | "ghast" | "hoglin" | "llama" | "nautilus" | "panda" | "piglin" | "piglin_brute" | "polar_bear" | "silverfish" | "turtle" | "vex"
        | "witch" | "wither" | "wither_skeleton" | "wolf" | "zoglin" | "zombie_pigman" | "zombie_villager" | "zombie_villager_v2" | "zombie_nautilus" => &[],
        _ => return None,
    })
}

/// The hunter `mob` was hurt by `by`, a mob or a player: it takes it at its next look, if its kind does.
pub(in crate::creatures) fn hurt(world: &mut World, mob: Entity, by: Entity) {
    if by == mob || world.get::<Hunter>(mob).is_none() {
        return;
    }
    let Some(kind) = world.get::<Kind>(mob).map(|kind| kind.0.clone()) else { return };
    let Some(spared) = spares(&kind) else { return };
    let player = world.get::<Kind>(by).is_none();
    let families: Vec<&str> = match world.get::<Kind>(by) {
        Some(other) => sapling_data::mob_families(&other.0).to_vec(),
        None => vec!["player"],
    };
    if families.iter().any(|family| spared.contains(family)) || (player && super::super::villagers::is_players_golem(world, mob)) {
        return;
    }
    let now = world.resource::<Tick>().0;
    world.entity_mut(mob).insert(HurtBy(by, now));
}

/// As the hunter looks for prey, a tick after it was hurt: it takes who hurt it, if that may still be hunted.
pub(super) fn avenge(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, body: &Body) {
    let now = world.resource::<Tick>().0;
    let Some(&HurtBy(by, at)) = world.get::<HurtBy>(entity) else { return };
    if now <= at {
        return;
    }
    world.entity_mut(entity).remove::<HurtBy>();
    let mob = world.get::<NetId>(by).is_some() && world.get::<Kind>(by).is_some();
    let same_layer = world.get::<InLayer>(by).is_some_and(|on| on.0 == layer);
    let huntable = if mob { super::super::is_alive(world, by) } else { crate::vitals::is_mortal(world, by) };
    let peaceful = world.resource::<crate::components::Settings>().0.difficulty == sapling_api::Difficulty::Peaceful;
    if peaceful || !same_layer || !huntable || world.get::<Hunter>(entity).is_some_and(|hunter| hunter.prey == Some(by)) {
        return;
    }
    take(world, entity, layer, id, kind, body, Some(by));
    if let Some(mut hunter) = world.get_mut::<Hunter>(entity) {
        hunter.angered = true;
    }
}
