//! A shut fence gate opened by hand from the side it faces turns to swing away from its user, as
//! BDS's did: facing west, opened by a player looking east, it opened facing east; across the
//! player's look it kept its way (research/captures/bds-n2gates.txt).

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::{level, send};
use crate::components::Position;

const FACING: &str = "minecraft:cardinal_direction";
/// The ways a look's yaw falls in, by quarter: 0 south, 90 west, 180 north, 270 east.
const WAYS: [&str; 4] = ["south", "west", "north", "east"];

fn opposite(way: &str) -> &str {
    WAYS[(WAYS.iter().position(|w| *w == way).unwrap_or(0) + 2) % 4]
}

/// The way a look of `yaw` degrees goes.
fn looks(yaw: f32) -> &'static str {
    WAYS[((yaw / 90.0).round() as i32).rem_euclid(4) as usize]
}

/// Turns the shut gate at `pos` to `player`'s way when it faces the player.
pub(super) fn turn_to_user(world: &mut World, player: Entity, layer: Entity, pos: BlockPos) {
    let Some(yaw) = world.get::<Position>(player).map(|position| position.yaw) else { return };
    let Some(level) = level(world, layer) else { return };
    let (registry, id) = (level.registry(), level.block(pos[0], pos[1], pos[2]));
    let Some(state) = registry.get(id) else { return };
    let get = |key: &str| state.properties.split(',').filter_map(|pair| pair.split_once('=')).find(|(name, _)| *name == key).map(|(_, value)| value);
    let way = looks(yaw);
    if get("open_bit") != Some("0") || get(FACING) != Some(opposite(way)) {
        return;
    }
    if let Some(turned) = super::with_property(registry, id, FACING, way) {
        send::set_quietly(world, layer, pos, turned);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_look_east_is_yaw_minus_90() {
        assert_eq!([looks(-90.0), looks(90.0), looks(180.0), looks(-179.0), looks(10.0)], ["east", "west", "north", "north", "south"]);
        assert_eq!(opposite("east"), "west");
    }
}
