//! The ender dragon fight of an End, as BDS runs it (research/dragon-fight-decompile.md,
//! docs/research/end-dimension.md "The dragon fight"): with a player within 200 of the island's
//! middle it builds the exit portal and spawns the dragon; once the dragon is dead the portal
//! opens, the first one's egg is laid and a gateway opens; four end crystals on the portal summon
//! another (respawn.rs). The fight is kept with the world (`Level::dragon_fight`, BDS's `TheEnd`).
//! Blocks the fight itself writes reach clients a tick later, as BDS's did.

mod boss_bar;
mod count;
mod crystals;
mod death;
mod exits;
mod flight;
mod fly;
mod gateway_shape;
mod gateways;
mod hooks;
mod parts;
mod podium;
mod random;
mod respawn;
mod spawn;
mod spikes;
mod wire;

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_world::{ChunkPos, DragonFightRecord, END};

use crate::body::Body;
use crate::components::{InLayer, Layer, Position, in_world};

pub(crate) use crystals::{Crystal, KIND as CRYSTAL, burst, from_storage as crystal_from_storage, place as place_crystal};
pub(crate) use hooks::{blasted, body_harm, describe, dies, hurt, is_dying, takes_blast};
pub(crate) use spawn::{Dragon, KIND as DRAGON, adopt};

/// A fight runs while a player is in this cube round the island's middle (`-200..201`).
const NEAR: f32 = 200.0;
/// Ticks a fight that made a dragon waits for it to come back with its column before making another.
const KEPT_DRAGON_WAIT: u32 = 200;
const PORTAL_SCAN: u32 = 100;
/// The columns that must be loaded for it to run.
const CENTRE: [(i32, i32); 4] = [(-1, -1), (-1, 0), (0, -1), (0, 0)];

/// The fight of one End layer: what is kept of it, its dragon, and what waits for the next tick.
#[derive(Component)]
pub(crate) struct Fight {
    record: DragonFightRecord,
    dragon: Option<Entity>,
    /// Writes to show next tick.
    pending: Vec<Vec<(BlockPos, u32)>>,
    gateways: gateways::Jobs,
    random: random::Random,
    /// Ticks spent waiting for a kept dragon to come in with its column.
    waited: u32,
    respawn: Option<respawn::Respawn>,
    portal_scan: u32,
    /// Crystals on the spikes (count.rs), and ticks since they were counted.
    crystals: u32,
    crystal_count: u32,
}

fn seed(world: &World, layer: Entity) -> i64 {
    world.get::<Layer>(crate::dimensions::home(world, layer)).map_or(0, |level| level.world.level().seed)
}

/// Keeps the fight's record with the realm's level, which saves it.
fn keep(world: &mut World, layer: Entity) {
    let Some(record) = world.get::<Fight>(layer).map(|fight| fight.record.clone()) else { return };
    let home = crate::dimensions::home(world, layer);
    if let Some(mut level) = world.get_mut::<Layer>(home) {
        level.world.level_mut().dragon_fight = Some(record);
    }
    crate::dimensions::share_levels(world);
}

fn fight_of(world: &mut World, layer: Entity) -> Option<Mut<'_, Fight>> {
    if world.get::<Fight>(layer).is_none() {
        let home = crate::dimensions::home(world, layer);
        let kept = world.get::<Layer>(home)?.world.level().dragon_fight.clone();
        let seed = seed(world, layer);
        let record = kept.unwrap_or_else(|| DragonFightRecord { dragon: -1, gateways: gateways::order(seed), ..Default::default() });
        world.entity_mut(layer).insert(Fight { record, dragon: None, pending: Vec::new(), gateways: gateways::Jobs::default(), random: random::Random::new(seed as u64), waited: 0, respawn: None, portal_scan: 0, crystals: 0, crystal_count: 0 });
    }
    world.get_mut::<Fight>(layer)
}

/// Whether a player of `layer` is near enough to the island's middle for the fight to run.
fn players_near(world: &World, layer: Entity) -> bool {
    let near = |player: &Entity| world.get::<InLayer>(*player).is_some_and(|in_layer| in_layer.0 == layer) && world.get::<Position>(*player).is_some_and(|at| at.feet.iter().all(|c| (-NEAR..NEAR + 1.0).contains(c)));
    in_world(world).iter().any(near)
}

