//! A player's items, health, hunger, bed, place, game mode, tags and `UniqueID` between visits: asked of the first world's storage at
//! login, handed back when the player leaves, every few seconds while something changed, and when the shard stops.

use bevy_ecs::prelude::*;
use sapling_api::GameMode;
use sapling_world::{PlayerData, PlayerLoad};

use super::{ARMOR, ENDER, Held, Inventory, MAIN, Place};
use crate::chunks::UNLOAD_INTERVAL;
use crate::components::{DefaultLayer, Identity, InLayer, Layer, Mode, NetId, Outgoing, Position, Stage, Tick};
use crate::dimensions::{self, ClientDimension};
use crate::experience::{self, Experience};
use crate::handoff::{Peers, ShardMsg};
use crate::sleep::BedSpawn;
use crate::tags::Marks;
use crate::vitals::Vitals;

fn fill(inventory: &mut Inventory, data: PlayerData) {
    for (slot, stack) in data.main.into_iter().take(MAIN).enumerate() {
        inventory.put(Place::Main(slot), stack);
    }
    for (slot, stack) in data.armor.into_iter().take(ARMOR).enumerate() {
        inventory.put(Place::Armor(slot), stack);
    }
    inventory.put(Place::Offhand, data.offhand);
    for (slot, stack) in data.ender.into_iter().take(ENDER).enumerate() {
        inventory.ender[slot] = stack.map(|stack| Held { stack, id: inventory.new_id() });
    }
    inventory.experience = Experience::new(data.level, data.progress);
    inventory.enchant_seed = data.enchant_seed.unwrap_or(inventory.enchant_seed);
    (inventory.selected, inventory.unsaved) = (data.selected, false);
}

/// What storage keeps; stacks on the cursor and in the crafting grid go back among the others.
fn kept(inventory: &Inventory) -> PlayerData {
    let mut inventory = inventory.clone();
    for held in inventory.cursor.take().into_iter().chain(std::mem::take(&mut inventory.grid).into_iter().flatten()) {
        inventory.add(&held.stack);
    }
    let stacks = |slots: Vec<Option<Held>>| slots.into_iter().map(|slot| slot.map(|held| held.stack)).collect();
    let (level, progress) = (inventory.experience.level, inventory.experience.progress);
    let (main, armor, ender, offhand) = (stacks(inventory.main), stacks(inventory.armor), stacks(inventory.ender), inventory.offhand.map(|held| held.stack));
    let enchant_seed = Some(inventory.enchant_seed);
    PlayerData { main, armor, offhand, selected: inventory.selected, ender, level, progress, enchant_seed, ..PlayerData::default() }
}

/// BDS's game type numbers (`PlayerGameMode`).
fn game_type(mode: GameMode) -> i32 {
    match mode {
        GameMode::Survival => 0,
        GameMode::Creative => 1,
        GameMode::Adventure => 2,
        GameMode::Spectator => 6,
    }
}

/// `None` for BDS's "the world's default" (5) and anything unknown.
fn game_mode(stored: i32) -> Option<GameMode> {
    [GameMode::Survival, GameMode::Creative, GameMode::Adventure, GameMode::Spectator].into_iter().find(|mode| game_type(*mode) == stored)
}

/// Gives the players storage answered for their items.
pub(crate) fn load(mut commands: Commands, mut layers: Query<(Entity, &mut Layer)>, mut players: Query<(Entity, &Identity, &mut Inventory, Option<&mut Vitals>, Option<&mut Marks>)>) {
    for (here, mut layer) in &mut layers {
        for (id, answer) in layer.world.take_players() {
            let Some((player, _, mut inventory, vitals, marks)) = players.iter_mut().find(|(_, identity, inventory, ..)| !inventory.loaded && identity.0.uuid.to_string() == id) else { continue };
            (inventory.loaded, inventory.keep) = (true, answer != PlayerLoad::Unreadable);
            inventory.enchant_seed = crate::workstations::new_seed();
            if let PlayerLoad::Stored(mut data) = answer {
                if let Some(mut marks) = marks {
                    marks.restore(data.unique_id, std::mem::take(&mut data.tags));
                }
                if let Some((mut vitals, kept)) = vitals.zip(data.vitals) {
                    vitals.restore(kept);
                }
                if let Some(effects) = crate::vitals::restored_effects(&data.effects) {
                    commands.entity(player).insert(effects);
                }
                if let Some(bed) = data.bed {
                    commands.entity(player).insert(BedSpawn::restored(bed, here));
                }
                if let Some(mode) = data.game_mode.and_then(game_mode) {
                    commands.entity(player).insert(Mode(mode));
                }
                if let Some(feet) = data.position {
                    let (dimension, rotation) = (data.dimension, data.rotation);
                    commands.queue(move |world: &mut World| dimensions::start_in(world, player, dimension, feet, rotation));
                }
                fill(&mut inventory, data);
            }
        }
    }
}

