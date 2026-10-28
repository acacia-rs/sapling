use acacia_proto::nbt::List;

use super::*;

fn held(level: u8, potion: Option<(i16, i16)>) -> Held {
    Held { level, potion, colour: None, snow: false }
}

fn snow(level: u8) -> Held {
    Held { snow: true, ..held(level, None) }
}

fn after(hand: &ItemStack, before: Held) -> (Held, Option<(LevelEventEvent, u8, i32)>) {
    let outcome = change(hand, before).expect("a change");
    (outcome.next, outcome.told)
}

/// bds-cauldrons.txt: swiftness twice, slowness on it, swiftness, a bottle out, splash on top.
#[test]
fn potions_fill_by_two_burst_when_unlike_and_take_the_last_kind() {
    let swift = stack("minecraft:potion", 14);
    assert_eq!(after(&swift, held(0, None)), (held(2, Some((14, 0))), Some((LevelEventEvent::CauldronFillPotion, 2, -13_374_465))));
    assert_eq!(after(&swift, held(2, Some((14, 0)))).0, held(4, Some((14, 0))));
    assert_eq!(after(&stack("minecraft:potion", 17), held(4, Some((14, 0)))), (held(0, None), Some((LevelEventEvent::CauldronExplode, 0, WATER))));
    assert_eq!(after(&stack("minecraft:glass_bottle", 0), held(4, Some((14, 0)))).0, held(2, Some((14, 0))));
    assert_eq!(after(&stack("minecraft:splash_potion", 14), held(2, Some((14, 0)))).0, held(4, Some((14, 1))));
    let arrows = ItemStack { count: 16, ..stack("minecraft:arrow", 0) };
    assert_eq!(after(&arrows, held(4, Some((14, 1)))), (held(3, Some((14, 1))), None));
    assert!(change(&stack("minecraft:potion", 0), held(3, None)).is_none(), "water by bottle is tools.rs's");
}

/// bds-cauldrons3.txt: swiftness into plain and red water and a water bottle onto swiftness all
/// burst, coloured as what was there; only the water bottle unheard.
#[test]
fn a_potion_poured_into_water_or_onto_another_bursts() {
    let swift = stack("minecraft:potion", 14);
    for level in [3, 6] {
        let outcome = change(&swift, held(level, None)).expect("a burst");
        assert_eq!((outcome.next, outcome.told, outcome.sound), (held(0, None), Some((LevelEventEvent::CauldronExplode, 0, WATER)), Some("bottle.empty")));
    }
    let red = Held { colour: Some(-5_231_066), ..held(3, None) };
    assert_eq!(after(&swift, red), (held(0, None), Some((LevelEventEvent::CauldronExplode, 0, -5_231_066))));
    let water = change(&stack("minecraft:potion", 0), held(2, Some((14, 0)))).expect("a burst");
    assert_eq!((water.next, water.sound), (held(0, None), None));
}

/// bds-cauldrons3.txt: powder snow in and out by bucket, a bottle and a snow bucket bursting.
#[test]
fn powder_snow_fills_by_bucket_and_bursts_under_anything_else() {
    let bucket = stack(snow::BUCKET, 0);
    assert_eq!(after(&bucket, held(0, None)), (snow(6), Some((LevelEventEvent::Unknown(3514), 6, WATER))));
    assert_eq!(after(&stack("minecraft:bucket", 0), snow(6)), (held(0, None), Some((LevelEventEvent::Unknown(3515), 6, WATER))));
    assert!(change(&stack("minecraft:glass_bottle", 0), snow(6)).is_none());
    let bottle = change(&stack("minecraft:potion", 0), snow(6)).expect("a burst");
    assert_eq!((bottle.next, bottle.told, bottle.sound), (held(0, None), Some((LevelEventEvent::CauldronExplode, 0, WATER)), None));
    assert_eq!(after(&bucket, held(4, None)), (held(0, None), Some((LevelEventEvent::CauldronExplode, 0, WATER))));
}

#[test]
fn dye_colours_water_which_dyes_leather_and_plain_water_washes_it() {
    let (dyed, told) = after(&stack("minecraft:red_dye", 0), held(6, None));
    assert_eq!((dyed.colour, told), (Some(-5_231_066), Some((LevelEventEvent::CauldronAddDye, 6, -5_231_066))));
    assert_eq!(after(&stack("minecraft:leather_helmet", 0), dyed).1, Some((LevelEventEvent::CauldronDyeArmor, 5, -5_231_066)));
    let Some(Outcome { cost: Cost::Changed(helmet), .. }) = change(&stack("minecraft:leather_helmet", 0), dyed) else { panic!("dyed") };
    assert_eq!(after(&helmet, held(6, None)).1, Some((LevelEventEvent::CauldronCleanArmor, 5, WATER)));
    assert!(change(&stack("minecraft:leather_helmet", 0), held(6, None)).is_none(), "an undyed cap");
    assert_eq!(after(&stack("minecraft:red_shulker_box", 0), held(5, None)).1, Some((LevelEventEvent::CauldronCleanArmor, 4, WATER)));
    assert!(change(&stack("minecraft:banner", 1), held(4, None)).is_none());
}

/// bds-cauldrons2.txt: blue on red mixes to 0xFF763968, which a bucket takes with the water.
#[test]
fn a_second_dye_mixes_and_a_bucket_takes_dyed_water_with_its_colour() {
    let (red, _) = after(&stack("minecraft:red_dye", 0), held(6, None));
    let (mixed, _) = after(&stack("minecraft:blue_dye", 0), red);
    assert_eq!(mixed.colour, Some(0xFF76_3968_u32 as i32));
    assert_eq!(after(&stack("minecraft:bucket", 0), mixed), (held(0, None), Some((LevelEventEvent::CauldronTakeWater, 6, 0xFF76_3968_u32 as i32))));
}

#[test]
fn water_washes_a_banners_last_pattern_off() {
    let pattern = |name: &str| Value::Compound(vec![("Color".into(), Value::Int(1)), ("Pattern".into(), Value::String(name.into()))]);
    let patterns = Value::List(List { tag: 10, items: vec![pattern("bo"), pattern("cr")] });
    let banner = ItemStack { tag: Some(Value::Compound(vec![("Patterns".into(), patterns), ("Type".into(), Value::Int(0))])), ..stack("minecraft:banner", 15) };
    assert_eq!(after(&banner, held(6, None)).1, Some((LevelEventEvent::CauldronCleanBanner, 5, WATER)));
    let Some(Outcome { cost: Cost::Changed(once), .. }) = change(&banner, held(6, None)) else { panic!("washed") };
    let Some(Value::List(left)) = once.tag.as_ref().and_then(|tag| tag.get("Patterns")) else { panic!("{once:?}") };
    assert_eq!(left.items, [pattern("bo")]);
    let Some(Outcome { cost: Cost::Changed(plain), .. }) = change(&once, held(5, None)) else { panic!("washed") };
    assert!(change(&plain, held(4, None)).is_none(), "nothing left to wash");
}
