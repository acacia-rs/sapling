use super::*;

#[test]
fn armour_takes_off_a_hit_what_it_took_off_on_bds() {
    // research/captures/bds-armour.txt: a leather and an iron chestplate, all of iron, of
    // diamond and of netherite against 5, and the netherite against 2.
    let behind = |armour: [f32; 2], amount: f32| {
        let mut health = Health::new(20.0, None, 0);
        health.wear(armour, Warding::default());
        (health.shielded(amount) * 1000.0).round() / 1000.0
    };
    let taken = [behind([0.0, 0.0], 5.0), behind([3.0, 0.0], 5.0), behind([6.0, 0.0], 5.0), behind([15.0, 0.0], 5.0), behind([20.0, 8.0], 5.0), behind([20.0, 12.0], 5.0), behind([20.0, 12.0], 2.0)];
    assert_eq!(taken, [5.0, 4.88, 4.3, 2.5, 1.25, 1.2, 0.432]);
}

#[test]
fn breach_pierces_what_armour_stops_as_on_bds() {
    // research/captures/bds-mace-breach.txt, bds-mace-push.txt: a mace's 6 on a zombie in a
    // diamond chestplate (10 points, 2 toughness) with Breach 0 to 4.
    let mut health = Health::new(20.0, None, 0);
    health.wear([10.0, 2.0], Warding::default());
    let taken = [0, 1, 2, 3, 4].map(|level| (health.shielded_but(6.0, sapling_data::enchant::mace::breach(level)) * 1000.0).round() / 1000.0);
    assert_eq!(taken, [4.176, 5.076, 5.976, 6.0, 6.0]);
}

#[test]
fn a_pig_in_lava_is_hurt_as_bds_hurt_one() {
    // research/captures/bds-burn.txt: touching from its 1st tick, in the lava from its 3rd.
    let mut pig = Health::new(10.0, None, 0);
    let mut seen = Vec::new();
    for tick in 1..=23 {
        pig.begin();
        let fire = pig.burn(Some((1.0, true)), false);
        let after_fire = pig.left;
        let lava = tick >= 3 && pig.scald();
        if fire.is_some() || lava {
            seen.push((tick, after_fire, pig.left));
        }
    }
    assert_eq!(seen, [(1, 9.0, 9.0), (3, 9.0, 6.0), (13, 5.0, 2.0), (23, 1.0, 0.0)]);
}

#[test]
fn out_of_lava_a_mob_burns_on_for_eight_seconds_hurt_each_second() {
    let mut golem = Health::new(100.0, None, 0);
    assert_eq!(golem.burn(Some((1.0, true)), false), Some(Cause::Fire));
    let hits: Vec<u16> = (1..=200)
        .filter(|_| {
            golem.begin();
            golem.burn(None, false) == Some(Cause::FireTick)
        })
        .collect();
    assert_eq!(hits, [20, 40, 60, 80, 100, 120, 140]);
    assert!(!golem.is_burning() && golem.left() == 92.0);
    golem.burn(Some((1.0, true)), false);
    assert!(golem.is_burning() && golem.burn(None, true).is_none() && !golem.is_burning(), "water puts it out");
}

#[test]
fn a_fall_costs_each_block_begun_past_three_as_on_bds() {
    // research/captures/bds-harm.txt: cows dropped 3 to 8 blocks.
    let at = |y: f32| [0.5, y, 0.5];
    let cost = |height: f32| {
        let mut cow = Health::new(10.0, None, 0);
        let middle = 60.0 + height / 2.0;
        assert_eq!((cow.fall(at(60.0 + height), middle, false, false), cow.fall(at(middle), 60.0, true, false)), (None, None), "not in the tick it lands");
        cow.fall(at(60.0), 60.0, true, false)
    };
    let from = |height: f32| at(60.0 + height / 2.0);
    assert_eq!([3.0, 3.5, 4.0, 8.0].map(cost), [None, Some((1.0, from(3.5))), Some((1.0, from(4.0))), Some((5.0, from(8.0)))]);
    assert_eq!([4.5, 5.0].map(|height| cost(height).map(|(harm, _)| harm)), [Some(2.0), Some(2.0)]);
    let mut cow = Health::new(10.0, None, 0);
    cow.fall(at(80.0), 70.0, false, false);
    cow.fall(at(70.0), 65.0, false, true);
    assert_eq!((cow.fall(at(65.0), 64.0, true, false), cow.fall(at(64.0), 64.0, true, false)), (None, None), "water breaks a fall");
}

