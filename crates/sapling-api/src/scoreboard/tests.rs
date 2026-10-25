use super::*;
use crate::testing::{Fake, player};

#[test]
fn operations_work_as_bdss_ints() {
    use ScoreOperation::*;
    let ops = |target, source| [Add, Subtract, Multiply, Divide, Remainder, Set, Min, Max].map(|op| op.apply(target, source).0);
    assert_eq!(ops(7, 3), [10, 4, 21, 2, 1, 3, 3, 7]);
    // research/captures/bds-scoreboard2.txt: C++'s truncating / and %.
    assert_eq!([Divide.apply(-7, 2).0, Divide.apply(-7, -3).0, Remainder.apply(-7, -3).0, Remainder.apply(7, -3).0], [-3, 2, -1, 1]);
    assert_eq!([Divide.apply(5, 0), Remainder.apply(5, 0)], [(5, 0); 2], "by zero: as it was");
    assert_eq!([Add.apply(i32::MAX, 1).0, Divide.apply(i32::MIN, -1).0, Remainder.apply(i32::MIN, -1).0], [i32::MIN, i32::MIN, i32::MIN]);
    assert_eq!(Swap.apply(1, 2), (2, 1));
    let symbols: Vec<&str> = ScoreOperation::SYMBOLS.iter().map(|(symbol, _)| *symbol).collect();
    assert_eq!(symbols, ["+=", "-=", "*=", "/=", "%=", "=", "<", ">", "><"]);
}

#[test]
fn holders_are_named_by_star_selectors_and_names() {
    let mut backend = Fake::with(&["Alex", "Bob"]);
    let mut ctx = ShardCtx::new(&mut backend);
    assert!(ctx.add_objective(&Objective::dummy("kills", "")) && !ctx.add_objective(&Objective::dummy("kills", "Kills")));
    assert_eq!(ctx.objective("kills").unwrap().display_name, "kills", "no display name is the name");
    let sender = Sender::Player(player(1, "Alex"));
    let holders = |ctx: &mut ShardCtx<'_>, line: &str| {
        let online: Vec<Player> = ctx.players().collect();
        let target = Selector::parse(&mut Args::new(line, 0, &sender, &online)).unwrap();
        ctx.select_holders(&sender, &target)
    };
    assert_eq!(holders(&mut ctx, "bob"), [ScoreHolder::Player(PlayerId(2))]);
    assert_eq!(holders(&mut ctx, "Ghost"), [ScoreHolder::Fake("Ghost".into())]);
    assert_eq!(holders(&mut ctx, "@a"), [ScoreHolder::Player(PlayerId(1)), ScoreHolder::Player(PlayerId(2))]);
    assert!(holders(&mut ctx, "*").is_empty(), "nobody has a score yet");
    assert!(ctx.set_score(&ScoreHolder::Fake("Ghost".into()), "kills", 3) && !ctx.set_score(&ScoreHolder::Fake("Ghost".into()), "none", 3));
    ctx.set_score(&ScoreHolder::Player(PlayerId(2)), "kills", 5);
    assert_eq!(holders(&mut ctx, "*").len(), 2);
    assert_eq!(ctx.score_text(&sender, "@a", "kills").as_deref(), Some("5"), "Alex has none: left out");
    assert_eq!(ctx.score_text(&sender, "Ghost", "kills").as_deref(), Some("3"));
    assert_eq!(ctx.score_text(&sender, "@s", "kills"), None);
}

#[test]
fn removing_an_objective_takes_its_scores_and_slots() {
    let mut backend = Fake::with(&["Alex"]);
    let mut ctx = ShardCtx::new(&mut backend);
    ctx.add_objective(&Objective::dummy("a", "A"));
    let shown = Display { objective: "a".into(), order: SortOrder::default() };
    assert!(ctx.set_display(DisplaySlot::Sidebar, Some(&shown)) && !ctx.set_display(DisplaySlot::List, Some(&Display { objective: "b".into(), ..shown.clone() })));
    ctx.set_score(&ScoreHolder::Player(PlayerId(1)), "a", 1);
    assert_eq!((ctx.display(DisplaySlot::Sidebar), ctx.scores(&ScoreHolder::Player(PlayerId(1)))), (Some(shown), vec![("a".into(), 1)]));
    assert!(ctx.remove_objective("a") && !ctx.remove_objective("a"));
    assert_eq!((ctx.display(DisplaySlot::Sidebar), ctx.score_holders()), (None, Vec::new()));
}
