//! Block leftovers of the parity round (docs/research/blocks.md): beds bursting outside the
//! Overworld.

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{
    LevelEvent, LevelEventEvent, PositionTrackingDbBroadcast, PositionTrackingDbBroadcastBroadcastAction, PositionTrackingDbRequest, PositionTrackingDbRequestAction,
};
use common::asking::ask;
use acacia_proto::types::Action as BlockAction;
use common::{Harness, all, block_actions, click, shooting};
use sapling_api::{Block, GameMode, Item, PlayerId, RuleValue, Slot};
use sapling_world::{NETHER, World};

const ALEX: PlayerId = PlayerId(1);
const FOOT: [i32; 3] = [2, 70, 0];
const HEAD: [i32; 3] = [2, 70, 1];
const NETHER_NAME: &str = "default:nether";

/// A void Nether with a netherrack floor at y 69 around the origin.
fn nether() -> World {
    let mut nether = World::void_in(NETHER);
    let netherrack = nether.block_id("minecraft:netherrack").unwrap();
    nether.fill([-8, 69, -8], [7, 69, 7], netherrack);
    nether
}

/// Alex in the Nether beside a bed, its head to the south.
fn by_a_nether_bed(bursts: bool) -> Harness {
    let mut h = Harness::new();
    assert!(h.shard.add_dimension("default", nether()));
    h.enter(1, "Alex");
    h.remote.run(|ctx| assert!(ctx.travel(ALEX, NETHER, [0.5, 70.0, 0.5])));
    (0..5).for_each(|_| drop(h.tick_all()));
    h.remote.run(move |ctx| {
        let placed = ctx.within(NETHER_NAME, |ctx| {
            let half = |head: u8| Block { name: "minecraft:bed".into(), properties: format!("direction=0,head_piece_bit={head},occupied_bit=0") };
            ctx.set_block(FOOT, &half(0)) && ctx.set_block(HEAD, &half(1)) && ctx.set_game_rule("respawnBlocksExplode", RuleValue::Bool(bursts))
        });
        assert_eq!(placed, Some(true));
    });
    h.tick_all();
    h
}

fn block_in_nether(h: &mut Harness, pos: [i32; 3]) -> String {
    ask(h, move |ctx| ctx.within(NETHER_NAME, |ctx| ctx.block(pos)).flatten().map(|block| block.name)).expect("a block")
}

#[test]
fn a_bed_bursts_in_the_nether_at_its_head() {
    // research/captures/bds-bknether.txt: the blast at the head's middle, not between the halves.
    let mut h = by_a_nether_bed(true);
    h.receive(1, &click(FOOT, 1, None));
    let sent: Vec<RawPacket> = h.tick_all().to(1).to_vec();
    let flash = all::<LevelEvent>(&sent).into_iter().find(|event| event.event == LevelEventEvent::ParticleExplosion).expect("a blast");
    assert_eq!((flash.data, flash.position.x, flash.position.y, flash.position.z), (5, 2.5, 70.5, 1.5));
    // bds-t3nether.txt: the clicked foot's dust at its corner, before the blast.
    let dust = all::<LevelEvent>(&sent).into_iter().position(|event| event.event == LevelEventEvent::ParticleDestroyBlockNoSound && [event.position.x, event.position.y, event.position.z] == FOOT.map(|c| c as f32));
    assert!(dust.is_some_and(|dust| dust < all::<LevelEvent>(&sent).iter().position(|event| event.event == LevelEventEvent::ParticleExplosion).unwrap()));
    for half in [FOOT, HEAD] {
        let left = block_in_nether(&mut h, half);
        assert!(left == "minecraft:air" || left == "minecraft:fire", "{half:?}: {left}");
    }
}

#[test]
fn a_bed_clicked_at_its_head_bursts_with_its_foots_dust() {
    // research/captures/bds-x2bed.txt: the head clicked, the dust at the foot's corner alone.
    let mut h = by_a_nether_bed(true);
    h.receive(1, &click(HEAD, 1, None));
    let sent: Vec<RawPacket> = h.tick_all().to(1).to_vec();
    let dust: Vec<[f32; 3]> = all::<LevelEvent>(&sent).into_iter().filter(|event| event.event == LevelEventEvent::ParticleDestroyBlockNoSound).map(|event| [event.position.x, event.position.y, event.position.z]).collect();
    assert_eq!(dust, [FOOT.map(|c| c as f32)]);
}

