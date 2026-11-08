//! How a mob comes into the world: spawned (through the API, an egg, a spawner, breeding) or made
//! from what a world folder kept of it, with its legs, health, raising and trade.

use bevy_ecs::prelude::*;
use sapling_api::{EntityId, EntitySpawnEvent, SpawnReason};
use sapling_world::Actor;

use super::shown::{announcement, reshow, spawn_sound};
use super::{Creature, Health, NO_DEATH, afflictions, bees, breezes, creaking, dolphins, drinks, frogs, genes, ghasts, harm, hunting, husbandry, legs, looks, rams, riders, slimes, spells, squids, stares, strides, strolling, swimming, teleports, vexes, villagers, wings, wither};
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{Layer, NetId};
use crate::entities;

/// A mob of `kind` with its feet at `feet` in `layer`: one BDS was recorded announcing, in a
/// loaded column. It is heard appearing.
pub(crate) fn spawn(world: &mut World, layer: Entity, kind: &str, feet: [f32; 3], reason: SpawnReason) -> Option<EntityId> {
    spawn_as(world, layer, kind, feet, reason, 0.0, None)
}

/// Spawn events that make a baby (`summon cow ~ ~ ~ 0 0 minecraft:entity_born`).
pub(super) const BABY_EVENTS: [&str; 3] = ["entity_born", "as_baby", "spawn_baby"];

/// Of these many of a kind's summons with no event, the pack's `entity_spawned` makes a baby of
/// this many (BDS: 2 of 4 plain horse summons, bds-h3herds.txt; of 100 each, 4 cows, 6 pigs, 6
/// chickens, 3 sheep, 3 mooshrooms and 24 cats, bds-k2babies.txt).
const BABIES: [(&str, u32, u32); 11] = [
    ("minecraft:horse", 9, 45),
    ("minecraft:donkey", 20, 100),
    ("minecraft:mule", 20, 100),
    ("minecraft:zombie_horse", 9, 45),
    ("minecraft:skeleton_horse", 9, 45),
    ("minecraft:cow", 5, 100),
    ("minecraft:pig", 5, 100),
    ("minecraft:chicken", 5, 100),
    ("minecraft:sheep", 5, 100),
    ("minecraft:mooshroom", 5, 100),
    ("minecraft:cat", 1, 4),
];
const BORN: &str = "minecraft:entity_born";

/// A `/summon` of `kind`: [`spawn_as`], one of no event rolling for a baby as its pack does.
pub(super) fn summoned(world: &mut World, layer: Entity, kind: &str, feet: [f32; 3], yaw: f32, event: Option<&str>) -> Option<EntityId> {
    let baby = event.is_none() && BABIES.iter().find(|(of, ..)| *of == kind).is_some_and(|&(_, babies, of)| world.get_resource_or_insert_with(Dice::default).roll(of) < babies);
    spawn_with(world, layer, kind, feet, SpawnReason::Command, yaw, event, baby)
}

/// [`spawn`] facing `yaw`, and as its kind's spawn `event` makes it (a panda's gene) where known.
pub(crate) fn spawn_as(world: &mut World, layer: Entity, kind: &str, feet: [f32; 3], reason: SpawnReason, yaw: f32, event: Option<&str>) -> Option<EntityId> {
    spawn_with(world, layer, kind, feet, reason, yaw, event, false)
}

