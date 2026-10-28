//! A frog on land leaps to a block near it, against research/captures/bds-amphibians.txt.

use crate::common;

use acacia_proto::packets::{AddEntity, LevelSoundEvent, SetEntityMotion};
use common::{Harness, all};

#[test]
fn a_frog_on_land_sits_then_leaps_at_seventy_degrees_with_no_drag_and_lands_slowed() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| assert!(ctx.spawn_entity("minecraft:frog", [4.5, 65.0, 4.5]).is_some()));
    let mut sent = Vec::new();
    for _ in 0..200 {
        sent.extend(h.tick(1));
    }
    let frog = all::<AddEntity>(&sent).into_iter().find(|mob| mob.entity_type == "minecraft:frog").expect("the frog").runtime_id;
    let told: Vec<[f32; 3]> = all::<SetEntityMotion>(&sent).into_iter().filter(|told| told.runtime_entity_id == frog).map(|told| [told.velocity.x, told.velocity.y, told.velocity.z]).collect();
    let leap = told.iter().position(|[_, up, _]| *up > 0.3).expect("it leapt");
    let [x, up, z] = told[leap];
    let across = x.hypot(z);
    assert!((up / across - 70f32.to_radians().tan()).abs() < 1e-3 && up.hypot(across) <= 1.0, "{:?}", told[leap]);
    let landed = told[leap..].iter().find(|[_, up, _]| *up == -0.08).expect("it landed");
    assert!((landed[0] - x * 0.1).abs() < 1e-5 && (landed[2] - z * 0.1).abs() < 1e-5, "{landed:?}");
    assert!(all::<LevelSoundEvent>(&sent).iter().any(|sound| sound.sound_id == "jump_to_block"));
}