fn centre_loaded(world: &mut World, layer: Entity) -> bool {
    let Some(mut level) = world.get_mut::<Layer>(layer) else { return false };
    CENTRE.iter().fold(true, |all, &(x, z)| level.world.request_urgent(ChunkPos::new(x, z)) && all)
}

/// A tick of every End's fight and of what is in it.
pub(crate) fn tick(world: &mut World) {
    crystals::tick(world);
    let layers: Vec<Entity> = world.query::<(Entity, &Layer)>().iter(world).filter(|(_, layer)| layer.world.dimension() == END).map(|(entity, _)| entity).collect();
    for layer in layers {
        if fight_of(world, layer).is_none() {
            continue;
        }
        show_pending(world, layer);
        fly::tick(world, layer);
        boss_bar::tick(world, layer);
        death::tick(world, layer);
        gateways::tick(world, layer);
        if !players_near(world, layer) || !centre_loaded(world, layer) {
            continue;
        }
        let summoning = world.get::<Fight>(layer).is_some_and(|fight| fight.respawn.is_some());
        if !summoning {
            find_or_make_dragon(world, layer);
            look_for_portal(world, layer);
            count::now_and_then(world, layer);
        }
        // A summoning a world folder kept has no crystals listed: they are looked for again.
        if world.get::<Fight>(layer).is_some_and(|fight| fight.record.respawning && fight.respawn.is_none()) {
            respawn::try_start(world, layer);
        }
        respawn::tick(world, layer);
    }
}

/// An end crystal summoned at `feet`, showing its base as a crystal does unless put down.
pub(crate) fn summon_crystal(world: &mut World, layer: Entity, feet: [f32; 3]) -> Option<Entity> {
    crystals::spawn(world, layer, feet, 0.0, true)
}

/// An end crystal was put in `layer`: in the End, a summoning may begin.
pub(crate) fn crystal_placed(world: &mut World, layer: Entity) {
    if world.get::<Layer>(layer).is_some_and(|level| level.world.dimension() == END) {
        respawn::try_start(world, layer);
    }
}

fn show_pending(world: &mut World, layer: Entity) {
    let pending = world.get_mut::<Fight>(layer).map(|mut fight| std::mem::take(&mut fight.pending)).unwrap_or_default();
    for cells in pending {
        crate::blocks::set_sectioned(world, layer, &cells);
        let mut columns: Vec<ChunkPos> = cells.iter().map(|&([x, _, z], _)| ChunkPos::of_block(x, z)).collect();
        columns.sort();
        columns.dedup();
        // The fight starts while its players' columns still stream in (research/captures/sapling-dragon-again.txt lost half the podium).
        crate::view::resend_to_streaming(world, layer, &columns);
    }
}

/// Writes `cells` next tick, as BDS's fight showed its own writes.
fn write_later(world: &mut World, layer: Entity, cells: Vec<(BlockPos, u32)>) {
    if let Some(mut fight) = world.get_mut::<Fight>(layer) {
        fight.pending.push(cells);
    }
}

/// The exit portal, lit or not, where the fight keeps it (found the first time).
fn spawn_exit_portal(world: &mut World, layer: Entity, active: bool, later: bool) -> Option<BlockPos> {
    let kept = world.get::<Fight>(layer)?.record.exit_portal;
    let origin = kept.or_else(|| podium::origin(world, layer))?;
    if let Some(mut fight) = world.get_mut::<Fight>(layer) {
        fight.record.exit_portal = Some(origin);
    }
    let cells = podium::ids(world, layer, origin, active);
    match later {
        true => write_later(world, layer, cells),
        false => crate::blocks::set_sectioned(world, layer, &cells),
    }
    Some(origin)
}

/// The fight's dragon, unless it was killed: the one it knows, else a dragon near, else none
/// while the one it made is kept unloaded, else a new one and the unlit portal.
fn find_or_make_dragon(world: &mut World, layer: Entity) {
    let Some(fight) = world.get::<Fight>(layer) else { return };
    if fight.record.killed || fight.dragon.is_some_and(|dragon| world.get::<Dragon>(dragon).is_some()) {
        return;
    }
    if let Some(found) = spawn::find(world, layer, NEAR) {
        world.get_mut::<Fight>(layer).expect("read above").dragon = Some(found);
        return;
    }
    // BDS waits while the level knows the dragon's id from a column not loaded; this waits a while.
    if let Some(mut fight) = world.get_mut::<Fight>(layer).filter(|fight| fight.record.spawned && fight.record.dragon != -1 && fight.waited < KEPT_DRAGON_WAIT) {
        fight.waited += 1;
        return;
    }
    make_dragon(world, layer);
    spawn_exit_portal(world, layer, false, true);
    keep(world, layer);
}