#[allow(clippy::too_many_arguments)]
fn spawn_with(world: &mut World, layer: Entity, kind: &str, feet: [f32; 3], reason: SpawnReason, yaw: f32, event: Option<&str>, rolled_baby: bool) -> Option<EntityId> {
    let look = looks::of(world, kind, event);
    let gene = if slimes::sizes::keeps_base(kind, event) {
        None
    } else {
        event.and_then(|event| genes::of_event(kind, event)).or_else(|| look.map(looks::Look::gene)).or_else(|| genes::roll(world, kind)).or_else(|| husbandry::frog_variant(world, layer, kind, feet))
    };
    // BDS played no `spawn` at any of 112 summons naming an event (research/captures/bds-*.txt), nor
    // for a struck pig's or a piglin's turning (bds-n3struck.txt, bds-t2barter.txt) or a shrieker's
    // emerging warden (bds-t2warden.txt).
    // A bred baby is heard `born` (bds-g11love-2.txt, bds-llamabreed.txt).
    let sound = match reason {
        SpawnReason::Bred => Some("born"),
        _ => (!matches!(reason, SpawnReason::Lightning | SpawnReason::Structure) && (!matches!(reason, SpawnReason::Command | SpawnReason::Summoned | SpawnReason::Transformed) || event.is_none())).then_some("spawn"),
    };
    let event = event.or(rolled_baby.then_some(BORN));
    let id = born_by(world, layer, kind, feet, Birth { reason, yaw, gene, sound }, event, rolled_baby)?;
    let entity = entities::find(world, id)?;
    if let Some(gene) = gene {
        entities::restore(world, entity, |compound, _| sapling_world::varied_actor(compound, gene.0, yaw).ok());
    }
    if let Some(look) = look {
        looks::wear(world, entity, look);
        reshow(world, entity);
    }
    if let Some(event) = event {
        riders::arrive(world, entity, kind, event);
    }
    super::spawn_gear::arm(world, entity, kind, event);
    villagers::arm(world, entity, layer, kind, false, event);
    super::drowned::arm(world, entity, kind, event);
    super::captains::arm(world, entity, kind, event);
    super::skeleton_traps::arm(world, entity, kind, event);
    if event.is_some() {
        super::warden::arm(world, entity, kind, event);
    }
    if kind == "minecraft:creeper" && event.is_some_and(|event| event.trim_start_matches("minecraft:") == "start_exploding_forced") {
        super::blast::light_forced(world, entity, layer, kind);
    }
    Some(id)
}

/// Fires a herd's spawn `event` on the spawned mob: the look it sets over the one rolled (looks.rs),
/// and the riders it makes (riders.rs).
pub(crate) fn fire_herd_event(world: &mut World, entity: Entity, kind: &str, event: &str) {
    if let Some(look) = looks::fired(kind, event, looks::worn(world, entity)) {
        looks::wear(world, entity, look);
        let yaw = world.get::<legs::Legs>(entity).map_or(0.0, |legs| legs.yaw);
        entities::restore(world, entity, |compound, _| sapling_world::varied_actor(compound, look.gene().0, yaw).ok());
        reshow(world, entity);
    }
    riders::arrive(world, entity, kind, event);
}

/// How a mob comes: why, facing where, of which gene, and heard as what, if at all.
pub(super) struct Birth {
    pub reason: SpawnReason,
    pub yaw: f32,
    pub gene: Option<genes::Gene>,
    pub sound: Option<&'static str>,
}

/// A mob its spawn event is being applied to: what the event changes goes into its announcement,
/// not into packets after it, as BDS's `AddEntity` carries a summon's TAMED, SADDLED and BABY
/// (research/captures/bds-mountflags.txt).
#[derive(Component)]
pub(crate) struct Arriving;

pub(crate) fn is_arriving(world: &World, entity: Entity) -> bool {
    world.get::<Arriving>(entity).is_some()
}

/// [`spawn`] as `birth` has it.
pub(super) fn born(world: &mut World, layer: Entity, kind: &str, feet: [f32; 3], birth: Birth) -> Option<EntityId> {
    born_by(world, layer, kind, feet, birth, None, false)
}

/// Makes the mob what its spawn `event` makes it (a baby, tamed, saddled), shown in its announcement.
fn arrive(world: &mut World, entity: Entity, kind: &str, event: &str) {
    world.entity_mut(entity).insert(Arriving);
    if BABY_EVENTS.contains(&event.trim_start_matches("minecraft:")) {
        husbandry::be_born(world, entity, kind);
    }
    husbandry::tame_by_event(world, entity, kind, event);
    crate::riding::saddle_by_event(world, entity, kind, event);
    crate::riding::arrive_ungrouped(world, entity, kind, event);
    world.entity_mut(entity).remove::<Arriving>();
    reshow(world, entity);
}

