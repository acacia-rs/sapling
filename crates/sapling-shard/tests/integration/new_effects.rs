//! Fatal poison, darkness and conduit power, against research/captures/bds-fx2.txt and bds-dig.txt;
//! the conduit's power and hunt by the Java game's rule (docs/research/effects-raids.md).

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::nbt::Value;
use acacia_proto::packets::{DeathInfo, EntityEvent, EntityEventEventId, LevelEvent, LevelEventEvent, MobEffect, MobEffectEventId, SetEntityData, UpdateAttributes};
use acacia_proto::types::{Action, MetadataDictionaryItemKey as Key};
use common::{Harness, all, block_actions};
use sapling_api::{Block, BlockEntityData, DamageCause, Difficulty, Effect, PlayerId};

const ALEX: PlayerId = PlayerId(1);

fn ready() -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    h
}

fn health(sent: &[RawPacket]) -> Vec<f32> {
    all::<UpdateAttributes>(sent).iter().flat_map(|update| &update.attributes).filter(|attribute| attribute.name == "minecraft:health").map(|attribute| attribute.current).collect()
}

fn effects(sent: &[RawPacket]) -> Vec<(MobEffectEventId, i32, bool, i32, bool)> {
    all::<MobEffect>(sent).into_iter().filter(|effect| effect.runtime_entity_id == 1).map(|effect| (effect.event_id, effect.effect_id, effect.particles, effect.duration, effect.ambient)).collect()
}

#[test]
fn fatal_poison_goes_on_past_one_health_and_kills() {
    let mut h = ready();
    h.remote.run(|ctx| {
        assert!(ctx.hurt_player(ALEX, 18.0, DamageCause::Generic));
        assert!(ctx.give_effect(ALEX, Effect::FatalPoison, 120, 0, true));
    });
    let sent: Vec<RawPacket> = (0..50).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    assert_eq!(effects(&sent)[0], (MobEffectEventId::Add, 25, true, 120, false));
    assert_eq!(health(&sent)[..3], [2.0, 1.0, 0.0], "a point every 25 ticks from its first, as poison, but on to death");
    assert_eq!(all::<DeathInfo>(&sent).len(), 1);
}

#[test]
fn darkness_never_shows_particles() {
    let mut h = ready();
    h.remote.run(|ctx| assert!(ctx.give_effect(ALEX, Effect::Darkness, 80, 0, true)));
    let sent = h.tick_all().to(2).to_vec();
    assert_eq!(effects(&sent), [(MobEffectEventId::Add, 30, false, 80, false)], "bds-fx2.txt: `effect @s darkness 4`");
    let shown = all::<SetEntityData>(&sent).into_iter().filter(|data| data.runtime_entity_id == 1).flat_map(|data| data.metadata).filter(|item| item.key == Key::VisibleMobEffects).count();
    assert_eq!(shown, 0);
}

/// The crack speed Steve sees of Alex's first blow on stone by hand under `effect`.
fn crack_under(effect: Effect) -> i32 {
    let mut h = ready();
    h.remote.run(move |ctx| assert!(ctx.give_effect(ALEX, effect, 600, 0, true)));
    h.tick_all();
    h.receive(1, &block_actions(&[(Action::StartBreak, [1, 64, 0])]));
    all::<LevelEvent>(h.tick_all().to(2)).into_iter().find(|event| event.event == LevelEventEvent::BlockStartBreak).expect("a crack").data
}

#[test]
fn conduit_power_digs_as_haste_of_its_level() {
    // bds-dig.txt: 629 under either (sapling's whole ticks make haste's 624).
    assert_eq!(crack_under(Effect::ConduitPower), crack_under(Effect::Haste));
}

const CONDUIT: [i32; 3] = [4, 70, 4];

/// A conduit at [`CONDUIT`] in water, with `frame` of its 42 frame blocks.
fn conduit(h: &mut Harness, frame: usize) {
    h.remote.run(move |ctx| {
        let [x, y, z] = CONDUIT;
        for [dx, dz] in (-1..=1).flat_map(|dx| (-1..=1).map(move |dz| [dx, dz])) {
            assert!(ctx.set_block([x + dx, y - 2, z + dz], &Block::new("glass")));
        }
        for [dx, dy, dz] in (-1..=1).flat_map(|dx| (-1..=1).flat_map(move |dy| (-1..=1).map(move |dz| [dx, dy, dz]))) {
            assert!(ctx.set_block([x + dx, y + dy, z + dz], &Block::new("water")));
        }
        let rings = (-2..=2_i32).flat_map(|dx| (-2..=2_i32).flat_map(move |dy| (-2..=2_i32).map(move |dz| [dx, dy, dz]))).filter(|&[dx, dy, dz]| {
            let far = |c: i32| c.abs() == 2;
            (dx == 0 && (far(dy) || far(dz))) || (dy == 0 && (far(dx) || far(dz))) || (dz == 0 && (far(dx) || far(dy)))
        });
        for [dx, dy, dz] in rings.take(frame) {
            assert!(ctx.set_block([x + dx, y + dy, z + dz], &Block::new("prismarine")));
        }
        ctx.set_difficulty(Difficulty::Easy);
        ctx.teleport(ALEX, [4.5, 69.0, 3.5]);
    });
    h.tick_all();
    let [x, y, z] = CONDUIT;
    let fields = [("Active", Value::Byte(0)), ("Target", Value::Long(-1)), ("id", Value::String("Conduit".into())), ("x", Value::Int(x)), ("y", Value::Int(y)), ("z", Value::Int(z))];
    let entity = Value::Compound(fields.into_iter().map(|(key, value)| (key.into(), value)).collect());
    let done = common::asking::ask(h, move |ctx| (ctx.set_block(CONDUIT, &Block::new("conduit")), ctx.set_block_entity(CONDUIT, &BlockEntityData::new(entity))));
    assert_eq!(done, (true, true));
}

#[test]
fn an_active_conduit_gives_conduit_power_to_a_player_in_its_water_and_bites_a_monster_there_with_its_frame_whole() {
    let mut h = ready();
    conduit(&mut h, 42);
    let zombie = common::shooting::spawn(&mut h, "minecraft:zombie", [3.5, 69.0, 5.5]);
    let sent: Vec<RawPacket> = (0..81).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    assert!(effects(&sent).contains(&(MobEffectEventId::Add, 26, true, 260, true)), "{:?}", effects(&sent));
    let bitten = all::<EntityEvent>(&sent).into_iter().filter(|event| event.runtime_entity_id == zombie && event.event_id == EntityEventEventId::HurtAnimation).count();
    assert!(bitten >= 1, "4 damage every 40 ticks");
}

#[test]
fn a_conduit_short_of_its_whole_frame_gives_power_but_bites_nothing() {
    let mut h = ready();
    conduit(&mut h, 16);
    let zombie = common::shooting::spawn(&mut h, "minecraft:zombie", [3.5, 69.0, 5.5]);
    let sent: Vec<RawPacket> = (0..81).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    assert!(effects(&sent).iter().any(|effect| effect.1 == 26), "16 frame blocks reach 32");
    assert!(!all::<EntityEvent>(&sent).iter().any(|event| event.runtime_entity_id == zombie && event.event_id == EntityEventEventId::HurtAnimation));
}
