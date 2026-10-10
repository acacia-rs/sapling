//! A player's health, death and rising, against research/captures/bds-hurt.txt.

mod common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddItemEntity, DeathInfo, EntityEvent, EntityEventEventId, Respawn, SetHealth, UpdateAttributes};
use acacia_proto::types::{InputData, Vec3f};
use common::{Harness, all, input_at};
use sapling_api::{Block, DamageCause, GameMode, Item, PlayerId, Slot};

const ALEX: PlayerId = PlayerId(1);

fn ready() -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    h
}

fn health(sent: &[RawPacket]) -> Vec<f32> {
    all::<UpdateAttributes>(sent).iter().flat_map(|update| &update.attributes).filter(|attribute| attribute.name == "minecraft:health").map(|attribute| attribute.current).collect()
}

fn events(sent: &[RawPacket]) -> Vec<(EntityEventEventId, i32)> {
    all::<EntityEvent>(sent).into_iter().map(|event| (event.event_id, event.data)).collect()
}

fn ask_to_rise(h: &mut Harness) {
    h.receive(1, &Respawn { position: Vec3f { x: 0.0, y: 0.0, z: 0.0 }, state: 2, runtime_entity_id: 0 });
}

#[test]
fn a_hit_is_flashed_and_told_to_the_player_and_its_viewers_and_guards_for_half_a_second() {
    let mut h = ready();
    h.remote.run(|ctx| {
        assert!(ctx.hurt_player(ALEX, 3.0, DamageCause::Generic));
        assert!(!ctx.hurt_player(ALEX, 3.0, DamageCause::Fall), "within the guard");
        assert_eq!(ctx.health(ALEX), Some(17.0));
    });
    let sent = h.tick_all();
    for conn in [1, 2] {
        assert_eq!((events(sent.to(conn)), health(sent.to(conn))), (vec![(EntityEventEventId::HurtAnimation, -1)], vec![17.0]), "to {conn}");
    }
    (0..10).for_each(|_| drop(h.tick_all()));
    h.remote.run(|ctx| {
        ctx.set_game_mode(ALEX, GameMode::Creative);
        assert!(!ctx.hurt_player(ALEX, 3.0, DamageCause::Generic) && !ctx.kill_player(ALEX), "not in creative");
    });
    assert!(health(h.tick_all().to(1)).is_empty());
}

#[test]
fn a_fall_costs_a_player_each_block_past_three_a_tick_after_it_lands() {
    let mut h = ready();
    for height in [73.0, 69.0] {
        h.receive(1, &input_at([0.5, height, 0.5]));
        h.tick_all();
    }
    let mut landing = input_at([0.5, 65.0, 0.5]);
    (landing.input_data, landing.delta.y) = (vec![InputData::VerticalCollision], -1.0);
    h.receive(1, &landing);
    assert!(health(h.tick_all().to(1)).is_empty(), "not in the tick it lands");
    let sent = h.tick_all();
    let flash = all::<EntityEvent>(sent.to(2)).pop().expect("a flash");
    assert_eq!((flash.event_id, flash.data, flash.fire_at_position.map(|at| at.y)), (EntityEventEventId::HurtAnimation, 5, Some(65.0 + sapling_data::EYE_HEIGHT)));
    assert_eq!(health(sent.to(1)), [15.0], "8 blocks");
}

