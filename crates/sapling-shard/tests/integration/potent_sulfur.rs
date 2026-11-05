//! Potent sulfur set in play: its place and neighbour rules, its eruptions and their vibrations
//! (blocks/sulfur.rs; docs/research/world-ticks.md "Potent sulfur").

use crate::common;

use common::Harness;
use sapling_api::Block;

const SULFUR: [i32; 3] = [4, 65, 0];
const SENSOR: [i32; 3] = [4, 65, 4];

fn sulfur(state: &str) -> Block {
    Block { name: "minecraft:potent_sulfur".into(), properties: format!("potent_sulfur_state={state}") }
}

/// Sets each block as `/setblock` does.
fn put(h: &mut Harness, blocks: Vec<([i32; 3], Block)>) {
    h.remote.run(move |ctx| blocks.iter().for_each(|(at, block)| assert!(ctx.update_block(*at, block), "{block:?} at {at:?}")));
    h.tick(1);
}

fn properties(h: &mut Harness, at: [i32; 3]) -> String {
    let (sent, got) = std::sync::mpsc::channel();
    h.remote.run(move |ctx| sent.send(ctx.block(at).map(|block| block.properties).unwrap_or_default()).unwrap());
    h.tick(1);
    got.recv().unwrap()
}

/// A geyser at [`SULFUR`] over `under` in a glass well, set dormant with nothing over it, then water set over it.
fn geyser(h: &mut Harness, under: &str) -> (String, String) {
    let mut walls: Vec<([i32; 3], Block)> = [[3, 66, 0], [5, 66, 0], [4, 66, -1], [4, 66, 1]].map(|at| (at, Block::new("glass"))).into();
    walls.push(([4, 64, 0], Block::new(under)));
    put(h, walls);
    put(h, vec![(SULFUR, sulfur("dormant"))]);
    let set = properties(h, SULFUR);
    put(h, vec![([4, 66, 0], Block::new("water"))]);
    (set, properties(h, SULFUR))
}

/// Ticks until the potent sulfur's state is `state` or `limit` ticks pass; the ticks it took.
fn until(h: &mut Harness, state: &str, limit: usize) -> Option<usize> {
    let wanted = format!("potent_sulfur_state={state}");
    (1..=limit).find(|_| properties(h, SULFUR) == wanted)
}

#[test]
fn a_potent_sulfur_set_dormant_shows_dry_under_air_and_turns_wet_under_water_set_over_it() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let (set, watered) = geyser(&mut h, "stone");
    assert_eq!((set.as_str(), watered.as_str()), ("potent_sulfur_state=dry", "potent_sulfur_state=wet"));
    put(&mut h, vec![([4, 66, 0], Block::new("air"))]);
    assert_eq!(properties(&mut h, SULFUR), "potent_sulfur_state=dry");
}

/// Under water over magma it lies dormant, erupts within 30 s and is heard by a sensor as `block_attach`
/// (frequency 10), then lies dormant again within 2 s.
#[test]
fn a_potent_sulfur_watered_over_magma_erupts_and_a_sensor_hears_it() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    put(&mut h, vec![(SENSOR, Block::new("sculk_sensor"))]);
    let (_, watered) = geyser(&mut h, "magma");
    assert_eq!(watered, "potent_sulfur_state=dormant");
    // The sensor has cooled down from the setting long before the first eruption (15 s at least).
    let erupted = until(&mut h, "erupting", 620);
    assert!(erupted.is_some_and(|ticks| ticks >= 290), "{erupted:?}");
    let heard = (0..8).find(|_| properties(&mut h, SENSOR).contains("sculk_sensor_phase=1"));
    assert!(heard.is_some(), "the sensor four cells off hears the eruption start");
    assert!(until(&mut h, "dormant", 45).is_some(), "an eruption under one water lasts 1 to 2 s");
}

/// Sets the blocks in one batch, as `/fill` does.
fn put_batched(h: &mut Harness, blocks: Vec<([i32; 3], Block)>) {
    h.remote.run(move |ctx| ctx.batch_blocks(|ctx| blocks.iter().for_each(|(at, block)| assert!(ctx.update_block(*at, block), "{block:?} at {at:?}"))));
    h.tick(1);
}

/// A batch reaches potent sulfur after all its writes, as BDS's `/fill` (research/captures/bds-ag2lava.txt):
/// the lava alone taken from under a continuous geyser wets it, but lava, sulfur and water taken together
/// leave no wet sulfur behind to be heard.
#[test]
fn a_batch_takes_a_geyser_away_unheard_but_its_lava_alone_wets_it() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    put(&mut h, [[3, 64, 0], [5, 64, 0], [4, 64, -1], [4, 64, 1]].map(|at| (at, Block::new("glass"))).into());
    put(&mut h, vec![(SENSOR, Block::new("sculk_sensor"))]);
    let (_, watered) = geyser(&mut h, "lava");
    assert_eq!(watered, "potent_sulfur_state=continuous");
    put_batched(&mut h, vec![([4, 64, 0], Block::new("air"))]);
    assert_eq!(properties(&mut h, SULFUR), "potent_sulfur_state=wet");
    put(&mut h, vec![([4, 64, 0], Block::new("lava"))]);
    assert_eq!(properties(&mut h, SULFUR), "potent_sulfur_state=continuous");
    let cooled = (0..120).find(|_| properties(&mut h, SENSOR).contains("sculk_sensor_phase=0"));
    assert!(cooled.is_some(), "the sensor cools down from the lava set back");
    put_batched(&mut h, [[4, 64, 0], SULFUR, [4, 66, 0]].map(|at| (at, Block::new("air"))).into());
    let heard = (0..10).find(|_| !properties(&mut h, SENSOR).contains("sculk_sensor_phase=0"));
    assert_eq!(heard, None, "nothing wet was left to be heard");
}

/// Water taken off a dormant potent sulfur dries it and drops its queued eruption.
#[test]
fn a_dormant_potent_sulfur_dried_does_not_erupt() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    geyser(&mut h, "magma");
    put(&mut h, vec![([4, 66, 0], Block::new("air"))]);
    assert_eq!(properties(&mut h, SULFUR), "potent_sulfur_state=dry");
    assert_eq!(until(&mut h, "erupting", 620), None);
}
