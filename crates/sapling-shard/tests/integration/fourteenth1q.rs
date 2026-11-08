//! Group Q of the fourteenth wave: pigmen's warnings (bds-o1pig.txt) and the weapons and armour
//! piglins take up (bds-q1piglin.txt).

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, AddItemEntity, LevelSoundEvent, MobEquipment, TakeItemEntity};
use acacia_proto::types::MetadataFlags1;
use common::raising::has_flag;
use common::shooting::{FEET, ready, spawn};
use common::{Harness, all};
use sapling_api::{DamageCause, Difficulty, EntityId, GameMode, Item, PlayerId, Target};

const ALEX: PlayerId = PlayerId(1);

fn warnings(sent: &[RawPacket]) -> usize {
    all::<LevelSoundEvent>(sent).iter().filter(|sound| sound.sound_id == "mob.warning").count()
}

#[test]
fn pigmen_near_a_hurt_one_warn_and_show_angry_a_tick_before_it() {
    let mut h = ready(GameMode::Survival, &[], 0.0);
    h.remote.run(|ctx| ctx.set_difficulty(Difficulty::Easy));
    let bystander = spawn(&mut h, "minecraft:zombie_pigman", [FEET[0] - 5.0, FEET[1], FEET[2] + 5.0]);
    let pigman = spawn(&mut h, "minecraft:zombie_pigman", [FEET[0] + 3.0, FEET[1], FEET[2] + 3.0]);
    h.remote.run(move |ctx| {
        let alex = ctx.players().find(|player| player.id == ALEX).expect("Alex");
        assert!(ctx.hurt_entity_from(EntityId(pigman), 1.0, DamageCause::Attack, &Target::Player(alex)));
    });
    let ticks: Vec<Vec<RawPacket>> = (0..3).map(|_| h.tick_all().to(1).to_vec()).collect();
    let told = |tick: usize| (warnings(&ticks[tick]), has_flag(&ticks[tick], bystander, MetadataFlags1::ANGRY), has_flag(&ticks[tick], pigman, MetadataFlags1::ANGRY));
    assert_eq!([told(0), told(1), told(2)], [(0, None, None), (1, Some(true), None), (1, None, Some(true))], "bds-o1pig.txt");
}

/// A piglin holding nothing by Alex in creative, and its runtime id.
pub(crate) fn piglin(h: &mut Harness) -> u64 {
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_game_mode(ALEX, GameMode::Creative);
        assert!(ctx.summon_as(None, "minecraft:piglin", [3.5, 65.0, 0.5], 0.0, 0.0, Some("minecraft:spawn_adult")).is_some());
    });
    let shown: Vec<RawPacket> = (0..2).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    all::<AddEntity>(&shown).into_iter().find(|added| added.entity_type == "minecraft:piglin").expect("shown").runtime_id
}

/// Drops `item` by the piglin and ticks 40 times: the tick it was taken, if it was, and all told.
pub(crate) fn offer(h: &mut Harness, piglin: u64, item: &'static str) -> (Option<usize>, Vec<Vec<RawPacket>>) {
    h.remote.run(move |ctx| assert!(ctx.drop_item(&Item::new(item, 1), [5.5, 65.5, 0.5])));
    let ticks: Vec<Vec<RawPacket>> = (0..40).map(|_| h.tick_all().to(1).to_vec()).collect();
    let took = ticks.iter().position(|sent| all::<TakeItemEntity>(sent).iter().any(|taken| taken.target as u64 == piglin));
    (took, ticks)
}

fn shown(sent: &[RawPacket], piglin: u64) -> Vec<(u8, u8, Option<&'static str>)> {
    all::<MobEquipment>(sent).iter().filter(|held| held.runtime_entity_id == piglin).map(|held| (held.slot, held.selected_slot, sapling_data::item_name(held.item.network_id.into()))).collect()
}

fn dropped(sent: &[RawPacket]) -> Vec<Option<&'static str>> {
    all::<AddItemEntity>(sent).iter().map(|item| sapling_data::item_name(item.item.network_id.into())).collect()
}

#[test]
fn a_piglin_takes_a_better_weapon_or_armour_piece_and_drops_the_one_it_had_two_ticks_on() {
    let mut h = Harness::new();
    let piglin = piglin(&mut h);
    let (took, ticks) = offer(&mut h, piglin, "minecraft:iron_sword");
    let took = took.expect("an iron sword into its empty hand");
    assert_eq!(shown(&ticks[took + 1], piglin), [(0, 0, Some("minecraft:iron_sword"))]);

    let (took, ticks) = offer(&mut h, piglin, "minecraft:crossbow");
    let took = took.expect("a crossbow over the iron sword");
    assert_eq!(shown(&ticks[took + 1], piglin), [(0, 0, Some("minecraft:crossbow"))]);
    assert!(dropped(&ticks[took + 1]).is_empty() && dropped(&ticks[took + 2]) == [Some("minecraft:iron_sword")], "the sword it had dropped a tick after");
    assert!(ticks[took + 3..].iter().all(|sent| all::<TakeItemEntity>(sent).is_empty()), "and left lying");

    let (took, ticks) = offer(&mut h, piglin, "minecraft:diamond_helmet");
    let took = took.expect("a diamond helmet onto its bare head");
    assert_eq!(shown(&ticks[took + 1], piglin), [(2, 120, Some("minecraft:diamond_helmet"))]);
    assert_eq!(offer(&mut h, piglin, "minecraft:iron_helmet").0, None, "not an iron helmet over a diamond one");
}
