//! Group L of the fourteenth brief (research/parity/FOURTEENTH-BRIEF.md): player lines.

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::manual::Uuid;
use acacia_proto::packets::{AddEntity, CommandRequest, DeathInfo, EntityEvent, EntityEventEventId, LevelEvent, LevelEventEvent, LevelSoundEvent, MobEffect, SetEntityData, UpdateAttributes};
use acacia_proto::types::{CommandOrigin, InputData,MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Meta, MetadataDictionaryItemValueDefault as Plain};
use common::stored::{enchanted, enter_stored, floor_with};
use common::{Harness, all, config, input_at};
use sapling_api::{Block, Command, DamageCause, Difficulty, Effect, GameMode, Item, MobSlot, Output, PlayerId, Sender, ShardCtx, Slot};
use sapling_world::{PlayerData, World, WorldStorage};

const ALEX: PlayerId = PlayerId(1);

/// Each `Air` Alex was told of itself.
fn airs(data: &[SetEntityData]) -> Vec<i16> {
    let air = |data: &SetEntityData| data.metadata.iter().find_map(|item| if let (Key::Air, Meta::Default(Plain::Short(air))) = (item.key, &item.value) { Some(*air) } else { None });
    data.iter().filter(|data| data.runtime_entity_id == 1).filter_map(air).collect()
}

/// Puts the sender under the stone at 3 66 0: `/under`.
#[derive(Command)]
#[command(crate = "sapling_api")]
struct Under;

impl Command for Under {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, _: &mut Output) {
        ctx.teleport(sender.player().expect("a player").id, [3.5, 65.0, 0.5]);
    }
}

/// research/captures/bds-t2ebreath.txt: put with only its head in stone, a survival player's `Air` goes
/// 299 the tick before it is laid down to crawl, then 300 again, unhurt.
#[test]
fn a_player_put_with_its_head_in_stone_dips_its_air_once_before_crawling() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_game_mode(ALEX, GameMode::Survival);
        assert!(ctx.set_block([3, 66, 0], &Block::new("stone")));
    });
    h.shard.add_command::<Under>();
    h.tick_all();
    // A command, as the drill's `/tp`: it runs before the tick's vitals, where a plugin's call runs after.
    let origin = CommandOrigin { r#type: "player".into(), uuid: Uuid::default(), request_id: String::new(), player_entity_id: 0 };
    h.receive(1, &CommandRequest { command: "/under".into(), origin, internal: false, version: "latest".into() });
    let mut told = Vec::new();
    for _ in 0..6 {
        told.extend(airs(&all::<SetEntityData>(h.tick_all().to(1))));
    }
    assert_eq!(told, [299, 300]);
}

/// research/captures/bds-l1stand.txt: each lone hit on an armour stand cost the survival hitter 0.1
/// exhaustion, told at once, as a hit on a mob.
#[test]
fn a_hit_on_an_armour_stand_tires_the_hitter() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_game_mode(ALEX, GameMode::Survival);
        assert!(ctx.spawn_entity("minecraft:armor_stand", [2.5, 65.0, 0.5]).is_some());
    });
    let shown: Vec<RawPacket> = (0..2).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    let stand = all::<AddEntity>(&shown).into_iter().find(|added| added.entity_type == "minecraft:armor_stand").expect("shown").runtime_id;
    let mut tired = Vec::new();
    for _ in 0..2 {
        h.receive(1, &common::attack(stand));
        let told = h.tick_all();
        tired.extend(all::<UpdateAttributes>(told.to(1)).into_iter().filter(|told| told.runtime_entity_id == 1).flat_map(|told| told.attributes).filter(|it| it.name == "minecraft:player.exhaustion").map(|it| (it.current * 10.0).round() / 10.0));
        (0..20).for_each(|_| drop(h.tick_all()));
    }
    assert_eq!(tired, [0.1, 0.2]);
}

