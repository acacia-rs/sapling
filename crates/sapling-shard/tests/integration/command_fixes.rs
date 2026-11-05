//! What BDS's commands did beyond the block or hit named (research/captures/bds-fixset.txt,
//! bds-fixmisc.txt, bds-fixreplace.txt): `/setblock` of beds, vines and coral fans, `/damage ...
//! entity <who>`, and `/replaceitem entity` on mobs and mounts.

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{DeathInfo, LevelSoundEvent, MobArmorEquipment, MobEquipment, SetEntityData, UpdateAttributes, UpdateBlock, UpdateSubchunkBlocks};
use common::asking::ask;
use common::{Harness, all};
use sapling_api::{Attacker, Block, DamageCause, Difficulty, EntityId, GameMode, Item, MobSlot, MobSlotRefusal, PlayerId, ShardCtx};

const ALEX: PlayerId = PlayerId(1);

fn alex() -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h
}

/// What `call` returned on the shard's next tick, and what that tick sent Alex.
fn told<T: Send + 'static>(h: &mut Harness, call: impl FnOnce(&mut ShardCtx<'_>) -> T + Send + 'static) -> (T, Vec<RawPacket>) {
    let answer = std::sync::Arc::new(std::sync::Mutex::new(None));
    let put = answer.clone();
    h.remote.run(move |ctx| *put.lock().unwrap() = Some(call(ctx)));
    let sent = h.tick_all().to(1).to_vec();
    (answer.lock().unwrap().take().expect("the shard ran the call"), sent)
}

fn properties(h: &mut Harness, pos: [i32; 3]) -> String {
    ask(h, move |ctx| ctx.block(pos).map(|block| format!("{}[{}]", block.name, block.properties)).unwrap_or_default())
}

#[test]
fn a_bed_set_by_command_lies_whole_with_its_head_in_the_cell_named() {
    let mut h = alex();
    let (set, sent) = told(&mut h, |ctx| ctx.update_block([10, 65, -4], &Block::new("minecraft:bed")));
    assert!(set);
    assert_eq!(all::<UpdateSubchunkBlocks>(&sent).len(), 1, "both shown together");
    // research/captures/bds-e3sets.txt: a tick on, both again, foot first, and a sensor hears it.
    let again = h.tick_all().to(1).to_vec();
    let shown: Vec<i32> = all::<UpdateSubchunkBlocks>(&again).iter().flat_map(|packet| packet.blocks.iter().map(|block| block.position.z)).collect();
    assert_eq!((shown, all::<acacia_proto::packets::BlockEntityData>(&again).len()), (vec![-5, -4], 2));
    assert!(properties(&mut h, [10, 65, -4]).contains("head_piece_bit=1"), "the named cell is the head");
    assert!(properties(&mut h, [10, 65, -5]).contains("head_piece_bit=0"), "the foot behind it, as BDS's direction 0");
    ask(&mut h, |ctx| ctx.update_block([12, 65, -2], &Block::new("minecraft:sculk_sensor")));
    ask(&mut h, |ctx| ctx.update_block([10, 65, -4], &Block::new("minecraft:air")));
    ask(&mut h, |ctx| ctx.update_block([10, 65, -4], &Block::new("minecraft:bed")));
    (0..6).for_each(|_| _ = h.tick_all());
    assert!(properties(&mut h, [12, 65, -2]).contains("sculk_sensor_phase=1"), "the set bed is heard");
    ask(&mut h, |ctx| ctx.update_block([12, 65, -6], &Block::new("minecraft:bed").with("direction=1,head_piece_bit=1")));
    assert!(properties(&mut h, [13, 65, -6]).starts_with("minecraft:bed["), "a west-facing bed's foot lies east");
}

