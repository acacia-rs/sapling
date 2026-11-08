//! Group G of the fourteenth wave, animal behaviour (research/captures/bds-g14panda.txt, bds-g14frog.txt,
//! bds-g14bat.txt, bds-g14wolf.txt; creatures/{genes,frogs,bats}.rs, husbandry/{avoiding,rolls,guarding}.rs).

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, EntityEventEventId as Event, LevelSoundEvent, MoveEntityDelta, SetEntityData, SetEntityMotion};
use acacia_proto::types::{MetadataDictionaryItemValue as Value, MetadataFlags1 as Flags};
use common::raising::{click_on, showed};
use common::shooting::{ready, spawn, watch};
use common::{Harness, all, asking::ask};
use sapling_api::{Block, Difficulty, GameMode};

const PANDA: &str = "minecraft:panda";
const ROLLING: i64 = 1 << 3;
const FLEEING: i64 = 1 << 22;
const CROAKING: i64 = 1 << 37;

/// Alex alone on the platform at midnight on easy, with `mobs` summoned by their spawn events, the
/// blocks `set` first; each tick's packets to Alex for `ticks`.
fn watched(set: &'static [([i32; 3], &'static str)], mobs: &'static [(&'static str, [f32; 3], Option<&'static str>)], ticks: usize) -> (Harness, Vec<Vec<RawPacket>>) {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    ask(&mut h, move |ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        ctx.set_time(18000);
        assert!(set.iter().all(|(at, block)| ctx.set_block(*at, &Block::new(*block))));
    });
    // Not `ask`, which would swallow the mobs' announcements.
    h.remote.run(move |ctx| {
        for (kind, feet, event) in mobs {
            assert!(ctx.summon_as(None, kind, *feet, 0.0, 0.0, *event).is_some(), "{kind}");
        }
    });
    let ticks = (0..ticks).map(|_| h.tick_all().to(1).to_vec()).collect();
    (h, ticks)
}

fn added(ticks: &[Vec<RawPacket>], kind: &str) -> Vec<AddEntity> {
    ticks.iter().flat_map(|sent| all::<AddEntity>(sent)).filter(|added| added.entity_type == kind).collect()
}

/// The `FlagsExtended` told of `id` in a tick's packets.
fn extended(sent: &[RawPacket], id: u64) -> Vec<i64> {
    all::<SetEntityData>(sent).into_iter().filter(|data| data.runtime_entity_id == id).flat_map(|data| data.metadata).filter_map(|item| if let Value::FlagsExtended(flags) = item.value { Some(flags.0) } else { None }).collect()
}

fn flags(sent: &[RawPacket], id: u64) -> Vec<i64> {
    all::<SetEntityData>(sent).into_iter().filter(|data| data.runtime_entity_id == id).flat_map(|data| data.metadata).filter_map(|item| if let Value::Flags(flags) = item.value { Some(flags.0) } else { None }).collect()
}

fn motions(sent: &[RawPacket], id: u64) -> Vec<[f32; 3]> {
    all::<SetEntityMotion>(sent).into_iter().filter(|told| told.runtime_entity_id == id).map(|told| [told.velocity.x, told.velocity.y, told.velocity.z]).collect()
}

/// The first tick from `from` on whose `FlagsExtended` of `id` has `bit` on (`on`) or off.
fn first(ticks: &[Vec<RawPacket>], from: usize, id: u64, bit: i64, on: bool) -> Option<usize> {
    (from..ticks.len()).find(|&tick| extended(&ticks[tick], id).iter().any(|flags| (flags & bit != 0) == on))
}

#[test]
fn a_lazy_panda_comes_with_movement_0_07_and_a_worried_one_flees_a_cow_near_it() {
    // A floor past the platform's -z edge, where a spot 16 from the cow can be.
    const FLOOR: &[([i32; 3], &str)] = &{
        let mut floor = [([0; 3], "stone"); 288];
        let mut at = 0;
        while at < 288 {
            floor[at].0 = [(at % 16) as i32 - 8, 64, (at / 16) as i32 - 26];
            at += 1;
        }
        floor
    };
    const MOBS: &[(&str, [f32; 3], Option<&str>)] = &[(PANDA, [-6.5, 65.0, 6.5], Some("minecraft:panda_lazy")), ("minecraft:cow", [6.5, 65.0, -2.5], None), (PANDA, [6.5, 65.0, -6.5], Some("minecraft:panda_worried"))];
    let (_, ticks) = watched(FLOOR, MOBS, 20);
    let pandas = added(&ticks, PANDA);
    let movement = |added: &AddEntity| added.attributes.iter().find(|attribute| attribute.name == "minecraft:movement").map(|attribute| attribute.value);
    assert_eq!(pandas.iter().map(movement).collect::<Vec<_>>(), [Some(0.07), Some(0.15)], "bds-g14panda.txt");
    assert!(first(&ticks, 0, pandas[1].runtime_id, FLEEING, true).is_some(), "the worried one flees the cow 4 off");
}

#[test]
fn a_playful_panda_rolls_0_2_along_and_0_27_up_with_bit_3_on_for_32_ticks() {
    let (_, ticks) = watched(&[], &[(PANDA, [0.5, 65.0, 4.5], Some("minecraft:panda_playful"))], 1200);
    let panda = added(&ticks, PANDA)[0].runtime_id;
    let rolled = first(&ticks, 0, panda, ROLLING, true).expect("a roll in a minute (0.013 a tick)");
    let [x, up, z] = motions(&ticks[rolled], panda)[0];
    assert!((x.hypot(z) - 0.2).abs() < 1e-4 && (up - 0.27).abs() < 1e-4, "{x} {up} {z}");
    assert_eq!(first(&ticks, rolled + 1, panda, ROLLING, false), Some(rolled + 32));
}

