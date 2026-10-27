use sapling_api::Sender;

use super::*;

fn parsed(rest: &str) -> Result<Loot, ParseError> {
    let sender = Sender::Console;
    let line = if rest.is_empty() { "loot".to_owned() } else { format!("loot {rest}") };
    let mut args = Args::new(&line, "loot".len(), &sender, &[]);
    Loot::parse(&mut args)
}

fn syntax(before: &str, token: &str, after: &str) -> Option<ParseError> {
    Some(ParseError::Syntax { before: before.into(), token: token.into(), after: after.into() })
}

/// research/captures/bds-lootcmd.txt and bds-lootcmd2.txt.
#[test]
fn errors_are_placed_as_bds_places_them() {
    let error = |line: &str| parsed(line).err();
    assert_eq!(error(""), syntax("/loot", "", ""));
    assert_eq!(error("spawn ~2 ~ ~ loot entities/chicken"), syntax("t entities", "/", "chicken"));
    assert_eq!(error("spawn ~2 ~ ~ loot loot_tables/entities/chicken.json"), syntax("oot_tables", "/", "entities/c"));
    assert_eq!(error("spawn ~2 ~ ~ loot entities/zombie mainhand"), syntax("t entities", "/", "zombie mai"));
    assert_eq!(error("give @s banana entities/chicken"), syntax("t give @s ", "banana", " entities/"));
}

#[test]
fn every_form_reads() {
    let reads = |line: &str| parsed(line).map(|loot| (loot.tool, matches!(loot.source, Source::Table(_)))).ok();
    assert_eq!(reads("spawn ~2 ~ ~ loot \"entities/chicken\""), Some((None, true)));
    assert_eq!(reads("give @a loot \"entities/zombie\" diamond_sword"), Some((Some("diamond_sword".into()), true)));
    assert_eq!(reads("insert ~3 ~ ~ mine ~2 ~ ~2 iron_pickaxe"), Some((Some("iron_pickaxe".into()), false)));
    assert_eq!(reads("give @s kill @e[type=cow] mainhand"), Some((Some("mainhand".into()), false)));
    let slots = |line: &str| match parsed(line).map(|loot| loot.target) {
        Ok(Target::Entity { slot, count, .. } | Target::Block { slot, count, .. }) => Some((slot, count)),
        _ => None,
    };
    assert_eq!(slots("replace entity @s slot.hotbar 1 2 loot \"entities/cow\""), Some((1, Some(2))));
    assert_eq!(slots("replace block ~3 ~ ~ slot.container 5 loot \"entities/chicken\""), Some((5, None)));
}
