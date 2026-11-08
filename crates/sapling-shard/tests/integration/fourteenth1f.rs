//! Group F of the fourteenth brief (research/captures/bds-ffpush.txt, bds-ffcomp.txt, bds-ffburn.txt,
//! bds-fftnt.txt): a blocked piston trying again, what it breaks bursting as it starts, composters and
//! hoppers, and vehicles in fire and lava.

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddItemEntity, BlockEntityData, EntityEvent, EntityEventEventId, LevelEvent, LevelEventEvent, LevelSoundEvent, RemoveEntity, SetEntityData};
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Meta, MetadataDictionaryItemValueDefault as Plain};
use common::{Harness, all, block_changes, hash};
use sapling_api::{Block, Item};

fn ready() -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    h
}

/// Puts `blocks` in the next tick, the first that [`watch`] sees.
fn put(h: &mut Harness, blocks: &[([i32; 3], &str, &str)]) {
    let blocks: Vec<([i32; 3], Block)> = blocks.iter().map(|(pos, name, properties)| (*pos, Block::new(*name).with(*properties))).collect();
    h.remote.run(move |ctx| {
        for (pos, block) in &blocks {
            assert!(ctx.update_block(*pos, block), "{pos:?} {block:?}");
        }
    });
}

/// What Steve is shown in each of the next `ticks` ticks.
fn watch(h: &mut Harness, ticks: usize) -> Vec<Vec<RawPacket>> {
    (0..ticks).map(|_| h.tick_all().to(2).to_vec()).collect()
}

fn when(ticks: &[Vec<RawPacket>], pos: [i32; 3], block: u32) -> Option<usize> {
    ticks.iter().position(|sent| block_changes(sent).contains(&(pos, block)))
}

fn heard(ticks: &[Vec<RawPacket>], name: &str) -> Vec<usize> {
    ticks.iter().enumerate().filter(|(_, sent)| all::<LevelSoundEvent>(sent).iter().any(|sound| sound.sound_id == name)).map(|(tick, _)| tick).collect()
}

fn state(name: &str, properties: &str) -> u32 {
    let registry = sapling_world::BlockRegistry::vanilla();
    registry.network_hash(registry.find(name, properties).unwrap_or_else(|| panic!("{name} [{properties}]")))
}

const EAST: &str = "facing_direction=4";

#[test]
fn a_blocked_piston_pushes_a_tick_after_what_blocked_it_goes_and_bursts_what_it_breaks_as_it_starts() {
    let mut h = ready();
    put(&mut h, &[([0, 65, 2], "piston", EAST), ([1, 65, 2], "stone", ""), ([2, 65, 2], "obsidian", ""), ([0, 65, 4], "piston", EAST), ([1, 65, 4], "redstone_torch", "")]);
    watch(&mut h, 4);
    put(&mut h, &[([0, 66, 2], "redstone_block", "")]);
    assert_eq!(when(&watch(&mut h, 6), [2, 65, 2], hash("minecraft:moving_block")), None, "blocked");
    put(&mut h, &[([2, 65, 2], "air", "")]);
    let out = watch(&mut h, 6);
    let started = out.iter().position(|sent| all::<BlockEntityData>(sent).iter().any(|data| data.position.x == 0 && data.position.z == 2)).expect("it tries again");
    assert_eq!(started, 1, "the tick after the obsidian went (bds-ffpush.txt)");
    assert_eq!(when(&out, [2, 65, 2], hash("minecraft:moving_block")), Some(started + 1));

    put(&mut h, &[([0, 66, 4], "redstone_block", "")]);
    let out = watch(&mut h, 6);
    let burst = out.iter().position(|sent| all::<LevelEvent>(sent).iter().any(|event| event.event == LevelEventEvent::ParticleDestroy)).expect("the torch bursts");
    assert!(all::<BlockEntityData>(&out[burst]).iter().any(|data| data.position.z == 4), "in the tick the piston starts");
    assert_eq!(when(&out, [1, 65, 4], state("minecraft:piston_arm_collision", EAST)), Some(burst + 1), "gone with the arm a tick on");
}

fn level(value: u8) -> u32 {
    state("minecraft:composter", &format!("composter_fill_level={value}"))
}

