//! Mobs, spawned through the plugin API or kept by a world folder: bodies (body.rs) that fall and
//! collide and do nothing else, shown as BDS announced their kind (`sapling_data::add_entity`) or
//! as they were stored; and the entity side of the handlers' context.
// TODO: no AI, no damage; their physics are the Java game's (`Physics::MOB`); a spawned mob is
// saved with the least a compound needs, which BDS was not tried on.

use bevy_ecs::prelude::*;
use bytes::Bytes;
use acacia_proto::packets::{EntityEvent, EntityEventEventId, LevelSoundEvent};
use acacia_proto::types::{MetadataFlags1, Vec3f};
use sapling_api::{EntityBackend, EntityId, EntityInfo, Item};
use sapling_data::AddEntityOptions;

use sapling_world::Actor;

use crate::blocks::Dice;
use crate::body::{Body, Physics};
use crate::components::{InLayer, Layer, NetId, Tick};
use crate::handlers::Ctx;
use crate::{blocks, drops, entities, inventory, mobs, orbs, worlds};

const MOB_LOOT: &str = "doMobLoot";
const ORB: &str = "minecraft:xp_orb";

/// A mob: spawned through the API, or kept by a world folder and dressed as it was stored
/// (`look`). Its [`Body`] and [`entities::Kind`] are beside it.
#[derive(Component)]
pub(crate) struct Creature {
    look: Option<Actor>,
}

fn announcement(id: u64, kind: &str, body: &Body, look: Option<&Actor>) -> Vec<Bytes> {
    if let Some(shown) = look.and_then(|look| mobs::announcement(&Actor { position: body.at, ..look.clone() }, id)) {
        return shown;
    }
    let options = AddEntityOptions {
        id,
        identifier: kind,
        feet: body.at,
        yaw: 0.0,
        pitch: 0.0,
        variant: 0,
        mark_variant: 0,
        color: 0,
        baby: false,
        flags: MetadataFlags1(0),
        health: None,
        name: "",
    };
    vec![sapling_net::encode(&sapling_data::add_entity(&options))]
}

/// Kinds whose head sits forward of their feet, by this much of their width (BDS's `Animal`
/// constructors); every other mob's is straight up (docs/research/spawn-sound.md).
const ANIMALS: &[&str] = &[
    "minecraft:chicken", "minecraft:cow", "minecraft:mooshroom", "minecraft:rabbit", "minecraft:ocelot", "minecraft:horse", "minecraft:llama", "minecraft:turtle", "minecraft:cat",
    "minecraft:pig",
];
const HEAD_FORWARD: f32 = 0.85;
/// A mob's head is this much of its height up.
const HEAD_UP: f32 = 0.85;

/// The sound a mob makes as it appears, at its head as BDS places it; it faces +z (yaw 0).
fn spawn_sound(id: u64, kind: &str, body: &Body) -> Bytes {
    let ([x, y, z], [width, height]) = (body.at, body.size);
    let forward = if ANIMALS.contains(&kind) { HEAD_FORWARD * width } else { 0.0 };
    let position = Vec3f { x, y: y + HEAD_UP * height, z: z + forward };
    let sound = LevelSoundEvent { sound_id: "spawn".into(), position, extra_data: -1, entity_type: kind.into(), is_baby_mob: false, is_global: false, entity_unique_id: id as i64, fire_at_position: None };
    sapling_net::encode(&sound)
}

/// A mob of `kind` with its feet at `feet` in `layer`: one BDS was recorded announcing, in a
/// loaded column. It is heard appearing.
fn spawn(world: &mut World, layer: Entity, kind: &str, feet: [f32; 3]) -> Option<EntityId> {
    let size = sapling_data::mob_size(kind)?;
    let level = &world.get::<Layer>(layer)?.world;
    let [x, y, z] = feet.map(|c| c.floor() as i32);
    if !(level.in_height(y) && level.is_loaded(sapling_world::ChunkPos::of_block(x, z))) {
        return None;
    }
    let body = Body::new(feet, [0.0; 3], size);
    let entity = entities::spawn(world, layer, kind, body, feet, |id| Some(announcement(id, kind, &body, None)))?;
    world.entity_mut(entity).insert(Creature { look: None });
    let id = world.get::<NetId>(entity)?.0;
    entities::after_announcing(world, layer, [x, y, z], vec![spawn_sound(id, kind, &body)]);
    // Kept by the world folder as BDS numbers actors: a high word that differs, a counter below.
    let unique_id = ((world.resource::<Tick>().0 as i64) << 32) | (id as i64 & 0xffff_ffff);
    entities::keep(world, entity, layer, unique_id, sapling_world::new_actor(unique_id, kind, feet, [0.0; 3]));
    Some(EntityId(id))
}

/// Makes the stored mob `actor` (its compound `compound`) an entity of `layer`; false for a kind
/// with no recorded box, which stays a statue.
pub(crate) fn from_storage(world: &mut World, layer: Entity, actor: Actor, unique_id: i64, compound: Vec<u8>) -> bool {
    let Some(size) = sapling_data::mob_size(&actor.identifier) else { return false };
    let body = Body::new(actor.position, [0.0; 3], size);
    let kind = actor.identifier.clone();
    let Some(entity) = entities::spawn(world, layer, &kind, body, actor.position, |id| Some(announcement(id, &kind, &body, Some(&actor)))) else { return false };
    world.entity_mut(entity).insert(Creature { look: Some(actor) });
    entities::keep(world, entity, layer, unique_id, compound);
    true
}

