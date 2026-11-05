//! Shots `/summon` makes (research/captures/bds-s7shots.txt): still the tick they come, then
//! falling by their own weight, shown and nameable a tick on, `MOVING` as any entity.

use crate::common;

use acacia_proto::packets::{AddEntity, MoveEntityDelta, SetEntityData};
use acacia_proto::types::{MetadataDictionaryItemValue as Value, MetadataFlags1 as Flags};
use common::{Harness, all};

#[test]
fn a_summoned_shot_falls_from_the_tick_after_and_is_shown_then() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    for (index, (kind, weight, ticks_to_moving)) in [("minecraft:arrow", 0.05f32, 1), ("minecraft:snowball", 0.03, 2)].into_iter().enumerate() {
        let x = index as f32 * 3.0 + 0.5;
        h.remote.run(move |ctx| assert!(ctx.summon_as(None, kind, [x, 75.0, 4.5], 0.0, 0.0, None).is_some()));
        let sent = h.tick(1);
        let added = all::<AddEntity>(&sent).into_iter().find(|added| added.entity_type == kind).expect("announced");
        let id = added.runtime_id;
        let moves = |sent: &[acacia_proto::RawPacket]| all::<MoveEntityDelta>(sent).into_iter().filter(|moved| moved.runtime_entity_id == id).filter_map(|moved| moved.y).collect::<Vec<f32>>();
        let words = |sent: &[acacia_proto::RawPacket]| {
            let data = all::<SetEntityData>(sent).into_iter().filter(|data| data.runtime_entity_id == id && data.metadata.len() == 1);
            data.flat_map(|data| data.metadata).filter_map(|item| if let Value::Flags(flags) = item.value { Some(flags.0) } else { None }).collect::<Vec<i64>>()
        };
        assert!(moves(&sent).is_empty(), "{kind} stays put the tick it came");
        let next = h.tick(1);
        assert_eq!(moves(&next).len(), 1, "{kind}");
        assert!((moves(&next)[0] - (75.0 - weight)).abs() < 1e-4, "{kind}: {:?}", moves(&next));
        let [shown] = words(&next).try_into().expect("its word");
        assert_eq!((shown & Flags::INVISIBLE.0, shown & Flags::CAN_SHOW_NAMETAG.0, shown & Flags::MOVING.0), (0, Flags::CAN_SHOW_NAMETAG.0, 0), "{kind}");
        // A snowball's first fall, 0.03, is short of the moving rule's: MOVING a tick later, as on BDS.
        let words_on: Vec<Vec<i64>> = (0..ticks_to_moving).map(|_| words(&h.tick(1))).collect();
        let [moving] = words_on.concat().try_into().expect("MOVING on");
        assert_eq!(moving, shown | Flags::MOVING.0, "{kind}");
    }
}
