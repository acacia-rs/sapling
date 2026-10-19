//! A player's death and its rising, in the packets BDS sends for them
//! (docs/research/player-damage.md, "Death").
// TODO: only the words of fall, fire, lava, `/kill` and a zombie were seen on BDS, the rest are
// the game's keys from memory; the body's box is not shrunk and `PlayerLastDeathPosition` is not told; every death
// rises by its bed or at the world's spawn (no anchor); `doImmediateRespawn` is not read.

use acacia_proto::packets::{DeathInfo, EntityEvent, EntityEventEventId, LevelSoundEvent, Respawn, SetHealth};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_data::EYE_HEIGHT;

use super::{MAX, Vitals, tell};
use crate::components::{Identity, InLayer, Layer, NetId, Outgoing, PacketQueues, Position, Tick};
use crate::creatures::Cause;
use crate::{chat, drops, experience, inventory, movement, orbs};

const KEEP_INVENTORY: &str = "keepInventory";
/// `Respawn` states: the server looks for a place, has one, and the client is ready for it.
const SEARCHING: u8 = 0;
const READY: u8 = 1;
const CLIENT_READY: u8 = 2;
/// Ticks a body lies before it goes and leaves its experience.
const LIES: u64 = 20;
/// The middle of a dead player's box over its feet, where its two sounds come from.
const BODY_MIDDLE: f32 = 0.1;

/// The kind of the mob that last hit a player: named if that hit is what it dies of.
#[derive(Component)]
pub(super) struct Slayer(pub String);

/// Before the name of a player in [`Slayer`], which else holds a mob's kind.
pub(super) const BY_PLAYER: &str = "player:";

fn words(cause: Cause) -> &'static str {
    match cause {
        Cause::Fall => "death.fell.accident.generic",
        Cause::Fire => "death.attack.inFire",
        Cause::FireTick => "death.attack.onFire",
        Cause::Lava => "death.attack.lava",
        Cause::Drowning => "death.attack.drown",
        Cause::Suffocation => "death.attack.inWall",
        Cause::Starving => "death.attack.starve",
        Cause::Magic => "death.attack.magic",
        Cause::Wither => "death.attack.wither",
        Cause::Contact => "death.attack.cactus",
        Cause::BlockExplosion | Cause::EntityExplosion => "death.attack.explosion",
        Cause::Void => "death.attack.outOfWorld",
        Cause::Anvil => "death.attack.anvil",
        Cause::FallingBlock => "death.attack.fallingBlock",
        Cause::FlyIntoWall => "death.attack.flyIntoWall",
        Cause::Magma => "death.attack.magma",
        Cause::Lightning => "death.attack.lightningBolt",
        Cause::Freezing => "death.attack.freeze",
        Cause::Stalactite => "death.attack.stalactite",
        Cause::Stalagmite => "death.attack.stalagmite",
        Cause::SonicBoom => "death.attack.sonicBoom",
        Cause::Thorns => "death.attack.thorns",
        Cause::Projectile | Cause::Campfire | Cause::SoulCampfire | Cause::Generic | Cause::Attack | Cause::RamAttack => "death.attack.generic",
    }
}

fn eyes([x, y, z]: [f32; 3]) -> Vec3f {
    Vec3f { x, y: y + EYE_HEIGHT, z }
}

fn sound(world: &mut World, player: Entity, id: u64, name: &str, [x, y, z]: [f32; 3]) {
    let position = Vec3f { x, y: y + BODY_MIDDLE, z };
    let sound = LevelSoundEvent { sound_id: name.into(), position, extra_data: -1, entity_type: "minecraft:player".into(), is_baby_mob: false, is_global: false, entity_unique_id: id as i64, fire_at_position: None };
    tell(world, player, &sound);
}

/// The player, out of health, dies of `cause`: what it held lies where it stood, unless the
/// world keeps inventories, and its client shows the death screen.
pub(super) fn die(world: &mut World, player: Entity, cause: Cause) {
    let now = world.resource::<Tick>().0;
    let (Some(id), Some(feet), Some(layer), Some(name)) = (
        world.get::<NetId>(player).map(|id| id.0),
        world.get::<Position>(player).map(|position| position.feet),
        world.get::<InLayer>(player).map(|layer| layer.0),
        world.get::<Identity>(player).map(|identity| identity.0.name.clone()),
    ) else {
        return;
    };
    let keeps = world.get::<Layer>(layer).is_some_and(|layer| layer.world.level().rules.on(KEEP_INVENTORY));
    let (stacks, points) = if keeps { Default::default() } else { inventory::change(world, player, |inventory| inventory.let_go()).unwrap_or_default() };
    for stack in stacks {
        drops::toss(world, layer, stack, feet);
    }
    if let Some(mut vitals) = world.get_mut::<Vitals>(player) {
        (vitals.died, vitals.leaves) = (Some(now), Some(points));
    }
    tell(world, player, &EntityEvent { runtime_entity_id: id, event_id: EntityEventEventId::DeathAnimation, data: 0, fire_at_position: None });
    let slayer = world.get::<Slayer>(player).filter(|_| matches!(cause, Cause::Attack | Cause::Projectile)).map(|slayer| slayer.0.clone());
    let by_player = slayer.as_deref().and_then(|slayer| slayer.strip_prefix(BY_PLAYER)).map(str::to_owned);
    let key = match (slayer.is_some(), by_player.is_some(), cause) {
        (true, true, _) => "death.attack.player",
        (true, _, Cause::Projectile) => "death.attack.arrow",
        (true, ..) => "death.attack.mob",
        (false, ..) => words(cause),
    };
    let slayer = by_player.or(slayer.map(|kind| format!("%entity.{}.name", kind.trim_start_matches("minecraft:"))));
    let messages: Vec<String> = std::iter::once(name).chain(slayer).collect();
    let said = chat::translated_packet(key, &messages);
    chat::send_to_layer(world, layer, &said);
    let mut out = world.get_mut::<Outgoing>(player).expect("players have an outbox");
    out.push(&DeathInfo { cause: key.into(), messages });
    out.push(&Respawn { position: eyes(feet), state: SEARCHING, runtime_entity_id: 0 });
}