/// [`born`] by the spawn `event`, if any (a `rolled_baby` is its plain summon's); its first breath,
/// which tells the whole flag word, comes after.
fn born_by(world: &mut World, layer: Entity, kind: &str, feet: [f32; 3], birth: Birth, event: Option<&str>, rolled_baby: bool) -> Option<EntityId> {
    let Birth { reason, yaw, gene, sound } = birth;
    let size = sapling_data::mob_size(kind)?;
    let level = &world.get::<Layer>(layer)?.world;
    let [x, y, z] = feet.map(|c| c.floor() as i32);
    if !(level.in_height(y) && level.is_loaded(sapling_world::ChunkPos::of_block(x, z))) {
        return None;
    }
    if crate::plugins::stopped_in(world, layer, |_| EntitySpawnEvent::new(kind.to_owned(), feet, reason)) {
        return None;
    }
    let body = Body::new(feet, [0.0; 3], slimes::sizes::body_size(kind, gene).unwrap_or(size));
    let properties = super::properties::rolled(world, layer, kind, feet, reason == SpawnReason::Bred, rolled_baby);
    let rolled = strides::roll(world, kind);
    let strength = crate::riding::roll_strength(world, kind);
    let born = super::bats::born_flags(&world.get::<Layer>(layer)?.world, kind, &body);
    let entity = entities::spawn_mob(world, layer, kind, body, feet, |id| Some(announcement(id, kind, &body, (None, None, Some(&properties), rolled.as_ref(), strength), None, [yaw, 0.0], (genes::variant(kind, gene), gene, born))))?;
    world.entity_mut(entity).insert((Creature { look: None }, properties));
    if let Some(rolled) = rolled {
        world.entity_mut(entity).insert(strides::Strode(rolled));
    }
    husbandry::adopt(world, entity, kind, None);
    if let Some(gene) = gene {
        world.entity_mut(entity).insert(gene);
    }
    give_legs(world, entity, kind);
    super::bats::arm(world, entity, layer, kind);
    if let Some(mut legs) = world.get_mut::<legs::Legs>(entity) {
        legs.yaw = yaw;
    }
    give_health(world, entity, kind, None, 0);
    wither::begin(world, entity, layer);
    let id = world.get::<NetId>(entity)?.0;
    // A hatched mob comes unheard (research/captures/bds-eggs.txt); its egg's sound is enough.
    let baby = event.is_some_and(|event| BABY_EVENTS.contains(&event.trim_start_matches("minecraft:")));
    let heard = sound.filter(|_| reason != SpawnReason::Hatched).map(|sound| spawn_sound(sound, id, kind, &body, baby));
    entities::after_announcing(world, layer, [x, y, z], heard.into_iter().collect());
    let unique_id = entities::new_unique_id(world, entity)?;
    entities::keep(world, entity, layer, unique_id, strides::keep(sapling_world::new_actor(unique_id, kind, feet, [0.0; 3]), rolled));
    if crate::riding::give_strength(world, entity, kind, strength) {
        reshow(world, entity);
    }
    if let Some(event) = event {
        arrive(world, entity, kind, event);
    }
    let body = world.get::<Body>(entity).copied().unwrap_or(body);
    harm::first_breath(world, entity, layer, id, kind, &body);
    Some(EntityId(id))
}

