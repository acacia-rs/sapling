use super::*;

fn fake(name: &str) -> ScoreIdentity {
    ScoreIdentity::Fake(name.into())
}

fn shown(objective: &str, order: SortOrder) -> Display {
    Display { objective: objective.into(), order }
}

#[test]
fn a_board_is_saved_and_read_back_as_it_was() {
    let mut board = Board::default();
    board.add_objective(&Objective::dummy("kills", "§cKills"));
    board.add_objective(&Objective::dummy("a", ""));
    board.set_score(&ScoreIdentity::Player(-4294967295), "kills", 7);
    board.set_score(&fake("Fake"), "a", -3);
    board.set_score(&ScoreIdentity::Entity(77), "kills", 1);
    board.reset_score(&ScoreIdentity::Entity(77), None);
    board.set_display(DisplaySlot::BelowName, Some(&shown("kills", SortOrder::Ascending)));
    let record = board.record();
    assert_eq!(record.entries, [(1, ScoreIdentity::Player(-4294967295)), (2, fake("Fake"))], "a holder without scores is no entry");
    assert_eq!(record.last_unique_id, 3);
    let back = Board::from_record(sapling_world::decode_scoreboard(&sapling_world::encode_scoreboard(&record)).unwrap());
    assert_eq!(back.record(), record);
    assert_eq!(back.display(DisplaySlot::BelowName), Some(shown("kills", SortOrder::Ascending)));
    assert_eq!(back.score(&fake("Fake"), "a"), Some(-3));
}

#[test]
fn only_what_is_shown_is_told() {
    let mut board = Board::default();
    board.add_objective(&Objective::dummy("a", "A"));
    board.add_objective(&Objective::dummy("b", "B"));
    board.set_score(&fake("x"), "a", 1);
    assert!(board.changes.is_empty(), "a is not shown");
    assert!(board.set_display(DisplaySlot::Sidebar, None) && board.changes.is_empty(), "clearing an empty slot tells nobody");
    board.set_display(DisplaySlot::Sidebar, Some(&shown("a", SortOrder::Descending)));
    board.set_score(&fake("x"), "a", 1);
    board.set_display(DisplaySlot::Sidebar, Some(&shown("b", SortOrder::Descending)));
    board.reset_score(&fake("x"), Some("a"));
    board.remove_objective("a");
    let a = (Objective::dummy("a", "A"), SortOrder::Descending);
    let b = (Objective::dummy("b", "B"), SortOrder::Descending);
    assert_eq!(board.changes, [
        Change::Displayed { slot: DisplaySlot::Sidebar, shown: Some(a), replaced: false },
        Change::Score { objective: "a".into(), id: 1, score: Some(1) },
        Change::Displayed { slot: DisplaySlot::Sidebar, shown: Some(b), replaced: true },
        Change::Removed("a".into()),
    ]);
    assert!(!board.set_display(DisplaySlot::List, Some(&shown("a", SortOrder::Ascending))), "a is gone");
}

#[test]
fn holders_and_scores_list_as_bds_listed_them() {
    let mut board = Board::default();
    board.add_objective(&Objective::dummy("b", ""));
    board.add_objective(&Objective::dummy("a", ""));
    for (name, score) in [("one", 1), ("two", 2)] {
        board.set_score(&fake(name), "a", score);
    }
    board.set_score(&fake("one"), "b", 5);
    assert_eq!(board.holders(), [fake("two"), fake("one")], "newest first");
    assert_eq!(board.scores(&fake("one")), [("b".to_owned(), 5), ("a".to_owned(), 1)], "in the objectives' order");
    assert_eq!(board.scores_on("a"), [(2, 2), (1, 1)]);
}