#[test]
fn a_bursting_bed_lists_its_halves_head_first_with_the_blast_after_its_particles() {
    // research/captures/bds-v2nether.txt: the flash, the blast's LevelEventGeneric 4052, then the head's and the
    // foot's air first in their sub-chunk's UpdateSubchunkBlocks, never an UpdateBlock of their own.
    use acacia_proto::packets::{LevelEventGeneric, UpdateBlock, UpdateSubchunkBlocks};
    let mut h = by_a_nether_bed(true);
    // A set bed is shown again a tick on (structure-block-entities.md, `shown_again::show_beds`).
    h.tick_all();
    h.receive(1, &click(FOOT, 1, None));
    let sent: Vec<RawPacket> = h.tick_all().to(1).to_vec();
    let flash = sent.iter().position(|packet| all::<LevelEvent>(std::slice::from_ref(packet)).iter().any(|event| event.event == LevelEventEvent::ParticleExplosion)).expect("a blast");
    let particles = sent.iter().position(|packet| all::<LevelEventGeneric>(std::slice::from_ref(packet)).iter().any(|event| event.event_id == 2026 << 1));
    assert_eq!(particles, Some(flash + 1));
    let halves = |update: &UpdateBlock| [FOOT, HEAD].contains(&[update.position.x, update.position.y, update.position.z]);
    assert!(!all::<UpdateBlock>(&sent).iter().any(halves), "no half alone");
    let listed = |sent: &[RawPacket]| -> Vec<[i32; 3]> { all::<UpdateSubchunkBlocks>(sent).into_iter().flat_map(|batch| batch.blocks).map(|cell| [cell.position.x, cell.position.y, cell.position.z]).collect() };
    assert!(!listed(&sent[..flash]).iter().any(|cell| [FOOT, HEAD].contains(cell)), "no half before the blast");
    let after = listed(&sent[flash..]);
    let (head, foot) = (after.iter().position(|cell| *cell == HEAD), after.iter().position(|cell| *cell == FOOT));
    assert!(head.is_some() && head < foot, "{head:?} {foot:?}");
}

#[test]
fn a_wet_sponge_set_in_the_nether_dries_with_bds_s_steam() {
    // research/captures/bds-bknether.txt: event 3501 at 0.875 up the cell, then the dry sponge.
    let mut h = by_a_nether_bed(false);
    h.remote.run(|ctx| assert_eq!(ctx.within(NETHER_NAME, |ctx| ctx.update_block([-3, 70, 0], &Block::new("minecraft:wet_sponge"))), Some(true)));
    let sent: Vec<RawPacket> = (0..3).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    let steam = all::<LevelEvent>(&sent).into_iter().find(|event| event.event == LevelEventEvent::CauldronExplode).expect("the steam");
    assert_eq!([steam.position.x, steam.position.y, steam.position.z], [-2.5, 70.875, 0.5]);
    assert_eq!(block_in_nether(&mut h, [-3, 70, 0]), "minecraft:sponge");
}

fn silverfish(h: &mut Harness) -> Vec<[f32; 3]> {
    ask(h, |ctx| ctx.entities().into_iter().filter(|entity| entity.kind == "minecraft:silverfish").map(|entity| entity.feet).collect())
}

#[test]
fn infested_stone_broken_by_hand_or_blasted_lets_a_silverfish_out() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| ctx.set_difficulty(sapling_api::Difficulty::Easy));
    h.remote.run(|ctx| assert!(ctx.set_block([1, 64, 0], &Block::new("minecraft:infested_stone")) && ctx.set_block([-6, 64, -6], &Block::new("minecraft:infested_cobblestone"))));
    h.tick_all();
    h.receive(1, &block_actions(&[(BlockAction::StartBreak, [1, 64, 0])]));
    (0..120).for_each(|_| drop(h.tick_all()));
    h.receive(1, &block_actions(&[(BlockAction::PredictBreak, [1, 64, 0])]));
    h.tick_all();
    let out = silverfish(&mut h);
    assert!(out.len() == 1 && (out[0][0] - 1.0).abs() < 0.3 && (out[0][2] - 0.0).abs() < 0.3, "{out:?}");
    h.remote.run(|ctx| assert!(ctx.spawn_entity("minecraft:tnt", [-5.5, 65.0, -5.5]).is_some()));
    (0..90).for_each(|_| drop(h.tick_all()));
    assert_eq!(silverfish(&mut h).len(), 2);
}

