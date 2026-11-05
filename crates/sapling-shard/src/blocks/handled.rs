//! What a click does to a block that needs nothing in the hand: a cake is bitten, redstone ore
//! lights, a berry bush and cave vines are picked, a note block plays, a full composter empties,
//! a dragon egg is gone elsewhere, a charged respawn anchor bursts outside the nether and is made
//! the clicker's spawn in it (docs/research/block-uses.md, sleep/spawn.rs).

use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Item};

use acacia_proto::packets::{LevelEvent, LevelEventEvent};

use super::{Dice, level, send, with_property};
use crate::components::Position;
use crate::{drops, inventory, vitals};

const CAKE_BITES: u8 = 6;
/// What a slice feeds: hunger and saturation.
const SLICE: [f32; 2] = [2.0, 0.4];
/// The lowest growth with berries on it, and the growth a picked bush is left at.
const RIPE: u8 = 2;
const PICKED: &str = "1";
/// Below the eyes, where BDS put the burp.
const MOUTH: f32 = sapling_data::EYE_HEIGHT - 0.1;
/// A dragon egg jumps up to this far across and up or down, each step the difference of two rolls
/// (BDS's 27 jumps: 8 of 54 lengths across were 10 or more, as two rolls give), and tries this
/// often for a free cell.
const EGG_REACH: [i32; 2] = [15, 7];
const EGG_TRIES: usize = 64;
/// A charged respawn anchor clicked outside the nether bursts with this power.
const ANCHOR_POWER: f32 = 5.0;
const ANCHOR_FULL: u8 = 4;
const ANCHORS_EXPLODE: &str = "respawnBlocksExplode";
const NETHER: i32 = 1;

/// What tells of an egg's jump by `by`: each length in a byte, and a bit for each that is positive
/// (research/captures/bds-n2egg.txt, against where the egg was put).
fn egg_jump([dx, dy, dz]: [i32; 3]) -> i32 {
    let sign = |by: i32, bit: i32| i32::from(by > 0) << bit;
    dx.abs() << 16 | dy.abs() << 8 | dz.abs() | sign(dx, 24) | sign(dy, 25) | sign(dz, 26)
}

fn property(properties: &str, key: &str) -> Option<u8> {
    properties.split(',').filter_map(|pair| pair.split_once('=')).find(|(name, _)| *name == key).and_then(|(_, value)| value.parse().ok())
}

/// Whether a click with `held` on the block `id` uses it before anything is built on it: a
/// charged respawn anchor, but for glowstone that still charges it (bds-t3uses.txt: dirt on a
/// charged anchor burst it outside the Nether).
pub(super) fn takes_click(registry: &sapling_world::BlockRegistry, id: u32, held: Option<&str>) -> bool {
    let Some(state) = registry.get(id).filter(|state| state.name == "minecraft:respawn_anchor") else { return false };
    let charge = property(state.properties, "respawn_anchor_charge").unwrap_or(0);
    charge > 0 && !(held == Some("minecraft:glowstone") && charge < ANCHOR_FULL)
}

