//! A painting falls when a player hits it, or when it no longer fits where it hangs (its wall
//! gone), as BDS's did (research/captures/bds-hangers.txt, bds-paintings.txt): it is heard and
//! seen breaking as oak planks at its middle, drops a painting unless a creative player broke it,
//! and goes. BDS looked at a painting's wall about every 100 ticks (one fell 4 ticks after its wall
//! went, another not in 19); each painting here is looked at once in 100 ticks.

use acacia_proto::packets::{LevelEvent, LevelEventEvent};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::GameMode;
use sapling_world::ItemStack;

use super::{Hung, PAINTING, still_fits};
use crate::components::{InLayer, Layer, Mode, NetId, Position, Tick};
use crate::{drops, mobs, view};

const CHECKS: u64 = 100;
/// How far from a player's eyes a painting it hits may be (as a mob's, creatures/attacks.rs).
const REACH: f32 = 6.7;
const CREATIVE_REACH: f32 = 12.0;
/// The block it breaks as, and how far above its middle its item comes out.
const BREAKS_AS: &str = "minecraft:oak_planks";
const DROP_ABOVE: f32 = 1.125;

/// This tick's hits on paintings: each in reach falls. The hits on anything else, left to the rest.
pub(crate) fn struck(world: &mut World, hits: Vec<(Entity, u64)>) -> Vec<(Entity, u64)> {
    let paintings: Vec<(Entity, u64)> = world.query_filtered::<(Entity, &NetId), With<Hung>>().iter(world).map(|(painting, id)| (painting, id.0)).collect();
    hits.into_iter()
        .filter(|&(player, id)| {
            let Some(&(painting, _)) = paintings.iter().find(|(_, painting)| *painting == id) else { return true };
            if in_reach(world, player, painting) && world.get::<Hung>(painting).is_some() {
                let creative = world.get::<Mode>(player).is_some_and(|mode| mode.0 == GameMode::Creative);
                fall(world, painting, !creative);
            }
            false
        })
        .collect()
}

fn in_reach(world: &World, player: Entity, painting: Entity) -> bool {
    let (Some(position), Some(mode), Some(hung)) = (world.get::<Position>(player), world.get::<Mode>(player), world.get::<Hung>(painting)) else { return false };
    let same = world.get::<InLayer>(player).map(|layer| layer.0) == world.get::<InLayer>(painting).map(|layer| layer.0);
    let reach = match mode.0 {
        GameMode::Creative => CREATIVE_REACH,
        GameMode::Spectator => return false,
        _ => REACH,
    };
    let eye = [position.feet[0], position.feet[1] + sapling_data::EYE_HEIGHT, position.feet[2]];
    same && (0..3).map(|axis| (eye[axis] - hung.middle[axis]).powi(2)).sum::<f32>().sqrt() <= reach
}

/// Paintings looked at this tick that no longer fit fall, with their item.
pub(crate) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let due: Vec<(Entity, Entity, Hung)> = world.query::<(Entity, &NetId, &InLayer, &Hung)>().iter(world).filter(|(_, id, ..)| (now + id.0).is_multiple_of(CHECKS)).map(|(painting, _, layer, hung)| (painting, layer.0, *hung)).collect();
    for (painting, layer, hung) in due {
        // A painting outlives its column's unloading by up to a sweep (mobs.rs); unloaded, its wall reads as air.
        if world.get::<Layer>(layer).is_some_and(|level| level.world.is_loaded(hung.home) && !still_fits(&level.world, &hung)) {
            fall(world, painting, true);
        }
    }
}

/// The painting breaks, dropping its item if `drops`, and is gone from its column.
fn fall(world: &mut World, painting: Entity, drops: bool) {
    let (Some(hung), Some(layer)) = (world.get::<Hung>(painting).copied(), world.get::<InLayer>(painting).map(|layer| layer.0)) else { return };
    let [x, y, z] = hung.middle;
    let hash = world.get::<Layer>(layer).and_then(|level| Some(level.world.registry().network_hash(level.world.block_id(BREAKS_AS)?) as i32)).unwrap_or(0);
    let at = || Vec3f { x, y, z };
    let told = [LevelEvent { event: LevelEventEvent::SoundItemframeRemove, position: at(), data: 0 }, LevelEvent { event: LevelEventEvent::ParticleDestroyBlockNoSound, position: at(), data: hash }];
    view::to_viewers(world, layer, hung.middle.map(|c| c.floor() as i32), &told.map(|event| sapling_net::encode(&event)));
    if drops {
        let stack = ItemStack { name: PAINTING.into(), count: 1, damage: 0, block: None, tag: None };
        drops::toss(world, layer, stack, [x, y + DROP_ABOVE, z]);
    }
    forget(world, layer, &hung);
    mobs::remove(world, painting);
}

/// Takes the painting out of what its column keeps.
fn forget(world: &mut World, layer: Entity, hung: &Hung) {
    let Some(mut level) = world.get_mut::<Layer>(layer) else { return };
    let Ok(stored) = sapling_world::stored_actors(level.world.actors(hung.home)) else { return };
    let kept: Vec<u8> = stored.into_iter().filter(|(_, id, _)| *id != Some(hung.unique_id)).flat_map(|(_, _, raw)| raw).collect();
    level.world.set_actors(hung.home, kept);
}
