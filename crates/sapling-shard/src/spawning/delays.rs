//! A spawn rule's gates that are not its spot's (`sapling_data::Gates`), as `_spawnMobCluster`
//! (1415ed230) asks them: the world's age and the rule's wait before the cluster, a chance roll
//! once the herd's spot passed, and a new wait once any of it spawned
//! (docs/research/structure-spawns.md "Patrols").
// TODO: the `/mobevent` switches live in the sapling crate's MobEvents; every event is taken as on.
// TODO: the waits are not kept with the world.

use std::collections::HashMap;

use bevy_ecs::prelude::*;
use sapling_data::{Delay, Gates};

use super::spots::Found;
use super::{SpawnDice, Watcher};
use crate::components::Layer;

const TICKS_A_SECOND: i64 = 20;

/// Each wait's end, in seconds of the world's age, by its id.
#[derive(Resource, Default)]
struct Waits(HashMap<&'static str, i64>);

fn seconds(world: &World, layer: Entity) -> i64 {
    world.get::<Layer>(layer).map_or(0, |layer| layer.world.level().age / TICKS_A_SECOND)
}

fn roll(world: &mut World, sides: u32) -> u32 {
    world.get_resource_or_insert_with(SpawnDice::default).0.roll(sides)
}

/// When the wait ends that starts now. BDS keeps one per id from the rules' load; one first asked
/// for is taken to start then [U].
fn wait_from(world: &mut World, now: i64, delay: &Delay) -> i64 {
    now + i64::from(delay.least + roll(world, delay.most.saturating_sub(delay.least).max(1)))
}

fn wait_end(world: &mut World, now: i64, delay: &Delay) -> i64 {
    if let Some(end) = world.get_resource::<Waits>().and_then(|waits| waits.0.get(delay.id)) {
        return *end;
    }
    let end = wait_from(world, now, delay);
    world.get_resource_or_insert_with(Waits::default).0.insert(delay.id, end);
    end
}

/// Whether the world is old enough, the rule's wait is over and no player near `found` is in a village.
pub(super) fn open(world: &mut World, layer: Entity, gates: &Gates, found: &Found, watchers: &[Watcher]) -> bool {
    if gates.is_open() {
        return true;
    }
    let now = seconds(world, layer);
    if gates.age.is_some_and(|age| now < i64::from(age)) || gates.delay.is_some_and(|delay| now <= wait_end(world, now, &delay)) {
        return false;
    }
    let Some(village) = gates.village else { return true };
    let near: Vec<[f32; 3]> = watchers.iter().filter(|watcher| watcher.layer == layer && watcher.distance(found.feet()) <= village.distance).map(|watcher| watcher.feet).collect();
    // Sapling's village is a bell or bed within 48 of the player, without BDS's bounds and tolerance.
    !near.into_iter().any(|feet| crate::creatures::raids::village_centre(world, layer, feet).is_some())
}

/// The wait's chance roll once a herd's spot passed: on a miss the wait starts again.
pub(super) fn lucky(world: &mut World, layer: Entity, gates: &Gates) -> bool {
    let Some(delay) = gates.delay.filter(|delay| delay.chance < 100) else { return true };
    if roll(world, 100) <= delay.chance {
        return true;
    }
    restart(world, layer, gates);
    false
}

/// The rule spawned: its wait starts again.
pub(super) fn restart(world: &mut World, layer: Entity, gates: &Gates) {
    let Some(delay) = gates.delay else { return };
    let now = seconds(world, layer);
    let end = wait_from(world, now, &delay);
    world.get_resource_or_insert_with(Waits::default).0.insert(delay.id, end);
}

#[cfg(test)]
mod tests {
    use super::*;

    const DELAY: Delay = Delay { least: 600, most: 660, chance: 20, id: "patrol" };

    #[test]
    fn a_patrol_waits_for_the_worlds_age_then_its_delay_and_its_chance() {
        let mut world = World::new();
        let nowhere = world.spawn_empty().id();
        let found = Found { cell: [0, 64, 0], biome: 1, surface: true, medium: sapling_data::Medium::Air, brightness: 0, weathered: 0, below: "minecraft:grass_block" };
        assert!(!open(&mut world, nowhere, &Gates { age: Some(6000), ..Gates::default() }, &found, &[]), "a world of age 0 is too young");
        let gates = Gates { delay: Some(DELAY), ..Gates::default() };
        assert!(!open(&mut world, nowhere, &gates, &found, &[]), "the first wait has only begun");
        let end = world.resource::<Waits>().0["patrol"];
        assert!((600..660).contains(&end), "{end}");
        world.resource_mut::<Waits>().0.insert("patrol", -1);
        assert!(open(&mut world, nowhere, &gates, &found, &[]));
        let hits = (0..1000).filter(|_| lucky(&mut world, nowhere, &gates)).count();
        assert!((150..280).contains(&hits), "21 in 100: {hits}");
        assert!(world.resource::<Waits>().0["patrol"] >= 600, "a miss waits again");
    }
}
