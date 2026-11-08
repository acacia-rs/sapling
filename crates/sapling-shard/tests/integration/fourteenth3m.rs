//! Group M of the fourteenth wave: items, packets and commands (research/parity/FOURTEENTH-BRIEF.md).

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, LevelEvent, LevelEventEvent, LevelSoundEvent, RemoveEntity};

use crate::common::all;
use crate::fourteenth2c::{thrown, tick_of};

/// Where in `sent` the first packet `found` holds is.
fn at(sent: &[RawPacket], found: impl Fn(&RawPacket) -> bool) -> Option<usize> {
    sent.iter().position(found)
}

fn sound(name: &'static str) -> impl Fn(&RawPacket) -> bool {
    move |packet| packet.decode::<LevelSoundEvent>().is_ok_and(|sound| sound.sound_id == name)
}

fn removed(packet: &RawPacket) -> bool {
    packet.decode::<RemoveEntity>().is_ok()
}

fn events(sent: &[RawPacket], event: LevelEventEvent) -> Vec<LevelEvent> {
    all::<LevelEvent>(sent).into_iter().filter(|told| told.event == event).collect()
}

/// The tick `thrown` landed in (its first LevelEvent), and that the next tick tells it gone, then `heard`.
fn landed_then_heard_going(ticks: &[Vec<RawPacket>], heard: &'static str) -> usize {
    let landed = tick_of(ticks, |sent| !all::<LevelEvent>(sent).is_empty()).expect("landed");
    let going = &ticks[landed + 1];
    let (gone, sounded) = (at(going, removed).expect("gone the tick after"), at(going, sound(heard)).expect("heard as it goes"));
    assert!(gone < sounded, "RemoveEntity, then {heard}");
    if let Some(back) = at(going, sound("reappeared")) {
        assert!(sounded < back, "{heard}, then reappeared");
    }
    landed
}

/// What Alex, creative, holds after a pick (with `data` or not) of `block` set at (2, 65, 0) on `under`.
fn picked(block: &'static str, under: &'static str, data: bool) -> (Option<String>, String) {
    use acacia_proto::packets::{BlockPickRequest, InventoryContent};
    use sapling_api::{Block, GameMode, PlayerId, Slot};
    let mut h = crate::common::Harness::new();
    h.enter(1, "Alex");
    h.remote.run(move |ctx| {
        ctx.set_game_mode(PlayerId(1), GameMode::Creative);
        assert!(ctx.set_block([2, 64, 0], &Block::new(under)));
        assert!(ctx.set_block([2, 65, 0], &Block::new(block)));
        if block == "minecraft:chest" {
            assert!(ctx.set_container_item([2, 65, 0], 0, Some(&sapling_api::Item::new("minecraft:diamond", 3))));
        }
    });
    h.tick(1);
    h.receive(1, &BlockPickRequest { x: 2, y: 65, z: 0, add_user_data: data, selected_slot: 0 });
    let sent = h.tick(1);
    let shown = format!("{:?}", all::<InventoryContent>(&sent));
    let held = std::sync::Arc::new(std::sync::Mutex::new(None));
    let kept = held.clone();
    h.remote.run(move |ctx| *kept.lock().unwrap() = ctx.item(PlayerId(1), Slot::Main(0)).map(|item| item.name));
    h.tick(1);
    let held = held.lock().unwrap().clone();
    (held, shown)
}

