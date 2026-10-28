//! Blocks a shot breaks while the world's `projectilesCanBreakBlocks` is on, as BDS's three
//! readers of the rule do (research/captures/bds-shotblocks.txt): a decorated pot shatters into
//! its sherds (0x14714ba10), a chorus flower breaks and leaves itself (0x148b29980), pointed
//! dripstone breaks under a trident faster than 0.6 a tick alone (0x14660dfd0).

use acacia_proto::nbt::Value;
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Item};

use super::loot::{self, Breaker};
use super::{level, send, updates};

const RULE: &str = "projectilesCanBreakBlocks";
const POT: &str = "minecraft:decorated_pot";
const CHORUS_FLOWER: &str = "minecraft:chorus_flower";
const DRIPSTONE: &str = "minecraft:pointed_dripstone";
const TRIDENT: &str = "minecraft:thrown_trident";
const TRIDENT_SPEED: f32 = 0.6;
/// A pot's four faces, each a sherd's name; a plain face is a brick.
const SHERDS: &str = "sherds";
const BRICK: &str = "minecraft:brick";
const SHATTER: &str = "shatter_pot";

/// The shot of entity type `kind` moving at `motion` has met the block at `pos`.
pub(super) fn shot(world: &mut World, layer: Entity, pos: BlockPos, kind: &str, motion: [f32; 3]) {
    let Some(name) = level(world, layer).filter(|level| level.in_height(pos[1])).and_then(|level| level.registry().get(level.block(pos[0], pos[1], pos[2]))).map(|state| state.name) else { return };
    let is_fast_trident = kind == TRIDENT && motion.iter().map(|c| c * c).sum::<f32>().sqrt() > TRIDENT_SPEED;
    let breaks = name == POT || name == CHORUS_FLOWER || name == DRIPSTONE && is_fast_trident;
    if !breaks || !crate::rules::on(world, layer, RULE) {
        return;
    }
    if name == POT {
        shatter(world, layer, pos);
    } else {
        updates::pop_by(world, layer, pos, &Breaker { by_entity: true, ..Breaker::UNAIDED });
        updates::settle(world, layer, pos);
    }
}

/// What a pot whose entity `kept` its faces and an item leaves when it shatters.
fn shards(kept: Option<&Value>) -> Vec<Item> {
    let faces: Vec<&str> = match kept.and_then(|kept| kept.get(SHERDS)) {
        Some(Value::List(list)) => list.items.iter().map(|face| if let Value::String(name) = face { name.as_str() } else { "" }).collect(),
        _ => Vec::new(),
    };
    (0..4).map(|face| faces.get(face).copied().filter(|name| !name.is_empty()).unwrap_or(BRICK)).map(|name| Item::new(name, 1)).collect()
}

/// The pot at `pos` breaks into its four faces and what it held, and is heard shattering from its
/// corner (bds-shotblocks.txt).
fn shatter(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(level) = level(world, layer) else { return };
    let (registry, id) = (level.registry(), level.block(pos[0], pos[1], pos[2]));
    let (air, particles) = (registry.air_id(), send::destroy_particles(registry, pos, id));
    let heard = send::sound_at(SHATTER, registry, Vec3f { x: pos[0] as f32, y: pos[1] as f32, z: pos[2] as f32 }, id);
    let mut left = shards(level.block_entity(pos).map(|kept| &kept.nbt));
    let mut tags = loot::add_entity_loot(world, layer, pos, &mut left);
    if send::set(world, layer, pos, air) {
        send::to_viewers(world, layer, pos, &[particles]);
        loot::spill_tagged(world, layer, pos, &left, &mut tags);
        send::to_viewers(world, layer, pos, &[heard]);
        updates::settle(world, layer, pos);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use acacia_proto::nbt::List;

    #[test]
    fn a_plain_face_is_a_brick() {
        let sherds = Value::List(List { tag: 8, items: vec![Value::String("minecraft:angler_pottery_sherd".into()), Value::String("".into())] });
        let kept = Value::Compound([(acacia_proto::nbt::Str::from(SHERDS), sherds)].into_iter().collect());
        let names = |kept| shards(kept).into_iter().map(|item| item.name).collect::<Vec<_>>();
        assert_eq!(names(Some(&kept)), ["minecraft:angler_pottery_sherd", BRICK, BRICK, BRICK]);
        assert_eq!(names(None), [BRICK; 4]);
    }
}