#[test]
fn a_bed_set_to_air_shows_the_other_half_s_dust_then_both_halves_together() {
    // research/captures/bds-e3sets.txt: dust at the head's corner, then one list, the cell set first.
    let mut h = alex();
    ask(&mut h, |ctx| ctx.update_block([10, 65, -4], &Block::new("minecraft:bed")));
    (0..2).for_each(|_| _ = h.tick_all());
    let (_, sent) = told(&mut h, |ctx| ctx.update_block([10, 65, -5], &Block::air()));
    let dust: Vec<[f32; 3]> = all::<acacia_proto::packets::LevelEvent>(&sent).iter().filter(|event| event.event == acacia_proto::packets::LevelEventEvent::ParticleDestroyBlockNoSound).map(|event| [event.position.x, event.position.y, event.position.z]).collect();
    let listed: Vec<Vec<i32>> = all::<UpdateSubchunkBlocks>(&sent).iter().map(|packet| packet.blocks.iter().map(|block| block.position.z).collect()).collect();
    assert_eq!((dust, listed, all::<UpdateBlock>(&sent).len()), (vec![[10.0, 65.0, -4.0]], vec![vec![-5, -4]], 0));
}

#[test]
fn nether_and_cave_vines_set_by_command_take_a_random_age_unless_one_is_named() {
    let mut h = alex();
    let aged: Vec<String> = (0..4)
        .map(|x| {
            ask(&mut h, move |ctx| ctx.update_block([x, 65, 2], &Block::new("minecraft:twisting_vines")));
            properties(&mut h, [x, 65, 2])
        })
        .collect();
    assert!(aged.iter().any(|block| !block.contains("twisting_vines_age=0")), "{aged:?}");
    // BDS lists an aged vine twice in one UpdateSubchunkBlocks (bds-fixset.txt 25081); a named age once.
    let (_, sent) = told(&mut h, |ctx| ctx.update_block([5, 65, 2], &Block::new("minecraft:twisting_vines")));
    assert!(all::<UpdateBlock>(&sent).is_empty() && all::<UpdateSubchunkBlocks>(&sent).iter().map(|shown| (shown.blocks.len(), shown.extra.len())).eq([(2, 0)]));
    let (_, sent) = told(&mut h, |ctx| ctx.update_block([6, 65, 2], &Block::new("minecraft:twisting_vines").with("twisting_vines_age=3")));
    assert!(properties(&mut h, [6, 65, 2]).contains("twisting_vines_age=3"));
    assert!(all::<UpdateSubchunkBlocks>(&sent).is_empty() && all::<UpdateBlock>(&sent).len() == 1);
}

#[test]
fn a_live_coral_fan_set_by_command_keeps_a_water_source_beside_itself() {
    let mut h = alex();
    let (_, sent) = told(&mut h, |ctx| ctx.update_block([4, 65, 2], &Block::new("minecraft:tube_coral_fan")));
    // BDS: one UpdateSubchunkBlocks, the fan in layer 0 and its water as `extra`, then as `flowing_water`
    // (bds-fixset.txt 15033, bds-bkticks.txt 11035).
    let cells = |updates: &[acacia_proto::types::BlockUpdate]| updates.iter().map(|update| [update.position.x, update.position.y, update.position.z]).collect::<Vec<_>>();
    let shown = all::<UpdateSubchunkBlocks>(&sent);
    assert!(all::<UpdateBlock>(&sent).is_empty() && shown.len() == 1, "{shown:?}");
    assert_eq!((cells(&shown[0].blocks), cells(&shown[0].extra)), (vec![[4, 65, 2]], vec![[4, 65, 2]; 2]));
    assert_ne!(shown[0].extra[0].runtime_id, shown[0].extra[1].runtime_id, "water, then flowing_water");
    for _ in 0..10 {
        h.tick_all();
    }
    let around = [[4, 64, 2], [3, 65, 2], [5, 65, 2], [4, 65, 1], [4, 65, 3]];
    assert!(around.into_iter().any(|pos| properties(&mut h, pos).contains("water")), "the water runs, as BDS's did");
    let (_, sent) = told(&mut h, |ctx| ctx.update_block([8, 65, 2], &Block::new("minecraft:dead_tube_coral_fan")));
    assert!(!all::<UpdateBlock>(&sent).iter().any(|update| update.layer == 1), "a dead fan stays dry");
}

