//! Mobs' entity properties in AddEntity (creatures/properties.rs), as BDS announced them in the
//! `properties` drill (research/captures/bds-properties.txt).

use crate::common;

use acacia_proto::packets::AddEntity;
use common::shooting::{added, ready, watch};
use sapling_api::GameMode;

/// What Alex is shown of mobs of `kind` spawned at `feet`.
fn spawned(kind: &'static str, feet: Vec<[f32; 3]>) -> Vec<Vec<(u32, i32)>> {
    let mut h = ready(GameMode::Creative, &[], 0.0);
    h.remote.run(move |ctx| feet.iter().for_each(|feet| assert!(ctx.spawn_entity(kind, *feet).is_some())));
    added(&watch(&mut h, 5), kind).iter().map(|mob: &AddEntity| mob.properties.ints.iter().map(|int| (int.index, int.value)).collect()).collect()
}

#[test]
fn spawned_pigs_roll_their_voice_and_take_the_biomes_climate() {
    let pigs = spawned("minecraft:pig", (0..9).map(|at| [at as f32 - 4.5, 65.0, 3.5]).collect());
    assert_eq!(pigs.len(), 9);
    assert!(pigs.iter().all(|pig| pig.len() == 2 && pig[0] == (0, 0) && pig[1].0 == 1 && (0..3).contains(&pig[1].1)), "temperate, a voice of three: {pigs:?}");
    let mut voices: Vec<i32> = pigs.iter().map(|pig| pig[1].1).collect();
    voices.sort_unstable();
    voices.dedup();
    assert!(voices.len() > 1, "voices are rolled: {pigs:?}");
}

#[test]
fn a_happy_ghast_shows_its_default_and_a_zombie_none() {
    assert_eq!(spawned("minecraft:happy_ghast", vec![[0.5, 70.0, 6.5]]), [vec![(0, 1)]]);
    assert_eq!(spawned("minecraft:zombie", vec![[3.5, 65.0, 6.5]]), [Vec::<(u32, i32)>::new()]);
}
