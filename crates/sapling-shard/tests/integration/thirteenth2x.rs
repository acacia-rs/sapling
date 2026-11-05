//! Group X's drills (research/captures/bds-x2*.txt, bds-v2drip.txt, bds-z2open.txt): the health a landing
//! block cost told after its events, instant health in creative told at once.

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{EntityEvent, EntityEventEventId, LevelEvent, LevelEventEvent, UpdateAttributes};
use common::{Harness, all, input_at};
use sapling_api::{Block, DamageCause, Effect, GameMode, PlayerId};

const ALEX: PlayerId = PlayerId(1);

fn ready() -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| ctx.set_game_mode(ALEX, GameMode::Survival));
    h.tick_all();
    h
}

/// Each of `sent` as what it shows: `health <n>`, `hurt`, a level event's name, or nothing.
fn shown(sent: &[RawPacket]) -> Vec<String> {
    sent.iter()
        .filter_map(|packet| {
            let one = std::slice::from_ref(packet);
            let health = all::<UpdateAttributes>(one).into_iter().flat_map(|update| update.attributes).find(|attribute| attribute.name == "minecraft:health");
            let hurt = all::<EntityEvent>(one).into_iter().any(|event| event.event_id == EntityEventEventId::HurtAnimation);
            let event = all::<LevelEvent>(one).into_iter().next().map(|event| format!("{:?}", event.event));
            health.map(|health| format!("health {}", health.current)).or(hurt.then(|| "hurt".to_owned())).or(event)
        })
        .collect()
}

/// Alex held at 0.5 65 0.5 for `ticks`, what it was sent.
fn stand(h: &mut Harness, ticks: usize) -> Vec<RawPacket> {
    let mut sent = Vec::new();
    for _ in 0..ticks {
        h.receive(1, &input_at([0.5, 65.0, 0.5]));
        sent.extend(h.tick_all().to(1).to_vec());
    }
    sent
}

#[test]
fn an_anvil_landing_on_a_player_tells_the_health_it_cost_after_its_sound() {
    // research/captures/bds-z2open.txt: SoundAnvilFall, then the health.
    let mut h = ready();
    h.remote.run(|ctx| assert!(ctx.update_block([0, 71, 0], &Block::new("anvil"))));
    let shown = shown(&stand(&mut h, 40));
    let (sound, health) = (shown.iter().position(|seen| seen == "SoundAnvilFall"), shown.iter().position(|seen| seen == "health 10"));
    assert!(sound.is_some() && sound < health, "{shown:?}");
}

#[test]
fn a_stalactite_landing_on_a_player_flashes_it_shows_its_landing_then_tells_the_health() {
    // research/captures/bds-v2drip.txt: HurtAnimation, the dust and PointedDripstoneLand, then the health.
    let mut h = ready();
    h.remote.run(|ctx| {
        assert!(ctx.set_block([0, 76, 0], &Block::new("stone")));
        assert!(ctx.set_block([0, 75, 0], &Block::new("pointed_dripstone").with("dripstone_thickness=tip,hanging=1")));
    });
    stand(&mut h, 2);
    h.remote.run(|ctx| assert!(ctx.update_block([0, 76, 0], &Block::air())));
    let shown = shown(&stand(&mut h, 40));
    let from = shown.iter().position(|seen| seen == "hurt").unwrap_or_else(|| panic!("hurt: {shown:?}"));
    assert_eq!(shown[from..from + 4].iter().map(|seen| seen.split(' ').next().unwrap()).collect::<Vec<_>>(), ["hurt", "ParticleDestroyBlockNoSound", "PointedDripstoneLand", "health"], "{shown:?}");
}

