//! Mobs a structure's template placed in a generated column (an outpost's pillagers): made once the
//! column ticks with its neighbours loaded, kept with it as any mob, not made again
//! (docs/research/structure-spawns.md "Template actors").

use crate::common;

use std::sync::{Arc, mpsc};

use acacia_proto::RawPacket;
use acacia_proto::packets::AddEntity;
use common::stored::enter_stored;
use common::{Harness, all, config};
use sapling_api::{Difficulty, EntityInfo};
use sapling_shard::ShardConfig;
use sapling_world::{BlockRegistry, Chunk, ChunkPos, Dimension, MemoryStorage, StoredColumn, World, WorldStorage};
use sapling_worldgen::TemplateActor;

/// Storage that generates columns with a stone floor at y 63, a captain on column `.1`'s (or `.2`,
/// moved to its middle), where `inner` has none.
struct Generating(Arc<MemoryStorage>, ChunkPos, Option<TemplateActor>);

impl WorldStorage for Generating {
    fn load_column(&self, pos: ChunkPos, dimension: i32) -> Result<Option<StoredColumn>, sapling_world::StorageError> {
        if let Some(stored) = self.0.load_column(pos, dimension)? {
            return Ok(Some(stored));
        }
        let registry = BlockRegistry::vanilla();
        let mut chunk = Chunk::empty(pos.x, pos.z, Dimension::overworld(registry.air_id()));
        let stone = registry.states_of("minecraft:stone").next().unwrap().0;
        (0..16).for_each(|x| (0..16).for_each(|z| chunk.set(x, 63, z, 0, stone)));
        let feet = [pos.x as f32 * 16.0 + 8.0, 64.0, pos.z as f32 * 16.0 + 8.0];
        let captain = TemplateActor { kind: "minecraft:pillager", event: "minecraft:spawn_as_illager_captain", feet, saved: None };
        let actor = self.2.clone().map_or(captain, |actor| TemplateActor { feet: [feet[0] + actor.feet[0].rem_euclid(1.0), 64.0, feet[2] + actor.feet[2].rem_euclid(1.0)], ..actor });
        let template_actors = if pos == self.1 { vec![actor] } else { Vec::new() };
        Ok(Some(StoredColumn { template_actors, ..StoredColumn::of(chunk) }))
    }

    fn store_column(&self, pos: ChunkPos, dimension: i32, column: &StoredColumn) -> Result<(), sapling_world::StorageError> {
        self.0.store_column(pos, dimension, column)
    }
}

/// Ticks `count` times; what player 1 was sent.
fn ticks(h: &mut Harness, count: usize) -> Vec<RawPacket> {
    (0..count).flat_map(|_| h.tick_all().to(1).to_vec()).collect()
}

/// Where the pillagers shown in `sent` were announced.
fn shown_at(sent: &[RawPacket]) -> Vec<[f32; 3]> {
    all::<AddEntity>(sent).into_iter().filter(|added| added.entity_type == "minecraft:pillager").map(|added| [added.position.x, added.position.y, added.position.z]).collect()
}

fn pillagers(h: &mut Harness) -> Vec<EntityInfo> {
    let (tx, rx) = mpsc::channel();
    h.remote.run(move |ctx| tx.send(ctx.entities()).unwrap());
    h.tick_all();
    rx.recv().unwrap().into_iter().filter(|entity| entity.kind == "minecraft:pillager").collect()
}

#[test]
fn a_template_mob_comes_once_with_its_column_and_stays_with_it() {
    let inner = Arc::new(MemoryStorage::default());
    let mut h = Harness::with(ShardConfig { difficulty: Difficulty::Normal, ..config() }, World::with_storage(Arc::new(Generating(inner.clone(), ChunkPos::new(0, 0), None))));
    let sent = [enter_stored(&mut h, 1), ticks(&mut h, 3)].concat();
    let first = pillagers(&mut h);
    assert_eq!(first.len(), 1, "{first:?}");
    assert_eq!(shown_at(&sent), [[8.0, 64.0, 8.0]], "at its cell's corner");
    h.shard.save_worlds();
    let stored = inner.load_column(ChunkPos::new(0, 0), 0).unwrap().expect("saved with its mob");
    assert!(stored.template_actors.is_empty());
    let kept = sapling_world::stored_actors(&stored.actors).unwrap();
    assert!(kept.iter().any(|(actor, ..)| actor.as_ref().is_some_and(|actor| actor.identifier == "minecraft:pillager")), "{kept:?}");
    h.leave(1);
    (0..350).for_each(|_| h.shard.tick());
    enter_stored(&mut h, 2);
    ticks(&mut h, 3);
    assert_eq!(pillagers(&mut h).len(), 1, "back from storage, not made again");
}