#[test]
fn a_composter_ripens_a_second_after_7_and_hoppers_feed_and_empty_it() {
    let mut h = ready();
    let (composter, above, below) = ([0, 65, 2], [0, 66, 2], [0, 64, 2]);
    put(&mut h, &[(composter, "composter", "composter_fill_level=6"), (above, "hopper", "facing_direction=0"), (below, "hopper", "facing_direction=0")]);
    watch(&mut h, 2);
    h.remote.run(move |ctx| {
        assert!(ctx.set_container_item(above, 0, Some(&Item::new("minecraft:stone", 2))));
        assert!(ctx.set_container_item(above, 1, Some(&Item::new("minecraft:pumpkin_pie", 4))));
    });
    let out = watch(&mut h, 40);
    let seven = when(&out, composter, level(7)).expect("the pie goes in");
    assert_eq!(heard(&out, "block.composter.ready"), [seven + 20]);
    assert_eq!(heard(&out, "block.composter.empty"), [seven + 20], "the hopper under it takes the bone meal at once");
    let kept = common::asking::ask(&mut h, move |ctx| (ctx.container_item(below, 0).map(|item| item.name), ctx.container_item(above, 0).map(|item| item.count)));
    assert_eq!(kept, (Some("minecraft:bone_meal".to_owned()), Some(2)), "the stone is skipped");
}

fn health(sent: &[RawPacket], id: u64) -> Vec<i32> {
    let of = |data: SetEntityData| data.metadata.iter().find_map(|item| match (&item.key, &item.value) {
        (Key::Health, Meta::Default(Plain::Int(value))) => Some(*value),
        _ => None,
    });
    all::<SetEntityData>(sent).into_iter().filter(|data| data.runtime_entity_id == id).filter_map(of).collect()
}

/// Summons `kind` at `feet`; its runtime id and what Steve saw in each of the next `ticks` ticks.
fn summon(h: &mut Harness, kind: &'static str, feet: [f32; 3], ticks: usize) -> (u64, Vec<Vec<RawPacket>>) {
    h.remote.run(move |ctx| assert!(ctx.summon_as(None, kind, feet, 0.0, 0.0, None).is_some()));
    let out = watch(h, ticks);
    let added = out.iter().flat_map(|sent| all::<acacia_proto::packets::AddEntity>(sent)).find(|added| added.entity_type == kind).expect("summoned");
    (added.runtime_id, out)
}

#[test]
fn boats_and_minecarts_burn_through_in_fire_and_lava_and_a_tnt_minecart_is_primed() {
    let mut h = ready();
    put(&mut h, &[([0, 64, 2], "netherrack", ""), ([0, 65, 2], "fire", ""), ([4, 65, 2], "lava", "")]);
    watch(&mut h, 2);
    let (cart, out) = summon(&mut h, "minecraft:minecart", [0.5, 65.0, 2.5], 6);
    let all_sent = out.concat();
    let hurt: Vec<i32> = health(&all_sent, cart).into_iter().filter(|integrity| *integrity < 40).collect();
    assert_eq!(hurt, [24, 8, -7], "2 a tick, mending 1 under 20 (bds-ffburn.txt)");
    let hurt_last = out.iter().rposition(|sent| !health(sent, cart).is_empty()).expect("hurt");
    let gone = out.iter().position(|sent| !all::<RemoveEntity>(sent).is_empty()).expect("broken");
    assert_eq!(gone, hurt_last + 1, "a tick after its integrity is gone");
    assert!(all::<AddItemEntity>(&out[gone]).iter().any(|item| item.item.network_id != 0), "into its item");

    let (boat, out) = summon(&mut h, "minecraft:boat", [4.5, 65.0, 2.5], 3);
    assert_eq!(health(&out.concat(), boat).last(), Some(&0), "all 40 at once in lava");

    let (tnt, out) = summon(&mut h, "minecraft:tnt_minecart", [0.5, 65.0, 2.5], 3);
    let primed = out.iter().position(|sent| all::<EntityEvent>(sent).iter().any(|event| event.runtime_entity_id == tnt && event.event_id == EntityEventEventId::MinecartTntPrimeFuse)).expect("primed");
    let fuse = |sent: &[RawPacket]| all::<SetEntityData>(sent).into_iter().filter(|data| data.runtime_entity_id == tnt).flat_map(|data| data.metadata).find_map(|item| match (item.key, item.value) {
        (Key::FuseLength, Meta::Default(Plain::Int(value))) => Some(value),
        _ => None,
    });
    assert_eq!((fuse(&out[primed]), fuse(&out[primed + 1])), (Some(79), Some(78)), "its fuse shown each tick (bds-fftnt.txt)");
}
