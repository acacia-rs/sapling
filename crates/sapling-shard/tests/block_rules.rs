//! Block events among several handlers, the per-tick limits, and what a later player is sent of
//! a changed column (harness: common/mod.rs).

mod common;

use std::sync::{Arc, Mutex};

use acacia_proto::packets::{
    ClientCacheBlobStatus, ClientCacheMissResponse, ClientCacheStatus, RequestChunkRadius,
    ResourcePackClientResponseResponseStatus as PackStatus, Subchunk, SubchunkRequest, UpdateBlock,
};
use acacia_proto::types::{Action, Vec3i8, Vec3li};
use acacia_proto::xxh64;
use common::{Harness, all, block_actions, click, hash, pack_response};
use sapling_api::{
    BreakEvent, GameMode, Handler, InteractEvent, PlaceEvent, Player, PlayerId, Priority, ShardCtx,
};
use sapling_world::{BlockRegistry, Chunk, Dimension};

type Log = Arc<Mutex<Vec<String>>>;

/// Logs each block event it is shown, then cancels it at x `cancels` or allows it at x `allows`.
struct Named {
    name: &'static str,
    cancels: i32,
    allows: i32,
    log: Log,
}

impl Named {
    /// `Some(true)` to allow, `Some(false)` to cancel.
    fn rule(&self, kind: &str, x: i32, cancelled: bool) -> Option<bool> {
        self.log.lock().unwrap().push(format!(
            "{} {kind} {x}{}",
            self.name,
            if cancelled { " cancelled" } else { "" }
        ));
        (x == self.allows)
            .then_some(true)
            .or((x == self.cancels).then_some(false))
    }
}

macro_rules! ruled {
    ($self:ident, $kind:literal, $event:ident) => {
        match $self.rule($kind, $event.pos()[0], $event.is_cancelled()) {
            Some(true) => $event.allow(),
            Some(false) => $event.cancel(),
            None => {}
        }
    };
}

impl Handler for Named {
    fn on_break(&mut self, _: &mut ShardCtx<'_>, _: &Player, event: &mut BreakEvent) {
        ruled!(self, "break", event);
    }

    fn on_place(&mut self, _: &mut ShardCtx<'_>, _: &Player, event: &mut PlaceEvent) {
        ruled!(self, "place", event);
    }

    fn on_interact(&mut self, _: &mut ShardCtx<'_>, _: &Player, event: &mut InteractEvent) {
        ruled!(self, "click", event);
    }
}

const NEVER: i32 = i32::MAX;

/// One player in `mode` and handlers added out of order: `high` cancels at x 3, `low` allows at
/// x 2, and a monitor tries to allow everything.
fn harness(mode: GameMode) -> (Harness, Log) {
    let mut h = Harness::new();
    let log = Log::default();
    let handlers = [
        (Priority::Monitor, "monitor", NEVER, 1),
        (Priority::Low, "low", NEVER, 2),
        (Priority::Highest, "highest", NEVER, NEVER),
        (Priority::High, "high", 3, NEVER),
        (Priority::Normal, "normal", NEVER, NEVER),
    ];
    for (priority, name, cancels, allows) in handlers {
        h.shard.add_handler_at(
            priority,
            Box::new(Named {
                name,
                cancels,
                allows,
                log: log.clone(),
            }),
        );
    }
    h.enter(1, "Alex");
    h.remote
        .run(move |ctx| ctx.set_game_mode(PlayerId(1), mode));
    h.tick(1);
    (h, log)
}

fn taken(log: &Log) -> Vec<String> {
    std::mem::take(&mut *log.lock().unwrap())
}

/// The layer-0 blocks `conn` was told of, as (position, block).
fn changes(h: &mut Harness, conn: u64) -> Vec<([i32; 3], u32)> {
    let sent = h.tick_all();
    all::<UpdateBlock>(sent.to(conn))
        .iter()
        .filter(|u| u.layer == 0)
        .map(|u| {
            (
                [u.position.x, u.position.y, u.position.z],
                u.block_runtime_id,
            )
        })
        .collect()
}