#[test]
fn a_frog_croaks_90_ticks_and_leaps_only_within_the_pen_it_is_in() {
    const PEN: &[([i32; 3], &str)] = &{
        let mut pen = [([0; 3], "glass"); 96];
        let mut at = 0;
        while at < 96 {
            let (ring, y) = (at % 16, 65 + (at / 16) as i32);
            let (x, z) = match ring {
                0..=4 => (ring as i32 - 2, -2),
                5..=9 => (ring as i32 - 7, 2),
                10..=12 => (-2, ring as i32 - 11),
                _ => (2, ring as i32 - 14),
            };
            pen[at].0 = [x, y, z];
            at += 1;
        }
        pen
    };
    let (_, ticks) = watched(PEN, &[("minecraft:frog", [0.5, 65.0, 0.5], None)], 600);
    let frog = added(&ticks, "minecraft:frog")[0].runtime_id;
    let croaked = first(&ticks, 0, frog, CROAKING, true).expect("a croak within 20 s");
    assert_eq!(first(&ticks, croaked + 1, frog, CROAKING, false), Some(croaked + 90));
    let leapt = ticks.iter().flat_map(|sent| all::<LevelSoundEvent>(sent)).any(|sound| sound.sound_id == "jump_to_block");
    let (mut at, mut out) = ([0.5, 0.5], false);
    for delta in ticks.iter().flat_map(|sent| all::<MoveEntityDelta>(sent)).filter(|delta| delta.runtime_entity_id == frog) {
        at = [delta.x.unwrap_or(at[0]), delta.z.unwrap_or(at[1])];
        out |= at.iter().any(|c| !(-1.0..2.0).contains(c));
    }
    assert!(leapt && !out, "leapt {leapt}, last at {at:?}");
}

#[test]
fn a_bat_under_a_ceiling_hangs_from_birth_and_lets_go_as_the_ceiling_goes() {
    let (mut h, ticks) = watched(&[([0, 66, 0], "stone")], &[("minecraft:bat", [0.5, 65.0, 0.5], None)], 40);
    let bat = added(&ticks, "minecraft:bat")[0].clone();
    let shown = bat.metadata.iter().find_map(|item| if let Value::Flags(flags) = item.value { Some(flags.0) } else { None }).unwrap_or(0);
    assert!(shown & Flags::RESTING.0 != 0, "born hanging (bds-g14bat.txt)");
    let ys: Vec<f32> = ticks.iter().flat_map(|sent| all::<MoveEntityDelta>(sent)).filter(|delta| delta.runtime_entity_id == bat.runtime_id).filter_map(|delta| delta.y).collect();
    assert!(ys.iter().all(|y| (y - 65.1).abs() < 1e-4), "its head against the ceiling: {ys:?}");
    h.remote.run(|ctx| assert!(ctx.set_block([0, 66, 0], &Block::new("air"))));
    let after = watch(&mut h, 3);
    assert!(flags(&after, bat.runtime_id).iter().any(|flags| flags & Flags::RESTING.0 == 0), "it lets go");
}

#[test]
fn a_tamed_wolf_leaps_at_what_its_owner_hits_and_its_bite_throws_it() {
    let mut h = ready(GameMode::Survival, &[("minecraft:bone", 64, 0)], 0.0);
    let wolf = spawn(&mut h, "minecraft:wolf", [1.5, 65.0, -2.5]);
    assert!((0..40).any(|_| showed(&click_on(&mut h, wolf), wolf, Event::TameSuccess)), "tamed");
    let pig = spawn(&mut h, "minecraft:pig", [1.5, 65.0, 0.5]);
    h.receive(1, &common::attack(pig));
    let ticks: Vec<Vec<RawPacket>> = (0..120).map(|_| watch(&mut h, 1)).collect();
    let told: Vec<(usize, [f32; 3])> = ticks.iter().enumerate().flat_map(|(tick, sent)| motions(sent, wolf).into_iter().map(move |motion| (tick, motion))).collect();
    let leap = told.first().expect("it leapt").1;
    assert!((leap[0].hypot(leap[2]) - 0.4).abs() < 1e-3 && leap[1] == 0.4, "{told:?}");
    let bit = ticks.iter().position(|sent| showed(sent, wolf, Event::ArmSwing)).expect("it bit");
    // Told at once as 0.4, then as moved, 0.4 less a tick's fall and drag (bds-g14wolf.txt).
    let [x, up, z] = motions(&ticks[bit], pig)[0];
    assert!((x.hypot(z) - 0.4).abs() < 1e-4 && up == 0.4, "the bite threw the pig at once: {x} {up} {z}");
    assert!(ticks[bit..bit + 2].iter().flat_map(|sent| motions(sent, pig)).any(|[_, up, _]| (up - 0.3136).abs() < 1e-4), "then told as moved");
    let heard = all::<LevelSoundEvent>(&ticks[bit + 1]).into_iter().any(|sound| sound.sound_id == "attack" && sound.entity_type == "minecraft:wolf");
    assert!(heard, "heard a tick later");
}