/// research/captures/bds-mace-breach.txt: a mace's 6 on a zombie in a diamond chestplate (10 points with
/// its own 2) took 4.176 with no Breach and the whole 6 with Breach IV.
#[test]
fn breach_pierces_the_armour_a_mob_wears() {
    let mut taken = Vec::new();
    for breach in [0, 4] {
        let data = PlayerData { main: vec![enchanted("minecraft:mace", &[(40, breach)], 0)], ..PlayerData::default() };
        let storage = floor_with(Vec::new());
        storage.store_player(&acacia_auth::login::offline_identity("Steve"), &data).unwrap();
        let mut h = Harness::with(config(), World::with_storage(storage));
        enter_stored(&mut h, 1);
        (0..12).for_each(|_| drop(h.tick(1)));
        common::asking::ask(&mut h, |ctx| ctx.set_difficulty(Difficulty::Easy));
        let zombie = common::asking::ask(&mut h, |ctx| ctx.spawn_entity("minecraft:zombie", [2.5, 64.0, 0.5])).expect("a zombie");
        common::asking::ask(&mut h, move |ctx| ctx.set_mob_item(zombie, MobSlot::Armor(1), Some(&Item::new("minecraft:diamond_chestplate", 1)))).expect("worn");
        h.tick(1);
        h.receive(1, &common::attack(zombie.0));
        let left: Vec<f32> = all::<UpdateAttributes>(&h.tick(1)).into_iter().filter(|told| told.runtime_entity_id == zombie.0).flat_map(|told| told.attributes).filter(|it| it.name == "minecraft:health").map(|it| it.current).collect();
        taken.push(20.0 - left[0]);
    }
    assert!((taken[0] - 4.176).abs() < 1e-3 && (taken[1] - 6.0).abs() < 1e-3, "{taken:?}");
}

/// research/captures/bds-q3witch.txt: after `effect clear` BDS told the removals, then at the sync
/// `VisibleMobEffects` and one UpdateAttributes of the walk and the health; bds-r1fx.txt: a speed given
/// told its walk at the sync too.
#[test]
fn effects_given_and_cleared_tell_their_attributes_at_the_sync_after_the_shown_effects() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    let names = |sent: &[RawPacket]| -> Vec<(usize, Vec<String>)> {
        sent.iter().enumerate().filter_map(|(at, packet)| packet.decode::<UpdateAttributes>().ok().filter(|told| told.runtime_entity_id == 1).map(|told| (at, told.attributes.into_iter().map(|item| item.name).collect()))).collect()
    };
    let shown_at = |sent: &[RawPacket]| sent.iter().rposition(|packet| packet.decode::<SetEntityData>().is_ok_and(|data| data.runtime_entity_id == 1 && data.metadata.iter().any(|item| item.key == Key::VisibleMobEffects)));
    h.remote.run(|ctx| assert!(ctx.give_effect(ALEX, Effect::Speed, 600, 1, true)));
    let sent = h.tick_all();
    for viewer in [1, 2] {
        let told = names(sent.to(viewer));
        assert_eq!(told.iter().map(|(_, names)| names.clone()).collect::<Vec<_>>(), [vec!["minecraft:movement".to_string()]]);
        assert!(shown_at(sent.to(viewer)).is_some_and(|shown| shown < told[0].0), "the shown effects first");
    }
    h.remote.run(|ctx| assert!(ctx.give_effect(ALEX, Effect::HealthBoost, 600, 1, false)));
    h.tick_all();
    h.remote.run(|ctx| assert!(ctx.clear_effects(ALEX)));
    let sent = h.tick_all();
    for viewer in [1, 2] {
        let told = sent.to(viewer);
        let last_removal = told.iter().rposition(|packet| packet.decode::<MobEffect>().is_ok()).expect("the removals");
        let attributes = names(told);
        assert_eq!(attributes.iter().map(|(_, names)| names.clone()).collect::<Vec<_>>(), [vec!["minecraft:movement".to_string(), "minecraft:health".to_string()]]);
        assert!(shown_at(told).is_some_and(|shown| last_removal < shown && shown < attributes[0].0), "removals, shown effects, then the attributes");
    }
}