#[test]
fn a_player_killed_by_damage_from_a_mob_dies_by_that_mob() {
    let mut h = alex();
    let (_, sent) = told(&mut h, |ctx| {
        ctx.set_game_mode(ALEX, GameMode::Survival);
        ctx.hurt_player_by(ALEX, 100.0, DamageCause::Projectile, &Attacker::Mob { kind: "minecraft:zombie".into() })
    });
    let said: Vec<(String, Vec<String>)> = all::<DeathInfo>(&sent).into_iter().map(|info| (info.cause, info.messages)).collect();
    assert_eq!(said, [("death.attack.mob".to_owned(), vec!["Alex".to_owned(), "%entity.zombie.name".to_owned()])]);
}

fn summoned(h: &mut Harness, kind: &'static str) -> EntityId {
    ask(h, move |ctx| ctx.set_difficulty(Difficulty::Easy));
    ask(h, move |ctx| ctx.summon_as(None, kind, [4.5, 65.0, 0.5], 0.0, 0.0, Some("minecraft:spawn_tame_adult"))).expect("summoned")
}

fn put(h: &mut Harness, id: EntityId, slot: MobSlot, item: &'static str) -> (Result<(), MobSlotRefusal>, Vec<RawPacket>) {
    told(h, move |ctx| ctx.set_mob_item(id, slot, Some(&Item::new(item, 1))))
}

fn sounds(sent: &[RawPacket]) -> Vec<String> {
    all::<LevelSoundEvent>(sent).into_iter().map(|sound| sound.sound_id).collect()
}

#[test]
fn a_zombie_wears_and_holds_what_replaceitem_gives_it_as_bds_showed() {
    let mut h = alex();
    let zombie = summoned(&mut h, "minecraft:zombie");
    let (put_on, sent) = put(&mut h, zombie, MobSlot::Armor(0), "minecraft:iron_helmet");
    assert_eq!(put_on, Ok(()));
    assert_eq!(all::<MobArmorEquipment>(&sent).len(), 1);
    assert_eq!(sounds(&sent), ["armor.equip_iron"]);
    let (_, sent) = put(&mut h, zombie, MobSlot::MainHand, "minecraft:iron_sword");
    assert_eq!((all::<MobEquipment>(&sent).len(), sounds(&sent).len()), (1, 0), "a hand is shown, not heard");
    let (put_on, sent) = put(&mut h, zombie, MobSlot::Armor(3), "minecraft:stone");
    assert_eq!((put_on, all::<MobArmorEquipment>(&sent).len()), (Ok(()), 0), "stone on the feet is taken unseen");
    let plated = summoned(&mut h, "minecraft:zombie");
    assert_eq!(put(&mut h, plated, MobSlot::Armor(1), "minecraft:diamond_chestplate").0, Ok(()));
    let (_, mut sent) = told(&mut h, move |ctx| assert!(ctx.hurt_entity(plated, 8.0, DamageCause::Generic)));
    sent.extend(h.tick_all().to(1).to_vec());
    let left: Vec<f32> = all::<UpdateAttributes>(&sent).into_iter().filter(|told| told.runtime_entity_id == plated.0).flat_map(|told| told.attributes).filter(|it| it.name == "minecraft:health").map(|it| it.current).collect();
    assert!(left.first().is_some_and(|left| (left - 14.176).abs() < 1e-3), "the chestplate is worn against harm, as bds-boosts.txt: {left:?}");
    assert_eq!(put(&mut h, zombie, MobSlot::Saddle, "minecraft:saddle").0, Err(MobSlotRefusal::NoSlot));
    assert_eq!(put(&mut h, zombie, MobSlot::Inventory(0), "minecraft:stone").0, Err(MobSlotRefusal::NoSlot));
}