#[test]
fn lava_kills_a_player_who_leaves_what_it_held_and_rises_whole_at_the_spawn() {
    let mut h = ready();
    h.remote.run(|ctx| {
        assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new("minecraft:stone", 5))));
        ctx.teleport(ALEX, [5.5, 65.0, 5.5]);
        assert!(ctx.set_block([5, 65, 5], &Block::new("lava")));
    });
    let (mut told, mut seen) = (Vec::new(), Vec::new());
    for _ in 0..45 {
        let sent = h.tick_all();
        told.extend(sent.to(1).to_vec());
        seen.extend(sent.to(2).to_vec());
    }
    assert_eq!(health(&told), [16.0, 12.0, 8.0, 4.0, 0.0], "1 of the fire and 4 of the lava each half second");
    assert_eq!(health(&seen), health(&told));
    assert_eq!(all::<DeathInfo>(&told).into_iter().map(|info| (info.cause, info.messages)).collect::<Vec<_>>(), [("death.attack.lava".to_owned(), vec!["Alex".to_owned()])]);
    assert_eq!(all::<Respawn>(&told).into_iter().map(|respawn| respawn.state).collect::<Vec<_>>(), [0]);
    assert!(events(&seen).contains(&(EntityEventEventId::DeathAnimation, 0)) && all::<AddItemEntity>(&seen).len() == 1, "its stone lies there");
    h.remote.run(|ctx| assert_eq!((ctx.health(ALEX), ctx.item(ALEX, Slot::Main(0))), (Some(0.0), None)));
    h.tick_all();

    ask_to_rise(&mut h);
    let sent = h.tick_all();
    let risen = sent.to(1);
    assert_eq!(all::<Respawn>(risen).into_iter().map(|respawn| (respawn.state, respawn.position.y)).collect::<Vec<_>>(), [(1, 65.0 + sapling_data::EYE_HEIGHT)]);
    assert_eq!((all::<SetHealth>(risen).len(), health(risen), events(sent.to(2))), (1, vec![20.0], vec![(EntityEventEventId::Respawn, 0)]));
    h.remote.run(|ctx| {
        assert_eq!((ctx.health(ALEX), ctx.position(ALEX).map(|position| position.feet), Some(ctx.spawn())), (Some(20.0), Some(ctx.spawn()), Some(ctx.spawn())));
        assert!(ctx.kill_player(ALEX) && !ctx.kill_player(ALEX));
    });
    assert_eq!(all::<DeathInfo>(h.tick_all().to(1)).pop().map(|info| info.cause), Some("death.attack.generic".to_owned()));
}

#[test]
fn water_over_its_head_drowns_a_player_out_of_breath_and_a_block_around_it_smothers() {
    let mut h = ready();
    h.remote.run(|ctx| {
        assert!(ctx.set_block([3, 65, 3], &Block::new("water")) && ctx.set_block([3, 66, 3], &Block::new("water")));
        ctx.teleport(ALEX, [3.5, 65.0, 3.5]);
    });
    let mut told = |h: &mut Harness, ticks: usize| (0..ticks).flat_map(|_| health(h.tick_all().to(2))).collect::<Vec<_>>();
    assert!(told(&mut h, 319).is_empty(), "300 ticks of breath and 20 more");
    assert_eq!(told(&mut h, 22), [18.0, 16.0], "2 each second");
    h.remote.run(|ctx| {
        assert!(ctx.set_block([3, 65, 3], &Block::new("stone")) && ctx.set_block([3, 66, 3], &Block::new("stone")));
    });
    // The last of the water guards against the first of the stone.
    assert_eq!(told(&mut h, 33), [14.0, 12.0, 10.0], "2 each half second");
    h.remote.run(|ctx| assert!(ctx.set_block([3, 65, 3], &Block::new("glass"))));
    assert!(told(&mut h, 20).is_empty(), "a block over its head alone does not, nor glass");
}

#[test]
fn a_wounded_player_mends_one_each_four_seconds_and_pays_with_its_hunger() {
    let mut h = ready();
    h.remote.run(|ctx| assert!(ctx.hurt_player(ALEX, 14.0, DamageCause::Generic)));
    let sent: Vec<RawPacket> = (0..82).flat_map(|_| h.tick_all().to(2).to_vec()).collect();
    assert_eq!(health(&sent), [6.0, 7.0]);
    let fed = |name: &str| all::<UpdateAttributes>(&sent).iter().flat_map(|update| &update.attributes).filter(|attribute| attribute.name == name).map(|attribute| attribute.current).collect::<Vec<_>>();
    assert_eq!((fed("minecraft:player.hunger"), fed("minecraft:player.saturation"), fed("minecraft:player.exhaustion")), (vec![20.0], vec![4.0], vec![2.0]));
}

