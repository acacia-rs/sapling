//! Snow golems, against research/captures/bds-snowgolem.txt.

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, SetEntityData};
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain};
use common::{Harness, all, asking};
use sapling_api::Difficulty;

fn added(sent: &[RawPacket], kind: &str) -> Option<AddEntity> {
    all::<AddEntity>(sent).into_iter().find(|added| added.entity_type == kind)
}

#[test]
fn a_snow_golem_leaves_snow_takes_a_monster_within_6_and_throws_snowballs_21_ticks_on_then_every_20() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        ctx.set_time(18000);
        assert!(ctx.spawn_entity("minecraft:snow_golem", [-6.5, 65.0, -6.5]).is_some());
    });
    let first: Vec<RawPacket> = (0..3).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    let golem = added(&first, "minecraft:snow_golem").expect("shown").runtime_id;
    let snow = asking::ask(&mut h, |ctx| ctx.block([-7, 65, -7]).map(|block| block.name));
    assert_eq!(snow.as_deref(), Some("minecraft:snow_layer"), "snow where it stands on plains");
    h.remote.run(|ctx| assert!(ctx.spawn_entity("minecraft:zombie", [-1.5, 65.0, -6.5]).is_some()));
    let ticks: Vec<Vec<RawPacket>> = (0..80).map(|_| h.tick_all().to(1).to_vec()).collect();
    let zombie = ticks.iter().find_map(|sent| added(sent, "minecraft:zombie")).expect("shown").unique_id;
    let targets = |sent: &[RawPacket]| all::<SetEntityData>(sent).into_iter().filter(|data| data.runtime_entity_id == golem).flat_map(|data| data.metadata).any(|item| item.key == Key::TargetEid && item.value == Value::Default(Plain::Long(zombie)));
    let took = ticks.iter().position(|sent| targets(sent)).expect("it takes the zombie");
    let throws: Vec<usize> = ticks.iter().enumerate().filter(|(_, sent)| added(sent, "minecraft:snowball").is_some()).map(|(tick, _)| tick - took).collect();
    assert_eq!(&throws[..2], [21, 41], "{throws:?}");
}
