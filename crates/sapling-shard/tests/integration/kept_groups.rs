//! Component groups a stored mob keeps as BDS's did: a skeleton horse trap stays a trap, a
//! charged creeper stays charged.

use crate::common;

use acacia_proto::nbt::{List, Value};
use acacia_proto::packets::AddEntity;
use acacia_proto::types::{MetadataDictionaryItemValue as Meta, MetadataFlags1};
use common::asking::ask;
use common::stored::{actor_with, enter_stored, floor_with};
use common::{Harness, all, config};
use sapling_api::{Difficulty, GameMode, PlayerId};
use sapling_world::{ChunkPos, World, WorldStorage, actor_groups, stored_actors};

const ALEX: PlayerId = PlayerId(1);

fn grouped(id: i64, kind: &str, feet: [f32; 3], group: &str) -> Vec<u8> {
    let groups = Value::List(List { tag: 8, items: vec![Value::String(format!("+{group}").into())] });
    actor_with(id, kind, feet, vec![("definitions", groups)])
}

fn kept(actors: Vec<u8>) -> (Harness, std::sync::Arc<sapling_world::MemoryStorage>, Vec<acacia_proto::RawPacket>) {
    let storage = floor_with(actors);
    let config = sapling_shard::ShardConfig { difficulty: Difficulty::Easy, ..config() };
    let mut h = Harness::with(config, World::with_storage(storage.clone()));
    let sent = enter_stored(&mut h, 1);
    (h, storage, sent)
}

fn saved_groups(h: &mut Harness, storage: &sapling_world::MemoryStorage, kind: &str) -> Vec<String> {
    h.shard.save_worlds();
    let column = storage.load_column(ChunkPos::new(0, 0), 0).unwrap().expect("the column");
    let saved = stored_actors(&column.actors).unwrap();
    saved.iter().find(|(actor, ..)| actor.as_ref().is_some_and(|actor| actor.identifier == kind)).map(|(.., compound)| actor_groups(compound)).unwrap_or_else(|| panic!("no {kind} saved"))
}

#[test]
fn a_stored_skeleton_trap_is_saved_a_trap_and_springs_when_a_player_comes() {
    let (mut h, storage, _) = kept(grouped(-21, "minecraft:skeleton_horse", [14.5, 64.0, 14.5], "minecraft:skeleton_trap"));
    ask(&mut h, |ctx| ctx.set_game_mode(ALEX, GameMode::Creative));
    (0..3).for_each(|_| drop(h.tick_all()));
    assert!(saved_groups(&mut h, &storage, "minecraft:skeleton_horse").contains(&"minecraft:skeleton_trap".to_owned()));
    ask(&mut h, |ctx| ctx.teleport(ALEX, [8.5, 64.0, 8.5]));
    (0..5).for_each(|_| drop(h.tick_all()));
    let skeletons = ask(&mut h, |ctx| ctx.entities().into_iter().filter(|entity| entity.kind == "minecraft:skeleton").count());
    assert_eq!(skeletons, 4, "sprung");
}

#[test]
fn a_stored_charged_creeper_is_shown_charged_and_saved_so() {
    let (mut h, storage, sent) = kept(grouped(-22, "minecraft:creeper", [4.5, 64.0, 4.5], "minecraft:charged_creeper"));
    let creeper = all::<AddEntity>(&sent).into_iter().find(|added| added.entity_type == "minecraft:creeper").expect("shown");
    let powered = creeper.metadata.iter().any(|item| matches!(item.value, Meta::Flags(flags) if flags.contains(MetadataFlags1::POWERED)));
    assert!(powered, "{:?}", creeper.metadata);
    assert!(saved_groups(&mut h, &storage, "minecraft:creeper").contains(&"minecraft:charged_creeper".to_owned()));
}