/// research/captures/bds-l1mace.txt: a smash on a player flashed `MaceSmash` (34); on a target off the
/// ground BDS sounded `mace.smash_air` and raised no dust.
#[test]
fn a_smash_on_a_player_flashes_mace_smash_and_one_off_the_ground_sounds_smash_air_without_dust() {
    for (steve_y, sound, dusty) in [(65.0, "mace.smash_ground", true), (67.0, "mace.smash_air", false)] {
        let mut h = Harness::new();
        h.enter(1, "Alex");
        h.enter(2, "Steve");
        h.remote.run(|ctx| assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new("minecraft:mace", 1)))));
        // A fall of 1.6: (6 + 6.4) x 1.5 leaves Steve standing.
        for y in [66.6, 68.6, 68.5, 67.0] {
            h.receive(1, &input_at([0.5, y, 0.5]));
            let mut steve = input_at([2.0, steve_y, 0.5]);
            if dusty {
                (steve.input_data, steve.delta.y) = (vec![InputData::VerticalCollision], -0.0784);
            }
            h.receive(2, &steve);
            h.tick_all();
        }
        h.receive(1, &common::attack(2));
        let sent = h.tick_all().to(2).to_vec();
        let flashed: Vec<i32> = all::<EntityEvent>(&sent).into_iter().filter(|event| event.runtime_entity_id == 2 && event.event_id == EntityEventEventId::HurtAnimation).map(|event| event.data).collect();
        assert_eq!(flashed, [34], "Steve at {steve_y}");
        assert!(all::<LevelSoundEvent>(&sent).iter().any(|heard| heard.sound_id == sound), "Steve at {steve_y}");
        assert_eq!(all::<LevelEvent>(&sent).iter().any(|event| event.event == LevelEventEvent::AddParticleSmashAttackGroundDust), dusty, "Steve at {steve_y}");
    }
}

/// Each death a player's message and its names.
fn deaths(sent: &[RawPacket]) -> Vec<(String, Vec<String>)> {
    all::<DeathInfo>(sent).into_iter().map(|info| (info.cause, info.messages)).collect()
}

/// research/captures/bds-l1death.txt: killed by the Thorns a guardian wore, BDS's player died of
/// `death.attack.thorns` naming the wearer.
#[test]
fn a_player_killed_by_the_thorns_another_wears_dies_of_them_by_the_wearer() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    h.remote.run(|ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        let thorny = Item { enchantments: vec![(5, 15)], ..Item::new("minecraft:iron_chestplate", 1) };
        assert!(ctx.set_item(PlayerId(2), Slot::Armor(1), Some(&thorny)));
        assert!(ctx.hurt_player(ALEX, 18.0, DamageCause::Generic));
    });
    (0..12).for_each(|_| drop(h.tick_all()));
    h.receive(1, &common::attack(2));
    let told: Vec<RawPacket> = (0..3).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    assert_eq!(deaths(&told), [("death.attack.thorns".to_owned(), vec!["Alex".to_owned(), "Steve".to_owned()])]);
}

/// research/captures/bds-l1death.txt: put at 1 health standing on magma, and on top of a cactus, BDS's
/// player died of `death.attack.magma` and `death.attack.cactus`.
#[test]
fn a_player_standing_on_magma_or_on_a_cactus_dies_of_it() {
    for (block, feet, said) in [("magma", 65.0, "death.attack.magma"), ("cactus", 66.0, "death.attack.cactus")] {
        let mut h = Harness::new();
        h.enter(1, "Alex");
        h.remote.run(move |ctx| {
            assert!(ctx.set_block([0, feet as i32 - 1, 0], &Block::new(block)));
            assert!(ctx.hurt_player(ALEX, 19.0, DamageCause::Generic));
        });
        let mut told = Vec::new();
        for _ in 0..16 {
            let mut standing = input_at([0.5, feet, 0.5]);
            (standing.input_data, standing.delta.y) = (vec![InputData::VerticalCollision], -0.0784);
            h.receive(1, &standing);
            told.extend(h.tick_all().to(1).to_vec());
        }
        assert_eq!(deaths(&told), [(said.to_owned(), vec!["Alex".to_owned()])], "{block}");
    }
}

/// research/captures/bds-l1death.txt: pricked to death walking in a ripe berry bush, BDS's player died
/// of `death.attack.sweetBerry`, not a cactus's `contact`.
#[test]
fn a_player_pricked_to_death_in_a_berry_bush_dies_of_sweet_berries() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        assert!(ctx.set_block([0, 65, 0], &Block::new("sweet_berry_bush").with("growth=3")));
        assert!(ctx.hurt_player(ALEX, 19.0, DamageCause::Generic));
    });
    (0..12).for_each(|_| drop(h.tick_all()));
    let mut told = Vec::new();
    for x in [0.3, 0.5, 0.7, 0.5, 0.3, 0.5] {
        h.receive(1, &input_at([x, 65.0, 0.5]));
        told.extend(h.tick_all().to(1).to_vec());
    }
    assert_eq!(deaths(&told), [("death.attack.sweetBerry".to_owned(), vec!["Alex".to_owned()])]);
}
