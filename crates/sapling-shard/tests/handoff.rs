//! A player's move to a world on another shard, stepped through message by message, and what
//! happens when a player or a shard goes away in the middle (harness: common/cluster.rs).

mod common;

use acacia_proto::Packet;
use acacia_proto::packets::{
    AddPlayer, LevelChunk, ModalFormResponse, MovePlayer, NetworkChunkPublisherUpdate, PlayerList, RemoveEntity, RequestChunkRadius,
    ResourcePackClientResponseResponseStatus as PackStatus, SetLocalPlayerAsInitialized, SyncWorldClocks, Text,
};
use common::cluster::Cluster;
use common::{all, ids, pack_response};
use sapling_api::{BossBar, Menu, PlayerId, Sidebar};

/// Alex (connection and entity 1) and Steve (2) in the hub on shard 0; the arena is on shard 1.
fn two_shards() -> Cluster {
    let mut c = Cluster::new(&[&["hub"], &["arena"]]);
    c.enter(1, "Alex");
    c.enter(2, "Steve");
    assert_eq!(c.journal(), ["0: join Alex in hub", "0: join Steve in hub"]);
    c
}

/// Steve's transfer up to the commit: asked, answered, handed over. The network has not been
/// told yet (`Cluster::pump`), so it still sends Steve's packets to shard 0.
fn commit(c: &mut Cluster) {
    c.transfer(0, 2, "arena");
    c.tick_shard(0);
    c.tick_shard(1);
    c.tick_shard(0);
    assert_eq!(c.journal(), ["0: transfer 2 to arena: true"]);
}

