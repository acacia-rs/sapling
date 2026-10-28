//! Powder snow and fish buckets (blocks/buckets.rs, creatures/scooping.rs; research/captures/bds-buckets.txt).

use crate::common;

use acacia_proto::packets::{LevelSoundEvent, RemoveEntity};
use common::raising::click_on;
use common::shooting::{added, held, ready, spawn, watch};
use common::{all, click};
use sapling_api::{Block, GameMode, Slot};

fn sounds(sent: &[acacia_proto::RawPacket]) -> Vec<String> {
    all::<LevelSoundEvent>(sent).into_iter().map(|sound| sound.sound_id).collect()
}

fn block_at(h: &mut common::Harness, pos: [i32; 3]) -> Option<String> {
    let seen = std::sync::Arc::new(std::sync::Mutex::new(None));
    let out = seen.clone();
    h.remote.run(move |ctx| *out.lock().unwrap() = ctx.block(pos).map(|block| block.name));
    h.tick_all();
    seen.lock().unwrap().take()
}

#[test]
fn an_empty_bucket_takes_powder_snow_and_puts_it_down_again() {
    let mut h = ready(GameMode::Survival, &[("minecraft:bucket", 1, 0)], 30.0);
    h.remote.run(|ctx| assert!(ctx.set_block([0, 65, 2], &Block::new("minecraft:powder_snow"))));
    h.tick_all();
    h.receive(1, &click([0, 65, 2], 1, None));
    let taken = h.tick_all().to(1).to_vec();
    assert_eq!(sounds(&taken), ["bucket.fill.powder_snow"]);
    assert_eq!(block_at(&mut h, [0, 65, 2]).as_deref(), Some("minecraft:air"));
    assert_eq!(held(&mut h, Slot::Main(0)).map(|item| item.name).as_deref(), Some("minecraft:powder_snow_bucket"));

    h.receive(1, &click([0, 64, 3], 1, None));
    let put = h.tick_all().to(1).to_vec();
    assert_eq!(sounds(&put), ["bucket.empty.powder_snow"]);
    assert_eq!(block_at(&mut h, [0, 65, 3]).as_deref(), Some("minecraft:powder_snow"));
    assert_eq!(held(&mut h, Slot::Main(0)).map(|item| item.name).as_deref(), Some("minecraft:bucket"));
}

#[test]
fn a_water_bucket_scoops_a_cod_and_lets_it_out_with_the_water() {
    let mut h = ready(GameMode::Survival, &[("minecraft:water_bucket", 1, 0)], 30.0);
    let cod = spawn(&mut h, "minecraft:cod", [0.5, 65.0, 2.5]);
    let scooped = [click_on(&mut h, cod), watch(&mut h, 1)].concat();
    assert!(all::<RemoveEntity>(&scooped).iter().any(|removed| removed.entity_id_self as u64 == cod), "the cod went");
    assert!(sounds(&scooped).contains(&"bucket.fill.fish".to_owned()));
    let bucket = held(&mut h, Slot::Main(0)).expect("a bucket");
    assert_eq!(bucket.name, "minecraft:cod_bucket");

    h.receive(1, &click([0, 64, 3], 1, None));
    let poured = [h.tick_all().to(1).to_vec(), watch(&mut h, 3)].concat();
    assert!(sounds(&poured).contains(&"bucket.empty.fish".to_owned()), "{:?}", sounds(&poured));
    assert_eq!(added(&poured, "minecraft:cod").len(), 1, "the cod is back");
    assert_eq!(block_at(&mut h, [0, 65, 3]).as_deref(), Some("minecraft:water"));
    assert_eq!(held(&mut h, Slot::Main(0)).map(|item| item.name).as_deref(), Some("minecraft:bucket"));
}