/// BDS made none of a mansion's illagers in a ticking column whose neighbours it had not loaded (bds-jxmansion3).
#[test]
fn a_template_mob_waits_until_its_column_ticks_with_its_neighbours_loaded() {
    let inner = Arc::new(MemoryStorage::default());
    let mut h = Harness::with(ShardConfig { difficulty: Difficulty::Normal, ..config() }, World::with_storage(Arc::new(Generating(inner, ChunkPos::new(2, 0), None))));
    enter_stored(&mut h, 1);
    ticks(&mut h, 20);
    assert!(pillagers(&mut h).is_empty(), "column (3, 0) is out of a radius-2 view");
    h.remote.run(|ctx| ctx.teleport(sapling_api::PlayerId(1), [40.5, 64.0, 8.5]));
    let sent = ticks(&mut h, 40);
    assert_eq!(pillagers(&mut h).len(), 1);
    assert_eq!(shown_at(&sent), [[40.0, 64.0, 8.0]]);
}

/// A bastion's crossbow piglin (seed 20261003, BDS's at (-345.553, 33, 244.60431), bds-az3bastion.txt)
/// comes from its saved compound: its crossbow, groups and persistence, its turned yaw, no event.
#[test]
fn a_bastion_piece_s_piglin_comes_as_its_template_saved_it() {
    let chunk = sapling_worldgen::NetherGenerator::new(20261003).chunk_blocks(-22, 15);
    let holds_crossbow = |actor: &&TemplateActor| actor.saved.is_some_and(|saved| saved.nbt.windows(18).any(|w| w == b"minecraft:crossbow"));
    let piglin = chunk.actors().iter().find(holds_crossbow).expect("the bastion's crossbow piglin").clone();
    assert_eq!(piglin.kind, "minecraft:piglin");
    let yaw = piglin.saved.unwrap().rotation[0];
    let inner = Arc::new(MemoryStorage::default());
    let mut h = Harness::with(ShardConfig { difficulty: Difficulty::Normal, ..config() }, World::with_storage(Arc::new(Generating(inner.clone(), ChunkPos::new(0, 0), Some(piglin)))));
    let sent = [enter_stored(&mut h, 1), ticks(&mut h, 3)].concat();
    let shown: Vec<_> = all::<AddEntity>(&sent).into_iter().filter(|added| added.entity_type == "minecraft:piglin").collect();
    assert_eq!(shown.len(), 1);
    let at = [shown[0].position.x, shown[0].position.y, shown[0].position.z];
    assert!(at.iter().zip([8.447, 64.0, 8.6043]).all(|(a, b)| (a - b).abs() < 1e-3), "{at:?}: turned as BDS's");
    assert_eq!(shown[0].yaw, yaw);
    h.shard.save_worlds();
    let stored = inner.load_column(ChunkPos::new(0, 0), 0).unwrap().expect("saved with its mob");
    let kept = sapling_world::stored_actors(&stored.actors).unwrap();
    let (actor, _, compound) = kept.iter().find(|(actor, ..)| actor.as_ref().is_some_and(|actor| actor.identifier == "minecraft:piglin")).expect("the piglin saved");
    assert_eq!(actor.as_ref().unwrap().mainhand.as_ref().map(|stack| stack.name.as_str()), Some("minecraft:crossbow"));
    assert!(sapling_world::actor_groups(compound).iter().any(|group| group == "ranged_unit"), "{:?}", sapling_world::actor_groups(compound));
    let read = acacia_proto::nbt::read::<acacia_proto::nbt::LittleEndian>(&mut &compound[..]).unwrap().value;
    assert_eq!(read.get("Persistent"), Some(&acacia_proto::nbt::Value::Byte(1)));
}