/// The dust's signal, after the circuit settles.
fn dust(h: &mut Harness) -> String {
    (0..6).for_each(|_| drop(h.tick_all()));
    ask(h, |ctx| ctx.block([4, 65, 0]).map(|dust| dust.properties).unwrap_or_default())
}

#[test]
fn a_comparator_reads_a_frame_behind_its_block_by_turn_and_a_framed_map_shows() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_game_mode(ALEX, GameMode::Creative);
        assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new("minecraft:diamond", 1))));
        assert!(ctx.update_block([2, 65, 0], &Block::new("minecraft:stone")));
        assert!(ctx.update_block([1, 65, 0], &Block::new("minecraft:frame").with("facing_direction=4,item_frame_map_bit=0,item_frame_photo_bit=0")));
        assert!(ctx.update_block([3, 65, 0], &Block { name: "minecraft:unpowered_comparator".into(), properties: "minecraft:cardinal_direction=west".into() }));
        assert!(ctx.update_block([4, 65, 0], &Block::new("minecraft:redstone_wire")));
        assert!(ctx.set_block([1, 65, 3], &Block::new("minecraft:frame").with("facing_direction=1,item_frame_map_bit=0,item_frame_photo_bit=0")));
    });
    assert_eq!(dust(&mut h), "redstone_signal=0");
    h.receive(1, &click([1, 65, 0], 4, None));
    assert_eq!(dust(&mut h), "redstone_signal=1");
    h.receive(1, &click([1, 65, 0], 4, None));
    assert_eq!(dust(&mut h), "redstone_signal=2");

    h.remote.run(|ctx| assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new("minecraft:filled_map", 1)))));
    h.tick_all();
    h.receive(1, &click([1, 65, 3], 1, None));
    h.tick_all();
    let shown = ask(&mut h, |ctx| ctx.block([1, 65, 3]).map(|frame| frame.properties)).unwrap_or_default();
    assert!(shown.contains("item_frame_map_bit=1"), "{shown}");
}

/// What the shard answers Alex's request for the target of `handle`.
fn tracked(h: &mut Harness, handle: i32) -> (PositionTrackingDbBroadcastBroadcastAction, String) {
    h.receive(1, &PositionTrackingDbRequest { action: PositionTrackingDbRequestAction::Query, tracking_id: handle });
    let told = all::<PositionTrackingDbBroadcast>(h.tick_all().to(1));
    let [told] = told.try_into().expect("one answer");
    assert_eq!(told.tracking_id, handle);
    (told.broadcast_action, format!("{:?}", told.nbt.decode().unwrap().value))
}

#[test]
fn a_compass_clicked_on_a_lodestone_points_at_it_until_it_goes() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new("minecraft:compass", 2))));
        assert!(ctx.set_block([2, 65, 0], &Block::new("minecraft:lodestone")));
    });
    h.tick_all();
    h.receive(1, &click([2, 65, 0], 1, None));
    // research/captures/bds-t3uses.txt: the record told unasked at the binding and at the going.
    let told: Vec<_> = all::<PositionTrackingDbBroadcast>(h.tick_all().to(1)).into_iter().map(|told| told.broadcast_action).collect();
    assert_eq!(told, [PositionTrackingDbBroadcastBroadcastAction::Update]);
    let held = ask(&mut h, |ctx| [Slot::Main(0), Slot::Main(1)].map(|slot| ctx.item(ALEX, slot).map(|item| (item.name, item.count))));
    assert_eq!(held, [Some(("minecraft:compass".into(), 1)), Some(("minecraft:lodestone_compass".into(), 1))]);
    // A second binding to the same lodestone told nothing on BDS.
    h.receive(1, &click([2, 65, 0], 1, None));
    assert!(all::<PositionTrackingDbBroadcast>(h.tick_all().to(1)).is_empty());
    let (told, record) = tracked(&mut h, 1);
    assert!(told == PositionTrackingDbBroadcastBroadcastAction::Update && record.contains("Int(2), Int(65), Int(0)") && record.contains("0x00000001"), "{told:?} {record}");
    assert_eq!(tracked(&mut h, 7).0, PositionTrackingDbBroadcastBroadcastAction::NotFound);
    h.remote.run(|ctx| assert!(ctx.update_block([2, 65, 0], &Block::air())));
    let told = all::<PositionTrackingDbBroadcast>(h.tick_all().to(1));
    assert!(told.len() == 1 && told[0].broadcast_action == PositionTrackingDbBroadcastBroadcastAction::Destory);
    let (told, record) = tracked(&mut h, 1);
    assert!(told == PositionTrackingDbBroadcastBroadcastAction::Destory && record.contains("(\"status\", Byte(2))"), "{record}");
}

