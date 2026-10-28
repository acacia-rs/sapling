//! The warden, against research/captures/bds-warden.txt (bds-warden-findings.md).

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, LevelSoundEvent, RemoveEntity, SetEntityData, SetEntityMotion, UpdateAttributes};
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain};
use common::{Harness, all, attack};
use sapling_api::{Block, Difficulty, GameMode, PlayerId};

const KIND: &str = "minecraft:warden";
const REST: i64 = i64::MIN | 1 << 25;
const ROARING: i64 = REST | 1 << 20;
const EMERGING: i64 = REST | 1 << 40;
const SNIFFING: i64 = REST | 1 << 41;
const DIGGING: i64 = REST | 1 << 42;
const BOOMING: i64 = REST | 1 << 43;

/// What Steve, watching in creative, was told of the warden on each tick: (tick, what).
#[derive(Debug, PartialEq)]
enum Told {
    Flags(i64),
    Heartbeat(i32),
    Ambient(String),
    Target(i64),
    Sound(String),
}

fn told(tick: usize, sent: &[RawPacket], warden: u64) -> Vec<(usize, Told)> {
    let mut out = Vec::new();
    for data in all::<SetEntityData>(sent).into_iter().filter(|data| data.runtime_entity_id == warden) {
        for item in data.metadata {
            out.push((tick, match (item.key, item.value) {
                (Key::FlagsExtended, Value::FlagsExtended(flags)) => Told::Flags(flags.0),
                (Key::HeartbeatIntervalTicks, Value::Default(Plain::Int(ticks))) => Told::Heartbeat(ticks),
                (Key::AmbientSoundEventName, Value::Default(Plain::String(name))) => Told::Ambient(name),
                (Key::TargetEid, Value::Default(Plain::Long(target))) => Told::Target(target),
                _ => continue,
            }));
        }
    }
    out.extend(all::<LevelSoundEvent>(sent).into_iter().filter(|sound| sound.entity_type == KIND && sound.sound_id != "spawn").map(|sound| (tick, Told::Sound(sound.sound_id))));
    out
}

/// Alex at (0.5, 65, 0.5) in `alex`, Steve watching in creative, on easy, and a warden at `feet`
/// summoned by `event`; the harness, the warden's id and what Steve was told of it on tick 0.
fn with_warden(feet: [f32; 3], event: Option<&'static str>, alex: GameMode) -> (Harness, u64, Vec<(usize, Told)>) {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    h.remote.run(move |ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        ctx.set_game_mode(PlayerId(1), alex);
        ctx.set_game_mode(PlayerId(2), GameMode::Creative);
        assert!(ctx.summon_as(None, KIND, feet, 0.0, event).is_some());
    });
    let sent = h.tick_all().to(2).to_vec();
    let warden = all::<AddEntity>(&sent).remove(0).runtime_id;
    (h, warden, told(0, &sent, warden))
}

fn run(h: &mut Harness, warden: u64, ticks: usize) -> Vec<(usize, Told)> {
    (1..=ticks).flat_map(|tick| told(tick, h.tick_all().to(2), warden)).collect()
}

fn first(timeline: &[(usize, Told)], what: Told) -> usize {
    timeline.iter().find(|(_, told)| *told == what).unwrap_or_else(|| panic!("never {what:?}: {timeline:?}")).0
}

#[test]
fn a_warden_sniffs_goes_to_look_grows_angry_at_a_player_touching_it_and_roars_before_it_hunts() {
    let (mut h, warden, mut timeline) = with_warden([0.5, 65.0, -6.5], None, GameMode::Survival);
    timeline.extend(run(&mut h, warden, 400));
    let sniffs = first(&timeline, Told::Flags(SNIFFING));
    assert!(sniffs <= 1 && timeline.contains(&(sniffs, Told::Heartbeat(40))), "{timeline:?}");
    assert_eq!(first(&timeline, Told::Flags(REST)), sniffs + 84, "a sniff of 4.16 s and a tick");
    let heard: Vec<(usize, &Told)> = timeline.iter().filter(|(_, told)| matches!(told, Told::Sound(_) | Told::Heartbeat(_) | Told::Ambient(_))).map(|(tick, told)| (*tick, told)).collect();
    let touched = first(&timeline, Told::Sound("listening".into()));
    let expected = [
        (touched, &Told::Sound("listening".into())),
        (touched, &Told::Heartbeat(27)),
        (touched + 20, &Told::Sound("listening_angry".into())),
        (touched + 20, &Told::Ambient("agitated".into())),
        (touched + 20, &Told::Heartbeat(14)),
        (touched + 40, &Told::Sound("listening_angry".into())),
        (touched + 40, &Told::Ambient("angry".into())),
        (touched + 40, &Told::Heartbeat(10)),
    ];
    let mut got: Vec<(usize, &Told)> = heard.iter().copied().filter(|(tick, _)| (touched..=touched + 40).contains(tick)).collect();
    got.sort_by_key(|(tick, told)| (*tick, matches!(told, Told::Heartbeat(_)), matches!(told, Told::Ambient(_))));
    let mut want = expected.to_vec();
    want.sort_by_key(|(tick, told)| (*tick, matches!(told, Told::Heartbeat(_)), matches!(told, Told::Ambient(_))));
    assert_eq!(got, want, "a touch a second, 35 each");
    assert_eq!(first(&timeline, Told::Flags(ROARING)), touched + 40, "angry, it roars at once");
    assert_eq!(first(&timeline, Told::Target(1)), touched + 40 + 85, "and hunts when the roar is over");
    assert_eq!(first(&timeline, Told::Sound("mob.warning".into())), touched + 40 + 86);
}

