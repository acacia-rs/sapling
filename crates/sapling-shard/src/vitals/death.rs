//! A player's death and its rising, in the packets BDS sends for them
//! (docs/research/player-damage.md, "Death"). `doImmediateRespawn` asks nothing of the server:
//! BDS answered a client's respawn alike with it on (research/captures/bds-anchor.txt).
// TODO: only the words of fall, fire, lava, `/kill` and a zombie were seen on BDS, the rest are
// the game's keys from memory.

use acacia_proto::packets::{DeathInfo, EntityEvent, EntityEventEventId, LevelSoundEvent, Respawn, SetHealth};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::{Attacker, DeathEvent, DeathMessage, RespawnEvent};
use sapling_data::EYE_HEIGHT;

use super::last_death::{self, LastDeath};
use super::{MAX, Vitals, tell};
use crate::components::{Identity, InLayer, Layer, NetId, Outgoing, PacketQueues, Position, Tick, in_world, in_world_identity};
use crate::creatures::Cause;
use crate::sleep::Own;
use crate::{chat, drops, experience, inventory, movement, orbs};

const KEEP_INVENTORY: &str = "keepInventory";
const SHOW_DEATH_MESSAGES: &str = "showDeathMessages";
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

/// Who the player's [`Slayer`] names, if anyone: a player still on the shard, or a mob's kind.
pub(super) fn attacker(world: &World, player: Entity) -> Option<Attacker> {
    let slayer = &world.get::<Slayer>(player)?.0;
    match slayer.strip_prefix(BY_PLAYER) {
        Some(name) => in_world(world).into_iter().filter_map(|other| in_world_identity(world, other)).find(|other| other.name == name).map(Attacker::Player),
        None => Some(Attacker::Mob { kind: slayer.clone() }),
    }
}

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
        Cause::Fireworks => "death.attack.fireworks",
        Cause::Freezing => "death.attack.freeze",
        Cause::Stalactite => "death.attack.stalactite",
        Cause::Stalagmite => "death.attack.stalagmite",
        Cause::SonicBoom => "death.attack.sonicBoom",
        Cause::Thorns => "death.attack.thorns",
        _ => "death.attack.generic",
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
/// world or a plugin keeps it, and its client shows the death screen.
pub(super) fn die(world: &mut World, player: Entity, cause: Cause) {
    let now = world.resource::<Tick>().0;
    let (Some(id), Some(feet), Some(layer), Some(who)) = (
        world.get::<NetId>(player).map(|id| id.0),
        world.get::<Position>(player).map(|position| position.feet),
        world.get::<InLayer>(player).map(|layer| layer.0),
        world.get::<Identity>(player).map(|identity| identity.0.clone()),
    ) else {
        return;
    };
    let name = who.name.clone();
    let keeps = crate::rules::on(world, layer, KEEP_INVENTORY);
    let mut death = DeathEvent::new(who, cause, attacker(world, player), keeps);
    if !crate::rules::on(world, layer, SHOW_DEATH_MESSAGES) {
        death.message = DeathMessage::Hidden;
    }
    let death = crate::plugins::offer(world, death);
    let (stacks, points) = if death.keep_inventory { Default::default() } else { inventory::change(world, player, |inventory| inventory.let_go()).unwrap_or_default() };
    for stack in stacks {
        drops::toss(world, layer, stack, feet);
    }
    // A catalyst blooms for a player kept whole too, but is offered nothing (144879f70).
    let kept = death.keep_inventory;
    let points = if crate::blocks::mob_died(world, layer, feet, |_| (!kept).then_some(points)) { 0 } else { points };
    if let Some(mut vitals) = world.get_mut::<Vitals>(player) {
        (vitals.died, vitals.leaves) = (Some(now), Some(points));
    }
    super::omens::died(world, player);
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
    let said = match &death.message {
        DeathMessage::Text(text) => Some(chat::raw_packet(text)),
        DeathMessage::Hidden => None,
        _ => Some(chat::translated_packet(key, &messages)),
    };
    if let Some(said) = said {
        chat::send_to_layer(world, layer, &said);
    }
    world.get_mut::<Outgoing>(player).expect("players have an outbox").push(&DeathInfo { cause: key.into(), messages });
    remember(world, player, id, feet, layer);
    world.get_mut::<Outgoing>(player).expect("players have an outbox").push(&Respawn { position: eyes(feet), state: SEARCHING, runtime_entity_id: 0 });
}

