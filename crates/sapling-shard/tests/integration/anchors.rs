//! A respawn anchor in the Nether: a click makes it a spawn, a death there rises by it and spends
//! a charge, a spent one is forgotten; and where a player died is told and kept
//! (research/captures/bds-anchor.txt).

use crate::common;

use acacia_proto::packets::{ChangeDimension, Respawn, SetEntityData, SetSpawnPosition, Text, TextContent};
use acacia_proto::types::{MetadataDictionaryItemKey as Key, Vec3f};
use common::{Harness, all, click};
use sapling_api::{Block, PlayerId};
use sapling_world::{NETHER, World};

const ALEX: PlayerId = PlayerId(1);
const ANCHOR: [i32; 3] = [2, 70, 0];

/// A void Nether with a netherrack floor at y 69 around the origin.
fn nether() -> World {
    let mut nether = World::void_in(NETHER);
    let netherrack = nether.block_id("minecraft:netherrack").unwrap();
    nether.fill([-8, 69, -8], [7, 69, 7], netherrack);
    nether
}

fn anchor(charge: u8) -> Block {
    Block { name: "minecraft:respawn_anchor".into(), properties: format!("respawn_anchor_charge={charge}") }
}

fn said(packets: &[acacia_proto::RawPacket]) -> Vec<String> {
    all::<Text>(packets).into_iter().filter_map(|text| if let TextContent::Json(json) = text.content { Some(json.message) } else { None }).collect()
}

/// Alex in the Nether beside an anchor charged twice.
fn by_an_anchor() -> Harness {
    let mut h = Harness::new();
    assert!(h.shard.add_dimension("default", nether()));
    h.enter(1, "Alex");
    h.remote.run(|ctx| assert!(ctx.travel(ALEX, NETHER, [0.5, 70.0, 0.5])));
    (0..5).for_each(|_| drop(h.tick_all()));
    h.remote.run(|ctx| assert_eq!(ctx.within("default:nether", |ctx| ctx.set_block(ANCHOR, &anchor(2))), Some(true)));
    h.tick_all();
    h
}

/// Alex dies and its client asks to rise: what it was sent as it died, and as it rose.
fn die_and_rise(h: &mut Harness) -> (Vec<acacia_proto::RawPacket>, Vec<acacia_proto::RawPacket>) {
    h.remote.run(|ctx| assert!(ctx.kill_player(ALEX)));
    let died = h.tick_all().to(1).to_vec();
    h.receive(1, &Respawn { position: Vec3f { x: 0.0, y: 0.0, z: 0.0 }, state: 2, runtime_entity_id: 0 });
    (died, h.tick_all().to(1).to_vec())
}

fn charge(h: &mut Harness) -> Option<String> {
    let seen = std::sync::Arc::new(std::sync::Mutex::new(None));
    let out = seen.clone();
    h.remote.run(move |ctx| *out.lock().unwrap() = ctx.within("default:nether", |ctx| ctx.block(ANCHOR)).flatten().map(|block| block.properties));
    h.tick_all();
    seen.lock().unwrap().clone()
}

#[test]
fn an_anchor_clicked_in_the_nether_is_risen_by_until_it_is_spent() {
    let mut h = by_an_anchor();
    h.receive(1, &click(ANCHOR, 1, None));
    let sent = h.tick_all();
    let spawn = all::<SetSpawnPosition>(sent.to(1));
    assert_eq!(spawn.iter().map(|spawn| (spawn.dimension, [spawn.world_position.x, spawn.world_position.y, spawn.world_position.z], spawn.player_position.y)).collect::<Vec<_>>(), [(1, ANCHOR, 70)]);
    assert!(said(sent.to(1)).iter().any(|line| line.contains("§7") && line.contains("tile.respawn_anchor.respawnSet")));
    h.receive(1, &click(ANCHOR, 1, None));
    assert!(all::<SetSpawnPosition>(h.tick_all().to(1)).is_empty(), "nothing a second time");

    let risen = |sent: &[acacia_proto::RawPacket]| all::<Respawn>(sent).into_iter().map(|respawn| (respawn.state, respawn.position.x, respawn.position.z)).collect::<Vec<_>>();
    assert_eq!(risen(&die_and_rise(&mut h).1), [(0, 0.0, 0.0), (1, 0.5, 0.5)], "the cell, then the place in it");
    assert_eq!(charge(&mut h).as_deref(), Some("respawn_anchor_charge=1"));
    die_and_rise(&mut h);
    assert_eq!(charge(&mut h).as_deref(), Some("respawn_anchor_charge=0"));

    let sent = die_and_rise(&mut h).1;
    let forgotten = all::<SetSpawnPosition>(&sent);
    assert_eq!(forgotten.iter().map(|spawn| (spawn.dimension, spawn.world_position.x)).collect::<Vec<_>>(), [(3, i32::MIN)]);
    assert_eq!(all::<ChangeDimension>(&sent).len(), 1, "home to the overworld");
    assert!(said(&sent).iter().any(|line| line.contains("tile.respawn_anchor.notValid") && !line.contains("§7")));
}

#[test]
fn a_death_tells_where_it_was_once_and_what_changed_after() {
    let keys = |sent: &[acacia_proto::RawPacket]| all::<SetEntityData>(sent).into_iter().flat_map(|data| data.metadata).map(|item| item.key).filter(|key| matches!(key, Key::PlayerLastDeathPosition | Key::PlayerLastDeathDimension | Key::PlayerHasDied | Key::BoundingboxWidth)).collect::<Vec<_>>();
    let mut h = by_an_anchor();
    h.remote.run(|ctx| assert!(ctx.kill_player(ALEX)));
    assert_eq!(keys(h.tick_all().to(1)), [Key::BoundingboxWidth, Key::PlayerLastDeathPosition, Key::PlayerLastDeathDimension, Key::PlayerHasDied], "first in the Nether");
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.tick_all();
    let (died, risen) = die_and_rise(&mut h);
    assert_eq!(keys(&died), [Key::BoundingboxWidth, Key::PlayerLastDeathPosition, Key::PlayerHasDied], "dimension 0 is what it was");
    assert_eq!(keys(&risen), [Key::BoundingboxWidth], "the box whole again");
    assert_eq!(keys(&die_and_rise(&mut h).0), [Key::BoundingboxWidth], "the same place again");
}
