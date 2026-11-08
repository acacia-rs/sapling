//! Group B of the fourteenth brief (research/captures/bds-b1eat.txt, bds-b1click.txt, bds-b1tread.txt,
//! bds-b1shield.txt, bds-b1ride.txt, bds-b1sculk.txt): creative eating, the swing of a click taken,
//! a creative player's trample, a shield on a rider and against a hit that stands a sneaker up, and
//! the vibrations of a mob's hurt and death.

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::nbt::Value as Nbt;
use acacia_proto::packets::{Animate, AnimateActionId, CompletedUsingItem, Interact, InteractActionId, LevelSoundEvent, SetEntityData};
use acacia_proto::types::{InputData, MetadataDictionaryItemValue as Meta};
use common::shooting::{click_air, ready};
use common::{Harness, all, asking::ask, input_at};
use sapling_api::{Block, DamageCause, Difficulty, GameMode, Item, PlayerId, Slot};

const ALEX: PlayerId = PlayerId(1);

/// The event a sculk sensor's block actor at `pos` last kept.
fn last_heard(h: &mut Harness, pos: [i32; 3]) -> Option<Nbt> {
    let nbt = ask(h, move |ctx| ctx.block_entity(pos).and_then(|data| data.get::<Nbt>().cloned())).expect("a block actor");
    nbt.get("VibrationListener").and_then(|listener| listener.get("event")).cloned()
}

fn ticks(h: &mut Harness, count: usize) -> Vec<RawPacket> {
    (0..count).flat_map(|_| h.tick_all().to(1).to_vec()).collect()
}

/// Alex's shield bits (8 to 11) in each `FlagsExtended` it was told of itself; a crawl's left out
/// (the harness lays a rider set down in the floor to crawl).
fn guard_bits(sent: &[RawPacket]) -> Vec<i64> {
    const CRAWLING: i64 = 1 << 50;
    let of = |data: SetEntityData| data.metadata.iter().find_map(|item| if let Meta::FlagsExtended(flags) = &item.value { (flags.0 & CRAWLING == 0).then_some(flags.0 >> 8 & 15) } else { None });
    all::<SetEntityData>(sent).into_iter().filter(|data| data.runtime_entity_id == 1).filter_map(of).collect()
}

#[test]
fn a_creative_player_eats_and_drinks_what_it_keeps_and_a_potion_leaves_a_bottle_as_well() {
    let mut h = ready(GameMode::Creative, &[("minecraft:apple", 1, 0)], 0.0);
    h.receive(1, &click_air());
    let sent = ticks(&mut h, 40);
    let burped = all::<LevelSoundEvent>(&sent).iter().any(|sound| sound.sound_id == "burp");
    assert_eq!((all::<CompletedUsingItem>(&sent).len(), burped), (1, true));
    assert_eq!(ask(&mut h, |ctx| ctx.item(ALEX, Slot::Main(0)).map(|item| (item.name, item.count))), Some(("minecraft:apple".into(), 1)));

    let feet = ask(&mut h, |ctx| ctx.position(ALEX).expect("in the world").feet);
    let sensor = [feet[0].floor() as i32 + 4, feet[1].floor() as i32, feet[2].floor() as i32];
    h.remote.run(move |ctx| {
        assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new("minecraft:potion", 1))));
        assert!(ctx.set_block(sensor, &Block::new("minecraft:sculk_sensor")));
    });
    ticks(&mut h, 60);
    h.receive(1, &click_air());
    let sent = ticks(&mut h, 50);
    assert_eq!(all::<CompletedUsingItem>(&sent).len(), 1);
    let kept = ask(&mut h, |ctx| [0, 1].map(|slot| ctx.item(ALEX, Slot::Main(slot)).map(|item| item.name)));
    assert_eq!(kept, [Some("minecraft:potion".into()), Some("minecraft:glass_bottle".into())]);
    assert_eq!(last_heard(&mut h, sensor), Some(Nbt::Int(14)), "drink, where eat is 15");
}

#[test]
fn a_click_a_mob_takes_swings_the_clickers_arm_for_it_and_its_viewers() {
    let mut h = ready(GameMode::Survival, &[("minecraft:wheat", 4, 0)], 0.0);
    let cow = common::shooting::spawn(&mut h, "minecraft:cow", [0.5, 65.0, 2.5]);
    let sent = common::raising::click_on(&mut h, cow);
    let swings: Vec<(u64, Option<String>)> = all::<Animate>(&sent).into_iter().filter(|animate| animate.action_id == AnimateActionId::SwingArm).map(|animate| (animate.runtime_entity_id, animate.swing_source)).collect();
    assert_eq!(swings, [(1, Some("interact".into()))]);
}

