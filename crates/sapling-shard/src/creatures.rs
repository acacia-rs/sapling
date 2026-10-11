//! Mobs, spawned through the plugin API or kept by a world folder: bodies (body.rs) that fall and
//! collide, stroll (strolling.rs) and hunt if their kind does (hunting.rs), shown as BDS announced their kind (`sapling_data::add_entity`) or
//! as they were stored; and the entity side of the handlers' context.
// TODO: no AI but strolling and hunting; their physics are the Java game's (`Physics::MOB`); a spawned mob is
// saved with the least a compound needs, which BDS was not tried on.

mod arrows;
mod attacks;
mod blast;
mod harm;
mod health;
mod hunting;
mod legs;
mod strolling;

use bevy_ecs::prelude::*;
use bytes::Bytes;
use acacia_proto::packets::{EntityEvent, EntityEventEventId, LevelSoundEvent};
use acacia_proto::types::{MetadataFlags1, Vec3f};
use sapling_api::{EntityBackend, EntityId, EntityInfo, Item};
use sapling_data::AddEntityOptions;

use sapling_world::Actor;

use crate::blocks::Dice;
use crate::body::{Body, Physics};
pub(crate) use attacks::attacked;
pub(crate) use harm::{holds, is_full_block, touches};
pub(crate) use health::{Cause, Health};
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

impl Creature {
    fn is_baby(&self) -> bool {
        self.look.as_ref().is_some_and(|look| look.baby)
    }
}