#[test]
fn a_blast_sends_its_particles_after_its_flash_naming_each_cell_it_broke() {
    // research/captures/bds-fights.txt: a creeper's ParticleExplosion, then LevelEventGeneric 4052 with its middle,
    // `pos<i>x/y/z` of the 3 floor cells it broke, `radius` and `size`, the tags in name order.
    use acacia_proto::nbt::Value;
    use acacia_proto::packets::LevelEventGeneric;
    let mut h = ready();
    h.remote.run(|ctx| {
        ctx.set_game_mode(ALEX, GameMode::Creative);
        assert!(ctx.spawn_entity("minecraft:tnt", [3.5, 65.0, 0.5]).is_some());
    });
    let sent: Vec<RawPacket> = (0..90).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    let flash = sent.iter().position(|packet| all::<LevelEvent>(std::slice::from_ref(packet)).iter().any(|event| event.event == LevelEventEvent::ParticleExplosion)).expect("a blast");
    let particles: LevelEventGeneric = all::<LevelEventGeneric>(&sent[flash + 1..=flash + 1]).pop().expect("the particles next");
    let names: Vec<&str> = particles.nbt.iter().map(|tag| tag.name.as_str()).collect();
    assert!(particles.event_id == 2026 << 1 && names.is_sorted(), "{names:?}");
    let float = |name: String| particles.nbt.iter().find(|tag| tag.name == name).and_then(|tag| if let Value::Float(c) = tag.value { Some(c as i32) } else { None });
    let size = particles.nbt.iter().find(|tag| tag.name == "size").and_then(|tag| if let Value::Int(size) = tag.value { Some(size) } else { None }).expect("size");
    let named: Vec<[i32; 3]> = (0..size).map(|i| ["x", "y", "z"].map(|axis| float(format!("pos{i}{axis}")).expect("a cell"))).collect();
    let mut broken: Vec<[i32; 3]> = common::block_changes(&sent).into_iter().map(|(at, _)| at).collect();
    let mut sorted = named.clone();
    (sorted.sort(), broken.sort(), broken.dedup());
    assert!(size > 0 && sorted == broken, "{named:?} against {broken:?}");
}

#[test]
fn a_drink_is_heard_from_the_drinkers_eyes() {
    // research/captures/bds-x2drink.txt: a water bottle, honey and milk each set off the sensor from the eyes' cell.
    use acacia_proto::nbt::Value;
    use acacia_proto::packets::{CompletedUsingItem, LevelEventGeneric};
    use acacia_proto::types::{TransactionTransactionData, TransactionUseItemActionType};
    for drink in ["minecraft:potion", "minecraft:milk_bucket"] {
        let mut h = ready();
        h.remote.run(move |ctx| assert!(ctx.set_item(ALEX, sapling_api::Slot::Main(0), Some(&sapling_api::Item::new(drink, 1)))));
        let feet = common::asking::ask(&mut h, |ctx| ctx.position(ALEX).expect("in the world").feet);
        let eyes = [feet[0], feet[1] + 1.62, feet[2]].map(|c| c.floor() + 0.5);
        let sensor = [eyes[0] as i32 + 4, feet[1].floor() as i32, eyes[2] as i32];
        h.remote.run(move |ctx| assert!(ctx.set_block(sensor, &Block::new("minecraft:sculk_sensor"))));
        h.tick_all();
        let mut use_air = common::click([0, 0, 0], 255, None);
        if let TransactionTransactionData::ItemUse(used) = &mut use_air.transaction.transaction_data {
            used.action_type = TransactionUseItemActionType::ClickAir;
        }
        h.receive(1, &use_air);
        let sent: Vec<RawPacket> = (0..60).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
        let origin = |event: &LevelEventGeneric| {
            let origin = &event.nbt.iter().find(|nbt| nbt.name == "origin")?.value;
            let axis = |key: &str| if let Some(Value::Float(c)) = origin.get(key) { Some(*c) } else { None };
            Some([axis("x")?, axis("y")?, axis("z")?])
        };
        let heard: Vec<[f32; 3]> = all::<LevelEventGeneric>(&sent).iter().filter_map(origin).collect();
        assert_eq!((all::<CompletedUsingItem>(&sent).len(), heard), (1, vec![eyes]), "{drink}");
        // The bottle's sound comes before the vibration.
        let at = |found: &dyn Fn(&[RawPacket]) -> bool| sent.iter().position(|packet| found(std::slice::from_ref(packet)));
        let emptied = at(&|one| all::<acacia_proto::packets::LevelSoundEvent>(one).iter().any(|sound| sound.sound_id == "bottle.empty"));
        let vibration = at(&|one| all::<LevelEventGeneric>(one).iter().any(|event| origin(event).is_some()));
        assert!(drink != "minecraft:potion" || emptied.is_some() && emptied < vibration, "{emptied:?} {vibration:?}");
    }
}