#[test]
fn a_campfire_and_fire_resistance_light_no_fire_and_rain_puts_one_out_at_its_next_burn() {
    // research/captures/bds-burn2.txt.
    let mut cow = Health::new(10.0, None, 0);
    assert!(cow.burn(Some((1.0, false)), false).is_some() && !cow.is_burning(), "a campfire hurts and lights nothing");
    let mut resisting = Health::new(10.0, None, 0);
    resisting.ward(1.0, true);
    assert!(resisting.burn(Some((1.0, true)), false).is_none() && !resisting.is_burning());
    cow.burn(Some((1.0, true)), false);
    let out_at = (1..=40u32).find(|_| {
        cow.begin();
        cow.rained_on();
        cow.burn(None, false);
        !cow.is_burning()
    });
    assert_eq!(out_at, Some(20), "where the first burn on would have come");
}

#[test]
fn a_fish_on_dry_land_holds_its_breath_15_s_then_loses_2_a_hit() {
    // research/captures/bds-harm2.txt: a cod's first hurt came 299 ticks after it was summoned, its second 10 after.
    let mut cod = Health::new(3.0, None, 0);
    let hurt_at: Vec<u32> = (1..=320u32).filter(|_| {
        cod.begin();
        cod.breathe(true);
        cod.gasp(true)
    }).collect();
    assert_eq!((hurt_at, cod.left()), (vec![300, 310], 0.0));
}

#[test]
fn hay_slime_and_a_bed_soften_a_fall_as_on_bds() {
    // research/captures/bds-harm2.txt: 8 blocks cost 5 on stone, none on hay or slime; 8.44 onto a bed cost 1.
    let cost = |height: f32, block: &str| {
        let mut cow = Health::new(10.0, None, 0);
        let soften = crate::creatures::heat::softens(block);
        cow.fall_onto([0.5, 60.0 + height, 0.5], 60.0, false, false, soften);
        cow.fall_onto([0.5, 60.0, 0.5], 60.0, true, false, soften);
        cow.fall_onto([0.5, 60.0, 0.5], 60.0, true, false, soften).map(|(harm, _)| harm)
    };
    assert_eq!([cost(8.0, "minecraft:stone"), cost(8.0, "minecraft:hay_block"), cost(8.0, "minecraft:slime"), cost(8.4375, "minecraft:bed")], [Some(5.0), None, None, Some(1.0)]);
}

#[test]
fn a_block_smothers_from_the_seventh_tick_and_water_once_breath_is_out() {
    let mut cow = Health::new(10.0, None, 0);
    let hits: Vec<i32> = (1..=27)
        .filter(|_| {
            cow.begin();
            cow.smother(true, WALLED)
        })
        .collect();
    assert_eq!((hits, cow.left()), (vec![7, 17, 27], 4.0));
    assert!(!cow.smother(false, WALLED) && !cow.smother(true, WALLED), "counted anew");

    let mut cow = Health::new(10.0, None, 0);
    let breaths: Vec<i16> = (0..AIR).filter_map(|_| cow.breathe(true)).collect();
    assert_eq!((breaths.len(), breaths.last()), (AIR as usize, Some(&0)));
    assert!(cow.breathe(true).is_none() && cow.drown(true) && !cow.drown(true), "2 a hit, guarded");
    assert_eq!((cow.left(), cow.breathe(false)), (8.0, Some(AIR)));
}

#[test]
fn a_spared_hit_gives_back_its_cost_and_guard_and_keeps_its_timers() {
    let at = |y: f32| [0.5, y, 0.5];
    let mut cow = Health::new(10.0, None, 0);
    cow.fall(at(70.0), 60.0, true, false);
    let before = cow;
    assert!(cow.burn(Some((1.0, true)), false).is_some() && cow.fall(at(60.0), 60.0, true, false).is_some());
    cow.reweigh(&before, None);
    assert_eq!((cow.left(), cow.guarded, cow.burning, cow.landed), (10.0, 0, BURNS, None), "the landing is spent, the fire lit");

    let before = cow;
    assert!(cow.hurt(2.0));
    cow.reweigh(&before, Some(5.0));
    assert_eq!((cow.left(), cow.guarded), (5.0, GUARD));
}