// TODO: a mob dressed as it was stored is not shown on fire to one who comes to see it burning.
fn announcement(id: u64, kind: &str, body: &Body, look: Option<&Actor>, health: Option<&Health>, yaw: f32) -> Vec<Bytes> {
    let left = health.map(Health::left);
    if let Some(shown) = look.and_then(|look| mobs::announcement(&Actor { position: body.at, health: left.or(look.health), ..look.clone() }, id)) {
        return shown;
    }
    let on_fire = health.is_some_and(Health::is_burning);
    let options = AddEntityOptions {
        id,
        identifier: kind,
        feet: body.at,
        yaw,
        pitch: 0.0,
        variant: 0,
        mark_variant: 0,
        color: 0,
        baby: false,
        flags: MetadataFlags1(if on_fire { MetadataFlags1::ONFIRE.0 } else { 0 }),
        health: left,
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

/// Where a mob's head is, as BDS places it; it faces +z (yaw 0).
fn head(kind: &str, body: &Body) -> [f32; 3] {
    let ([x, y, z], [width, height]) = (body.at, body.size);
    let forward = if ANIMALS.contains(&kind) { HEAD_FORWARD * width } else { 0.0 };
    [x, y + HEAD_UP * height, z + forward]
}

/// The sound a mob makes as it appears, at its head.
fn spawn_sound(id: u64, kind: &str, body: &Body) -> Bytes {
    let [x, y, z] = head(kind, body);
    let position = Vec3f { x, y, z };
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
    let entity = entities::spawn(world, layer, kind, body, feet, |id| Some(announcement(id, kind, &body, None, None, 0.0)))?;
    world.entity_mut(entity).insert(Creature { look: None });
    give_legs(world, entity, kind);
    give_health(world, entity, kind, None, 0);
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
    let Some(entity) = entities::spawn(world, layer, &kind, body, actor.position, |id| Some(announcement(id, &kind, &body, Some(&actor), None, 0.0))) else { return false };
    give_legs(world, entity, &kind);
    give_health(world, entity, &kind, actor.health, actor.fire.max(0) as u16);
    world.entity_mut(entity).insert(Creature { look: Some(actor) });
    entities::keep(world, entity, layer, unique_id, compound);
    true
}

/// Gives a mob of a kind that strolls or hunts what it does so with.
fn give_legs(world: &mut World, entity: Entity, kind: &str) {
    let traits = sapling_data::mob_traits(kind);
    if traits.strolls.is_some() || traits.panics.is_some() || traits.hunts.is_some() {
        world.entity_mut(entity).insert((legs::Legs::default(), strolling::Stroll::default()));
    }
    hunting::arm(world, entity, kind);
}

/// Gives a mob of a kind that dies the health BDS announced its kind with, `left` of it, and
/// `burning` ticks of fire.
fn give_health(world: &mut World, entity: Entity, kind: &str, left: Option<f32>, burning: u16) {
    if let Some(max) = sapling_data::mob_health(kind).filter(|_| !NO_DEATH.contains(&kind)) {
        world.entity_mut(entity).insert(Health::new(max, left, burning));
    }
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
    let death = EntityEvent { runtime_entity_id: id.0, event_id: EntityEventEventId::DeathAnimation, data: 0, fire_at_position: None };
    entities::tell_viewers(world, entity, layer, body.at.map(|c| c.floor() as i32), vec![sapling_net::encode(&death)]);
    die(world, entity, layer, &kind, &body);
    true
}

/// The mob, seen to die, is dead where it is, no longer kept by the world folder, and leaves its
/// loot, cooked if it was on fire.
fn die(world: &mut World, entity: Entity, layer: Entity, kind: &str, body: &Body) {
    let now = world.resource::<Tick>().0;
    if let Some(id) = world.get::<NetId>(entity).map(|id| id.0) {
        hunting::let_go(world, entity, layer, id, body);
    }
    world.entity_mut(entity).insert(Dying(now)).remove::<entities::Stored>();
    let on_fire = world.get_mut::<Health>(entity).is_some_and(|mut health| {
        health.end();
        health.is_burning()
    });
    if leaves_loot(world, layer) {
        leave_loot(world, layer, kind, on_fire, body.at);
    }
}

fn leaves_loot(world: &World, layer: Entity) -> bool {
    world.get::<Layer>(layer).is_some_and(|layer| layer.world.level().rules.on(MOB_LOOT))
}

fn leave_loot(world: &mut World, layer: Entity, kind: &str, on_fire: bool, [x, y, z]: [f32; 3]) {
    let loot = {
        let mut dice = world.get_resource_or_insert_with(Dice::default);
        sapling_data::mob_loot(kind, on_fire, &mut || dice.fraction())
    };
    let Some(registry) = world.get::<Layer>(layer).map(|layer| layer.world.registry()) else { return };
    let stacks: Vec<_> = loot.into_iter().filter_map(|(name, count)| inventory::stored(&Item::new(name, count.min(64) as u8), registry)).collect();
    for stack in stacks {
        drops::toss(world, layer, stack, [x, y + LOOT_LIFT, z]);
    }
}

/// Moves every spawned mob a tick and hurts it by where it is; a dead one goes once it has died.
pub(crate) fn tick(world: &mut World) {
    arrows::tick(world);
    let now = world.resource::<Tick>().0;
    let dead: Vec<Entity> = world.query::<(Entity, &Dying)>().iter(world).filter(|(_, dying)| now >= dying.0 + DYING).map(|(entity, _)| entity).collect();
    for entity in dead {
        attacks::leave_experience(world, entity);
        mobs::remove(world, entity);
    }
    let all: Vec<(Entity, Entity, u64, String)> =
        world.query_filtered::<(Entity, &InLayer, &NetId, &entities::Kind), With<Creature>>().iter(world).map(|(entity, layer, id, kind)| (entity, layer.0, id.0, kind.0.clone())).collect();
    for (entity, layer, id, kind) in all {
        let Some(start) = world.get::<Body>(entity).copied() else { continue };
        let is_alive = world.get::<Dying>(entity).is_none();
        if is_alive && !hunting::steer(world, entity, layer, id, &kind) {
            let traits = sapling_data::mob_traits(&kind);
            let strolls = world.resource::<crate::components::Settings>().0.mobs_stroll;
            if strolls {
                strolling::steer(world, entity, layer, traits.strolls.or(traits.hunts.map(|hunts| hunts.speed)), traits.panics);
            }
        }
        let Some((body, packets)) = entities::step(world, entity, layer, id, &Physics::MOB, 0.0) else { continue };
        blocks::to_viewers(world, layer, body.at.map(|c| c.floor() as i32), &packets);
        if is_alive {
            hunting::strike(world, entity, layer, id, &kind, &body);
            if blast::tick(world, entity, layer, id, &kind, &body) {
                continue;
            }
        }
        let before = world.get::<Health>(entity).map(|health| (health.left(), health.is_burning()));
        if harm::tick(world, entity, layer, id, &kind, &start, &body, world.get::<Dying>(entity).is_some()) {
            die(world, entity, layer, &kind, &body);
        }
        let health = world.get::<Health>(entity);
        if !packets.is_empty() || health.map(|health| (health.left(), health.is_burning())) != before {
            let shown = announcement(id, &kind, &body, world.get::<Creature>(entity).and_then(|creature| creature.look.as_ref()), health, world.get::<legs::Legs>(entity).map_or(0.0, |legs| legs.yaw));
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