/// Makes the stored mob `actor` (its compound `compound`) an entity of `layer`; false for a kind
/// with no recorded box, which stays a statue.
pub(crate) fn from_storage(world: &mut World, layer: Entity, actor: Actor, unique_id: i64, compound: Vec<u8>) -> bool {
    if actor.identifier == crate::dragon::CRYSTAL {
        return crate::dragon::crystal_from_storage(world, layer, actor.position, actor.yaw, unique_id, compound);
    }
    if actor.identifier == husbandry::KNOT {
        return husbandry::knot_from_storage(world, layer, actor.position, unique_id, compound);
    }
    let Some(size) = sapling_data::mob_size(&actor.identifier) else { return false };
    let kind = actor.identifier.clone();
    let gene = genes::stored(&kind, actor.variant);
    let body = Body::new(actor.position, [0.0; 3], slimes::sizes::body_size(&kind, gene).unwrap_or(size));
    let (kept, compound) = strides::stored(world, &kind, compound);
    let strength = crate::riding::kept_strength(&compound);
    let Some(entity) = entities::spawn_mob(world, layer, &kind, body, actor.position, |id| Some(announcement(id, &kind, &body, (Some(&actor), None, None, kept.as_ref(), strength), None, [0.0; 2], (actor.variant, gene, acacia_proto::types::MetadataFlags1(0))))) else { return false };
    if let Some(gene) = gene {
        world.entity_mut(entity).insert(gene);
    }
    if let Some(kept) = kept {
        world.entity_mut(entity).insert(strides::Strode(kept));
    }
    give_legs(world, entity, &kind);
    give_health(world, entity, &kind, actor.health, actor.fire.max(0) as u16);
    super::worn::adopt(world, entity, &actor.armor);
    afflictions::adopt(world, entity, &compound);
    wither::adopt(world, entity, &compound);
    husbandry::adopt(world, entity, &kind, Some((&actor, &compound)));
    crate::spawning::adopt_natural(world, entity);
    world.entity_mut(entity).insert(Creature { look: Some(actor) });
    super::piglins::adopt(world, entity);
    super::captains::adopt(world, entity);
    super::raids::adopt_patrol(world, entity, &compound);
    if kind == crate::dragon::DRAGON {
        crate::dragon::adopt(world, entity);
    }
    entities::keep(world, entity, layer, unique_id, compound);
    villagers::arm(world, entity, layer, &kind, true, None);
    super::skeleton_traps::arm(world, entity, &kind, None);
    crate::riding::give_strength(world, entity, &kind, None);
    reshow(world, entity);
    super::struck::adopt(world, entity);
    crate::riding::adopt(world, entity);
    true
}

/// A mob of `kind` that appears at `feet` for `reason`, as [`spawn`] but the entity.
pub(super) fn spawn_entity(world: &mut World, layer: Entity, kind: &str, feet: [f32; 3], reason: SpawnReason) -> Option<Entity> {
    spawn(world, layer, kind, feet, reason).and_then(|id| entities::find(world, id))
}

/// Gives a mob of a kind that strolls or hunts what it does so with.
fn give_legs(world: &mut World, entity: Entity, kind: &str) {
    let traits = sapling_data::mob_traits(kind);
    if traits.strolls.is_some() || traits.panics.is_some() || traits.hunts.is_some() {
        world.entity_mut(entity).insert((legs::Legs::default(), strolling::Stroll::default()));
    }
    hunting::arm(world, entity, kind);
    slimes::arm(world, entity, kind);
    teleports::arm(world, entity, kind);
    rams::arm(world, entity, kind);
    spells::arm(world, entity, kind);
    breezes::arm(world, entity, kind);
    ghasts::arm(world, entity, kind);
    vexes::arm(world, entity, kind);
    stares::arm(world, entity, kind);
    creaking::arm(world, entity, kind);
    super::warden::arm(world, entity, kind, None);
    dolphins::arm(world, entity, kind);
    swimming::arm(world, entity, kind);
    squids::arm(world, entity, kind);
    frogs::arm(world, entity, kind);
    wings::arm(world, entity, kind);
    bees::arm(world, entity, kind);
    drinks::arm(world, entity, kind);
    wither::arm(world, entity, kind);
}

/// Gives a mob of a kind that dies the health BDS announced its kind with (a mount its own),
/// `left` of it, and `burning` ticks of fire.
fn give_health(world: &mut World, entity: Entity, kind: &str, left: Option<f32>, burning: u16) {
    if let Some(max) = sapling_data::mob_health(kind).filter(|_| !NO_DEATH.contains(&kind)) {
        let max = genes::health(kind, world.get::<genes::Gene>(entity).copied(), max);
        let max = world.get::<strides::Strode>(entity).map_or(max, |strode| strode.0.health);
        let mut health = Health::new(max, left, burning);
        health.wear(super::worn::skin(kind), Default::default());
        world.entity_mut(entity).insert(health);
    }
}
