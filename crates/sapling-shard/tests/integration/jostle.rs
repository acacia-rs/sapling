//! Players and mobs pushing each other, against research/captures/bds-jostle.txt.

use crate::common;

use acacia_proto::packets::{AddEntity, MoveEntityDelta};
use common::{Harness, all, input_at};
use sapling_api::RuleValue;

/// Where a `kind` summoned 0.3 east of Alex beside an armour stand or not is told it went, tick by tick.
fn pushed(kind: &str, beside_a_stand: bool) -> Vec<f32> {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    h.receive(1, &input_at([0.5, 65.0, -1.5]));
    h.receive(2, &input_at([0.5, 65.0, 6.5]));
    let kind = kind.to_owned();
    h.remote.run(move |ctx| {
        assert!(ctx.set_game_rule("sapling:mobAi", RuleValue::Bool(false)));
        if beside_a_stand {
            assert!(ctx.spawn_entity("minecraft:armor_stand", [10.5, 65.0, 0.5]).is_some());
        }
        assert!(ctx.spawn_entity(&kind, if beside_a_stand { [10.8, 65.0, 0.5] } else { [0.8, 65.0, 0.5] }).is_some());
    });
    // Settled on the ground before Alex steps up to it, as the capture's was when summoned.
    let shown: Vec<_> = (0..5).flat_map(|_| h.tick_all().to(2).to_vec()).collect();
    let mob = all::<AddEntity>(&shown).into_iter().find(|added| added.entity_type != "minecraft:armor_stand").expect("shown").runtime_id;
    h.receive(1, &input_at([0.5, 65.0, -0.5]));
    h.tick_all();
    h.receive(1, &input_at([0.5, 65.0, 0.5]));
    let sent: Vec<_> = (0..6).flat_map(|_| h.tick_all().to(2).to_vec()).collect();
    all::<MoveEntityDelta>(&sent).into_iter().filter(|moved| moved.runtime_entity_id == mob).filter_map(|moved| moved.x).collect()
}

#[test]
fn a_player_pushes_a_cow_off_it_and_an_armour_stand_pushes_nothing() {
    // The capture's cow 0.3 off a player went 0.8548, 0.9442, 1.0597 and on.
    let moved = pushed("minecraft:cow", false);
    let wanted = [0.85477, 0.94424, 1.05974];
    assert!(moved.len() >= 3 && moved.iter().zip(wanted).all(|(x, wanted)| (x - wanted).abs() < 1e-3), "{moved:?}");
    assert_eq!(pushed("minecraft:cow", true), Vec::<f32>::new(), "beside the stand it stood still");
}
