use super::*;

fn parsed(rest: &str) -> Result<Execute, ParseError> {
    let sender = Sender::Console;
    let line = if rest.is_empty() { "execute".to_owned() } else { format!("execute {rest}") };
    let mut args = Args::new(&line, "execute".len(), &sender, &[]);
    let execute = Execute::parse(&mut args)?;
    args.finish()?;
    Ok(execute)
}

fn syntax(before: &str, token: &str, after: &str) -> Option<ParseError> {
    Some(ParseError::Syntax { before: before.into(), token: token.into(), after: after.into() })
}

#[test]
fn every_sub_command_reads() {
    let steps = |line: &str| parsed(line).map(|execute| (execute.steps.len(), execute.run.map(|(command, _)| command))).ok();
    assert_eq!(steps("in nether run tp @s 0 70 0"), Some((1, Some("tp @s 0 70 0".into()))));
    assert_eq!(steps("as @e[type=cow] at @s positioned ~ ~1 ~ rotated 0 ~ run setblock ^ ^ ^1 stone"), Some((4, Some("setblock ^ ^ ^1 stone".into()))));
    assert_eq!(steps("positioned as @s rotated as @p facing entity @e[c=1] eyes facing 1 2 3 align xz anchored eyes run say hi").map(|(n, _)| n), Some(6));
    assert_eq!(steps("if entity @s unless score @s s matches !1..4 if score a s >= b s if blocks ~ ~ ~ ~1 ~1 ~1 0 0 0 masked").map(|(n, run)| (n, run.is_none())), Some((4, true)));
}

#[test]
fn errors_are_placed_as_bds_places_them() {
    let error = |line: &str| parsed(line).err();
    assert_eq!(error(""), syntax("/execute", "", ""));
    assert_eq!(error("run"), syntax("xecute run", "", ""));
    assert_eq!(error("banana"), syntax("/execute ", "banana", ""));
    assert_eq!(error("as @s"), syntax("cute as @s", "", ""));
    assert_eq!(error("as @s run"), syntax(" as @s run", "", ""));
    assert_eq!(error("facing entity @e[type=pig] banana run say x"), syntax("type=pig] ", "banana", " run say x"));
    assert_eq!(error("in hell run say x"), syntax("xecute in ", "hell", " run say x"));
    assert_eq!(error("if banana"), syntax("xecute if ", "banana", ""));
    assert_eq!(error("if entity"), syntax(" if entity", "", ""));
    assert_eq!(error("if score @s s == fake s"), syntax("ore @s s =", "=", " fake s"));
    assert_eq!(error("if score @s s matches banana"), syntax("s matches ", "banana", ""));
    assert_eq!(error("if blocks ~2 ~ ~2 ~2 ~ ~2 ~10 ~ ~10 banana"), syntax("~10 ~ ~10 ", "banana", ""));
    assert_eq!(error("align banana run say x"), Some(ParseError::Invalid("commands.execute.align.invalidInput".into())));
    assert_eq!(error("align xx run say x"), Some(ParseError::Invalid("commands.execute.align.invalidInput".into())));
    assert_eq!(error("positioned ^ ~ ^ run say x"), syntax("itioned ^ ", "~", " ^ run say x"), "`^` all or none");
}

#[test]
fn ranges_and_orders_hold_as_bds_tested_them() {
    let holds = |range: &str, score: i32| parse::Range::read(range).map(|range| range.holds(score));
    assert_eq!([holds("5", 5), holds("1..4", 5), holds("5..", 5), holds("..5", 5), holds("!5", 5), holds("!1..4", 5)], [Some(true), Some(false), Some(true), Some(true), Some(false), Some(true)]);
    assert_eq!([holds("..", 0), holds("banana", 0), holds("1..x", 0)], [None, None, None]);
}
