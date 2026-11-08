//! Group T of the fourteenth wave: a ghast fireball's stand on a player (bds-tt1ghast.txt), a
//! shulker bullet turned with its motion, and the armour piglins take and wear (bds-tt1piglin.txt,
//! bds-tt1worn.txt).

use crate::common;
use crate::fourteenth1q::{offer, piglin};
use crate::fourteenth3d::{moved, with};

use acacia_proto::RawPacket;
use acacia_proto::nbt::Value;
use acacia_proto::packets::{AddEntity, AddItemEntity, EntityEvent, EntityEventEventId, MoveEntityDelta, RemoveEntity};
use acacia_proto::types::ItemV4Extra;
use common::{Harness, all, asking::ask};
use sapling_api::{DamageCause, EntityId};

fn added(sent: &[RawPacket], kind: &str) -> Option<(u64, [f32; 3])> {
    all::<AddEntity>(sent).into_iter().find(|added| added.entity_type == kind).map(|added| (added.runtime_id, [added.position.x, added.position.y, added.position.z]))
}

#[test]
fn a_ghast_fireball_on_a_player_stands_six_ticks_then_hurts_steps_once_and_bursts_the_tick_after() {
    let (mut h, _) = with(&[("minecraft:ghast", [3.0, 65.0, 9.5])]);
    let (mut fireball, mut at, mut steps, mut hurt, mut burst) = (None, [0.0; 3], Vec::new(), None, None);
    for tick in 0..200 {
        let sent = h.tick_all().to(1).to_vec();
        fireball = fireball.or(added(&sent, "minecraft:fireball").map(|(id, from)| {
            at = from;
            id
        }));
        let Some(id) = fireball else { continue };
        let was = at;
        moved(&sent, id, &mut at);
        if at != was {
            steps.push((tick, std::array::from_fn::<f32, 3, _>(|axis| at[axis] - was[axis])));
        }
        if hurt.is_none() && all::<EntityEvent>(&sent).iter().any(|event| event.runtime_entity_id == 1 && event.event_id == EntityEventEventId::HurtAnimation) {
            hurt = Some(tick);
        }
        if all::<RemoveEntity>(&sent).iter().any(|gone| gone.entity_id_self as u64 == id) {
            burst = Some(tick);
            break;
        }
    }
    let hurt = hurt.expect("Alex hurt");
    assert_eq!(burst, Some(hurt + 1), "{steps:?}");
    let [.., (met, flight), (last, after)] = steps[..] else { panic!("{steps:?}") };
    assert_eq!((met, last), (hurt - 6, hurt), "five ticks still, then a step as it hurts: {steps:?}");
    let length = |step: [f32; 3]| step.iter().map(|c| c * c).sum::<f32>().sqrt();
    // The players it overlaps push it back the way it came, once a tick (crowding.rs).
    assert!(length(after) < length(flight), "{flight:?} then {after:?}");
}

#[test]
fn a_shulker_bullets_pitch_follows_its_motion() {
    // On a pillar, so that it does not hop down for want of a floor.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    ask(&mut h, |ctx| {
        ctx.set_difficulty(sapling_api::Difficulty::Easy);
        (65..69).for_each(|y| assert!(ctx.set_block([0, y, 6], &sapling_api::Block::new("minecraft:stone"))));
        ctx.spawn_entity("minecraft:shulker", [0.5, 69.0, 6.5]).expect("spawned")
    });
    let (mut bullet, mut pitches) = (None, Vec::new());
    for _ in 0..120 {
        let sent = h.tick_all().to(1).to_vec();
        bullet = bullet.or(added(&sent, "minecraft:shulker_bullet").map(|(id, _)| id));
        let Some(id) = bullet else { continue };
        pitches.extend(all::<MoveEntityDelta>(&sent).iter().filter(|delta| delta.runtime_entity_id == id).filter_map(|delta| delta.rot_x));
    }
    assert!(bullet.is_some(), "a bullet");
    // Shot from above Alex, it heads down: a pitch below level (bds-tt1shulker.txt fits atan2 of its step).
    assert!(pitches.iter().any(|pitch| *pitch > 128), "{pitches:?}");
}

#[test]
fn a_bare_piglin_takes_a_turtle_helmet() {
    let mut h = Harness::new();
    let piglin = piglin(&mut h);
    assert!(offer(&mut h, piglin, "minecraft:turtle_helmet").0.is_some(), "bds-tt1piglin.txt: taken");
}

fn dropped_wear(sent: &[RawPacket], name: &str) -> Option<i32> {
    let item = all::<AddItemEntity>(sent).into_iter().find(|item| sapling_data::item_name(item.item.network_id.into()) == Some(name))?;
    let tag = match item.item.extra {
        ItemV4Extra::Default(Some(extra)) => extra.nbt.map(|nbt| nbt.nbt.value),
        _ => None,
    };
    Some(match tag.as_ref().and_then(|tag| tag.get("Damage")) {
        Some(Value::Int(damage)) => *damage,
        _ => 0,
    })
}

#[test]
fn a_piglins_picked_up_helmet_wears_one_a_hit_of_four_and_drops_so_worn() {
    let mut h = Harness::new();
    let id = piglin(&mut h);
    assert!(offer(&mut h, id, "minecraft:diamond_helmet").0.is_some());
    let piglin = EntityId(id);
    // Four, as its 16 health takes no fifth unhealed; then killed, as the drill's `/kill`.
    for _ in 0..4 {
        assert!(ask(&mut h, move |ctx| ctx.hurt_entity(piglin, 4.0, DamageCause::Attack)));
        for _ in 0..11 {
            h.tick_all();
        }
    }
    // Not `ask`: the gear goes in the death's own tick.
    h.remote.run(move |ctx| assert!(ctx.kill_entity(piglin)));
    let sent: Vec<RawPacket> = (0..3).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    assert_eq!(dropped_wear(&sent, "minecraft:diamond_helmet"), Some(4), "bds-tt1worn.txt: five hits of 4, Damage 5");
}