/// A mob that died at the tick it holds; it goes [`DYING`] ticks after.
#[derive(Component)]
struct Dying(u64);

/// Ticks from a mob's death to its going, and how far above its feet its loot appears (where the
/// wire puts it), as BDS was seen to (research/captures/bds-kill.txt).
const DYING: u64 = 21;
const LOOT_LIFT: f32 = 0.075;
/// Killed, these go at once, without a death or loot, as BDS's armor stand.
const NO_DEATH: &[&str] = &["minecraft:armor_stand"];

/// Kills the entity `id` (see `ShardCtx::kill_entity`).
fn kill(world: &mut World, id: EntityId) -> bool {
    let Some(entity) = entities::find(world, id).filter(|entity| world.get::<Dying>(*entity).is_none()) else { return false };
    let (Some(kind), Some(body), Some(layer)) = (world.get::<entities::Kind>(entity).map(|kind| kind.0.clone()), world.get::<Body>(entity).copied(), world.get::<InLayer>(entity).map(|layer| layer.0)) else { return false };
    if world.get::<Creature>(entity).is_none() || NO_DEATH.contains(&kind.as_str()) {
        mobs::remove(world, entity);
        return true;
    }
    let now = world.resource::<Tick>().0;
    world.entity_mut(entity).insert(Dying(now));
    let death = EntityEvent { runtime_entity_id: id.0, event_id: EntityEventEventId::DeathAnimation, data: 0, fire_at_position: None };
    blocks::to_viewers(world, layer, body.at.map(|c| c.floor() as i32), &[sapling_net::encode(&death)]);
    if world.get::<Layer>(layer).is_some_and(|layer| layer.world.level().rules.on(MOB_LOOT)) {
        leave_loot(world, layer, &kind, body.at);
    }
    true
}

fn leave_loot(world: &mut World, layer: Entity, kind: &str, [x, y, z]: [f32; 3]) {
    let loot = {
        let mut dice = world.get_resource_or_insert_with(Dice::default);
        sapling_data::mob_loot(kind, &mut || dice.fraction())
    };
    let Some(registry) = world.get::<Layer>(layer).map(|layer| layer.world.registry()) else { return };
    let stacks: Vec<_> = loot.into_iter().filter_map(|(name, count)| inventory::stored(&Item::new(name, count.min(64) as u8), registry)).collect();
    for stack in stacks {
        drops::toss(world, layer, stack, [x, y + LOOT_LIFT, z]);
    }
}

/// Moves every spawned mob a tick; a dead one goes once it has died.
pub(crate) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let dead: Vec<Entity> = world.query::<(Entity, &Dying)>().iter(world).filter(|(_, dying)| now >= dying.0 + DYING).map(|(entity, _)| entity).collect();
    for entity in dead {
        mobs::remove(world, entity);
    }
    let all: Vec<(Entity, Entity, u64, String)> =
        world.query_filtered::<(Entity, &InLayer, &NetId, &entities::Kind), With<Creature>>().iter(world).map(|(entity, layer, id, kind)| (entity, layer.0, id.0, kind.0.clone())).collect();
    for (entity, layer, id, kind) in all {
        let Some((body, packets)) = entities::step(world, entity, layer, id, &Physics::MOB, 0.0) else { continue };
        if !packets.is_empty() {
            blocks::to_viewers(world, layer, body.at.map(|c| c.floor() as i32), &packets);
            let shown = announcement(id, &kind, &body, world.get::<Creature>(entity).and_then(|creature| creature.look.as_ref()));
            mobs::reannounce(world, entity, shown);
        }
    }
}

impl EntityBackend for Ctx<'_> {
    /// An orb is an orb, worth 1 and thrown as orbs are, as BDS's `/summon xp_orb` makes one.
    fn spawn_entity(&mut self, kind: &str, feet: [f32; 3]) -> Option<EntityId> {
        let layer = worlds::current(self.world);
        if kind == ORB {
            let orb = orbs::throw(self.world, layer, feet, 1)?;
            return Some(EntityId(self.world.get::<NetId>(orb)?.0));
        }
        spawn(self.world, layer, kind, feet)
    }

    fn entity(&self, id: EntityId) -> Option<EntityInfo> {
        entities::describe(self.world, id)
    }

    fn entities(&self) -> Vec<EntityInfo> {
        entities::listed(self.world, worlds::current(self.world))
    }

    fn move_entity(&mut self, id: EntityId, feet: [f32; 3]) -> bool {
        entities::change(self.world, id, |body| body.at = feet)
    }

    fn set_velocity(&mut self, id: EntityId, velocity: [f32; 3]) -> bool {
        entities::change(self.world, id, |body| body.motion = velocity)
    }

    fn remove_entity(&mut self, id: EntityId) -> bool {
        entities::remove(self.world, id)
    }

    fn kill_entity(&mut self, id: EntityId) -> bool {
        kill(self.world, id)
    }
}