/// Keeps where the player died and shows its body shrunk, with the place, to it and its viewers.
fn remember(world: &mut World, player: Entity, id: u64, feet: [f32; 3], layer: Entity) {
    let dimension = world.get::<Layer>(layer).map_or(0, |layer| layer.world.dimension());
    let now = LastDeath::at_feet(feet, dimension);
    let before = world.get::<LastDeath>(player).copied();
    let tick = world.resource::<Tick>().0;
    tell(world, player, &last_death::told(id, before, now, crate::sleep::LYING_BOX, tick));
    world.entity_mut(player).insert(now);
    if let Some(mut inventory) = world.get_mut::<inventory::Inventory>(player) {
        inventory.unsaved = true;
    }
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

/// The dead whose clients are ready rise by the bed or anchor they last clicked, or at their
/// world's spawn, whole.
pub(super) fn rise(world: &mut World) {
    let ready: Vec<Entity> = world.resource_mut::<PacketQueues>().take::<Respawn>().into_iter().filter(|(_, asked)| asked.state == CLIENT_READY).map(|(player, _)| player).collect();
    for player in ready {
        if !world.get::<Vitals>(player).is_some_and(|vitals| vitals.died.is_some()) {
            continue;
        }
        let (Some(id), Some(died_in), Some(body)) = (world.get::<NetId>(player).copied(), world.get::<InLayer>(player).map(|layer| layer.0), world.get::<Position>(player).map(|position| position.feet)) else { continue };
        // The dead of the Nether and the End rise in their overworld, but by an anchor.
        let realm = crate::dimensions::members(world, died_in);
        let (own, anchor, lapsed, layer) = match crate::sleep::spawn_of(world, player, &realm) {
            Own::At { layer, feet, anchor } => (Some(feet), anchor, None, layer),
            Own::Lapsed(anchor) => (None, None, Some(anchor), crate::dimensions::home(world, died_in)),
            Own::None => (None, None, None, crate::dimensions::home(world, died_in)),
        };
        let Some(spawn) = world.get::<Layer>(layer).map(|layer| layer.spawn) else { continue };
        let unfound = layer != died_in;
        if unfound {
            leave_now(world, player, died_in);
            crate::dimensions::respawn_home(world, player, layer, own.unwrap_or(spawn));
            world.get_mut::<Outgoing>(player).expect("players have an outbox").push(&Respawn { position: Vec3f { x: 0.0, y: 32767.0, z: 0.0 }, state: SEARCHING, runtime_entity_id: 0 });
        }
        if let Some([x, y, z]) = own {
            // BDS names the cell first, then the place in it.
            let cell = Vec3f { x: x.floor(), y, z: z.floor() };
            world.get_mut::<Outgoing>(player).expect("players have an outbox").push(&Respawn { position: cell, state: SEARCHING, runtime_entity_id: 0 });
        }
        let spawn = own.unwrap_or(spawn);
        let Some(who) = world.get::<Identity>(player).map(|identity| identity.0.clone()) else { continue };
        let spawn = crate::plugins::offer(world, RespawnEvent::new(who, spawn)).feet;
        leave_now(world, player, layer);
        world.entity_mut(player).remove::<super::effects::Effects>();
        *world.get_mut::<Vitals>(player).expect("read above") = Vitals::default();
        world.get_mut::<Outgoing>(player).expect("players have an outbox").push(&Respawn { position: eyes(spawn), state: READY, runtime_entity_id: 0 });
        if let Some(anchor) = anchor {
            crate::sleep::sound_anchor_spent(world, layer, anchor);
        }
        movement::teleport_onto_ground(world, player, spawn);
        if crate::border::is_hardcore(world, layer) {
            crate::join::set_game_mode(world, player, sapling_api::GameMode::Spectator);
        }
        tell(world, player, &EntityEvent { runtime_entity_id: id.0, event_id: EntityEventEventId::Respawn, data: 0, fire_at_position: None });
        tell(world, player, &sapling_data::mob_health_update(id.0, MAX, MAX));
        let tick = world.resource::<Tick>().0;
        let shown = world.get::<inventory::Inventory>(player).map(|inventory| experience::attributes(&id, &inventory.experience, true, tick));
        tell(world, player, &Vitals::default().hunger.told(id.0, tick));
        let mut out = world.get_mut::<Outgoing>(player).expect("players have an outbox");
        out.push(&SetHealth { health: MAX as i32 });
        shown.iter().for_each(|shown| out.push(shown));
        if let Some(anchor) = lapsed {
            crate::sleep::tell_lapsed(world, player, anchor);
        }
        if let Some(anchor) = anchor {
            crate::sleep::spend_anchor(world, layer, anchor);
        }
        // Heard where the body lay, or from another dimension at the place named before one was
        // found (research/captures/bds-hurt.txt, bds-anchor.txt).
        sound(world, player, id.0, "reappeared", if unfound { [0.0, 32767.0, 0.0] } else { body });
        tell(world, player, &whole(world, player, id.0, tick));
    }
}

/// The risen body's flags and its box whole again, as BDS told them.
fn whole(world: &World, player: Entity, id: u64, tick: u64) -> acacia_proto::packets::SetEntityData {
    let mut whole = crate::sleep::box_data(id, false);
    if let Some(pose) = world.get::<movement::Motion>(player).map(|motion| motion.pose) {
        let flags = movement::pose_packet(id, pose, tick).metadata.into_iter().filter(|item| item.key == acacia_proto::types::MetadataDictionaryItemKey::Flags);
        whole.metadata = flags.chain(whole.metadata).collect();
    }
    whole
}

/// What [`leave`] would leave later, left as the player rises before its body went.
fn leave_now(world: &mut World, player: Entity, layer: Entity) {
    let Some(points) = world.get_mut::<Vitals>(player).and_then(|mut vitals| vitals.leaves.take()) else { return };
    let Some(feet) = world.get::<Position>(player).map(|position| position.feet) else { return };
    if points > 0 {
        orbs::spawn(world, layer, feet, points);
    }
}
