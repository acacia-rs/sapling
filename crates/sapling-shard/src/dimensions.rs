//! An overworld's other dimensions. The Nether of a world is another layer of the same shard,
//! linked to it as one [`Realm`]: the members share the overworld's level (time, rules, weather)
//! and a player goes between them as BDS moves one (docs/research/nether-dimension.md).

use acacia_proto::packets::{ChangeDimension, PlayerAction, ServerboundLoadingScreen};
use acacia_proto::types::{Action, BlockCoordinates, Vec3f};
use bevy_ecs::prelude::*;
use sapling_world::OVERWORLD;

use crate::components::{InLayer, Layer, Outgoing, PacketQueues, Position};
use crate::view::View;
use crate::worlds::{Change, Changes, name_of};
use crate::{blobs, chunks, containers, movement, sleep, visibility};

/// `ServerboundLoadingScreen` type of a screen the client closed.
const SCREEN_ENDED: i32 = 2;

/// The layers of one overworld's dimensions by protocol id (0 overworld, 1 nether, 2 end);
/// every member layer holds the same.
#[derive(Component, Clone, Copy, Default)]
pub(crate) struct Realm(pub [Option<Entity>; 3]);

/// The dimension a player's client is in, and the id of its next loading screen: BDS counts a
/// player's dimension changes from 0.
#[derive(Component, Clone, Copy, Default)]
pub(crate) struct ClientDimension {
    pub dimension: i32,
    next_screen: u32,
}

impl ClientDimension {
    pub fn new(dimension: i32) -> Self {
        Self { dimension, next_screen: 0 }
    }
}

/// A dimension change under way: until its columns near the player are sent, then until the
/// client says it is there. Inputs meanwhile are from the loading screen and are not taken.
#[derive(Component)]
pub(crate) struct Arriving {
    pub told_ready: bool,
    /// Through a portal: where it is put is settled once the columns there are loaded
    /// (portals/arrive.rs), and the client is put there, not corrected, after it says it arrived.
    pub trip: Option<Trip>,
    /// Through an end portal: put on the End's platform once the columns there are loaded
    /// (portals/end.rs).
    pub to_platform: bool,
}

/// A trip through a portal on its way: where the traveller stood (its eyes, as BDS keeps a
/// player's place) and in which dimension.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Trip {
    pub entrance: [f32; 3],
    pub from: i32,
}

/// Makes `member` a dimension of the realm of `overworld`.
pub(crate) fn link(world: &mut World, overworld: Entity, member: Entity) {
    let Some(dimension) = world.get::<Layer>(member).map(|layer| layer.world.dimension()) else { return };
    let mut realm = world.get::<Realm>(overworld).copied().unwrap_or_default();
    realm.0[OVERWORLD as usize] = Some(overworld);
    realm.0[dimension.clamp(0, 2) as usize] = Some(member);
    for layer in realm.0.into_iter().flatten() {
        world.entity_mut(layer).insert(realm);
    }
}

/// The overworld of a layer's realm, which holds the realm's level; the layer itself outside one.
pub(crate) fn home(world: &World, layer: Entity) -> Entity {
    world.get::<Realm>(layer).and_then(|realm| realm.0[OVERWORLD as usize]).unwrap_or(layer)
}

/// The layers of a layer's realm, the layer alone outside one.
pub(crate) fn members(world: &World, layer: Entity) -> Vec<Entity> {
    world.get::<Realm>(layer).map_or_else(|| vec![layer], |realm| realm.0.iter().flatten().copied().collect())
}

/// The layer of `dimension` in a layer's realm.
pub(crate) fn of(world: &World, layer: Entity, dimension: i32) -> Option<Entity> {
    let Some(realm) = world.get::<Realm>(layer) else { return (world.get::<Layer>(layer)?.world.dimension() == dimension).then_some(layer) };
    realm.0.get(usize::try_from(dimension).ok()?).copied().flatten()
}

/// Copies each realm's level from its overworld into its other dimensions: they keep no time
/// or rules of their own. Call after the overworlds' clocks advanced.
pub(crate) fn share_levels(world: &mut World) {
    let followers: Vec<(Entity, Entity)> = world.query::<(Entity, &Realm)>().iter(world).filter_map(|(layer, realm)| realm.0[0].filter(|home| *home != layer).map(|home| (layer, home))).collect();
    for (layer, home) in followers {
        let Some(level) = world.get::<Layer>(home).map(|home| home.world.level().clone()) else { continue };
        if let Some(mut follower) = world.get_mut::<Layer>(layer) {
            *follower.world.level_mut() = level;
        }
    }
}

/// Whether the client of `player` shows another dimension than the one of `layer`.
pub(crate) fn crosses(world: &World, player: Entity, layer: Entity) -> bool {
    let shown = world.get::<ClientDimension>(player).map_or(OVERWORLD, |client| client.dimension);
    world.get::<Layer>(layer).is_some_and(|layer| layer.world.dimension() != shown)
}

/// Moves a player into `layer`, a world of another dimension than its client shows, with its
/// feet at `feet`, as BDS moves one: what it saw goes, then ChangeDimension with a new loading
/// screen; its columns follow, and its inputs are not taken until it says it arrived.
pub(crate) fn change(world: &mut World, player: Entity, layer: Entity, feet: [f32; 3]) {
    let screen = world.get_mut::<ClientDimension>(player).map(|mut client| {
        client.next_screen += 1;
        client.next_screen - 1
    });
    if !switch(world, player, layer, feet, feet, screen) {
        return;
    }
    movement::hold(world, player);
    world.entity_mut(player).insert(Arriving { told_ready: false, trip: None, to_platform: false });
    sleep::tell_sleepers(world, home(world, layer));
}