const STONE: &str = "minecraft:stone";
const AIR: &str = "minecraft:air";

#[test]
fn a_cancel_stops_later_handlers_and_a_monitor_changes_nothing() {
    let (mut h, log) = harness(GameMode::Creative);
    h.receive(1, &block_actions(&[(Action::PredictBreak, [3, 64, 0])]));
    assert_eq!(
        changes(&mut h, 1),
        [([3, 64, 0], hash(STONE)), ([3, 65, 0], hash(AIR))],
        "cancelled: shown again"
    );
    assert_eq!(
        taken(&log),
        [
            "highest break 3",
            "high break 3",
            "monitor break 3 cancelled"
        ]
    );

    // Nobody objects at x 1; the monitor's own verdict counts for nothing either way.
    h.receive(1, &block_actions(&[(Action::PredictBreak, [1, 64, 0])]));
    assert_eq!(changes(&mut h, 1), [([1, 64, 0], hash(AIR))]);
    assert_eq!(
        taken(&log),
        [
            "highest break 1",
            "high break 1",
            "normal break 1",
            "low break 1",
            "monitor break 1"
        ]
    );
}

#[test]
fn an_event_that_starts_cancelled_is_shown_to_every_handler_and_may_be_allowed() {
    let (mut h, log) = harness(GameMode::Adventure);
    h.receive(1, &block_actions(&[(Action::PredictBreak, [2, 64, 0])]));
    assert_eq!(
        changes(&mut h, 1),
        [([2, 64, 0], hash(AIR))],
        "the low handler opened x 2"
    );
    let all_saw = [
        "highest break 2 cancelled",
        "high break 2 cancelled",
        "normal break 2 cancelled",
        "low break 2 cancelled",
        "monitor break 2",
    ];
    assert_eq!(taken(&log), all_saw);

    // Only the monitor would allow x 1: the break stays refused.
    h.receive(1, &block_actions(&[(Action::PredictBreak, [1, 64, 0])]));
    assert_eq!(
        changes(&mut h, 1),
        [([1, 64, 0], hash(STONE)), ([1, 65, 0], hash(AIR))]
    );
    assert_eq!(taken(&log).last().unwrap(), "monitor break 1 cancelled");

    // Places and clicks go the same way: allowed at x 2, cancelled by `high` at x 3.
    h.receive(1, &click([2, 64, -1], 1, Some(STONE)));
    h.receive(1, &click([3, 64, -1], 1, Some(STONE)));
    h.receive(1, &click([3, 64, -2], 1, None));
    let placed = changes(&mut h, 1);
    assert_eq!(placed[0], ([2, 65, -1], hash(STONE)));
    assert!(
        placed[1..]
            .iter()
            .all(|(pos, block)| *block == hash(if pos[1] == 64 { STONE } else { AIR })),
        "the rest is resync: {placed:?}"
    );
    let seen = taken(&log);
    assert_eq!(
        seen[..5],
        [
            "highest place 2 cancelled",
            "high place 2 cancelled",
            "normal place 2 cancelled",
            "low place 2 cancelled",
            "monitor place 2"
        ]
    );
    assert_eq!(
        seen[5..8],
        [
            "highest place 3 cancelled",
            "high place 3 cancelled",
            "monitor place 3 cancelled"
        ]
    );
    assert_eq!(
        seen[8..],
        [
            "highest click 3",
            "high click 3",
            "monitor click 3 cancelled"
        ]
    );
}

