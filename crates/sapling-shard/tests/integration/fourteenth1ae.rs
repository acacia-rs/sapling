//! Group AE of the fourteenth wave: dolphins flocking (bds-ab1flock.txt; creatures/dolphins/flock.rs)
//! and the wither's middle head looking back to its body after a volley (bds-ab1wither2.txt;
//! wither/heads.rs).

use crate::common;
use crate::fourteenth1ab::pool;

use acacia_proto::packets::{AddEntity, MoveEntityDelta};
use common::{Harness, all, asking::ask};
use sapling_api::PlayerId;

#[test]
fn dolphins_in_a_wide_pool_keep_together_as_a_flock() {
    // bds-ab1flock.txt: six in a pool 13 across kept 2.4 to 3.8 from their middle as they wandered.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        pool(ctx, [-12, -12], [24, 24], [58, 64]);
        for at in [[-1.5, 61.0, -1.5], [1.5, 61.0, -1.5], [-1.5, 61.0, 1.5], [1.5, 61.0, 1.5], [0.5, 62.0, 0.5], [0.5, 60.0, 2.5]] {
            assert!(ctx.spawn_entity("minecraft:dolphin", at).is_some());
        }
    });
    let mut spreads = Vec::new();
    for tick in 0..1200 {
        let dolphins: Vec<[f32; 3]> = ask(&mut h, |ctx| ctx.entities().into_iter().filter(|entity| entity.kind == "minecraft:dolphin").map(|entity| entity.feet).collect());
        if tick % 20 != 0 || tick < 200 {
            continue;
        }
        let middle: [f32; 3] = std::array::from_fn(|axis| dolphins.iter().map(|at| at[axis]).sum::<f32>() / dolphins.len() as f32);
        spreads.push(dolphins.iter().map(|at| (0..3).map(|axis| (at[axis] - middle[axis]).powi(2)).sum::<f32>().sqrt()).sum::<f32>() / dolphins.len() as f32);
    }
    spreads.sort_by(f32::total_cmp);
    let median = spreads[spreads.len() / 2];
    assert!(median < 4.5, "kept together, {median} from their middle: {spreads:?}");
}

#[test]
fn the_withers_middle_head_looks_back_to_its_body_after_a_blue_skull() {
    // bds-ab1wither2.txt: after each volley's blue skull the head went to about the body's yaw for 1.4 s.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_difficulty(sapling_api::Difficulty::Normal);
        assert!(ctx.spawn_entity("minecraft:wither", [8.5, 66.0, 0.5]).is_some());
        assert!(ctx.give_effect(PlayerId(1), sapling_api::Effect::Resistance, 1000, 4, false));
    });
    let added: Vec<AddEntity> = (0..201).flat_map(|_| all::<AddEntity>(h.tick_all().to(1))).collect();
    let id = added.iter().find(|added| added.entity_type == "minecraft:wither").expect("the wither shown").runtime_id;
    let body = added.iter().find(|added| added.runtime_id == id).map(|added| added.body_yaw).unwrap_or(0.0);
    let alex = ask(&mut h, |ctx| ctx.position(PlayerId(1)).map(|at| at.feet)).expect("Alex");
    let mut at = ask(&mut h, |ctx| ctx.entities().into_iter().find(|entity| entity.kind == "minecraft:wither").map(|entity| entity.feet)).expect("the wither");
    let off = |a: f32, b: f32| ((a - b + 540.0).rem_euclid(360.0) - 180.0).abs();
    let (mut head, mut blue, mut back) = (None, None, Vec::new());
    for tick in 0..300 {
        let told = h.tick_all();
        if blue.is_none() && all::<AddEntity>(told.to(1)).iter().any(|added| added.entity_type == "minecraft:wither_skull_dangerous") {
            blue = Some(tick);
        }
        for moved in all::<MoveEntityDelta>(told.to(1)).into_iter().filter(|moved| moved.runtime_entity_id == id) {
            head = moved.rot_z.or(head);
            at = [moved.x.unwrap_or(at[0]), at[1], moved.z.unwrap_or(at[2])];
        }
        let wanted = (at[0] - alex[0]).atan2(alex[2] - at[2]).to_degrees();
        let degrees = head.map(|byte| f32::from(byte) * 360.0 / 256.0);
        if let (Some(from), Some(degrees)) = (blue, degrees) {
            back.push((tick - from, off(degrees, body), off(degrees, wanted), off(body, wanted)));
        }
    }
    let from = blue.expect("a blue skull in 15 s");
    let near = back.iter().filter(|(since, to_body, _, apart)| (19..=26).contains(since) && *to_body <= 6.0 && *apart >= 30.0).count();
    let after = back.iter().filter(|(since, _, to_prey, apart)| (50..=60).contains(since) && *to_prey <= 6.0 && *apart >= 30.0).count();
    let shown: Vec<_> = back.iter().step_by(4).take(20).collect();
    assert!(near >= 6 && after >= 6, "back to the body ({near}) and round to Alex again ({after}) after the blue skull at {from}: {shown:?}");
}
