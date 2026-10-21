//! Summoning another dragon (`tryRespawn` 148272320 and the stage machine, research/dragon-fight-
//! decompile.md 6), as BDS did (research/captures/bds-dragon-again.txt): four end crystals on the
//! killed fight's portal, one at each side, start it; their beams go up, the dragon is heard, then
//! each spike in turn is shown, blown up, built again with a crystal on bedrock, and at last the
//! four crystals blow up and the dragon comes. A crystal lost on the way ends it.

use acacia_proto::packets::LevelSoundEvent;
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, EntityId};

use super::{Crystal, Fight, crystals, spikes};
use crate::body::Body;
use crate::components::{InLayer, Layer, NetId};
use crate::creatures::{Wreck, explode};

/// Where the four crystals stand, from the portal's middle.
const SIDES: [[i32; 3]; 4] = [[0, 1, 3], [-3, 1, 0], [0, 1, -3], [3, 1, 0]];
const SKY: BlockPos = [0, 128, 0];
const PREPARING: u32 = 100;
const HEARD_PREPARING: [u32; 4] = [0, 50, 51, 52];
const HEARD_LATE: std::ops::Range<u32> = 95..100;
/// Ticks each spike takes, and the ticks of it on which it is blown up and built.
const PER_SPIKE: u32 = 40;
const BLOWN: u32 = 38;
const BUILT: u32 = 39;
const SPIKE_BLAST: f32 = 5.0;
const SUMMONING: u32 = 100;
const HEARD_SUMMONING: [std::ops::Range<u32>; 2] = [1..5, 80..100];
const CRYSTAL_BLAST: f32 = 6.0;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Stage {
    Start,
    Preparing,
    Pillars,
    Dragon,
}

/// A summoning under way: its stage, the ticks into it, and the crystals that began it.
pub(super) struct Respawn {
    stage: Stage,
    ticks: u32,
    crystals: Vec<Entity>,
}

/// The crystal standing at each side of the portal at `origin`, all four or none.
fn four(world: &mut World, layer: Entity, [x, y, z]: BlockPos) -> Option<Vec<Entity>> {
    let mut crystals = world.query_filtered::<(Entity, &InLayer, &Body), With<Crystal>>();
    let all: Vec<(Entity, Body)> = crystals.iter(world).filter(|(_, in_layer, _)| in_layer.0 == layer).map(|(crystal, _, body)| (crystal, *body)).collect();
    SIDES
        .iter()
        .map(|[dx, dy, dz]| {
            let (low, high) = ([(x + dx) as f32, (y + dy) as f32, (z + dz) as f32], [(x + dx + 1) as f32, (y + dy + 1) as f32, (z + dz + 1) as f32]);
            all.iter().find(|(_, body)| {
                let (min, max) = body.bounds();
                (0..3).all(|axis| min[axis] < high[axis] && max[axis] > low[axis])
            }).map(|(crystal, _)| *crystal)
        })
        .collect()
}

/// Starts a summoning if the fight's dragon was killed and the four crystals stand; the portal is
/// built unlit at once.
pub(crate) fn try_start(world: &mut World, layer: Entity) {
    let Some(fight) = world.get::<Fight>(layer).filter(|fight| fight.record.killed && fight.respawn.is_none()) else { return };
    let Some(origin) = fight.record.exit_portal else { return };
    let Some(crystals) = four(world, layer, origin) else { return };
    let ids: Vec<EntityId> = crystals.iter().filter_map(|crystal| world.get::<NetId>(*crystal)).map(|id| EntityId(id.0)).collect();
    if let Some(mut fight) = world.get_mut::<Fight>(layer) {
        fight.respawn = Some(Respawn { stage: Stage::Start, ticks: 0, crystals });
        fight.record.respawning = true;
    }
    super::spawn_exit_portal(world, layer, false, false);
    super::keep(world, layer);
    crate::plugins::offer_in(world, layer, sapling_api::DragonRespawnEvent::new(ids));
}

/// The dragon growls over the island, as BDS's summoning is heard, from a player of the End.
fn growl(world: &mut World, layer: Entity) {
    let heard_by = world.query::<(&InLayer, &NetId, &crate::components::Position)>().iter(world).find(|(in_layer, ..)| in_layer.0 == layer).map_or(-1, |(_, id, _)| id.0 as i64);
    let [x, y, z] = SKY.map(|c| c as f32);
    let sound = LevelSoundEvent { sound_id: "mad".into(), position: Vec3f { x, y, z }, extra_data: -1, entity_type: super::DRAGON.into(), is_baby_mob: false, is_global: false, entity_unique_id: heard_by, fire_at_position: None };
    crate::blocks::to_viewers(world, layer, SKY, &[sapling_net::encode(&sound)]);
}

fn beams(world: &mut World, crystals: &[Entity], at: Option<BlockPos>) {
    for &crystal in crystals {
        crystals::set_beam(world, crystal, at);
    }
}