#[test]
fn a_hungry_player_eats_the_apple_in_its_hand_in_32_ticks() {
    use acacia_proto::packets::{CompletedUsingItem, LevelSoundEvent};
    use acacia_proto::types::{TransactionTransactionData, TransactionUseItemActionType};
    let mut h = ready();
    h.remote.run(|ctx| {
        assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new("minecraft:apple", 2))));
        assert!(ctx.hurt_player(ALEX, 14.0, DamageCause::Generic));
    });
    let mut click = common::click([0, 0, 0], 255, None);
    if let TransactionTransactionData::ItemUse(used) = &mut click.transaction.transaction_data {
        used.action_type = TransactionUseItemActionType::ClickAir;
    }
    h.receive(1, &click);
    assert!(all::<CompletedUsingItem>(h.tick_all().to(1)).is_empty(), "a full hunger eats nothing");
    (0..330).for_each(|_| drop(h.tick_all()));
    h.receive(1, &click);
    let sent: Vec<RawPacket> = (0..33).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    assert!(all::<CompletedUsingItem>(&sent).iter().all(|used| used.used_item_id > 100), "the item, not its stack");
    assert_eq!((all::<CompletedUsingItem>(&sent).len(), all::<LevelSoundEvent>(&sent).iter().filter(|sound| sound.sound_id == "burp").count()), (1, 1));
    let fed = |name: &str| all::<UpdateAttributes>(&sent).iter().flat_map(|update| &update.attributes).filter(|attribute| attribute.name == name).map(|attribute| attribute.current).next_back();
    assert_eq!((fed("minecraft:player.hunger"), fed("minecraft:player.saturation")), (Some(20.0), Some(2.4)));
    h.remote.run(|ctx| assert_eq!(ctx.item(ALEX, Slot::Main(0)).map(|item| item.count), Some(1)));
    h.tick_all();
}

#[test]
fn a_jump_tires_a_player_a_little_and_a_sprint_by_the_block() {
    let mut h = ready();
    let exhaustion = |sent: &[RawPacket]| all::<UpdateAttributes>(sent).iter().flat_map(|update| &update.attributes).filter(|attribute| attribute.name == "minecraft:player.exhaustion").map(|attribute| (attribute.current * 1000.0).round()).collect::<Vec<_>>();
    let mut step = |h: &mut Harness, z: f32, flags: Vec<InputData>| {
        let mut input = input_at([0.5, 65.0, z]);
        input.input_data = flags;
        h.receive(1, &input);
        // Counted in the tick the input is read or the one after.
        [h.tick_all(), h.tick_all()].iter().flat_map(|sent| exhaustion(sent.to(1))).collect::<Vec<_>>()
    };
    assert_eq!(step(&mut h, 0.5, vec![InputData::VerticalCollision, InputData::StartJumping]), [50.0]);
    assert!(step(&mut h, 0.75, vec![InputData::VerticalCollision]).is_empty(), "walking costs nothing");
    assert_eq!(step(&mut h, 1.0, vec![InputData::VerticalCollision, InputData::StartSprinting]), [75.0], "a quarter block");
    assert_eq!(step(&mut h, 1.0, vec![InputData::VerticalCollision, InputData::StartJumping]), [275.0]);
}

#[test]
fn armour_stops_part_of_a_hit_and_none_of_a_fall() {
    let mut h = ready();
    h.remote.run(|ctx| {
        assert!(ctx.set_item(ALEX, Slot::Armor(1), Some(&Item::new("minecraft:iron_chestplate", 1))));
        assert!(ctx.hurt_player(ALEX, 5.0, DamageCause::Attack));
    });
    assert_eq!(health(h.tick_all().to(2)), [15.7], "6 points against 5 stop 14 in 100");
    (0..12).for_each(|_| drop(h.tick_all()));
    h.remote.run(|ctx| assert!(ctx.hurt_player(ALEX, 2.0, DamageCause::Fall)));
    assert_eq!(health(h.tick_all().to(2)), [13.7]);
}
