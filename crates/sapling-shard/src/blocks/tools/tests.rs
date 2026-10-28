//! What tools and filled things made of blocks on BDS (research/captures/bds-clicks.txt, bds-clicks2.txt).

use super::*;

fn made(held: &str, block: &str, properties: &str) -> Option<(String, String)> {
    let registry = BlockRegistry::vanilla();
    let named = format!("minecraft:{block}");
    let id = registry.find(&named, properties).or_else(|| registry.states_of(&named).next().map(|(id, _)| id)).expect("a block");
    let change = change(&registry, &format!("minecraft:{held}"), id, Face::Up)?;
    registry.get(change.id).map(|state| (state.name.trim_start_matches("minecraft:").to_owned(), state.properties.to_owned()))
}

fn name(held: &str, block: &str, properties: &str) -> Option<String> {
    made(held, block, properties).map(|(name, _)| name)
}

// research/captures/bds-clicks.txt
#[test]
fn tools_make_of_blocks_what_they_made_on_bds() {
    assert_eq!([name("iron_hoe", "dirt", ""), name("iron_hoe", "coarse_dirt", ""), name("iron_hoe", "stone", "")], [Some("farmland".into()), Some("dirt".into()), None]);
    assert_eq!([name("iron_shovel", "grass_block", ""), name("iron_shovel", "stone", "")], [Some("grass_path".into()), None]);
    assert_eq!(made("iron_axe", "oak_log", "pillar_axis=x"), Some(("stripped_oak_log".into(), "pillar_axis=x".into())));
    assert_eq!([name("iron_axe", "stripped_oak_log", "pillar_axis=y"), name("iron_axe", "copper_block", "")], [None, None]);
    assert_eq!([name("iron_axe", "waxed_copper", ""), name("iron_axe", "exposed_copper", ""), name("iron_axe", "oxidized_cut_copper", "")], [Some("copper_block".into()), Some("copper_block".into()), Some("weathered_cut_copper".into())]);
    assert_eq!([name("honeycomb", "copper_block", ""), name("honeycomb", "waxed_copper", ""), name("honeycomb", "exposed_cut_copper", "")], [Some("waxed_copper".into()), None, Some("waxed_exposed_cut_copper".into())]);
    assert_eq!(made("shears", "pumpkin", ""), Some(("carved_pumpkin".into(), "minecraft:cardinal_direction=south".into())));
}

#[test]
fn what_holds_something_gives_and_takes_it() {
    assert_eq!(made("glass_bottle", "cauldron", "cauldron_liquid=water,fill_level=6"), Some(("cauldron".into(), "cauldron_liquid=water,fill_level=4".into())));
    assert_eq!([name("glass_bottle", "cauldron", "cauldron_liquid=water,fill_level=0"), name("water_bucket", "cauldron", "cauldron_liquid=water,fill_level=6")], [None, None]);
    assert_eq!(made("water_bucket", "cauldron", "cauldron_liquid=water,fill_level=0"), Some(("cauldron".into(), "cauldron_liquid=water,fill_level=6".into())));
    assert_eq!(made("shears", "beehive", "direction=0,honey_level=5"), Some(("beehive".into(), "direction=0,honey_level=0".into())));
    assert_eq!([name("shears", "beehive", "direction=0,honey_level=4"), name("glowstone", "respawn_anchor", "respawn_anchor_charge=4")], [None, None]);
    assert_eq!(made("glowstone", "respawn_anchor", "respawn_anchor_charge=0"), Some(("respawn_anchor".into(), "respawn_anchor_charge=1".into())));
    assert_eq!(made("flint_and_steel", "candle", "candles=0,lit=0"), Some(("candle".into(), "candles=0,lit=1".into())));
}

// research/captures/bds-clicks2.txt
#[test]
fn buckets_bottles_and_what_else_bds_took_change_their_block() {
    let empty = Some(("cauldron".to_owned(), "cauldron_liquid=water,fill_level=0".to_owned()));
    assert_eq!([made("bucket", "cauldron", "cauldron_liquid=water,fill_level=6"), made("bucket", "cauldron", "cauldron_liquid=lava,fill_level=6")], [empty.clone(), empty]);
    assert_eq!(made("lava_bucket", "cauldron", "cauldron_liquid=water,fill_level=0"), Some(("cauldron".into(), "cauldron_liquid=lava,fill_level=6".into())));
    assert_eq!(made("potion", "cauldron", "cauldron_liquid=water,fill_level=2"), Some(("cauldron".into(), "cauldron_liquid=water,fill_level=4".into())));
    assert_eq!([name("potion", "dirt", ""), name("candle", "cake", "bite_counter=0"), name("candle", "cake", "bite_counter=1"), name("bucket", "cauldron", "cauldron_liquid=water,fill_level=4")], [Some("mud".into()), Some("candle_cake".into()), None, None]);
    assert_eq!(made("iron_shovel", "campfire", "extinguished=0,minecraft:cardinal_direction=south"), Some(("campfire".into(), "extinguished=1,minecraft:cardinal_direction=south".into())));
    assert_eq!(made("flint_and_steel", "campfire", "extinguished=1,minecraft:cardinal_direction=south"), Some(("campfire".into(), "extinguished=0,minecraft:cardinal_direction=south".into())));
    assert_eq!(made("ender_eye", "end_portal_frame", "end_portal_eye_bit=0,minecraft:cardinal_direction=south"), Some(("end_portal_frame".into(), "end_portal_eye_bit=1,minecraft:cardinal_direction=south".into())));
}
