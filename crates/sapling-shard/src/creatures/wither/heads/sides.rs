//! The wither's two side heads, as BDS's step runs them (Linux `WitherBoss` 0xbbbd900, the loop from
//! 0xbbbe1e1): each looks again 10 to 19 ticks on. A head holding a player keeps it while it lives within
//! 30 of the wither, is seen, and is neither creative nor a spectator, shooting it off easy and then
//! looking again 40 to 59 on; else it lets go (told -1). A head holding none takes, without a shot and on
//! any difficulty, a player at random in the wither's box grown 30 across and 8 up and down, never the
//! wither's own prey (10 tries). Off easy each head counts its looks since it last shot; head 1's 17th
//! shoots a blue skull at random near (research/captures/bds-y3wither.txt: 231 to 264 ticks apart).

use bevy_ecs::prelude::*;
use sapling_api::Difficulty;

use super::super::super::hunting;
use super::super::skulls;
use super::{Heads, aim, mouth};
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{InLayer, Position, Settings, in_world};
use crate::vitals;

const REACH: [f32; 2] = [30.0, 8.0];
const TRIES: usize = 10;
const AGAIN: [u64; 2] = [40, 20];
const LOOKS_AGAIN: [u64; 2] = [10, 10];
const IDLE: u32 = 15;
/// A blue skull shot at random goes this far off across, and up or down.
const STRAY: [f32; 2] = [10.0, 5.0];
const PLAYER_SIZE: [f32; 2] = [0.6, 1.8];

fn roll(world: &mut World, sides: u64) -> u64 {
    u64::from(world.get_resource_or_insert_with(Dice::default).roll(sides as u32))
}

/// Where `player` stands if a side head may keep it.
fn kept(world: &World, layer: Entity, body: &Body, player: Entity) -> Option<[f32; 3]> {
    let feet = world.get::<Position>(player)?.feet;
    // BDS measures from the player's position, its eyes.
    let near = hunting::distance([feet[0], feet[1] + sapling_data::EYE_HEIGHT, feet[2]], body.at) <= REACH[0];
    (world.get::<InLayer>(player)?.0 == layer && near && vitals::is_mortal(world, player) && hunting::in_sight(world, layer, body, feet)).then_some(feet)
}

/// A player of `layer` at random whose box meets the wither's grown by [`REACH`], mortal and not `prey`.
fn pick(world: &mut World, layer: Entity, body: &Body, prey: Option<Entity>) -> Option<Entity> {
    let [half, height] = [body.size[0] / 2.0, body.size[1]];
    let meets = |feet: [f32; 3]| {
        let across = |axis: usize| (feet[axis] - body.at[axis]).abs() <= half + REACH[0] + PLAYER_SIZE[0] / 2.0;
        across(0) && across(2) && feet[1] + PLAYER_SIZE[1] >= body.at[1] - REACH[1] && feet[1] <= body.at[1] + height + REACH[1]
    };
    let mut near: Vec<Entity> = in_world(world)
        .into_iter()
        .filter(|player| world.get::<InLayer>(*player).is_some_and(|on| on.0 == layer) && world.get::<Position>(*player).is_some_and(|at| meets(at.feet)))
        .collect();
    for _ in 0..TRIES {
        if near.is_empty() {
            break;
        }
        let picked = near[roll(world, near.len() as u64) as usize];
        if vitals::is_mortal(world, picked) && Some(picked) != prey {
            return Some(picked);
        }
        near.retain(|player| *player != picked);
    }
    None
}

/// A tick of the side heads, the wither facing `yaw`.
pub(super) fn tick(world: &mut World, wither: Entity, layer: Entity, body: &Body, yaw: f32, heads: &mut Heads, now: u64) {
    let fights = matches!(world.resource::<Settings>().0.difficulty, Difficulty::Normal | Difficulty::Hard);
    for head in 1..3 {
        if now < heads.next[head] {
            continue;
        }
        heads.next[head] = now + LOOKS_AGAIN[0] + roll(world, LOOKS_AGAIN[1]);
        if fights {
            let looks = heads.idle[head - 1];
            heads.idle[head - 1] = looks + 1;
            if looks > IDLE {
                heads.idle[head - 1] = 0;
                if head == 1 {
                    let to = {
                        let mut dice = world.get_resource_or_insert_with(Dice::default);
                        let mut off = |reach: f32| (dice.fraction() * 2.0 - 1.0) * reach;
                        [body.at[0] + off(STRAY[0]), body.at[1] + off(STRAY[1]), body.at[2] + off(STRAY[0])]
                    };
                    skulls::fire(world, layer, wither, mouth(body, yaw, head), to, true);
                }
            }
        }
        let Some(player) = heads.targets[head] else {
            heads.targets[head] = pick(world, layer, body, heads.targets[0]);
            continue;
        };
        match kept(world, layer, body, player) {
            Some(feet) if fights => {
                let to = aim(world, player, feet);
                skulls::fire(world, layer, wither, mouth(body, yaw, head), to, false);
                heads.next[head] = now + AGAIN[0] + roll(world, AGAIN[1]);
                heads.idle[head - 1] = 0;
            }
            Some(_) => {}
            None => heads.targets[head] = None,
        }
    }
}