#[test]
fn a_player_moves_to_a_world_on_another_shard() {
    let mut c = two_shards();
    c.run(0, |ctx| {
        ctx.set_sidebar(PlayerId(2), &Sidebar::new("Hub").line("one"));
        ctx.set_boss_bar(PlayerId(2), &BossBar::new("bar"));
        ctx.send_form(PlayerId(2), Menu::new("Menu").button("ok"), |ctx, player, _| ctx.broadcast(&format!("{} answered in {}", player.name, ctx.world())));
    });
    c.tick();
    commit(&mut c);
    assert_eq!(c.owner(2), Some(0), "until the directory is told");
    let sent = c.pump();
    assert_eq!((c.owner(2), ids(sent.to(1)), sent.to(2).len()), (Some(1), vec![RemoveEntity::ID], 0), "the old shard's last words travel with the player");

    let sent = c.ticks(5);
    let first = [RemoveEntity::ID, MovePlayer::ID, SyncWorldClocks::ID, NetworkChunkPublisherUpdate::ID];
    assert_eq!(ids(sent.to(2))[..4], first, "the hub's players gone, then the arena's spawn, time and columns");
    assert_eq!((all::<LevelChunk>(sent.to(2)).len(), sent.to(2).len(), sent.to(1).len()), (49, 53, 0));
    assert_eq!(c.journal(), ["1: change Steve hub -> arena (1 here)"], "no quit, no join");
    assert_eq!(c.census(), ["0: 1 [Alex in hub]", "1: 1 [Steve in arena]"]);

    // What the client was shown stays as it is: nothing of the HUD is sent again, a change is
    // a change, and the form's answer runs on the shard the player is on now.
    c.run(1, |ctx| {
        ctx.set_sidebar(PlayerId(2), &Sidebar::new("Hub").line("one"));
        ctx.set_boss_bar(PlayerId(2), &BossBar::new("bar"));
    });
    assert!(c.tick().to(2).is_empty());
    c.receive(2, &ModalFormResponse { form_id: 0, has_response_data: true, data: Some("0".into()), has_cancel_reason: false, content: None });
    assert_eq!(ids(c.tick().to(2)), [Text::ID]);

    // Chat is the world's; the list is the server's; the other follows and they meet again.
    c.chat(2, "alone here");
    c.note(1, |ctx| {
        ctx.broadcast_in("hub", "to the hub");
        ctx.broadcast_all("to all");
        format!("{:?}", ctx.worlds())
    });
    let sent = c.ticks(2);
    assert_eq!((ids(sent.to(2)), ids(sent.to(1))), (vec![Text::ID, Text::ID], vec![Text::ID, Text::ID]));
    assert_eq!(c.journal(), ["1: chat Steve alone here in arena", r#"1: ["arena", "hub"]"#]);
    c.transfer(0, 1, "arena");
    let sent = c.ticks(8);
    assert_eq!(all::<AddPlayer>(sent.to(1)).iter().map(|p| p.runtime_id).collect::<Vec<_>>(), [2], "ids are the server's: Steve is still 2");
    assert_eq!(all::<AddPlayer>(sent.to(2)).iter().map(|p| p.runtime_id).collect::<Vec<_>>(), [1]);
    assert!(all::<PlayerList>(sent.to(1)).is_empty() && all::<PlayerList>(sent.to(2)).is_empty());
    assert_eq!(c.census(), ["0: 0 []", "1: 2 [Alex in arena, Steve in arena]"]);
}

#[test]
fn the_player_list_spans_the_shards() {
    let mut c = two_shards();
    c.transfer(0, 2, "arena");
    c.ticks(8);
    c.journal();
    // A newcomer to the hub is listed for Steve on the other shard and has him in its own list.
    c.join(3, "Late");
    c.tick();
    c.receive(3, &pack_response(PackStatus::HaveAllPacks));
    c.tick();
    c.receive(3, &pack_response(PackStatus::Completed));
    c.tick();
    c.receive(3, &RequestChunkRadius { chunk_radius: 4, max_radius: 4 });
    c.ticks(5);
    c.receive(3, &SetLocalPlayerAsInitialized { runtime_entity_id: 0 });
    let sent = c.ticks(2);
    assert_eq!((all::<PlayerList>(sent.to(3)).len(), ids(sent.to(2))), (3, vec![PlayerList::ID]));
    assert_eq!(all::<AddPlayer>(sent.to(3)).len(), 1, "only Alex is in its world");

    c.leave(3);
    let sent = c.ticks(2);
    assert_eq!((ids(sent.to(2)), ids(sent.to(1))), (vec![PlayerList::ID], vec![RemoveEntity::ID, PlayerList::ID]));
    // A player who quits on the shard it moved to is unlisted on the one it came from.
    c.leave(2);
    let sent = c.ticks(2);
    assert_eq!(ids(sent.to(1)), [PlayerList::ID]);
    assert_eq!(c.journal(), ["0: join Late in hub", "0: quit Late in hub", "1: quit Steve in arena"]);
}

#[test]
fn packets_during_a_handoff_reach_the_new_shard_in_order_and_never_the_old() {
    let mut c = two_shards();
    c.chat(2, "before");
    c.tick();
    assert_eq!(c.journal(), ["0: chat Steve before in hub"]);
    commit(&mut c);
    // Committed, but the network still sends to the old shard: it keeps these for the forward.
    c.chat(2, "one");
    c.chat(2, "two");
    c.tick_shard(0);
    c.pump();
    // The directory has switched: this one goes straight to the new shard, ahead of the forward.
    c.chat(2, "three");
    c.tick_shard(1);
    assert_eq!(c.journal(), ["1: change Steve hub -> arena (1 here)"], "the player is there, its packets wait for the forward");
    c.tick_shard(0);
    c.tick_shard(1);
    assert_eq!(c.journal(), ["1: chat Steve one in arena", "1: chat Steve two in arena", "1: chat Steve three in arena"]);
    c.chat(2, "after");
    c.tick();
    assert_eq!(c.journal(), ["1: chat Steve after in arena"]);
}

#[test]
fn a_player_who_disconnects_during_a_handoff_is_gone_from_both_shards() {
    // Before the answer: it leaves the old shard as anyone does, and the answer finds nobody.
    let mut c = two_shards();
    c.transfer(0, 2, "arena");
    c.tick_shard(0);
    c.leave(2);
    c.ticks(3);
    assert_eq!(c.journal(), ["0: transfer 2 to arena: true", "0: quit Steve in hub"]);
    assert_eq!(c.census(), ["0: 1 [Alex in hub]", "1: 0 []"]);

    // After the commit, before the directory is told: the old shard forwards the leave.
    let mut c = two_shards();
    commit(&mut c);
    c.leave(2);
    c.ticks(3);
    assert_eq!(c.journal(), ["1: change Steve hub -> arena (1 here)", "1: quit Steve in arena"]);
    assert_eq!((c.census(), c.owner(2)), (vec!["0: 1 [Alex in hub]".to_owned(), "1: 0 []".to_owned()], None));

    // After the directory switched, before the forward: the leave waits for it on the new shard.
    let mut c = two_shards();
    commit(&mut c);
    c.pump();
    c.leave(2);
    c.tick_shard(1);
    let sent = c.ticks(3);
    assert_eq!(c.journal(), ["1: change Steve hub -> arena (1 here)", "1: quit Steve in arena"]);
    assert_eq!(ids(sent.to(1)), [PlayerList::ID], "unlisted once, by the shard it quit on");
    assert_eq!((c.census(), c.owner(2)), (vec!["0: 1 [Alex in hub]".to_owned(), "1: 0 []".to_owned()], None));
}

#[test]
fn a_transfer_that_cannot_be_made_leaves_the_player_where_it_is() {
    // A shard that never answers: the wait ends, the handlers are told, the player can go on.
    let mut c = two_shards();
    c.transfer(0, 2, "arena");
    c.tick_shard(0);
    c.transfer(0, 2, "arena");
    (0..110).for_each(|_| c.tick_shard(0));
    assert_eq!(c.journal(), ["0: transfer 2 to arena: true", "0: transfer 2 to arena: false", "0: failed Steve arena, still in Some(\"hub\")"]);
    c.chat(2, "still here");
    c.tick_shard(0);
    assert_eq!((c.journal(), c.owner(2)), (vec!["0: chat Steve still here in hub".to_owned()], Some(0)));

    // A shard that answers and stops before the commit: the commit comes back.
    let mut c = two_shards();
    c.transfer(0, 2, "arena");
    c.tick_shard(0);
    c.tick_shard(1);
    c.stop(1);
    let sent = c.tick();
    assert_eq!(c.journal(), ["0: transfer 2 to arena: true", "0: failed Steve arena, still in Some(\"hub\")"]);
    assert_eq!(ids(sent.to(2)), [RemoveEntity::ID, AddPlayer::ID], "what the handoff took off its screen is back");
    assert_eq!((c.census(), c.owner(2)), (vec!["0: 2 [Alex in hub, Steve in hub]".to_owned()], Some(0)));

    // A shard that is gone, and the world that went with it: refused at once.
    c.transfer(0, 2, "arena");
    c.tick();
    assert_eq!(c.journal(), ["0: transfer 2 to arena: false"]);
    c.note(0, |ctx| format!("{:?}", ctx.worlds()));
    c.tick();
    assert_eq!(c.journal(), [r#"0: ["hub"]"#]);
}

#[test]
fn a_shard_that_stops_during_a_handoff_leaves_nothing_dangling() {
    // The new shard stops with the commit in its mailbox: it takes the player in and
    // disconnects it with its own; the old shard owes a forward nobody waits for.
    let mut c = two_shards();
    commit(&mut c);
    c.stop(1);
    assert_eq!((c.kicked.clone(), c.owner(2)), (vec![2], Some(1)), "the network closes a connection routed to a shard that is gone");
    c.leave(2);
    c.ticks(3);
    assert_eq!(c.census(), ["0: 1 [Alex in hub]"]);

    // The old shard stops after the commit: its forward goes out as it is, the player plays on.
    let mut c = two_shards();
    commit(&mut c);
    c.chat(2, "one");
    c.tick_shard(0);
    c.stop(0);
    assert_eq!(c.kicked, [1], "its own players only");
    c.chat(2, "two");
    c.ticks(2);
    assert_eq!(c.journal(), ["1: change Steve hub -> arena (1 here)", "1: chat Steve one in arena", "1: chat Steve two in arena"]);
    assert_eq!((c.census(), c.owner(2)), (vec!["1: 1 [Steve in arena]".to_owned()], Some(1)));
}

#[test]
fn hundreds_of_transfers_leave_every_player_on_exactly_one_shard() {
    let mut c = Cluster::new(&[&["hub", "yard"], &["arena", "pit"]]);
    let names = ["Alex", "Steve", "Efe", "Kai"];
    names.iter().zip(1..).for_each(|(name, conn)| c.enter(conn, name));
    c.journal();
    let worlds = ["hub", "yard", "arena", "pit"];
    let mut seed = 20261004u64;
    let mut next = |below: u64| {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (seed >> 33) % below
    };
    for round in 0..400 {
        let (conn, world) = (1 + next(4), worlds[next(4) as usize]);
        // Asked of both shards: the one that has the player acts, the other refuses.
        c.transfer(0, conn, world);
        c.transfer(1, conn, world);
        c.chat(conn, &format!("round {round}"));
        match next(3) {
            0 => drop(c.tick()),
            1 => {
                c.tick_shard(1);
                c.tick_shard(0);
                c.pump();
            }
            _ => drop(c.ticks(2)),
        }
    }
    c.ticks(120);
    let journal = c.journal();
    let count = |what: &str| journal.iter().filter(|line| line.contains(what)).count();
    assert_eq!((count("join"), count("quit"), count("failed")), (0, 0, 0));
    assert_eq!(count(" chat "), 400, "every line was heard once, by one shard");
    assert!(count("change") > 100, "{} moves", count("change"));
    let census = c.census().join(" | ");
    for name in names {
        assert_eq!(census.matches(name).count(), 1, "{census}");
    }
    assert!(c.kicked.is_empty());
    // Every player is where the network sends its packets, and can still be moved.
    for conn in 1..=4 {
        let owner = c.owner(conn).unwrap();
        c.note(owner, move |ctx| format!("has {conn}: {}", ctx.world_of(PlayerId(conn)).is_some()));
    }
    c.tick();
    let mut owned: Vec<String> = c.journal().into_iter().map(|line| line[3..].to_owned()).collect();
    owned.sort();
    assert_eq!(owned, ["has 1: true", "has 2: true", "has 3: true", "has 4: true"]);
}
