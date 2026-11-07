//! Against the group AQ drills: a note block's sound a tick after its event (research/captures/bds-ap3note.txt),
//! a scute shown before its turtle grows (bds-ak1scute.txt), an allay's ambient check and carried item
//! (bds-aq3allay.txt), and a rider let off into lava (bds-aq3exit.txt).

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, AddItemEntity, BlockEvent, CorrectPlayerMovePrediction, LevelSoundEvent, SetEntityData};
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain};
use common::asking::ask;
use common::raising::click_on;
use common::shooting::{FEET, ready};
use common::stored::{enter_stored, floor_with};
use common::{Harness, all, config};
use sapling_api::{Block, GameMode, Item, MobSlot, PlayerId, Slot, Target};
use sapling_world::{ChunkPos, World, WorldStorage, stored_actors};

const DIRT: &str = "minecraft:dirt";

/// What Alex was told each of `n` ticks.
fn per_tick(h: &mut Harness, n: usize) -> Vec<Vec<RawPacket>> {
    (0..n).map(|_| h.tick_all().to(1).to_vec()).collect()
}

fn first_tick<T: acacia_proto::Packet>(sent: &[Vec<RawPacket>], wanted: impl Fn(&T) -> bool) -> Option<usize> {
    sent.iter().position(|packets| all::<T>(packets).iter().any(&wanted))
}

#[test]
fn a_powered_note_block_is_heard_a_tick_after_its_block_event() {
    let mut h = ready(GameMode::Creative, &[], 0.0);
    let note = [4, 65, -6];
    h.remote.run(move |ctx| {
        assert!(ctx.update_block(note, &Block::new("noteblock")));
        assert!(ctx.update_block([note[0], note[1], note[2] - 1], &Block::new("redstone_block")));
    });
    let sent = per_tick(&mut h, 4);
    let event = first_tick::<BlockEvent>(&sent, |_| true).expect("the note played");
    let sound = first_tick::<LevelSoundEvent>(&sent, |sound| sound.sound_id == "note").expect("heard");
    assert_eq!(sound, event + 1, "bds-ap3note.txt: the sound a tick after the event");
}

#[test]
fn a_turtle_s_scute_is_shown_before_it_grows_up() {
    // bds-ak1scute.txt: AddItemEntity, then the turtle's grown data, then the scute's.
    let mut h = ready(GameMode::Creative, &[("minecraft:seagrass", 64, 0)], 0.0);
    let feet = [FEET[0] + 2.0, FEET[1], FEET[2]];
    let turtle = ask(&mut h, move |ctx| ctx.summon_as(None, "minecraft:turtle", feet, 0.0, 0.0, Some("minecraft:entity_born"))).expect("summoned").0;
    common::shooting::watch(&mut h, 3);
    let mut told: Vec<RawPacket> = (0..10).flat_map(|_| click_on(&mut h, turtle)).collect();
    told.extend(common::shooting::watch(&mut h, 2));
    let scute = told.iter().position(|packet| !all::<AddItemEntity>(std::slice::from_ref(packet)).is_empty()).expect("the scute shown");
    let grown = told.iter().rposition(|packet| all::<SetEntityData>(std::slice::from_ref(packet)).iter().any(|data| data.runtime_entity_id == turtle)).expect("grown");
    assert!(scute < grown, "the scute before the turtle's data: {scute} then {grown}");
}

fn ambient_named(data: &SetEntityData, id: u64, name: &str) -> bool {
    data.runtime_entity_id == id && data.metadata.iter().any(|item| item.key == Key::AmbientSoundEventName && matches!(&item.value, Value::Default(Plain::String(said)) if said == name))
}

