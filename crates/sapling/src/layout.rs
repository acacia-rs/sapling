//! Which world goes on which shard.

use sapling_world::World;

use crate::StartError;

/// Half the side of the default spawn platform, in blocks.
const PLATFORM_HALF: i32 = 8;
const PLATFORM_Y: i32 = 64;

pub(crate) struct WorldSpec {
    pub name: String,
    pub world: World,
    pub spawn: [f32; 3],
    pub shard: usize,
}

/// A void overworld with a 16x16 stone platform under the default spawn.
pub(crate) fn platform_world() -> World {
    let mut world = World::void();
    let stone = world.block_id("minecraft:stone").expect("the vanilla palette has stone");
    world.fill([-PLATFORM_HALF, PLATFORM_Y, -PLATFORM_HALF], [PLATFORM_HALF - 1, PLATFORM_Y, PLATFORM_HALF - 1], stone);
    world
}

/// Each shard's worlds, and the index of the shard new players join: the one with the world
/// `join`, which comes first on it, since a shard's first world is where its players appear.
pub(crate) fn plan(mut worlds: Vec<WorldSpec>, join: &str) -> Result<(Vec<Vec<WorldSpec>>, usize), StartError> {
    if let Some(twice) = worlds.iter().enumerate().find(|(i, world)| worlds[..*i].iter().any(|earlier| earlier.name == world.name)) {
        return Err(StartError::DuplicateWorld(twice.1.name.clone()));
    }
    let at = worlds.iter().position(|world| world.name == join).ok_or_else(|| StartError::UnknownWorld(join.to_owned()))?;
    let first = worlds.remove(at);
    let entry = first.shard;
    worlds.insert(0, first);
    let count = worlds.iter().map(|world| world.shard).max().unwrap_or(0) + 1;
    let mut shards: Vec<Vec<WorldSpec>> = (0..count).map(|_| Vec::new()).collect();
    worlds.into_iter().for_each(|world| shards[world.shard].push(world));
    match shards.iter().position(Vec::is_empty) {
        Some(empty) => Err(StartError::EmptyShard(empty)),
        None => Ok((shards, entry)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(name: &str, shard: usize) -> WorldSpec {
        WorldSpec { name: name.into(), world: World::void(), spawn: [0.5, 65.0, 0.5], shard }
    }

    fn names(shards: &[Vec<WorldSpec>]) -> Vec<Vec<&str>> {
        shards.iter().map(|worlds| worlds.iter().map(|world| world.name.as_str()).collect()).collect()
    }

    #[test]
    fn the_join_world_leads_its_shard_and_names_the_entry() {
        let (shards, entry) = plan(vec![spec("hub", 0), spec("a", 1), spec("b", 1), spec("c", 0)], "b").ok().unwrap();
        assert_eq!((names(&shards), entry), (vec![vec!["hub", "c"], vec!["b", "a"]], 1));
    }

    #[test]
    fn a_layout_with_a_gap_a_repeat_or_no_join_world_is_refused() {
        assert!(matches!(plan(vec![spec("hub", 0), spec("a", 2)], "hub"), Err(StartError::EmptyShard(1))));
        assert!(matches!(plan(vec![spec("hub", 0), spec("hub", 1)], "hub"), Err(StartError::DuplicateWorld(name)) if name == "hub"));
        assert!(matches!(plan(vec![spec("hub", 0)], "lobby"), Err(StartError::UnknownWorld(name)) if name == "lobby"));
    }

    #[test]
    fn the_default_world_is_a_platform_under_the_default_spawn() {
        let world = platform_world();
        let stone = world.block_id("minecraft:stone").unwrap();
        let [x, y, z] = sapling_shard::ShardConfig::default().spawn.map(|c| c.floor() as i32);
        assert_eq!(world.block(x, y - 1, z), stone);
        assert_eq!((world.block(-8, 64, -8), world.block(7, 64, 7)), (stone, stone));
        let air = world.registry().air_id();
        assert_eq!((world.block(x, y, z), world.block(8, 64, 0), world.block(0, 63, 0)), (air, air, air));
    }
}
