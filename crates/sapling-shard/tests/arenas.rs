//! Worlds made from a template at runtime: separate for those who play in them, one map in
//! memory and on the wire until they change it (harness: common/cluster.rs).

mod common;

use acacia_proto::Packet;
use acacia_proto::packets::{
    ClientCacheStatus, LevelChunk, MovePlayer, Subchunk, SubchunkRequest, SyncWorldClocks,
    UpdateBlock,
};
use acacia_proto::types::{Action as BlockAction, InputData, Vec3i8, Vec3li};
use common::cluster::{Cluster, platform_of};
use common::{all, block_actions, ids};
use sapling_world::Template;

/// Alex (1) and Steve (2), both with a blob cache, in the hub of a shard that knows the template `map`.
fn with_a_template() -> Cluster {
    let mut c = Cluster::new(&[&["hub"]]);
    c.shards[0].as_mut().unwrap().add_template(
        "map",
        Template::of(platform_of("minecraft:sandstone")),
        [0.5, 65.0, 0.5],
    );
    for (conn, name) in [(1, "Alex"), (2, "Steve")] {
        c.enter(conn, name);
        c.receive(conn, &ClientCacheStatus { enabled: true });
    }
    c.journal();
    c
}

/// Asks for the platform's section under the spawn and returns the blob the answer names.
fn platform_blob(c: &mut Cluster, conn: u64) -> u64 {
    c.receive(
        conn,
        &SubchunkRequest {
            dimension: 0,
            requests: vec![Vec3i8 { x: 0, y: 4, z: 0 }],
            origin: Vec3li { x: 0, y: 0, z: 0 },
        },
    );
    all::<Subchunk>(c.tick().to(conn))[0].entries[0]
        .blob_id
        .expect("a section hash")
}

#[test]
fn arenas_share_their_template_until_one_is_changed_and_again_after_a_reset() {
    let mut c = with_a_template();
    c.note(0, |ctx| {
        let made = (0..50)
            .filter(|game| ctx.create_world(&format!("game{game}"), "map"))
            .count();
        format!(
            "{made} {} {} {}",
            ctx.create_world("game0", "map"),
            ctx.create_world("other", "no map"),
            ctx.worlds().len()
        )
    });
    c.transfer(0, 1, "game0");
    c.transfer(0, 2, "game1");
    c.ticks(7);
    assert_eq!(c.journal()[0], "0: 50 false false 51");
    let shared = platform_blob(&mut c, 1);
    assert_eq!(
        platform_blob(&mut c, 2),
        shared,
        "one blob for every arena: a client's cache holds the map once"
    );

    // Alex digs in his arena: nobody else's world changes, the template's least of all.
    let mut dig = block_actions(&[(BlockAction::PredictBreak, [0, 64, 0])]);
    dig.input_data.push(InputData::HandledTeleport);
    c.receive(1, &dig);
    let sent = c.tick();
    assert_eq!(
        (all::<UpdateBlock>(sent.to(1)).len(), sent.to(2).len()),
        (1, 0)
    );
    let blocks = |ctx: &mut sapling_api::ShardCtx<'_>| {
        ctx.create_world("late", "map");
        let at = |ctx: &mut sapling_api::ShardCtx<'_>, world: &str| {
            ctx.block_in(world, [0, 64, 0])
                .map(|block| block.name.replace("minecraft:", ""))
        };
        format!(
            "{:?} {:?} {:?} {:?}",
            at(ctx, "game0"),
            at(ctx, "game1"),
            at(ctx, "game7"),
            at(ctx, "late")
        )
    };
    c.note(0, blocks);
    c.tick();
    assert_eq!(
        c.journal(),
        [r#"0: Some("air") Some("sandstone") Some("sandstone") Some("sandstone")"#]
    );
    assert_ne!(
        platform_blob(&mut c, 1),
        shared,
        "his section is his own now"
    );
    assert_eq!(platform_blob(&mut c, 2), shared);

    // A reset drops what he changed: the column is announced again and is the template's.
    c.note(0, |ctx| {
        format!(
            "{} {} {}",
            ctx.reset_world("game0"),
            ctx.reset_world("hub"),
            ctx.reset_world("nowhere")
        )
    });
    let sent = c.ticks(2);
    let again: Vec<(i32, i32)> = all::<LevelChunk>(sent.to(1))
        .iter()
        .map(|chunk| (chunk.x, chunk.z))
        .collect();
    assert_eq!(
        (again, sent.to(2).len(), c.journal()),
        (vec![(0, 0)], 0, vec!["0: true false false".to_owned()])
    );
    assert_eq!(platform_blob(&mut c, 1), shared);
    assert_eq!(c.census(), ["0: 2 [Alex in game0, Steve in game1]"]);
}

#[test]
fn removing_an_arena_sends_its_players_to_the_first_world() {
    let mut c = with_a_template();
    c.run(0, |ctx| assert!(ctx.create_world("game", "map")));
    c.transfer(0, 1, "game");
    c.ticks(7);
    c.journal();
    c.note(0, |ctx| {
        format!(
            "{} {} {} {:?}",
            ctx.remove_world("game"),
            ctx.remove_world("hub"),
            ctx.remove_world("game"),
            ctx.worlds()
        )
    });
    let sent = c.tick();
    assert_eq!(
        ids(sent.to(1))[..2],
        [MovePlayer::ID, SyncWorldClocks::ID],
        "as any move to another world"
    );
    assert_eq!(
        c.journal(),
        [
            r#"0: true false false ["hub"]"#,
            "0: change Alex game -> hub (2 here)"
        ]
    );
    c.transfer(0, 1, "game");
    let sent = c.ticks(6);
    assert_eq!(
        all::<LevelChunk>(sent.to(1)).len(),
        49,
        "the hub's columns again"
    );
    assert_eq!(c.journal(), ["0: transfer 1 to game: false"]);
    assert_eq!(c.census(), ["0: 2 [Alex in hub, Steve in hub]"]);
}
