//! A trial spawner turning ominous (docs/research/spawners.md "Ominous", the Java game's
//! `TrialSpawner.tryDetectPlayers`): a player it may take on with trial omen, else with bad omen,
//! makes it so. Bad omen turns to trial omen for 18000 ticks a level; the spawner shows LevelEvent
//! 3616, loses its wave's mobs and starts over on its ominous config.
// TODO: the ominous item spawners an active one drops over its players are not built.

use acacia_proto::packets::LevelEventEvent;
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Effect};

use super::{Trial, configs};
use crate::spawners::{Near, centre, distance, set_state};
use crate::tags::UniqueId;
use crate::vitals::{self, effects};

const TRIAL_OMEN_PER_LEVEL: u32 = 18000;

/// Takes the mobs of the `UniqueID`s `gone` out of the world.
fn discard(world: &mut World, gone: &[i64]) {
    let mobs: Vec<Entity> = world.query::<(Entity, &UniqueId)>().iter(world).filter(|(_, id)| gone.contains(&id.0)).map(|(mob, _)| mob).collect();
    mobs.into_iter().for_each(|mob| crate::mobs::remove(world, mob));
}

/// Makes the spawner ominous if an omened player in `near` is in its range; whether it did.
pub(super) fn awaken(world: &mut World, layer: Entity, pos: BlockPos, near: &[Near], trial: &mut Trial, now: i64) -> bool {
    if trial.ominous {
        return false;
    }
    let corner = pos.map(|c| c as f32);
    let in_range: Vec<Entity> = near.iter().filter(|player| vitals::is_mortal(world, player.entity) && distance(player.eyes(), corner) <= trial.range).map(|player| player.entity).collect();
    let has = |effect| in_range.iter().copied().find(|player| effects::on(world, *player, effect).is_some());
    let Some(player) = has(Effect::TrialOmen).or_else(|| has(Effect::BadOmen)) else { return false };
    if let Some((amplifier, _)) = effects::on(world, player, Effect::BadOmen) {
        effects::end(world, player, Effect::BadOmen);
        vitals::give_effect(world, player, Effect::TrialOmen, TRIAL_OMEN_PER_LEVEL * (u32::from(amplifier) + 1), 0, true);
    }
    let gone: Vec<i64> = std::mem::take(&mut trial.mobs);
    discard(world, &gone);
    trial.ominous = true;
    trial.config = configs::of(&trial.fields, true);
    trial.total_spawned = 0;
    trial.next_spawn = now + trial.config.ticks_between;
    set_state(world, layer, pos, "ominous", "1");
    super::event(world, layer, pos, LevelEventEvent::OminousItemSpawner, centre(pos), 1);
    true
}