/// A click on the block `name` at `pos`: false when it is none of these.
pub(super) fn use_by_hand(world: &mut World, player: Entity, layer: Entity, pos: BlockPos, name: &str) -> bool {
    let [x, y, z] = pos;
    let Some(level) = level(world, layer) else { return false };
    let (registry, id) = (level.registry(), level.block(x, y, z));
    let Some(properties) = registry.get(id).map(|state| state.properties) else { return false };
    match name {
        "minecraft:noteblock" => {
            super::notes::tuned(world, layer, pos);
            true
        }
        super::composter::COMPOSTER => super::composter::empty(world, layer, pos),
        "minecraft:cake" => {
            let Some(bitten) = property(properties, "bite_counter") else { return false };
            let left = if bitten >= CAKE_BITES { Some(registry.air_id()) } else { with_property(registry, id, "bite_counter", &(bitten + 1).to_string()) };
            if let (Some(left), true) = (left, vitals::bite(world, player, SLICE)) {
                send::set(world, layer, pos, left);
                if let Some([x, y, z]) = world.get::<Position>(player).map(|position| position.feet) {
                    send::to_viewers(world, layer, pos, &[send::noise("burp", Vec3f { x, y: y + MOUTH, z }, -1)]);
                }
            }
            true
        }
        "minecraft:redstone_ore" | "minecraft:deepslate_redstone_ore" => {
            if let Some(lit) = registry.find(&name.replace("minecraft:", "minecraft:lit_"), properties) {
                send::set(world, layer, pos, lit);
            }
            // Not a use: the hand's block is still put against it.
            false
        }
        "minecraft:sweet_berry_bush" => {
            let Some(growth) = property(properties, "growth").filter(|growth| *growth >= RIPE) else { return false };
            let (Some(picked), Some(berry)) = (with_property(registry, id, "growth", PICKED), inventory::stored(&Item::new("minecraft:sweet_berries".to_owned(), 1), registry)) else { return false };
            send::set(world, layer, pos, picked);
            // BDS gave two off a full bush; one off a half-grown one is the Java game's least.
            for _ in 0..growth - 1 {
                drops::spill(world, layer, berry.clone(), pos);
            }
            let at = Vec3f { x: x as f32 + 0.5, y: y as f32, z: z as f32 + 0.5 };
            send::to_viewers(world, layer, pos, &[send::noise("block.sweet_berry_bush.pick", at, -1)]);
            true
        }
        "minecraft:cave_vines_head_with_berries" | "minecraft:cave_vines_body_with_berries" => {
            let (Some(picked), Some(berries)) = (registry.find("minecraft:cave_vines", properties), inventory::stored(&Item::new("minecraft:glow_berries".to_owned(), 1), registry)) else { return false };
            send::set(world, layer, pos, picked);
            drops::spill(world, layer, berries, pos);
            send::to_viewers(world, layer, pos, &[send::noise("pick_berries.cave_vines", Vec3f { x: x as f32, y: y as f32, z: z as f32 }, -1)]);
            true
        }
        "minecraft:respawn_anchor" => {
            if !property(properties, "respawn_anchor_charge").is_some_and(|charge| charge > 0) {
                return false;
            }
            if level.dimension() == NETHER {
                return crate::sleep::set_anchor_spawn(world, player, layer, pos);
            }
            // Still a use: BDS resent nothing for the click (bds-p2last.txt).
            if !level.level().rules.on(ANCHORS_EXPLODE) {
                return true;
            }
            let air = registry.air_id();
            send::set(world, layer, pos, air);
            crate::creatures::explode_burning(world, layer, pos.map(|c| c as f32 + 0.5), ANCHOR_POWER, crate::creatures::Wreck::Decays);
            true
        }
        "minecraft:dragon_egg" => {
            let air = registry.air_id();
            let rolls: Vec<[i32; 3]> = {
                let mut dice = world.get_resource_or_insert_with(Dice::default);
                let mut step = |reach: i32| dice.roll(reach as u32 + 1) as i32 - dice.roll(reach as u32 + 1) as i32;
                let [across, up] = EGG_REACH;
                (0..EGG_TRIES).map(|_| [step(across), step(up), step(across)]).collect()
            };
            let Some(level) = super::level(world, layer) else { return false };
            let is_free = |[dx, dy, dz]: &[i32; 3]| level.in_height(y + dy) && level.is_loaded(send::column([x + dx, y + dy, z + dz])) && level.block(x + dx, y + dy, z + dz) == air;
            let Some(by) = rolls.into_iter().find(|by| *by != [0; 3] && is_free(by)) else { return true };
            let jump = LevelEvent { event: LevelEventEvent::ParticleDragonEggTeleport, position: Vec3f { x: x as f32, y: y as f32, z: z as f32 }, data: egg_jump(by) };
            send::to_viewers(world, layer, pos, &[sapling_net::encode(&jump)]);
            send::set(world, layer, pos, air);
            send::set(world, layer, [x + by[0], y + by[1], z + by[2]], id);
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_eggs_jump_is_told_as_bds_told_fifteen_west_and_nine_south() {
        assert_eq!(egg_jump([-15, 0, 9]), 68_091_913);
        assert_eq!(egg_jump([2, 4, -1]), 50_463_745);
    }
}