fn tilt_at(h: &mut Harness, pos: [i32; 3]) -> String {
    let properties = ask(h, move |ctx| ctx.block(pos).map(|leaf| leaf.properties)).unwrap_or_default();
    properties.split(',').find_map(|pair| pair.strip_prefix("big_dripleaf_tilt=")).unwrap_or_default().to_owned()
}

#[test]
fn a_big_dripleaf_tilts_under_a_player_and_comes_back_up() {
    const LEAF: [i32; 3] = [2, 65, 0];
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| assert!(ctx.set_block([2, 64, 0], &Block::new("minecraft:dirt")) && ctx.set_block(LEAF, &Block::new("minecraft:big_dripleaf").with("big_dripleaf_head=1,big_dripleaf_tilt=none,minecraft:cardinal_direction=south"))));
    h.tick_all();
    shooting::face(&mut h, 1, ALEX, [2.5, 65.9375, 0.5], 0.0);
    let mut seen = vec![tilt_at(&mut h, LEAF)];
    let sent: Vec<RawPacket> = (0..10).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    // research/captures/bds-bkblocks.txt: heard at the cell's corner.
    let down = all::<acacia_proto::packets::LevelSoundEvent>(&sent).into_iter().find(|sound| sound.sound_id == "tilt_down.big_dripleaf").expect("tilted");
    assert_eq!([down.position.x, down.position.y, down.position.z], [2.0, 65.0, 0.0]);
    seen.push(tilt_at(&mut h, LEAF));
    (0..10).for_each(|_| drop(h.tick_all()));
    seen.push(tilt_at(&mut h, LEAF));
    shooting::face(&mut h, 1, ALEX, [-4.5, 65.0, -4.5], 0.0);
    (0..100).for_each(|_| drop(h.tick_all()));
    seen.push(tilt_at(&mut h, LEAF));
    assert_eq!(seen, ["unstable", "partial_tilt", "full_tilt", "none"]);
}

#[test]
fn a_big_dripleaf_on_stone_shows_its_tilt_and_then_breaks() {
    // research/captures/bds-t3uses.txt: `unstable` shown, then the leaf gone.
    const LEAF: [i32; 3] = [2, 65, 0];
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| assert!(ctx.set_block([2, 64, 0], &Block::new("minecraft:stone")) && ctx.set_block(LEAF, &Block::new("minecraft:big_dripleaf").with("big_dripleaf_head=1,big_dripleaf_tilt=none,minecraft:cardinal_direction=south"))));
    h.tick_all();
    let mut sent = shooting::face(&mut h, 1, ALEX, [2.5, 65.9375, 0.5], 0.0);
    sent.extend((0..3).flat_map(|_| h.tick_all().to(1).to_vec()));
    let shown: Vec<u32> = all::<acacia_proto::packets::UpdateBlock>(&sent).into_iter().filter(|update| [update.position.x, update.position.y, update.position.z] == LEAF).map(|update| update.block_runtime_id).collect();
    let registry = sapling_world::BlockRegistry::vanilla();
    let unstable = registry.find("minecraft:big_dripleaf", "big_dripleaf_head=1,big_dripleaf_tilt=unstable,minecraft:cardinal_direction=south").map(|id| registry.network_hash(id));
    assert_eq!(shown, [unstable.expect("a vanilla state"), common::hash("minecraft:air")]);
}

#[test]
fn a_big_dripleaf_tilts_again_under_a_player_who_fell_into_its_cell() {
    // research/captures/bds-leafstand.txt: back to none and unstable in one update, the player at the cell's bottom.
    const LEAF: [i32; 3] = [2, 65, 0];
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| assert!(ctx.set_block([2, 64, 0], &Block::new("minecraft:dirt")) && ctx.set_block(LEAF, &Block::new("minecraft:big_dripleaf").with("big_dripleaf_head=1,big_dripleaf_tilt=none,minecraft:cardinal_direction=south"))));
    h.tick_all();
    shooting::face(&mut h, 1, ALEX, [2.5, 65.9375, 0.5], 0.0);
    (0..25).for_each(|_| drop(h.tick_all()));
    assert_eq!(tilt_at(&mut h, LEAF), "full_tilt");
    shooting::face(&mut h, 1, ALEX, [2.5, 65.0, 0.5], 0.0);
    let mut seen = Vec::new();
    for _ in 0..100 {
        h.tick_all();
        seen.push(tilt_at(&mut h, LEAF));
    }
    assert!(!seen.contains(&"none".to_owned()) && seen.contains(&"unstable".to_owned()), "{seen:?}");
}