#[test]
fn a_warden_booms_through_armour_at_prey_it_cannot_reach_every_five_seconds() {
    let (mut h, warden, _) = with_warden([0.5, 65.0, 2.5], None, GameMode::Survival);
    h.receive(1, &attack(warden));
    h.tick_all();
    h.remote.run(|ctx| {
        for y in 65..=69 {
            ctx.set_block([5, y, 5], &Block::new("minecraft:stone"));
        }
        for (x, y, z) in [(4, 70, 5), (6, 70, 5), (5, 70, 4), (5, 70, 6), (4, 71, 5), (6, 71, 5), (5, 71, 4), (5, 71, 6), (5, 72, 5)] {
            ctx.set_block([x, y, z], &Block::new("minecraft:glass"));
        }
        ctx.teleport(PlayerId(1), [5.5, 70.0, 5.5]);
    });
    let mut timeline = Vec::new();
    let mut thrown = Vec::new();
    let mut health = Vec::new();
    for tick in 1..=240 {
        let sent = h.tick_all();
        timeline.extend(told(tick, sent.to(2), warden));
        thrown.extend(all::<SetEntityMotion>(sent.to(1)).into_iter().filter(|motion| motion.runtime_entity_id == 1).map(|motion| (tick, [motion.velocity.x, motion.velocity.y, motion.velocity.z])));
        health.extend(all::<UpdateAttributes>(sent.to(1)).into_iter().flat_map(|update| update.attributes).filter(|attribute| attribute.name == "minecraft:health").map(|attribute| (tick, attribute.current)));
    }
    let booms: Vec<usize> = timeline.iter().filter(|(_, told)| *told == Told::Flags(BOOMING)).map(|(tick, _)| *tick).collect();
    assert!(booms.len() >= 2 && booms[1] - booms[0] == 61 + 41, "{timeline:?}");
    let start = booms[0];
    assert_eq!(first(&timeline, Told::Sound("sonic_charge".into())), start + 1);
    assert_eq!(first(&timeline, Told::Sound("sonic_boom".into())), start + 36);
    assert!(timeline.contains(&(start + 61, Told::Flags(REST))));
    assert_eq!(health.iter().find(|(tick, _)| *tick >= start).copied(), Some((start + 35, 14.0)), "10, 6 on easy");
    let throws: Vec<[f32; 3]> = thrown.iter().filter(|(tick, _)| *tick == start + 35).map(|(_, speed)| *speed).collect();
    assert_eq!(throws.len(), 2, "{thrown:?}");
    let [plain, boom] = [throws[0], throws[1]];
    assert!((plain[0].hypot(plain[2]) - 0.4).abs() < 1e-4 && plain[1] == 0.4, "{plain:?}");
    assert!((boom[0] - 3.0 * plain[0]).abs() < 1e-5 && (boom[2] - 3.0 * plain[2]).abs() < 1e-5 && boom[1] == 0.4167, "{boom:?}");
}

#[test]
fn a_warden_summoned_emerging_climbs_out_and_left_alone_a_minute_digs_down_and_goes() {
    let (mut h, warden, mut timeline) = with_warden([0.5, 65.0, -4.5], Some("minecraft:spawn_emerging"), GameMode::Creative);
    let mut gone = None;
    for tick in 1..=1500 {
        let sent = h.tick_all();
        timeline.extend(told(tick, sent.to(2), warden));
        if gone.is_none() && !all::<RemoveEntity>(sent.to(2)).is_empty() {
            gone = Some(tick);
        }
    }
    assert_eq!(timeline[0], (1, Told::Flags(EMERGING)), "a tick after it is announced, as on BDS");
    let out = first(&timeline, Told::Flags(REST));
    assert!((140..=141).contains(&out), "7 s and a tick: {out}");
    let digs = first(&timeline, Told::Flags(DIGGING));
    assert_eq!(digs, out + 1200, "a minute after it came out, undisturbed by creative players");
    assert!(gone.is_some_and(|gone| (digs + 111..=digs + 112).contains(&gone)), "5.5 s and a tick of digging, then gone: {gone:?}");
    assert!(!timeline.iter().any(|(_, told)| *told == Told::Flags(SNIFFING)), "no player to sniff for");
}