#[test]
fn a_pick_gives_the_item_that_places_the_block_and_a_chest_s_items_with_data() {
    // research/captures/bds-m3pick.txt.
    for (block, under, item) in [
        ("minecraft:wheat", "minecraft:farmland", Some("minecraft:wheat_seeds")),
        ("minecraft:wall_sign", "minecraft:stone", Some("minecraft:oak_sign")),
        ("minecraft:lit_furnace", "minecraft:stone", Some("minecraft:furnace")),
        ("minecraft:unlit_redstone_torch", "minecraft:stone", Some("minecraft:redstone_torch")),
        ("minecraft:piston_arm_collision", "minecraft:stone", Some("minecraft:piston")),
        ("minecraft:fire", "minecraft:stone", None),
    ] {
        assert_eq!(picked(block, under, false).0.as_deref(), item, "{block}");
    }
    let (held, plain) = picked("minecraft:chest", "minecraft:stone", false);
    assert_eq!(held.as_deref(), Some("minecraft:chest"));
    assert!(!plain.contains("(+DATA)"));
    let (_, with_data) = picked("minecraft:chest", "minecraft:stone", true);
    assert!(with_data.contains("(+DATA)") && with_data.contains("minecraft:diamond") && with_data.contains("Findable"), "{with_data}");
}

#[test]
fn an_ender_chest_slot_is_set_and_read() {
    use sapling_api::{Item, PlayerId, Slot};
    let mut h = crate::common::Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        assert!(ctx.set_item(PlayerId(1), Slot::EnderChest(26), Some(&Item::new("minecraft:bread", 2))));
        assert!(!ctx.set_item(PlayerId(1), Slot::EnderChest(27), Some(&Item::new("minecraft:bread", 2))));
        assert_eq!(ctx.item(PlayerId(1), Slot::EnderChest(26)).map(|item| (item.name, item.count)), Some(("minecraft:bread".into(), 2)));
    });
    h.tick(1);
}

#[test]
fn an_egg_breaks_in_six_crumbs_and_is_heard_after_it_goes() {
    // research/captures/bds-f14cthrow.txt: six ParticleProjectileHit of data 25755662 (393 << 16 | 14), then
    // RemoveEntity, `undefined` and `reappeared` the tick after.
    let ticks = thrown("minecraft:egg", 0, 8);
    let landed = landed_then_heard_going(&ticks, "undefined");
    let crumbs = events(&ticks[landed], LevelEventEvent::ParticleProjectileHit);
    assert_eq!(crumbs.iter().map(|crumb| crumb.data).collect::<Vec<_>>(), [25_755_662; 6]);
}

#[test]
fn a_snowball_is_heard_after_it_goes() {
    landed_then_heard_going(&thrown("minecraft:snowball", 0, 8), "undefined");
}

#[test]
fn a_splash_potion_breaks_in_five_crumbs_and_its_glass_follows_its_going() {
    // bds-f14cthrow.txt: ParticleSplash, then five ParticleProjectileHit of 37421070 (571 << 16 | 14).
    let ticks = thrown("minecraft:splash_potion", 7, 8);
    let landed = landed_then_heard_going(&ticks, "glass");
    let told: Vec<(LevelEventEvent, i32)> = all::<LevelEvent>(&ticks[landed]).into_iter().map(|event| (event.event, event.data)).collect();
    assert_eq!((told.len(), &told[0].0), (6, &LevelEventEvent::ParticleSplash), "{told:?}");
    assert!(told[1..].iter().all(|crumb| *crumb == (LevelEventEvent::ParticleProjectileHit, 37_421_070)), "{told:?}");
}

#[test]
fn a_bottle_lets_out_its_orbs_before_it_goes_and_its_glass_after() {
    let ticks = thrown("minecraft:experience_bottle", 0, 8);
    let landed = landed_then_heard_going(&ticks, "glass");
    let going = &ticks[landed + 1];
    let orb = at(going, |packet| packet.decode::<AddEntity>().is_ok_and(|added| added.entity_type == "minecraft:xp_orb")).expect("orbs");
    assert!(orb < at(going, removed).unwrap());
}

