//! Group AK of the fourteenth wave: the wither's kept prey, told as `TargetEid` with `mob.warning` a
//! tick on (wither/target.rs); docs/research/wither-lightning-fireworks.md.

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, LevelSoundEvent, SetEntityData};
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain};
use common::{Harness, all};
use sapling_api::{Difficulty, EntityId, GameMode, PlayerId};

const WITHER: &str = "minecraft:wither";
const CHARGE: usize = 200;

/// The ticks, from 0, in which `key` was told of `id`, with its value.
fn told(ticks: &[Vec<RawPacket>], id: u64, key: Key) -> Vec<(usize, i64)> {
    let long = |value: &Value| if let Value::Default(Plain::Long(long)) = value { Some(*long) } else { None };
    let of = |data: &SetEntityData| data.metadata.iter().filter(|item| item.key == key).filter_map(|item| long(&item.value)).collect::<Vec<_>>();
    ticks.iter().enumerate().flat_map(|(tick, sent)| all::<SetEntityData>(sent).into_iter().filter(|data| data.runtime_entity_id == id).flat_map(|data| of(&data)).map(move |value| (tick, value)).collect::<Vec<_>>()).collect()
}

#[test]
fn a_wither_takes_its_prey_while_charging_and_growls_a_tick_on_then_its_head_follows() {
    // bds-ab1wither2.txt, bds-ah1wither-gdb2.txt: `TargetEid` a held cow 5 ticks into the charge,
    // `mob.warning` 2.55 over its feet a tick on, `WitherTarget1` the cow only after the charge; the
    // cow gone, `TargetEid` 0 and a tick on `WitherTarget1` -1.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_difficulty(Difficulty::Normal);
        ctx.set_game_mode(PlayerId(1), GameMode::Creative);
        assert!(ctx.spawn_entity(WITHER, [0.5, 65.0, 8.5]).is_some());
        let cow = ctx.spawn_entity("minecraft:cow", [0.5, 65.0, 3.5]).expect("a cow");
        assert!(ctx.give_entity_effect(cow, sapling_api::Effect::Slowness, 2000, 255));
        assert!(ctx.give_entity_effect(cow, sapling_api::Effect::Resistance, 2000, 255));
    });
    let mut ticks: Vec<Vec<RawPacket>> = (0..CHARGE + 20).map(|_| h.tick_all().to(1).to_vec()).collect();
    let added: Vec<AddEntity> = ticks.iter().flat_map(|sent| all::<AddEntity>(sent)).collect();
    let id = |kind: &str| added.iter().find(|added| added.entity_type == kind).expect("shown").runtime_id;
    let (wither, cow) = (id(WITHER), id("minecraft:cow"));
    let feet = added.iter().find(|added| added.runtime_id == wither).expect("shown").position.y;
    // Past the 0 its first metadata carries.
    let taken: Vec<(usize, i64)> = told(&ticks, wither, Key::TargetEid).into_iter().filter(|(_, prey)| *prey != 0).collect();
    assert!(matches!(taken[..], [(tick, prey)] if tick < 15 && prey == cow as i64), "{taken:?}");
    let growls: Vec<(usize, f32)> = ticks.iter().enumerate().flat_map(|(tick, sent)| all::<LevelSoundEvent>(sent).into_iter().filter(|sound| sound.sound_id == "mob.warning" && sound.entity_type == WITHER).map(move |sound| (tick, sound.position.y))).collect();
    assert!(matches!(growls[..], [(tick, y)] if tick == taken[0].0 + 1 && (y - feet - 2.55).abs() < 1e-3), "{growls:?} over {feet}, taken {taken:?}");
    let head = told(&ticks, wither, Key::WitherTarget1);
    assert!(head.iter().all(|(tick, prey)| *prey == -1 || (*tick > taken[0].0 + 100 && *prey == cow as i64)), "{head:?}");
    assert!(head.iter().any(|(_, prey)| *prey == cow as i64), "{head:?}");
    h.remote.run(move |ctx| assert!(ctx.kill_entity(EntityId(cow))));
    let from = ticks.len();
    ticks.extend((0..5).map(|_| h.tick_all().to(1).to_vec()));
    let lost: Vec<(usize, i64)> = told(&ticks, wither, Key::TargetEid).into_iter().filter(|(tick, _)| *tick >= from).collect();
    let dropped: Vec<(usize, i64)> = told(&ticks, wither, Key::WitherTarget1).into_iter().filter(|(tick, _)| *tick >= from).collect();
    assert!(matches!((&lost[..], &dropped[..]), ([(at, 0)], [(then, -1)]) if *then == *at + 1), "{lost:?} {dropped:?}");
}