#[test]
fn an_allay_handed_an_item_sounds_tame_at_its_next_ambient_check() {
    // bds-aq3allay.txt and four earlier drills: handed between 104 and 126 ticks in, switched 152 ticks in.
    let mut h = ready(GameMode::Survival, &[(DIRT, 4, 0)], 0.0);
    h.remote.run(|ctx| assert!(ctx.spawn_entity("minecraft:allay", [FEET[0] + 1.0, FEET[1], FEET[2]]).is_some()));
    let mut sent = per_tick(&mut h, 3);
    let (added, allay) = sent.iter().enumerate().find_map(|(at, packets)| all::<AddEntity>(packets).into_iter().find(|added| added.entity_type == "minecraft:allay").map(|added| (at, added.runtime_id))).expect("shown");
    sent.extend(per_tick(&mut h, added + 110 - sent.len()));
    sent.push(click_on(&mut h, allay));
    sent.extend(per_tick(&mut h, 60));
    let switched = first_tick::<SetEntityData>(&sent, |data| ambient_named(data, allay, "ambient.tame")).expect("ambient.tame");
    assert_eq!(switched - added, 152);
    let data = all::<SetEntityData>(&sent[switched]).into_iter().find(|data| data.runtime_entity_id == allay).expect("its data");
    assert_eq!(data.metadata.len(), 1, "the name alone");
}

#[test]
fn an_allay_keeps_what_it_collected_in_its_chest_items() {
    // bds-aq3allay.txt's world: ChestItems slot 0 held the dirt it carried.
    let storage = floor_with(Vec::new());
    let mut h = Harness::with(config(), World::with_storage(storage.clone()));
    enter_stored(&mut h, 1);
    let allay = ask(&mut h, |ctx| ctx.summon_as(None, "minecraft:allay", [4.5, 64.0, 8.5], 0.0, 0.0, None)).expect("summoned");
    assert_eq!(ask(&mut h, move |ctx| ctx.set_mob_item(allay, MobSlot::MainHand, Some(&Item::new(DIRT, 1)))), Ok(()));
    assert!(ask(&mut h, |ctx| ctx.drop_item(&Item::new(DIRT, 1), [6.5, 64.5, 8.5])));
    per_tick(&mut h, 100);
    h.shard.save_worlds();
    let column = storage.load_column(ChunkPos::new(0, 0), 0).unwrap().expect("the column");
    let saved = stored_actors(&column.actors).unwrap();
    let kept = saved.iter().find_map(|(actor, _, compound)| actor.as_ref().filter(|actor| actor.identifier == "minecraft:allay").map(|_| compound.clone())).expect("the allay");
    let chest: Vec<(u8, String, u8)> = sapling_world::actor_chest(&kept).into_iter().map(|(slot, stack)| (slot, stack.name, stack.count)).collect();
    assert_eq!(chest, [(0, DIRT.to_owned(), 1)]);
}

#[test]
fn a_rider_let_off_into_lava_keeps_its_seated_height_and_sinks() {
    // bds-aq3exit.txt: corrected to the lava cell at the seated eye height, delta -0.02, off the ground.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let pig = crate::riding::spawned(&mut h, &[("minecraft:pig", [2.5, 65.0, 0.5])])[0];
    h.remote.run(|ctx| assert!(ctx.set_item(PlayerId(1), Slot::Main(0), Some(&Item::new("minecraft:saddle", 1)))));
    h.tick(1);
    for _ in 0..2 {
        h.receive(1, &crate::riding::interact(pig.0));
        h.tick(1);
        h.tick(1);
    }
    h.remote.run(|ctx| {
        for (x, z) in [(1, -1), (2, -1), (3, -1), (1, 0), (3, 0), (1, 1), (2, 1), (3, 1)] {
            assert!(ctx.update_block([x, 65, z], &Block::new("lava")));
        }
    });
    h.tick(1);
    // As `/ride stop_riding`: a client leaving by itself is not corrected.
    h.remote.run(|ctx| {
        let alex = ctx.players().find(|player| player.id == PlayerId(1)).expect("Alex");
        assert!(ctx.dismount(&Target::Player(alex)));
    });
    let sent: Vec<RawPacket> = (0..2).flat_map(|_| h.tick(1)).collect();
    let corrected = all::<CorrectPlayerMovePrediction>(&sent).into_iter().next().expect("corrected");
    assert_eq!(corrected.delta.y, -0.02);
    assert!(!corrected.on_ground);
    assert!(corrected.position.y > 65.0 + sapling_data::EYE_HEIGHT + 0.1, "seated, not on the floor: {:?}", corrected.position);
}