#[test]
fn a_creative_player_treads_farmland_it_drops_onto() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_game_mode(ALEX, GameMode::Creative);
        assert!(ctx.set_block([0, 64, 0], &Block::new("minecraft:farmland")));
    });
    for height in [68.0, 66.0] {
        h.receive(1, &input_at([0.5, height, 0.5]));
        h.tick_all();
    }
    let mut landing = input_at([0.5, 64.9375, 0.5]);
    (landing.input_data, landing.delta.y) = (vec![InputData::VerticalCollision], -1.0);
    h.receive(1, &landing);
    ticks(&mut h, 3);
    assert_eq!(ask(&mut h, |ctx| ctx.block([0, 64, 0]).map(|block| block.name)), Some("minecraft:dirt".into()));
}

#[test]
fn a_rider_with_a_shield_raises_it_settles_and_shows_it_down_when_off() {
    let mut h = ready(GameMode::Survival, &[], 0.0);
    h.remote.run(|ctx| assert!(ctx.set_item(ALEX, Slot::Offhand, Some(&Item::new("minecraft:shield", 1)))));
    let boat = common::shooting::spawn(&mut h, "minecraft:boat", [2.5, 65.0, 0.5]);
    h.receive(1, &common::raising::use_on(boat));
    assert_eq!(guard_bits(&ticks(&mut h, 5)), [3, 1], "raised and moving, then raised (bds-b1ride.txt)");
    ticks(&mut h, 5);
    h.receive(1, &Interact { action_id: InteractActionId::LeaveVehicle, target_entity_id: boat, has_position: false, position: None });
    assert_eq!(guard_bits(&ticks(&mut h, 3)), [0]);
}

#[test]
fn a_blocked_mob_hit_stands_a_sneaker_up_for_a_tick() {
    let mut h = ready(GameMode::Survival, &[], 0.0);
    h.remote.run(|ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        ctx.set_time(18000);
        assert!(ctx.set_item(ALEX, Slot::Offhand, Some(&Item::new("minecraft:shield", 1))));
    });
    let mut input = input_at(common::shooting::FEET);
    input.input_data = vec![InputData::StartSneaking];
    h.receive(1, &input);
    ticks(&mut h, 8);
    let feet = common::shooting::FEET;
    common::shooting::spawn(&mut h, "minecraft:zombie", [feet[0], feet[1], feet[2] + 2.0]);
    let mut sent = Vec::new();
    for _ in 0..80 {
        let tick = h.tick_all().to(1).to_vec();
        let blocked = all::<LevelSoundEvent>(&tick).iter().any(|sound| sound.sound_id == "item.shield.block");
        sent.extend(tick);
        if blocked {
            break;
        }
    }
    let from = sent.len();
    sent.extend(ticks(&mut h, 4));
    let bits = guard_bits(&sent[..from]);
    assert!(bits.ends_with(&[13, 13, 0]), "struck, stood up struck, then down: {bits:?}");
    assert_eq!(&guard_bits(&sent[from..])[..2], [0, 3], "sneaking again, then raised and moving");
}

#[test]
fn a_mob_hurt_and_a_mob_killed_are_heard() {
    let mut h = ready(GameMode::Creative, &[], 0.0);
    let feet = common::shooting::FEET;
    let cow = ask(&mut h, move |ctx| {
        assert!(ctx.set_block([feet[0] as i32 + 6, 65, feet[2] as i32 + 3], &Block::new("minecraft:sculk_sensor")));
        ctx.spawn_entity("minecraft:cow", [feet[0] + 2.0, 65.0, feet[2] + 3.0]).expect("a cow")
    });
    let sensor = [feet[0] as i32 + 6, 65, feet[2] as i32 + 3];
    ticks(&mut h, 60);
    assert!(ask(&mut h, move |ctx| ctx.hurt_entity(cow, 1.0, DamageCause::Generic)));
    ticks(&mut h, 5);
    assert_eq!(last_heard(&mut h, sensor), Some(Nbt::Int(17)), "entity_damage");
    ticks(&mut h, 60);
    assert!(ask(&mut h, move |ctx| ctx.hurt_entity(cow, 50.0, DamageCause::Generic)));
    ticks(&mut h, 5);
    assert_eq!(last_heard(&mut h, sensor), Some(Nbt::Int(18)), "entity_die");
}