#[test]
fn a_pearl_shows_the_teleport_and_two_portal_sounds_at_the_eyes() {
    // bds-f14cthrow.txt: ParticleEndermanTeleport where it landed, SoundPortal 1.62 over the thrower's
    // old feet and over the landing; RemoveEntity, `undefined` and `reappeared` the tick after.
    let ticks = thrown("minecraft:ender_pearl", 0, 8);
    let landed = landed_then_heard_going(&ticks, "undefined");
    let told = all::<LevelEvent>(&ticks[landed]);
    assert_eq!(told.iter().map(|event| event.event).collect::<Vec<_>>(), [LevelEventEvent::ParticleEndermanTeleport, LevelEventEvent::SoundPortal, LevelEventEvent::SoundPortal]);
    let (landing, left, arrived) = (&told[0].position, &told[1].position, &told[2].position);
    assert!((left.y - (65.0 + 1.62)).abs() < 1e-3, "{left:?}");
    assert!((arrived.y - (landing.y + 1.62)).abs() < 1e-3 && arrived.x == landing.x && arrived.z == landing.z, "{arrived:?}");
}

#[test]
fn a_bare_torch_with_no_floor_takes_the_name_of_its_first_wall() {
    use sapling_api::Block;
    // research/captures/bds-m3torch.txt: west before east before north before south; none is `unknown`.
    let side = |name: &str| match name {
        "east" => [3, 70, 0],
        "west" => [1, 70, 0],
        "north" => [2, 70, -1],
        _ => [2, 70, 1],
    };
    let cases: [(&[&str], &str); 7] = [(&["east"], "east"), (&["south"], "south"), (&["east", "north"], "east"), (&["west", "south"], "west"), (&["north", "south"], "north"), (&["east", "west", "north", "south"], "west"), (&[], "unknown")];
    let mut h = crate::common::Harness::new();
    h.enter(1, "Alex");
    for (walls, facing) in cases {
        let walls: Vec<[i32; 3]> = walls.iter().map(|wall| side(wall)).collect();
        let properties = crate::common::asking::ask(&mut h, move |ctx| {
            for name in ["east", "west", "north", "south"] {
                ctx.update_block(side(name), &Block::new("minecraft:air"));
            }
            for wall in walls {
                ctx.update_block(wall, &Block::new("minecraft:stone"));
            }
            ctx.update_block([2, 70, 0], &Block::new("minecraft:torch"));
            ctx.block([2, 70, 0]).map(|block| block.properties).unwrap_or_default()
        });
        assert_eq!(properties, format!("torch_facing_direction={facing}"));
    }
}

#[test]
fn a_player_s_eyes_under_water_clear_its_breathing_flag_told_with_its_air() {
    use acacia_proto::packets::SetEntityData;
    use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, MetadataFlags1};
    use sapling_api::{Block, PlayerId};
    // research/captures/bds-m3breath.txt: [Flags without BREATHING, Air 299] to the diver and its viewers
    // the tick its eyes go under, [Flags with it, Air] the tick they come out.
    let mut h = crate::common::Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    let breathing = |sent: &[RawPacket]| -> Vec<bool> {
        let told = all::<SetEntityData>(sent).into_iter().filter(|data| data.metadata.iter().any(|item| item.key == Key::Air));
        told.flat_map(|data| data.metadata).filter_map(|item| match item.value {
            Value::Flags(flags) if item.key == Key::Flags => Some(flags.contains(MetadataFlags1::BREATHING)),
            _ => None,
        })
        .collect()
    };
    h.remote.run(|ctx| {
        assert!(ctx.set_block([3, 65, 3], &Block::new("water")) && ctx.set_block([3, 66, 3], &Block::new("water")));
        ctx.teleport(PlayerId(1), [3.5, 65.0, 3.5]);
    });
    let (mut mine, mut seen) = (Vec::new(), Vec::new());
    for _ in 0..5 {
        let sent = h.tick_all();
        mine.extend(breathing(sent.to(1)));
        seen.extend(breathing(sent.to(2)));
    }
    assert_eq!((&mine, &seen), (&vec![false], &vec![false]));
    h.remote.run(|ctx| assert!(ctx.set_block([3, 65, 3], &Block::new("air")) && ctx.set_block([3, 66, 3], &Block::new("air"))));
    for _ in 0..5 {
        seen.extend(breathing(h.tick_all().to(2)));
    }
    assert_eq!(seen, [false, true]);
}

