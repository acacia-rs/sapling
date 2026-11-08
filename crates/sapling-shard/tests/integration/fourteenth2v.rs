//! Group V of the fourteenth wave: a frog eating a small slime and laying frogspawn
//! (research/captures/bds-g14frog.txt); mobs treading farmland.

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddItemEntity, LevelSoundEvent, RemoveEntity, SetEntityData, SetEntityMotion};
use acacia_proto::types::{MetadataDictionaryItemValue as MetaValue, MetadataFlags1 as Flags};
use common::asking::ask;
use common::raising::{click_on, has_flag};
use common::shooting::ready;
use common::{Harness, all};
use sapling_api::{Block, Difficulty, Effect, EntityId, GameMode, RuleValue};

const FROG: &str = "minecraft:frog";
const TONGUE: i64 = 1 << 38;

/// What Alex was told each tick, `ticks` of them.
fn ticks(h: &mut Harness, ticks: usize) -> Vec<Vec<RawPacket>> {
    (0..ticks).map(|_| h.tick_all().to(1).to_vec()).collect()
}

fn tongue_shown(sent: &[RawPacket], frog: u64) -> Option<bool> {
    let data = all::<SetEntityData>(sent).into_iter().filter(|data| data.runtime_entity_id == frog);
    data.flat_map(|data| data.metadata).filter_map(|item| if let MetaValue::FlagsExtended(flags) = item.value { Some(flags.0 & TONGUE != 0) } else { None }).last()
}

#[test]
fn a_frog_tongues_a_small_slime_and_leaves_a_slime_ball_where_it_met_it() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let [frog, slime] = ask(&mut h, |ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        ctx.set_game_mode(sapling_api::PlayerId(1), GameMode::Creative);
        let frog = ctx.summon_as(None, FROG, [0.5, 65.0, 0.5], 0.0, 0.0, None).expect("a frog").0;
        let slime = ctx.summon_as(None, "minecraft:slime", [4.5, 65.0, 0.5], 0.0, 0.0, Some("spawn_small")).expect("a slime").0;
        assert!(ctx.give_entity_effect(EntityId(slime), Effect::Slowness, 400, 255));
        [frog, slime]
    });
    let told = ticks(&mut h, 300);
    let first = |found: &dyn Fn(&[RawPacket]) -> bool| told.iter().position(|sent| found(sent)).expect("seen");
    let targeted = first(&|sent| all::<SetEntityData>(sent).iter().any(|data| data.runtime_entity_id == frog && common::raising::set_in(sent, frog, acacia_proto::types::MetadataDictionaryItemKey::TargetEid).is_some()));
    let warned = first(&|sent| all::<LevelSoundEvent>(sent).iter().any(|sound| sound.sound_id == "mob.warning" && sound.entity_type == FROG));
    assert_eq!(warned, targeted + 1, "warned a tick after it took the slime");
    let tongue = first(&|sent| tongue_shown(sent, frog) == Some(true));
    let pull = all::<SetEntityMotion>(&told[tongue]).into_iter().find(|motion| motion.runtime_entity_id == slime).expect("the slime pulled at once");
    assert!((pull.velocity.x.hypot(pull.velocity.z) - 0.75).abs() < 1e-3 && pull.velocity.x < 0.0 && pull.velocity.y == 0.0, "{pull:?}");
    assert!(all::<LevelSoundEvent>(&told[tongue + 1]).iter().any(|sound| sound.sound_id == "tongue"), "heard a tick on");
    assert_eq!(tongue_shown(&told[tongue + 8], frog), Some(false), "hidden 8 ticks on");
    assert!(all::<RemoveEntity>(&told[tongue + 9]).iter().any(|gone| gone.entity_id_self == slime as i64), "gone 9 ticks on");
    let ball = all::<AddItemEntity>(&told[tongue + 9]).into_iter().next().expect("its loot");
    assert!(ball.position.x > 3.0, "where the tongue met it, not where it was pulled to: {:?}", ball.position);
}

#[test]
fn two_fed_frogs_make_one_pregnant_and_it_lays_frogspawn_on_water() {
    let mut h = ready(GameMode::Survival, &[("minecraft:slime_ball", 8, 0)], 0.0);
    let frogs = ask(&mut h, |ctx| {
        for x in 3..=5 {
            for z in -1..=1 {
                assert!(ctx.set_block([x, 64, z], &Block::new("water")));
            }
        }
        [0.5, 1.5].map(|x| ctx.summon_as(None, FROG, [x, 65.0, -1.5], 0.0, 0.0, None).expect("a frog").0)
    });
    let held = frogs;
    assert!(ask(&mut h, move |ctx| held.iter().all(|frog| ctx.give_entity_effect(EntityId(*frog), Effect::Slowness, 400, 255))));
    frogs.iter().for_each(|frog| drop(click_on(&mut h, *frog)));
    assert!(ask(&mut h, move |ctx| held.iter().all(|frog| ctx.clear_entity_effects(EntityId(*frog)))));
    let told: Vec<RawPacket> = ticks(&mut h, 600).concat();
    let was_pregnant = |frog: u64| all::<SetEntityData>(&told).into_iter().filter(|data| data.runtime_entity_id == frog).flat_map(|data| data.metadata).any(|item| matches!(item.value, MetaValue::Flags(flags) if flags.0 & Flags::PREGNANT.0 != 0));
    let pregnant: Vec<u64> = frogs.into_iter().filter(|frog| was_pregnant(*frog)).collect();
    assert_eq!(pregnant.len(), 1, "one carries the spawn, no tadpole: {pregnant:?}");
    assert!(common::shooting::added(&told, "minecraft:tadpole").is_empty());
    assert_eq!(has_flag(&told, pregnant[0], Flags::LAYING_EGG), Some(false), "done laying");
    let spawn = ask(&mut h, |ctx| (3..=5).flat_map(|x| (-1..=1).map(move |z| [x, 65, z])).filter(|pos| ctx.block(*pos).is_some_and(|block| block.name == "minecraft:frog_spawn")).count());
    assert_eq!(spawn, 1, "frogspawn on the water");
    assert!(all::<LevelSoundEvent>(&told).iter().any(|sound| sound.sound_id == "lay_spawn" && sound.entity_type == FROG), "heard from the frog");
}

/// research/captures/bds-f14v2tread2.txt: under gdb, cows that fell 2.7 onto farmland trod it, a calf did not.
#[test]
fn a_cow_dropped_onto_farmland_treads_it_and_a_calf_or_a_griefless_cow_does_not() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let drop = |h: &mut Harness, x: f32, event: Option<&'static str>, griefs: bool| {
        ask(h, move |ctx| {
            assert!(ctx.set_game_rule("randomTickSpeed", RuleValue::Int(0)) && ctx.set_game_rule("mobGriefing", RuleValue::Bool(griefs)));
            assert!(ctx.set_block([x as i32, 64, 0], &Block::new("farmland")));
            ctx.summon_as(None, "minecraft:cow", [x + 0.5, 68.0, 0.5], 0.0, 0.0, event).expect("a cow");
        });
        ticks(h, 40);
        ask(h, move |ctx| ctx.block([x as i32, 64, 0]).map(|block| block.name))
    };
    assert_eq!(drop(&mut h, 0.0, None, true).as_deref(), Some("minecraft:dirt"));
    assert_eq!(drop(&mut h, 3.0, Some("minecraft:entity_born"), true).as_deref(), Some("minecraft:farmland"), "a calf's box holds under 0.512");
    assert_eq!(drop(&mut h, 6.0, None, false).as_deref(), Some("minecraft:farmland"), "mobGriefing off");
}
