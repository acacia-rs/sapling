//! What a world change leaves on a client's screen. A vanilla client kept a column of the old
//! world that the new view did not cover, so every column a player holds is sent again: from
//! the new world inside the new view, empty outside it (harness: common/cluster.rs).

use crate::common;

use std::collections::{HashMap, HashSet};

use acacia_proto::packets::{ClientCacheBlobStatus, ClientCacheMissResponse, ClientCacheStatus, LevelChunk, MovePlayer, Subchunk, SubchunkRequest};
use acacia_proto::types::{InputData, Vec3i8, Vec3li};
use acacia_proto::{Packet, RawPacket};
use common::cluster::Cluster;
use common::{all, input_at};

type Column = (i32, i32);

/// Views are four chunks wide; the walk ends six chunks east of the spawn every world has.
const RADIUS: i32 = 4;

fn in_new_view((x, z): Column) -> bool {
    x * x + z * z <= RADIUS * RADIUS
}

/// Walks the player to chunk (6, 0) in steps a player can make, its last teleport followed, and
/// returns the columns it was sent on the way: all outside a view at the spawn, which still
/// shares the columns around x 3 with the view it leaves.
fn walk_east(c: &mut Cluster, conn: u64) -> HashSet<Column> {
    let mut first = input_at([0.5, 65.0, 0.5]);
    first.input_data.push(InputData::HandledTeleport);
    c.receive(conn, &first);
    for step in 1..=11 {
        c.receive(conn, &input_at([0.5 + 9.0 * step as f32, 65.0, 0.5]));
    }
    let sent = c.ticks(6);
    let held: HashSet<Column> = all::<LevelChunk>(sent.to(conn)).iter().map(|chunk| (chunk.x, chunk.z)).collect();
    assert!(held.contains(&(10, 0)) && held.contains(&(5, 0)) && held.iter().all(|column| !in_new_view(*column)), "{held:?}");
    held
}

/// Checks what a player was sent after a change of world against the columns it held before:
/// each is sent again, empty and ahead of the teleport if the new view does not cover it, and
/// nothing with blocks goes out for a place outside the new view.
fn assert_swapped(held: &HashSet<Column>, after: &[RawPacket], cached: bool) {
    let jump = after.iter().position(|packet| packet.id == MovePlayer::ID).expect("a teleport to the new spawn");
    let mut last: HashMap<Column, LevelChunk> = HashMap::new();
    for (index, packet) in after.iter().enumerate().filter(|(_, packet)| packet.id == LevelChunk::ID) {
        let chunk: LevelChunk = packet.decode().unwrap();
        let (column, empty) = ((chunk.x, chunk.z), chunk.highest_subchunk_count == Some(0));
        assert_eq!(in_new_view(column), index > jump, "{column:?}: emptied before the teleport, the new world's after");
        assert!(in_new_view(column) || empty, "{column:?} is outside the new view and has sections to ask for");
        assert_eq!((chunk.cache_enabled, chunk.blobs.len()), (cached, usize::from(cached)), "{column:?}");
        last.insert(column, chunk);
    }
    let missing: Vec<&Column> = held.iter().filter(|column| !last.contains_key(*column)).collect();
    assert!(missing.is_empty(), "held before and never sent again: {missing:?}");
    let covered = last.keys().filter(|column| in_new_view(**column)).count();
    assert_eq!(covered, 49, "the whole new view");
}

#[test]
fn a_move_inside_a_shard_empties_the_columns_the_new_view_does_not_cover() {
    let mut c = Cluster::new(&[&["hub", "arena"]]);
    c.enter(1, "Alex");
    let held = walk_east(&mut c, 1);
    c.transfer(0, 1, "arena");
    let sent = c.ticks(6);
    assert_swapped(&held, sent.to(1), false);

    // And back, from the far end of the arena.
    let held = walk_east(&mut c, 1);
    c.transfer(0, 1, "hub");
    let sent = c.ticks(6);
    assert_swapped(&held, sent.to(1), false);
}

/// acacia-bot answers every announced column with a request, one for no sections if the column
/// has none: a view's worth of emptied columns must not use up the room real requests need.
#[test]
fn requests_for_no_sections_do_not_crowd_out_real_ones() {
    let mut c = Cluster::new(&[&["hub"]]);
    c.enter(1, "Alex");
    let request = |sections: Vec<Vec3i8>| SubchunkRequest { dimension: 0, requests: sections, origin: Vec3li { x: 0, y: 0, z: 0 } };
    for _ in 0..200 {
        c.receive(1, &request(Vec::new()));
    }
    c.receive(1, &request(vec![Vec3i8 { x: 0, y: 4, z: 0 }]));
    assert_eq!(all::<Subchunk>(c.tick().to(1)).len(), 1, "the one request with a section is answered");
}

#[test]
fn a_move_across_shards_empties_them_too_also_for_a_blob_cache() {
    let mut c = Cluster::new(&[&["hub"], &["arena"]]);
    c.enter(1, "Alex");
    c.receive(1, &ClientCacheStatus { enabled: true });
    let held = walk_east(&mut c, 1);
    c.transfer(0, 1, "arena");
    // Columns the old shard still announces while the other is asked are held as well.
    let sent = c.ticks(8);
    assert_eq!(c.owner(1), Some(1));
    assert_swapped(&held, sent.to(1), true);

    // An empty column names the one biome blob; the client may ask for it like any other.
    let emptied: LevelChunk = all::<LevelChunk>(sent.to(1)).into_iter().find(|chunk| !in_new_view((chunk.x, chunk.z))).unwrap();
    c.receive(1, &ClientCacheBlobStatus { missing: emptied.blobs.clone(), have: vec![] });
    let replies = all::<ClientCacheMissResponse>(c.tick().to(1));
    assert_eq!((replies.len(), replies[0].blobs.len(), replies[0].blobs[0].hash), (1, 1, emptied.blobs[0]));

    let held = walk_east(&mut c, 1);
    c.transfer(1, 1, "hub");
    let sent = c.ticks(8);
    assert_eq!(c.owner(1), Some(0));
    assert_swapped(&held, sent.to(1), true);
}