#[test]
fn a_viewer_who_comes_while_a_witch_drinks_sees_the_potion_after_its_add_entity() {
    use acacia_proto::packets::{AddEntity, MobEquipment};
    use sapling_api::{Difficulty, GameMode, PlayerId};
    // research/captures/bds-m3witch.txt: AddEntity, then MobEquipment of the potion (430, data 19) at once.
    let mut h = crate::common::Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    h.remote.run(|ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        ctx.set_game_mode(PlayerId(1), GameMode::Creative);
        ctx.teleport(PlayerId(2), [0.5, 65.0, 400.5]);
        assert!(ctx.spawn_entity("minecraft:witch", [0.5, 65.0, 2.5]).is_some());
    });
    let witch = all::<AddEntity>(h.tick_all().to(1))[0].runtime_id;
    h.receive(1, &crate::common::attack(witch));
    let drinking = (0..2400).any(|_| all::<MobEquipment>(h.tick_all().to(1)).iter().any(|held| held.runtime_entity_id == witch && held.item.metadata == 21));
    assert!(drinking, "the hurt witch drinks healing");
    h.remote.run(|ctx| ctx.teleport(PlayerId(2), [0.5, 65.0, 0.5]));
    let mut seen = Vec::new();
    for _ in 0..10 {
        seen.extend(h.tick_all().to(2).to_vec());
    }
    let added = at(&seen, |packet| packet.decode::<AddEntity>().is_ok_and(|added| added.runtime_id == witch)).expect("Steve is shown the witch");
    let held = at(&seen, |packet| packet.decode::<MobEquipment>().is_ok_and(|held| held.runtime_entity_id == witch && held.item.metadata == 21));
    assert!(held.is_some_and(|held| held > added), "{held:?} after {added}");
}

#[test]
fn damage_by_mace_smash_flashes_34_is_stopped_by_armour_and_dies_by_its_source() {
    use acacia_proto::packets::{DeathInfo, EntityEvent, EntityEventEventId};
    use sapling_api::{Attacker, DamageCause, GameMode, Item, PlayerId, Slot};
    // research/captures/bds-m3mace.txt: 4 through a diamond chestplate left 17.024, as entity_attack.
    let mut h = crate::common::Harness::new();
    h.enter(1, "Alex");
    let health = crate::common::asking::ask(&mut h, |ctx| {
        ctx.set_game_mode(PlayerId(1), GameMode::Survival);
        assert!(ctx.set_item(PlayerId(1), Slot::Armor(1), Some(&Item::new("minecraft:diamond_chestplate", 1))));
        assert!(ctx.hurt_player(PlayerId(1), 4.0, DamageCause::MaceSmash));
        ctx.health(PlayerId(1))
    });
    assert!(health.is_some_and(|health| (health - 17.024).abs() < 1e-3), "{health:?}");
    h.remote.run(|ctx| drop(ctx.hurt_player_by(PlayerId(1), 100.0, DamageCause::MaceSmash, &Attacker::Mob { kind: "minecraft:zombie".into() })));
    let mut sent = Vec::new();
    for _ in 0..12 {
        sent.extend(h.tick(1));
    }
    let flashes: Vec<i32> = all::<EntityEvent>(&sent).into_iter().filter(|event| event.event_id == EntityEventEventId::HurtAnimation).map(|event| event.data).collect();
    assert!(flashes.iter().all(|data| *data == 34), "{flashes:?}");
    let said: Vec<(String, Vec<String>)> = all::<DeathInfo>(&sent).into_iter().map(|info| (info.cause, info.messages)).collect();
    assert_eq!(said, [("death.attack.maceSmash.player".to_owned(), vec!["Alex".to_owned(), "%entity.zombie.name".to_owned()])]);
}
