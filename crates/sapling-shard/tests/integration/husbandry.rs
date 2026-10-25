//! Breeding, babies, taming, owners, leads and raising kept in a world folder, driven by a
//! player's clicks (creatures/husbandry, docs/research/husbandry.md).

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::nbt::Value;
use acacia_proto::packets::{AddEntity, AddItemEntity, EntityEvent, EntityEventEventId as Event};
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValueDefault as Plain, MetadataFlags1 as Flags};
use common::raising::{click_on, feet_of, has_flag, plain_in, set_in, showed};
use common::shooting::{ALEX, added, held, ready, spawn, watch};
use common::stored::{actor_with, enter_stored, floor_holding};
use common::{Harness, all, attack, click, orbs};
use sapling_api::{Block, GameMode, Slot};
use sapling_world::{Actor, ActorState, ChunkPos, World, WorldStorage};

const COW: &str = "minecraft:cow";
const WOLF: &str = "minecraft:wolf";

#[test]
fn cows_fed_wheat_have_a_calf_with_experience_that_feeding_grows_up_sooner() {
    let mut h = ready(GameMode::Survival, &[("minecraft:wheat", 12, 0)], 0.0);
    let (a, b) = (spawn(&mut h, COW, [2.5, 65.0, 0.5]), spawn(&mut h, COW, [3.5, 65.0, 0.5]));
    let fed = [click_on(&mut h, a), click_on(&mut h, b)].concat();
    assert!(showed(&fed, a, Event::InLoveParticles) && showed(&fed, b, Event::InLoveParticles), "hearts");
    assert_eq!((has_flag(&fed, a, Flags::INLOVE), has_flag(&fed, b, Flags::INLOVE)), (Some(true), Some(true)));

    let sent = watch(&mut h, 200);
    let calves = added(&sent, COW);
    assert_eq!(calves.len(), 1, "one calf");
    let calf = calves[0].runtime_id;
    assert_eq!(has_flag(&sent, calf, Flags::BABY), Some(true));
    assert_eq!(plain_in(&calves[0].metadata, Key::Scale), Some(Plain::Float(0.5)));
    assert!(matches!(orbs::worths(&sent).iter().sum::<i32>(), 1..=7), "experience");
    assert_eq!(has_flag(&sent, a, Flags::INLOVE), Some(false), "out of love once bred");
    click_on(&mut h, a);
    assert_eq!(held(&mut h, Slot::Main(0)).map(|wheat| wheat.count), Some(10), "a parent resting from breeding takes no wheat");

    let mut fed: Vec<RawPacket> = (0..10).flat_map(|_| click_on(&mut h, calf)).collect();
    assert!(showed(&fed, calf, Event::BabyAnimalFeed));
    fed.extend(watch(&mut h, 2));
    assert_eq!(has_flag(&fed, calf, Flags::BABY), Some(false), "ten wheat grow a calf up");
    assert_eq!(set_in(&fed, calf, Key::Scale), Some(Plain::Float(1.0)));
    assert!(held(&mut h, Slot::Main(0)).is_none(), "every wheat was eaten");
}

/// A wolf at `feet` tamed by Alex with bones: its runtime id, and what Alex was told as it was tamed.
fn tamed_wolf(h: &mut Harness, feet: [f32; 3]) -> (u64, Vec<RawPacket>) {
    let wolf = spawn(h, WOLF, feet);
    for _ in 0..40 {
        let sent = click_on(h, wolf);
        if showed(&sent, wolf, Event::TameSuccess) {
            return (wolf, sent);
        }
        assert!(showed(&sent, wolf, Event::TameFail), "a failed bone smokes");
    }
    panic!("a third of bones tame");
}

#[test]
fn a_wolf_fed_bones_is_tamed_sits_at_its_owners_click_and_follows_it_far() {
    let mut h = ready(GameMode::Survival, &[("minecraft:bone", 64, 0)], 0.0);
    let (wolf, tamed) = tamed_wolf(&mut h, [2.5, 65.0, 0.5]);
    assert_eq!(has_flag(&tamed, wolf, Flags::TAMED), Some(true));
    assert!(matches!(set_in(&tamed, wolf, Key::OwnerEid), Some(Plain::Long(owner)) if owner >= 0), "its owner shown");
    let sat = click_on(&mut h, wolf);
    assert_eq!(has_flag(&sat, wolf, Flags::SITTING), Some(true));
    h.remote.run(|ctx| ctx.teleport(ALEX, [-7.5, 65.0, -7.5]));
    watch(&mut h, 40);
    assert!((feet_of(&mut h, wolf)[0] - 2.5).abs() < 0.5, "a sitting wolf stays");

    let stood = click_on(&mut h, wolf);
    assert_eq!(has_flag(&stood, wolf, Flags::SITTING), Some(false));
    watch(&mut h, 10);
    let at = feet_of(&mut h, wolf);
    assert!((at[0] + 7.5).hypot(at[2] + 7.5) < 4.5, "left 12 blocks behind, it came to its owner: {at:?}");
}

