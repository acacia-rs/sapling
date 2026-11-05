//! An enderman takes no starer who wears a carved pumpkin (its pack's `looked_at` filter).

use crate::common;

use acacia_proto::packets::{AddEntity, LevelSoundEvent, PlayerAuthInput};
use common::{Harness, all};
use sapling_api::{Difficulty, EntityId, Item, PlayerId, Slot};

/// Whether an enderman 6 off, stared at straight for 12 ticks, says `stare`; `worn` on the head.
fn stared(worn: Option<&'static str>) -> bool {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(move |ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        if let Some(worn) = worn {
            assert!(ctx.set_item(PlayerId(1), Slot::Armor(0), Some(&Item::new(worn, 1))));
        }
        assert!(ctx.spawn_entity("minecraft:enderman", [0.5, 65.0, 6.5]).is_some());
    });
    let added = h.tick_all().to(1).to_vec();
    let enderman = all::<AddEntity>(&added).into_iter().find(|added| added.entity_type == "minecraft:enderman").expect("the enderman").runtime_id;
    let pitch = -(2.465_f32 - 1.62).atan2(6.0).to_degrees();
    h.receive(1, &PlayerAuthInput { pitch, ..common::input_at([0.5, 65.0, 0.5]) });
    let sent: Vec<acacia_proto::RawPacket> = (0..12)
        .flat_map(|_| {
            h.remote.run(move |ctx| assert!(ctx.move_entity(EntityId(enderman), [0.5, 65.0, 6.5])));
            h.tick_all().to(1).to_vec()
        })
        .collect();
    all::<LevelSoundEvent>(&sent).iter().any(|sound| sound.sound_id == "stare")
}

#[test]
fn an_enderman_takes_no_starer_in_a_carved_pumpkin() {
    assert_eq!([stared(None), stared(Some("minecraft:carved_pumpkin")), stared(Some("minecraft:iron_helmet"))], [true, false, true]);
}