#[test]
fn block_requests_beyond_a_ticks_limit_are_dropped() {
    let (mut h, _) = harness(GameMode::Creative);
    h.enter(2, "Steve");
    for x in -3..3 {
        h.receive(1, &click([x, 64, -2], 1, Some(STONE)));
    }
    let placed = changes(&mut h, 2);
    assert_eq!(
        placed,
        [-3, -2, -1, 0].map(|x| ([x, 65, -2], hash(STONE))),
        "four clicks a tick"
    );

    // Twenty finished breaks in one input and four in the next: sixteen actions a tick.
    let cells: Vec<_> = (-2..=2)
        .flat_map(|x| (1..=4).map(move |z| (Action::PredictBreak, [x, 64, z])))
        .collect();
    h.receive(1, &block_actions(&cells));
    h.receive(
        1,
        &block_actions(&[
            (Action::PredictBreak, [4, 64, 0]),
            (Action::PredictBreak, [4, 64, 1]),
        ]),
    );
    let broken = changes(&mut h, 2);
    assert_eq!(broken.len(), 16);
    assert!(
        broken
            .iter()
            .zip(&cells)
            .all(|(told, (_, pos))| told == &(*pos, hash(AIR)))
    );
    // The next tick has its own allowance.
    h.receive(1, &block_actions(&[(Action::PredictBreak, [4, 64, 0])]));
    assert_eq!(changes(&mut h, 2), [([4, 64, 0], hash(AIR))]);
}

fn section_request() -> SubchunkRequest {
    SubchunkRequest {
        dimension: 0,
        requests: vec![Vec3i8 { x: 0, y: 4, z: 0 }],
        origin: Vec3li { x: 0, y: 0, z: 0 },
    }
}

/// The blocks at (1, 64, 0) and (0, 64, 0) of an encoded section 4.
fn decoded(payload: &[u8]) -> [&'static str; 2] {
    let registry = BlockRegistry::vanilla();
    let mut chunk = Chunk::empty(0, 0, Dimension::overworld(registry.air_id()));
    chunk
        .set_sub_chunk_mapped(4, payload, &|hash| {
            registry.runtime_id_from_hash(hash).unwrap()
        })
        .unwrap();
    [chunk.block(1, 64, 0), chunk.block(0, 64, 0)].map(|id| registry.get(id).unwrap().name)
}

#[test]
fn a_player_who_loads_the_column_later_gets_the_changed_section_under_a_new_hash() {
    let (mut h, _) = harness(GameMode::Creative);
    // The section is encoded and cached before the break.
    h.receive(1, &section_request());
    let before = all::<Subchunk>(&h.tick(1))[0].entries[0]
        .payload
        .clone()
        .expect("a payload");
    assert_eq!(decoded(&before), [STONE, STONE]);
    h.receive(1, &block_actions(&[(Action::PredictBreak, [1, 64, 0])]));
    h.tick(1);
    h.receive(1, &section_request());
    let after = all::<Subchunk>(&h.tick(1))[0].entries[0]
        .payload
        .clone()
        .expect("a payload");
    assert_eq!(decoded(&after), [AIR, STONE]);

    // A client with a blob cache joins: it is named the new blob, never the old one.
    h.join(2, "Steve");
    h.tick_all();
    h.receive(2, &pack_response(PackStatus::HaveAllPacks));
    h.tick_all();
    h.receive(2, &pack_response(PackStatus::Completed));
    h.tick_all();
    h.receive(
        2,
        &RequestChunkRadius {
            chunk_radius: 4,
            max_radius: 4,
        },
    );
    h.receive(2, &ClientCacheStatus { enabled: true });
    (0..5).for_each(|_| drop(h.tick_all()));
    h.receive(2, &section_request());
    let answer = all::<Subchunk>(h.tick_all().to(2));
    let named = answer[0].entries[0].blob_id.expect("a blob hash");
    assert!(answer[0].cache_enabled && named == xxh64(&after) && named != xxh64(&before));
    h.receive(
        2,
        &ClientCacheBlobStatus {
            missing: vec![named],
            have: vec![],
        },
    );
    let blobs = all::<ClientCacheMissResponse>(h.tick_all().to(2));
    assert_eq!(
        (blobs[0].blobs[0].hash, decoded(&blobs[0].blobs[0].payload)),
        (named, [AIR, STONE])
    );
}