/// Where BDS's ChangeDimension names a respawn place it has not found yet.
const UNFOUND: [f32; 3] = [0.0, 32767.0, 0.0];

/// Moves a dead player of another dimension to its overworld `home` to rise there, as BDS does
/// (research/captures/bds-end-death-none.txt): ChangeDimension to [`UNFOUND`] with no loading
/// screen, the sleepers' count, and at once the word that it is there. Its feet go to `feet`.
pub(crate) fn respawn_home(world: &mut World, player: Entity, home: Entity, feet: [f32; 3]) {
    if !switch(world, player, home, feet, UNFOUND, None) {
        return;
    }
    world.entity_mut(player).remove::<Arriving>();
    movement::release(world, player);
    sleep::tell_sleepers(world, home);
    let id = world.get::<crate::components::NetId>(player).map_or(0, |id| id.0);
    if let Some(mut out) = world.get_mut::<Outgoing>(player) {
        out.push(&ready(id));
    }
}

/// The part of a dimension change both kinds share: the player is in `layer` with its feet at
/// `feet`, what it saw goes and it is sent ChangeDimension naming `told`.
fn switch(world: &mut World, player: Entity, layer: Entity, feet: [f32; 3], told: [f32; 3], screen: Option<u32>) -> bool {
    containers::close(world, player);
    let Some(dimension) = world.get::<Layer>(layer).map(|level| level.world.dimension()) else { return false };
    let Some(mut in_layer) = world.get_mut::<InLayer>(player) else { return false };
    in_layer.0 = layer;
    visibility::forget_seen(world, player);
    if let Some(mut simulated) = world.get_mut::<crate::simulation::Simulated>(player) {
        simulated.restart();
    }
    if let Some(mut client) = world.get_mut::<ClientDimension>(player) {
        client.dimension = dimension;
    }
    if let Ok((mut position, mut view, mut cache, mut backlog, mut out)) = world.query::<(&mut Position, &mut View, &mut blobs::BlobCache, &mut chunks::SectionBacklog, &mut Outgoing)>().get_mut(world, player) {
        // The client drops every column and entity of the old dimension by itself.
        view.leave(sapling_world::ChunkPos::of_block(feet[0].floor() as i32, feet[2].floor() as i32));
        cache.release();
        backlog.clear();
        (position.feet, position.on_ground) = (feet, true);
        out.push(&ChangeDimension { dimension, position: Vec3f { x: told[0], y: told[1], z: told[2] }, respawn: false, loading_screen_id: screen });
    }
    // The dimension is kept with the items (inventory/store.rs).
    if let Some(mut inventory) = world.get_mut::<crate::inventory::Inventory>(player) {
        inventory.unsaved = true;
    }
    true
}

/// [`change`] with the handlers told of the move (`on_world_change`).
pub(crate) fn travel(world: &mut World, player: Entity, layer: Entity, feet: [f32; 3]) {
    let Some(from) = world.get::<InLayer>(player).map(|in_layer| in_layer.0) else { return };
    change(world, player, layer, feet);
    let (from, to) = (name_of(world, from), name_of(world, layer));
    world.resource_mut::<Changes>().0.push(Change::Moved { entity: player, from, to });
}

/// Puts a joining player whose last visit ended in another dimension back there, before its
/// client is told where it is (StartGame); later, or where the world has no such dimension, it
/// stays where it joins.
pub(crate) fn start_in(world: &mut World, player: Entity, dimension: i32, feet: [f32; 3]) {
    let (Some(stage), Some(layer)) = (world.get::<crate::components::Stage>(player).copied(), world.get::<InLayer>(player).map(|in_layer| in_layer.0)) else { return };
    let Some(to) = of(world, layer, dimension).filter(|_| stage < crate::components::Stage::StartGame) else { return };
    world.get_mut::<InLayer>(player).expect("checked above").0 = to;
    if let Some(mut position) = world.get_mut::<Position>(player) {
        position.feet = feet;
    }
    if let Some(mut client) = world.get_mut::<ClientDimension>(player) {
        client.dimension = dimension;
    }
}

/// What BDS tells a client once the columns near it in its new dimension are sent.
pub(crate) fn ready(id: u64) -> PlayerAction {
    let origin = BlockCoordinates { x: 0, y: 0, z: 0 };
    PlayerAction { runtime_entity_id: id, action: Action::DimensionChangeAck, position: origin.clone(), result_position: origin, face: 0 }
}

/// Ends the changes the clients say they finished (their own DimensionChangeAck, or the end of
/// the loading screen): the player is put where the server holds it, as BDS corrects it then.
pub(crate) fn acknowledge(world: &mut World, actions: &[(Entity, PlayerAction)]) {
    let screens = world.resource_mut::<PacketQueues>().take::<ServerboundLoadingScreen>();
    let ended = screens.into_iter().filter(|(_, screen)| screen.r#type == SCREEN_ENDED && screen.loading_screen_id.is_some()).map(|(player, _)| player);
    let acked = actions.iter().filter(|(_, action)| action.action == Action::DimensionChangeAck).map(|(player, _)| *player);
    let arrived: Vec<Entity> = ended.chain(acked).collect();
    for player in arrived {
        let Some(arriving) = world.entity_mut(player).take::<Arriving>() else { continue };
        let Some(feet) = world.get::<Position>(player).map(|position| position.feet) else { continue };
        movement::release(world, player);
        // A portal's traveller was put in place when it was told it was ready.
        if arriving.trip.is_none() && !arriving.to_platform {
            movement::correct(world, player, feet);
        }
    }
}