/// Every [`PORTAL_SCAN`] ticks of a fight whose dragon was killed, the lit portal is built again
/// if it is gone (BDS looks for its end portal blocks).
fn look_for_portal(world: &mut World, layer: Entity) {
    let Some(mut fight) = world.get_mut::<Fight>(layer).filter(|fight| fight.record.killed) else { return };
    fight.portal_scan += 1;
    if fight.portal_scan < PORTAL_SCAN {
        return;
    }
    fight.portal_scan = 0;
    let Some([x, y, z]) = fight.record.exit_portal else { return };
    let lit = world.get::<Layer>(layer).is_some_and(|level| level.world.registry().get(level.world.block(x + 1, y, z)).is_some_and(|state| state.name == "minecraft:end_portal"));
    if !lit {
        spawn_exit_portal(world, layer, true, true);
    }
}

/// A new dragon for the fight (`_createNewDragon` 148272c10); spikes that lost obsidian are built
/// again, without crystals.
fn make_dragon(world: &mut World, layer: Entity) {
    let Some(mut random) = world.get::<Fight>(layer).map(|fight| fight.random) else { return };
    let made = spawn::new(world, layer, &mut random);
    if let Some(mut fight) = world.get_mut::<Fight>(layer) {
        fight.random = random;
        if let Some((dragon, stored)) = made {
            (fight.dragon, fight.record.spawned, fight.record.dragon) = (Some(dragon), true, stored);
        }
    }
    let obsidian = world.get::<Layer>(layer).and_then(|level| level.world.registry().find("minecraft:obsidian", ""));
    for spike in spikes(world, layer) {
        let damaged = world.get::<Layer>(layer).is_some_and(|level| spikes::is_damaged(&spike, |[x, y, z]| Some(level.world.block(x, y, z)) == obsidian));
        if damaged {
            let cells = resolve(world, layer, &spikes::cells(&spike));
            write_later(world, layer, cells);
        }
    }
}

/// The spikes' crystals let their beams go (`resetSpikeCrystals` 1482754c0).
fn reset_spike_crystals(world: &mut World, layer: Entity) {
    let spikes = spikes(world, layer);
    let on_spikes: Vec<Entity> = world.query::<(Entity, &InLayer, &Body, &Crystal)>().iter(world).filter(|(_, in_layer, body, _)| in_layer.0 == layer && crystals::on_spike(&spikes, body.at)).map(|(crystal, ..)| crystal).collect();
    for crystal in on_spikes {
        crystals::set_beam(world, crystal, None);
    }
}

/// Named blocks as `layer`'s runtime ids.
fn resolve(world: &World, layer: Entity, named: &[(BlockPos, (&'static str, String))]) -> Vec<(BlockPos, u32)> {
    let Some(registry) = world.get::<Layer>(layer).map(|level| level.world.registry()) else { return Vec::new() };
    named.iter().filter_map(|(pos, (name, state))| Some((*pos, registry.find(name, state)?))).collect()
}

/// Called as a dragon's death ends (BDS's kill callback): the portal opens, a gateway is due and
/// the first dragon's egg is laid.
fn dragon_gone(world: &mut World, layer: Entity) {
    let Some(origin) = spawn_exit_portal(world, layer, true, false) else { return };
    let seed = seed(world, layer);
    let next = world.get_mut::<Fight>(layer).and_then(|mut fight| {
        if fight.record.gateways.is_empty() {
            fight.record.gateways = gateways::order(seed);
        }
        fight.record.gateways.pop()
    });
    if let Some(index) = next {
        gateways::queue(world, layer, index);
    }
    if !world.get::<Fight>(layer).is_some_and(|fight| fight.record.previously_killed) {
        let egg = podium::egg(world, layer, origin);
        crate::blocks::set_sectioned(world, layer, &egg);
    }
    if let Some(mut fight) = world.get_mut::<Fight>(layer) {
        (fight.record.killed, fight.record.previously_killed, fight.dragon) = (true, true, None);
    }
    keep(world, layer);
}

/// The spikes of `layer`'s world, in BDS's order.
fn spikes(world: &World, layer: Entity) -> [sapling_worldgen::end::Spike; 10] {
    sapling_worldgen::end::spikes(seed(world, layer) as u32)
}
