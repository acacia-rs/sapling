use super::*;

fn parsed(line: &str) -> Result<Scoreboard, ParseError> {
    Scoreboard::parse(&mut Args::new(line, 11, &Sender::Console, &[]))
}

fn syntax(before: &str, token: &str, after: &str) -> Option<ParseError> {
    Some(ParseError::Syntax { before: before.into(), token: token.into(), after: after.into() })
}

#[test]
fn lines_read_as_bds_read_them() {
    assert!(matches!(parsed(r#"scoreboard objectives add "q r" dummy "Long name""#), Ok(Scoreboard::Add { name, display_name: Some(shown) }) if name == "q r" && shown == "Long name"));
    assert!(matches!(parsed("scoreboard objectives setdisplay sidebar a"), Ok(Scoreboard::SetDisplay { slot: DisplaySlot::Sidebar, shown: Some((_, SortOrder::Descending)) })));
    assert!(matches!(parsed("scoreboard objectives setdisplay list a ascending"), Ok(Scoreboard::SetDisplay { slot: DisplaySlot::List, shown: Some((_, SortOrder::Ascending)) })));
    assert!(matches!(parsed("scoreboard objectives setdisplay belowname"), Ok(Scoreboard::SetDisplay { slot: DisplaySlot::BelowName, shown: None })));
    assert!(matches!(parsed("scoreboard players test @s obj * 5"), Ok(Scoreboard::Test { min: i32::MIN, max: 5, .. })));
    assert!(matches!(parsed("scoreboard players test Fake obj 3"), Ok(Scoreboard::Test { min: 3, max: i32::MAX, .. })));
    assert!(matches!(parsed("scoreboard players operation @s a >< Fake b"), Ok(Scoreboard::Operation { operation: ScoreOperation::Swap, .. })));
    assert!(matches!(parsed("scoreboard players list"), Ok(Scoreboard::ListScores(None))));
    assert!(matches!(parsed("scoreboard players reset *"), Ok(Scoreboard::Reset { target: Selector::Named(star), objective: None }) if star == "*"));
    // research/captures/bds-scoreboard.txt, bds-scoreboard2.txt
    assert_eq!(parsed("scoreboard objectives add bad stat").err(), syntax("s add bad ", "stat", ""));
    assert_eq!(parsed("scoreboard objectives setdisplay belowname obj ascending").err(), syntax("wname obj ", "ascending", ""));
    assert_eq!(parsed("scoreboard players set @s obj banana").err(), syntax("et @s obj ", "banana", ""));
    assert_eq!(parsed("scoreboard players test @s a").err(), syntax(" test @s a", "", ""));
}