#[test]
fn a_tamed_horse_leaves_its_tack_after_its_loot_or_at_once_without_mob_loot() {
    use acacia_proto::packets::AddItemEntity;
    // research/captures/bds-e1loot.txt.
    for loot in [true, false] {
        let mut h = alex();
        ask(&mut h, move |ctx| assert!(ctx.set_game_rule("doMobLoot", sapling_api::RuleValue::Bool(loot))));
        let horse = summoned(&mut h, "minecraft:horse");
        put(&mut h, horse, MobSlot::Saddle, "minecraft:saddle");
        put(&mut h, horse, MobSlot::MountArmor, "minecraft:iron_horse_armor");
        let (_, at_death) = told(&mut h, move |ctx| assert!(ctx.kill_entity(horse)));
        let after = h.tick_all().to(1).to_vec();
        let names = |sent: &[RawPacket]| -> Vec<String> { all::<AddItemEntity>(sent).into_iter().filter_map(|item| sapling_data::item_name(item.item.network_id.into())).map(|name| name.trim_start_matches("minecraft:").to_owned()).filter(|name| name != "leather").collect() };
        let (now, then) = (names(&at_death), names(&after));
        match loot {
            true => assert_eq!((now, then), (vec![], vec!["iron_horse_armor".to_owned(), "saddle".to_owned()])),
            false => assert_eq!((now, then), (vec!["saddle".to_owned(), "iron_horse_armor".to_owned()], vec![])),
        }
    }
}

#[test]
fn mounts_take_tack_by_slot_as_bds_s_did() {
    let mut h = alex();
    let horse = summoned(&mut h, "minecraft:horse");
    let (saddled, sent) = put(&mut h, horse, MobSlot::Saddle, "minecraft:saddle");
    assert_eq!((saddled, sounds(&sent)), (Ok(()), vec!["saddle".to_owned()]));
    assert_eq!((all::<SetEntityData>(&sent).len(), all::<MobArmorEquipment>(&sent).len()), (1, 0), "a saddle shows in the flags alone");
    let (armoured, sent) = put(&mut h, horse, MobSlot::MountArmor, "minecraft:iron_horse_armor");
    assert_eq!((armoured, sounds(&sent), all::<MobArmorEquipment>(&sent).len()), (Ok(()), vec!["armor".to_owned()], 1));
    assert!(all::<SetEntityData>(&sent).is_empty(), "armour leaves the flags be");
    let (_, sent) = put(&mut h, horse, MobSlot::Inventory(0), "minecraft:diamond");
    assert_eq!((all::<SetEntityData>(&sent).len(), all::<MobArmorEquipment>(&sent).len()), (1, 0), "the saddle swapped out");
    let (_, sent) = put(&mut h, horse, MobSlot::Inventory(0), "minecraft:emerald");
    assert!(all::<SetEntityData>(&sent).is_empty(), "flags unchanged are not retold");
    assert_eq!(put(&mut h, horse, MobSlot::Chest(0), "minecraft:chest").0, Err(MobSlotRefusal::NoSlot));
    let donkey = summoned(&mut h, "minecraft:donkey");
    assert_eq!(put(&mut h, donkey, MobSlot::MountArmor, "minecraft:iron_horse_armor").0, Err(MobSlotRefusal::NoSlot));
    assert_eq!(put(&mut h, donkey, MobSlot::Chest(0), "minecraft:chest").0, Err(MobSlotRefusal::BadSlot { size: 0 }), "no chest carried");
    let llama = summoned(&mut h, "minecraft:llama");
    assert_eq!(put(&mut h, llama, MobSlot::Saddle, "minecraft:saddle").0, Err(MobSlotRefusal::NoSlot));
    let (_, sent) = put(&mut h, llama, MobSlot::MountArmor, "minecraft:iron_horse_armor");
    assert!(sent.is_empty() || all::<MobArmorEquipment>(&sent).is_empty(), "horse armour on a llama is taken unseen");
    let (_, sent) = put(&mut h, llama, MobSlot::Inventory(0), "minecraft:white_carpet");
    assert_eq!(sounds(&sent), ["armor"], "a llama's first slot is its carpet");
}