/// A dead player's body goes once it has lain, leaving its experience.
pub(super) fn leave(world: &mut World, player: Entity, layer: Entity) {
    let now = world.resource::<Tick>().0;
    let Some((died, points)) = world.get::<Vitals>(player).and_then(|vitals| vitals.died.zip(vitals.leaves)) else { return };
    let (Some(id), Some(feet)) = (world.get::<NetId>(player).map(|id| id.0), world.get::<Position>(player).map(|position| position.feet)) else { return };
    if now < died + LIES {
        return;
    }
    world.get_mut::<Vitals>(player).expect("read above").leaves = None;
    if points > 0 {
        orbs::spawn(world, layer, feet, points);
    }
    sound(world, player, id, "disappeared", feet);
}

/// The dead whose clients are ready rise by the bed they last clicked, or at their world's
/// spawn, whole.
pub(super) fn rise(world: &mut World) {
    let ready: Vec<Entity> = world.resource_mut::<PacketQueues>().take::<Respawn>().into_iter().filter(|(_, asked)| asked.state == CLIENT_READY).map(|(player, _)| player).collect();
    for player in ready {
        if !world.get::<Vitals>(player).is_some_and(|vitals| vitals.died.is_some()) {
            continue;
        }
        let (Some(id), Some(died_in)) = (world.get::<NetId>(player).copied(), world.get::<InLayer>(player).map(|layer| layer.0)) else { continue };
        // The dead of the Nether and the End rise in their overworld.
        let layer = crate::dimensions::home(world, died_in);
        let Some(spawn) = world.get::<Layer>(layer).map(|layer| layer.spawn) else { continue };
        let by_bed = crate::sleep::spawn_of(world, player, layer).flatten();
        let unfound = layer != died_in;
        if unfound {
            leave_now(world, player, died_in);
            crate::dimensions::respawn_home(world, player, layer, by_bed.unwrap_or(spawn));
            world.get_mut::<Outgoing>(player).expect("players have an outbox").push(&Respawn { position: Vec3f { x: 0.0, y: 32767.0, z: 0.0 }, state: SEARCHING, runtime_entity_id: 0 });
        }
        if let Some([x, y, z]) = by_bed {
            // BDS names the cell first, then the place in it.
            let cell = Vec3f { x: x.floor(), y, z: z.floor() };
            world.get_mut::<Outgoing>(player).expect("players have an outbox").push(&Respawn { position: cell, state: SEARCHING, runtime_entity_id: 0 });
        }
        let spawn = by_bed.unwrap_or(spawn);
        leave_now(world, player, layer);
        world.entity_mut(player).remove::<super::effects::Effects>();
        *world.get_mut::<Vitals>(player).expect("read above") = Vitals::default();
        world.get_mut::<Outgoing>(player).expect("players have an outbox").push(&Respawn { position: eyes(spawn), state: READY, runtime_entity_id: 0 });
        movement::teleport_onto_ground(world, player, spawn);
        tell(world, player, &EntityEvent { runtime_entity_id: id.0, event_id: EntityEventEventId::Respawn, data: 0, fire_at_position: None });
        tell(world, player, &sapling_data::mob_health_update(id.0, MAX, MAX));
        let tick = world.resource::<Tick>().0;
        let shown = world.get::<inventory::Inventory>(player).map(|inventory| experience::attributes(&id, &inventory.experience, true, tick));
        tell(world, player, &Vitals::default().hunger.told(id.0, tick));
        let mut out = world.get_mut::<Outgoing>(player).expect("players have an outbox");
        out.push(&SetHealth { health: MAX as i32 });
        shown.iter().for_each(|shown| out.push(shown));
        // From another dimension BDS heard it at the place it named before it found one.
        sound(world, player, id.0, "reappeared", if unfound { [0.0, 32767.0, 0.0] } else { spawn });
    }
}

/// What [`leave`] would leave later, left as the player rises before its body went.
fn leave_now(world: &mut World, player: Entity, layer: Entity) {
    let Some(points) = world.get_mut::<Vitals>(player).and_then(|mut vitals| vitals.leaves.take()) else { return };
    let Some(feet) = world.get::<Position>(player).map(|position| position.feet) else { return };
    if points > 0 {
        orbs::spawn(world, layer, feet, points);
    }
}