/// Sends each joining player its windows once it has items to show, in BDS's order: inventory,
/// armour, the screen's own slots, offhand, then the slot in hand.
pub(crate) fn send_own(tick: Res<Tick>, mut players: Query<(&NetId, &Stage, &mut Inventory, &mut Outgoing, Option<&mut Vitals>)>) {
    for (id, stage, mut inventory, mut out, vitals) in &mut players {
        if *stage >= Stage::StartGame && inventory.loaded && !inventory.sent {
            inventory.send_all(&mut out, true);
            if inventory.experience != Experience::default() {
                out.push(&experience::attributes(id, &inventory.experience, true, tick.0));
            }
            for told in vitals.map(|mut vitals| vitals.greeting(id.0, tick.0)).unwrap_or_default() {
                out.push(&told);
            }
            inventory.sent = true;
        }
    }
}

/// Where a player was, which way it faced, its dimension and game mode, when it was last kept.
#[derive(Component, Clone, Copy, PartialEq)]
pub(crate) struct KeptPlace(Option<([f32; 3], [f32; 2])>, i32, Option<i32>);

/// Has a player kept if anything of it changed since it last was: by this shard's storage, or
/// by the entry shard's when this is another (`Peers::entry`).
pub(crate) fn save(world: &mut World, player: Entity) {
    let Some(id) = world.get::<Identity>(player).map(|identity| identity.0.uuid.to_string()) else { return };
    let vitals = world.get::<Vitals>(player).copied();
    let effects = crate::vitals::kept_effects(world, player);
    let dimension = world.get::<ClientDimension>(player).map_or(0, |client| client.dimension);
    let first = world.resource::<DefaultLayer>().0;
    let (index, entry) = (world.resource::<Peers>().index, world.resource::<Peers>().entry);
    // The player starts in the entry shard's first world (load): a place in another world is not kept.
    let home = index == entry && world.get::<InLayer>(player).is_some_and(|in_layer| dimensions::of(world, first, dimension) == Some(in_layer.0));
    let place = world.get::<Position>(player).filter(|_| home).map(|position| (position.feet, [position.yaw, position.pitch]));
    let now = KeptPlace(place, dimension, world.get::<Mode>(player).map(|mode| game_type(mode.0)));
    let marks = world.get::<Marks>(player).cloned().unwrap_or_default();
    let stale = vitals.is_some_and(|vitals| vitals.is_unsaved()) || effects.is_some() || marks.unsaved || world.get::<KeptPlace>(player) != Some(&now);
    let Some(mut inventory) = world.get_mut::<Inventory>(player).filter(|inventory| inventory.keep && (inventory.unsaved || stale)) else { return };
    inventory.unsaved = false;
    let KeptPlace(place, _, game_mode) = now;
    let (position, rotation) = (place.map(|place| place.0), place.map(|place| place.1));
    let data = PlayerData { vitals: vitals.map(|vitals| vitals.kept()), effects: effects.unwrap_or_default(), dimension, position, rotation, game_mode, ..kept(&inventory) };
    let data = PlayerData { bed: world.get::<BedSpawn>(player).and_then(|bed| bed.kept(first)), tags: marks.tags, unique_id: Some(marks.unique_id), ..data };
    world.entity_mut(player).insert(now);
    if let Some(mut marks) = world.get_mut::<Marks>(player) {
        marks.unsaved = false;
    }
    if let Some(mut vitals) = world.get_mut::<Vitals>(player) {
        vitals.mark_saved();
    }
    if index == entry {
        return keep_here(world, &id, data);
    }
    if world.resource::<Peers>().tell(entry, ShardMsg::KeepPlayer { id, data: Box::new(data) }).is_err() {
        tracing::warn!("the shard that keeps players is gone: a player's items are not saved");
    }
}

/// Hands a player's items to the storage of this shard's first world.
pub(crate) fn keep_here(world: &mut World, id: &str, data: PlayerData) {
    let layer = world.resource::<DefaultLayer>().0;
    if let Some(mut layer) = world.get_mut::<Layer>(layer) {
        layer.world.save_player(id, data);
    }
}

pub(crate) fn save_all(world: &mut World) {
    let players: Vec<Entity> = world.query_filtered::<Entity, With<Inventory>>().iter(world).collect();
    players.into_iter().for_each(|player| save(world, player));
}

/// On the ticks columns are saved, saves the players something of whom changed.
pub(crate) fn save_changed(world: &mut World) {
    if world.resource::<Tick>().0 % UNLOAD_INTERVAL == 0 {
        save_all(world);
    }
}