#[test]
fn a_big_dripleaf_on_stone_breaks_when_stood_on() {
    // research/captures/bds-bkblocks.txt: BDS broke one set on stone as it tilted.
    const LEAF: [i32; 3] = [2, 65, 0];
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| assert!(ctx.set_block([2, 64, 0], &Block::new("minecraft:stone")) && ctx.set_block(LEAF, &Block::new("minecraft:big_dripleaf").with("big_dripleaf_head=1,big_dripleaf_tilt=none,minecraft:cardinal_direction=south"))));
    h.tick_all();
    shooting::face(&mut h, 1, ALEX, [2.5, 65.9375, 0.5], 0.0);
    h.tick_all();
    assert_eq!(ask(&mut h, |ctx| ctx.block(LEAF).map(|block| block.name)).as_deref(), Some("minecraft:air"));
}

#[test]
fn an_arrow_tilts_a_big_dripleaf_fully() {
    const LEAF: [i32; 3] = [0, 65, 0];
    let mut h = shooting::ready(GameMode::Survival, &[("minecraft:bow", 1, 0), ("minecraft:arrow", 4, 0)], 90.0);
    h.remote.run(|ctx| assert!(ctx.set_block([0, 64, 0], &Block::new("minecraft:dirt")) && ctx.set_block(LEAF, &Block::new("minecraft:big_dripleaf").with("big_dripleaf_head=1,big_dripleaf_tilt=none,minecraft:cardinal_direction=south"))));
    h.tick_all();
    shooting::face(&mut h, 1, ALEX, [0.5, 69.0, 0.5], 90.0);
    shooting::draw(&mut h, 20, 10);
    assert_eq!(tilt_at(&mut h, LEAF), "full_tilt");
}

#[test]
fn a_seeded_pot_broken_in_creative_leaves_its_loot_and_not_itself() {
    // research/captures/bds-loot-probe2.txt: the pot of seed 1434522187 left item 307, one, and no pot.
    const POT: [i32; 3] = [1, 64, 0];
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_game_mode(ALEX, GameMode::Creative);
        assert!(ctx.set_block(POT, &Block::new("minecraft:decorated_pot")));
    });
    h.tick_all();
    let kept = acacia_proto::nbt::Value::Compound(vec![
        ("id".into(), acacia_proto::nbt::Value::String("DecoratedPot".into())),
        ("LootTable".into(), acacia_proto::nbt::Value::String("loot_tables/pots/trial_chambers/corridor.json".into())),
        ("LootTableSeed".into(), acacia_proto::nbt::Value::Int(1_434_522_187)),
    ]);
    h.remote.run(move |ctx| assert!(ctx.set_block_entity(POT, &sapling_api::BlockEntityData::new(kept))));
    h.tick_all();
    let table = ask(&mut h, |ctx| ctx.block_entity(POT).and_then(|data| data.get::<acacia_proto::nbt::Value>().and_then(|nbt| nbt.get("LootTable").cloned())));
    assert!(table.is_some(), "the pot keeps the entity it was given");
    h.receive(1, &block_actions(&[(BlockAction::StartBreak, POT)]));
    h.receive(1, &block_actions(&[(BlockAction::PredictBreak, POT)]));
    (0..3).for_each(|_| drop(h.tick_all()));
    let left: Vec<(String, u8)> = ask(&mut h, |ctx| ctx.entities().into_iter().filter_map(|entity| entity.item).map(|item| (item.name, item.count)).collect());
    let bds = sapling_data::item_name(307).expect("item 307").to_owned();
    assert_eq!(left, [(bds, 1)]);
}

#[test]
fn a_bed_in_the_nether_does_nothing_with_respawn_blocks_explode_off() {
    let mut h = by_a_nether_bed(false);
    h.receive(1, &click(FOOT, 1, None));
    let sent: Vec<RawPacket> = h.tick_all().to(1).to_vec();
    assert!(all::<LevelEvent>(&sent).iter().all(|event| event.event != LevelEventEvent::ParticleExplosion));
    assert_eq!(block_in_nether(&mut h, HEAD), "minecraft:bed");
}