#[test]
fn a_clone_lists_each_copy_twice_with_its_layer_one_and_shows_its_entity_twice_after() {
    // research/captures/bds-x2clone.txt: `A; B; A; B | extra: airA; airB`, then the chest's BlockEntityData twice.
    use acacia_proto::packets::{BlockEntityData, UpdateSubchunkBlocks};
    use sapling_api::{Copied, Item};
    let mut h = ready();
    h.remote.run(|ctx| {
        assert!(ctx.set_block([2, 65, -2], &Block::new("minecraft:stone")));
        assert!(ctx.set_block([3, 65, -2], &Block::new("minecraft:chest")));
        assert!(ctx.set_container_item([3, 65, -2], 0, Some(&Item::new("minecraft:stone", 5))));
    });
    h.tick_all();
    h.remote.run(|ctx| {
        let copy = |ctx: &sapling_api::ShardCtx<'_>, from: [i32; 3]| Copied { from, to: [from[0] + 3, 65, -2], block: ctx.block(from).expect("loaded"), entity: ctx.block_entity(from) };
        let copies = [copy(ctx, [2, 65, -2]), copy(ctx, [3, 65, -2])];
        ctx.copy_blocks(&[], &copies);
    });
    let sent = h.tick_all().to(1).to_vec();
    let at = |cells: Vec<acacia_proto::types::BlockUpdate>| cells.into_iter().map(|cell| [cell.position.x, cell.position.y, cell.position.z]).collect::<Vec<_>>();
    let listed: Vec<(Vec<[i32; 3]>, Vec<[i32; 3]>)> = all::<UpdateSubchunkBlocks>(&sent).into_iter().map(|batch| (at(batch.blocks), at(batch.extra))).collect();
    let (a, b) = ([5, 65, -2], [6, 65, -2]);
    assert_eq!(listed, [(vec![a, b, a, b], vec![a, b])]);
    let shown = |packet: &RawPacket| all::<BlockEntityData>(std::slice::from_ref(packet)).iter().any(|data| [data.position.x, data.position.y, data.position.z] == b);
    let listing = sent.iter().position(|packet| !all::<UpdateSubchunkBlocks>(std::slice::from_ref(packet)).is_empty()).expect("listed");
    let entities: Vec<usize> = sent.iter().enumerate().filter(|(_, packet)| shown(packet)).map(|(i, _)| i).collect();
    assert!(entities.len() == 2 && entities[0] > listing, "{entities:?} after {listing}");
}

#[test]
fn a_cloned_waterlogged_fence_takes_its_water_and_counts_it() {
    // research/captures/bds-x2clone.txt: a fence set over water cloned answers 2, its copy listing the water as `extra`.
    use sapling_api::Copied;
    let mut h = ready();
    h.remote.run(|ctx| {
        assert!(ctx.set_block([2, 64, 5], &Block::new("minecraft:water")));
        assert!(ctx.set_block([2, 64, 5], &Block::new("minecraft:oak_fence")));
    });
    h.tick_all();
    let (told, got) = std::sync::mpsc::channel();
    h.remote.run(move |ctx| {
        let copies = [Copied { from: [2, 64, 5], to: [4, 64, 5], block: ctx.block([2, 64, 5]).expect("loaded"), entity: None }];
        told.send(ctx.copy_blocks(&[], &copies)).expect("the test waits");
    });
    let sent = h.tick_all().to(1).to_vec();
    let counted = got.recv().expect("ran");
    let extra: Vec<String> = all::<acacia_proto::packets::UpdateSubchunkBlocks>(&sent).into_iter().flat_map(|batch| batch.extra).map(|cell| format!("{} {} {}", cell.position.x, cell.position.y, cell.position.z)).collect();
    assert_eq!((counted, extra), (2, vec!["4 64 5".to_owned()]));
}

#[test]
fn instant_health_in_creative_is_told_at_once() {
    // research/captures/bds-v2drip.txt: hurt in survival, then in creative `effect @s instant_health 1 5` told 20.
    let mut h = ready();
    h.remote.run(|ctx| assert!(ctx.hurt_player(ALEX, 10.0, DamageCause::Generic)));
    h.tick_all();
    h.remote.run(|ctx| ctx.set_game_mode(ALEX, GameMode::Creative));
    h.tick_all();
    h.remote.run(|ctx| assert!(ctx.give_effect(ALEX, Effect::InstantHealth, 1, 5, true)));
    let sent: Vec<RawPacket> = (0..3).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    assert_eq!(shown(&sent).into_iter().filter(|seen| seen.starts_with("health")).collect::<Vec<_>>(), ["health 20"]);
}