/// A tick of the summoning of `layer`'s fight, if one is under way.
pub(super) fn tick(world: &mut World, layer: Entity) {
    let Some((stage, ticks, crystals)) = world.get::<Fight>(layer).and_then(|fight| fight.respawn.as_ref()).map(|respawn| (respawn.stage, respawn.ticks, respawn.crystals.clone())) else { return };
    if crystals.iter().any(|crystal| world.get::<Crystal>(*crystal).is_none()) {
        return lost(world, layer);
    }
    let next = match stage {
        Stage::Start => {
            beams(world, &crystals, Some(SKY));
            Some(Stage::Preparing)
        }
        Stage::Preparing => {
            if HEARD_PREPARING.contains(&ticks) || HEARD_LATE.contains(&ticks) {
                growl(world, layer);
            }
            (ticks >= PREPARING).then_some(Stage::Pillars)
        }
        Stage::Pillars => pillar(world, layer, &crystals, ticks),
        Stage::Dragon => {
            if ticks == 0 {
                beams(world, &crystals, Some(SKY));
            }
            if HEARD_SUMMONING.iter().any(|heard| heard.contains(&ticks)) {
                growl(world, layer);
            }
            if ticks >= SUMMONING {
                summon(world, layer, &crystals);
                return;
            }
            None
        }
    };
    if let Some(respawn) = world.get_mut::<Fight>(layer).and_then(|fight| fight.into_inner().respawn.as_mut()) {
        match next {
            Some(stage) => (respawn.stage, respawn.ticks) = (stage, 0),
            None => respawn.ticks += 1,
        }
    }
}

/// The spike this tick of the pillars stage is at; the next stage once all are built.
fn pillar(world: &mut World, layer: Entity, crystals: &[Entity], ticks: u32) -> Option<Stage> {
    let spikes = super::spikes(world, layer);
    let (k, s) = ((ticks / PER_SPIKE) as usize, ticks % PER_SPIKE);
    let Some(spike) = spikes.get(k) else { return (s == 0).then_some(Stage::Dragon) };
    let (bedrock, feet) = spikes::top(spike);
    match s {
        0 => beams(world, crystals, Some([spike.x, spike.height + 1, spike.z])),
        BLOWN => {
            let cleared = (spike.x - 10..=spike.x + 10).flat_map(|x| (spike.height - 10..=spike.height + 10).flat_map(move |y| (spike.z - 10..=spike.z + 10).map(move |z| [x, y, z]))).collect::<Vec<_>>();
            let air = world.get::<Layer>(layer).map(|level| level.world.registry().air_id()).unwrap_or(0);
            super::write_later(world, layer, cleared.into_iter().map(|pos| (pos, air)).collect());
            explode(world, None, layer, [spike.x as f32 + 0.5, spike.height as f32, spike.z as f32 + 0.5], SPIKE_BLAST, Wreck::Breaks);
        }
        BUILT => {
            let mut cells = super::resolve(world, layer, &spikes::cells(spike));
            cells.extend(super::resolve(world, layer, &[(bedrock, (super::podium::BEDROCK.0, super::podium::BEDROCK.1.to_owned()))]));
            super::write_later(world, layer, cells);
            let yaw = world.get_mut::<Fight>(layer).map_or(0.0, |mut fight| fight.random.next_float() * 360.0);
            if let Some(crystal) = crystals::spawn(world, layer, feet, yaw, true) {
                crystals::set_beam(world, crystal, Some(SKY));
            }
        }
        _ => {}
    }
    None
}

/// The four crystals blow up, the spikes' crystals let their beams go, and the dragon comes.
fn summon(world: &mut World, layer: Entity, crystals: &[Entity]) {
    if let Some(mut fight) = world.get_mut::<Fight>(layer) {
        (fight.respawn, fight.record.killed, fight.record.respawning) = (None, false, false);
    }
    super::make_dragon(world, layer);
    super::reset_spike_crystals(world, layer);
    for &crystal in crystals {
        crystals::set_beam(world, crystal, None);
        let Some(at) = world.get::<Body>(crystal).map(|body| body.at) else { continue };
        world.entity_mut(crystal).remove::<Crystal>();
        explode(world, Some(crystal), layer, at, CRYSTAL_BLAST, Wreck::Nothing);
        crate::blocks::to_viewers(world, layer, at.map(|c| c.floor() as i32), &[crystals::blast_particles(at, CRYSTAL_BLAST)]);
    }
    super::keep(world, layer);
}

/// A crystal of the summoning was lost: it ends, and the portal opens again.
fn lost(world: &mut World, layer: Entity) {
    if let Some(mut fight) = world.get_mut::<Fight>(layer) {
        (fight.respawn, fight.record.respawning) = (None, false);
    }
    super::reset_spike_crystals(world, layer);
    super::spawn_exit_portal(world, layer, true, false);
    super::keep(world, layer);
}