#[test]
fn a_tamed_wolf_bites_what_its_owner_hits() {
    let mut h = ready(GameMode::Survival, &[("minecraft:bone", 64, 0)], 0.0);
    let (wolf, _) = tamed_wolf(&mut h, [-2.5, 65.0, 0.5]);
    let pig = spawn(&mut h, "minecraft:pig", [1.5, 65.0, 0.5]);
    h.receive(1, &attack(pig));
    let sent = watch(&mut h, 80);
    let hurt = all::<EntityEvent>(&sent).iter().filter(|event| event.runtime_entity_id == pig && event.event_id == Event::HurtAnimation).count();
    assert!(hurt >= 2, "the wolf bit the pig too: {hurt} hurts");
    assert!(showed(&sent, wolf, Event::ArmSwing));
}

#[test]
fn a_lead_ties_a_cow_to_its_holder_or_a_fence_knot_and_snaps_far_away() {
    let mut h = ready(GameMode::Survival, &[("minecraft:lead", 2, 0)], 0.0);
    h.remote.run(|ctx| assert!(ctx.set_block([2, 65, 2], &Block::new("minecraft:oak_fence"))));
    let cow = spawn(&mut h, COW, [2.5, 65.0, 0.5]);
    let tied = [click_on(&mut h, cow), watch(&mut h, 1)].concat();
    assert_eq!(has_flag(&tied, cow, Flags::LEASHED), Some(true));
    assert!(matches!(set_in(&tied, cow, Key::LeadHolderEid), Some(Plain::Long(holder)) if holder >= 0), "its holder shown");
    assert_eq!(held(&mut h, Slot::Main(0)).map(|lead| lead.count), Some(1));

    h.receive(1, &click([2, 65, 2], 1, None));
    let knotted = watch(&mut h, 2);
    let knot = added(&knotted, "minecraft:leash_knot");
    assert_eq!(knot.len(), 1, "a knot on the fence");
    assert_eq!(set_in(&knotted, cow, Key::LeadHolderEid), Some(Plain::Long(knot[0].runtime_id as i64)));
    let undone = [click_on(&mut h, knot[0].runtime_id), watch(&mut h, 2)].concat();
    assert_eq!(has_flag(&undone, cow, Flags::LEASHED), Some(false), "a click on the knot lets go");
    assert!(!all::<AddItemEntity>(&undone).is_empty(), "the lead dropped");

    click_on(&mut h, cow);
    h.remote.run(|ctx| ctx.teleport(ALEX, [-7.5, 65.0, -7.5]));
    let snapped = watch(&mut h, 20);
    assert_eq!(has_flag(&snapped, cow, Flags::LEASHED), Some(false), "past 10 blocks the lead snaps");
}

#[test]
fn raising_kept_in_a_world_folder_is_shown_grows_on_and_is_kept() {
    let wolf = actor_with(-2, WOLF, [3.5, 64.0, 3.5], vec![("IsTamed", Value::Byte(1)), ("Sitting", Value::Byte(1)), ("OwnerNew", Value::Long(77))]);
    let calf = actor_with(-3, COW, [5.5, 64.0, 5.5], vec![("IsBaby", Value::Byte(1)), ("Age", Value::Int(-30)), ("CustomName", Value::String("Daisy".into()))]);
    let storage = floor_holding([wolf, calf].concat(), &[]);
    let mut h = Harness::with(common::config(), World::with_storage(storage.clone()));
    let sent = enter_stored(&mut h, 1);
    let shown = all::<AddEntity>(&sent);
    let [wolf, calf] = [WOLF, COW].map(|kind| shown.iter().find(|added| added.entity_type == kind).map(|added| added.runtime_id).expect("both stand again"));
    assert_eq!((has_flag(&sent, wolf, Flags::TAMED), has_flag(&sent, wolf, Flags::SITTING)), (Some(true), Some(true)));
    assert_eq!(has_flag(&sent, calf, Flags::BABY), Some(true));
    let grown: Vec<RawPacket> = (0..40).flat_map(|_| h.tick(1)).collect();
    assert_eq!(has_flag(&grown, calf, Flags::BABY), Some(false), "30 ticks were left to it");

    h.shard.save_worlds();
    let stored = storage.load_column(ChunkPos::new(0, 0), 0).unwrap().expect("the column");
    let kept: Vec<Actor> = sapling_world::actors(&stored.actors).unwrap();
    let wolf = kept.iter().find(|actor| actor.identifier == WOLF).unwrap();
    assert!(wolf.states.contains(&ActorState::Tamed) && wolf.states.contains(&ActorState::Sitting), "{:?}", wolf.states);
    let cow = kept.iter().find(|actor| actor.identifier == COW).unwrap();
    assert_eq!((cow.baby, cow.name.as_str()), (false, "Daisy"));
}
