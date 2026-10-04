//! Sidebar, boss bar, titles and toasts as packets (harness: common/mod.rs). The expected shapes
//! are BDS 1.26.52's (`sapling` example `bds_ui_probe`).

mod common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{
    BossEvent, BossEventColor, BossEventOverlay, BossEventType, RemoveObjective, SetDisplayObjective, SetScore, SetScoreEntriesItemContent as Content, SetTitle, SetTitleType,
    ToastRequest,
};
use common::{Harness, all};
use sapling_api::{BossBar, BossBarColour, PlayerId, ShardCtx, Sidebar, Title};

/// Runs `action` in the next tick and returns what it made the shard send to player 1.
fn after(h: &mut Harness, action: impl FnOnce(&mut ShardCtx<'_>) + Send + 'static) -> Vec<RawPacket> {
    h.remote.run(action);
    h.tick(1)
}

fn board(title: &str, lines: &[&str]) -> Sidebar {
    lines.iter().fold(Sidebar::new(title), |board, line| board.line(*line))
}

fn show(h: &mut Harness, title: &'static str, lines: &'static [&'static str]) -> Vec<RawPacket> {
    after(h, move |ctx| ctx.set_sidebar(PlayerId(1), &board(title, lines)))
}

/// Every score entry among `packets` in order: `+id score text` for a set, `-id` for a remove.
fn scores(packets: &[RawPacket]) -> Vec<String> {
    let entry = |e: &acacia_proto::packets::SetScoreEntriesItem| match &e.content {
        Content::FakePlayer(line) => {
            assert_eq!((line.objective_name.as_str(), e.entry_type_name.as_str()), ("sapling", "changefakeplayer"));
            format!("+{} {} {}", e.scoreboard_id, line.score, line.custom_name)
        }
        Content::Remove(removed) => {
            assert_eq!((removed.objective_name.as_deref(), e.entry_type_name.as_str()), (Some("sapling"), "remove"));
            format!("-{}", e.scoreboard_id)
        }
        Content::Default(other) => panic!("an entity score: {other:?}"),
    };
    all::<SetScore>(packets).iter().flat_map(|p| p.entries.iter().map(entry).collect::<Vec<_>>()).collect()
}

fn kinds(packets: &[RawPacket]) -> Vec<&'static str> {
    packets.iter().map(|p| acacia_proto::packet_name(p.id).unwrap_or("?")).collect()
}

#[test]
fn a_sidebar_is_an_objective_with_a_fake_player_per_line() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let sent = show(&mut h, "§lHub", &["Online: 1", "", "x 0"]);
    assert_eq!(kinds(&sent), ["SetDisplayObjective", "SetScore"]);
    let objective = &all::<SetDisplayObjective>(&sent)[0];
    let expected = SetDisplayObjective { display_slot: "sidebar".into(), objective_name: "sapling".into(), display_name: "§lHub".into(), criteria_name: "dummy".into(), sort_order: 0 };
    assert_eq!(objective, &expected);
    assert_eq!(scores(&sent), ["+1 0 Online: 1", "+2 1 ", "+3 2 x 0"]);
}

#[test]
fn a_sidebar_update_sends_only_the_lines_that_changed() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    show(&mut h, "Hub", &["Online: 1", "", "x 0"]);
    assert!(show(&mut h, "Hub", &["Online: 1", "", "x 0"]).is_empty());

    // A changed line is removed, then set again under its id.
    let sent = show(&mut h, "Hub", &["Online: 2", "", "x 0"]);
    assert_eq!((kinds(&sent), scores(&sent)), (vec!["SetScore", "SetScore"], vec!["-1".to_owned(), "+1 0 Online: 2".to_owned()]));
    let sent = show(&mut h, "Hub", &["Online: 2", "", "x 5", "z 7"]);
    assert_eq!(scores(&sent), ["-3", "+3 2 x 5", "+4 3 z 7"]);
    let sent = show(&mut h, "Hub", &["Online: 2"]);
    assert_eq!((kinds(&sent), scores(&sent)), (vec!["SetScore"], vec!["-2".to_owned(), "-3".to_owned(), "-4".to_owned()]));

    // A new title rebuilds the board.
    let sent = show(&mut h, "Lobby", &["Online: 2", "new"]);
    assert_eq!(kinds(&sent), ["RemoveObjective", "SetDisplayObjective", "SetScore"]);
    assert_eq!((all::<SetDisplayObjective>(&sent)[0].display_name.as_str(), scores(&sent)), ("Lobby", vec!["+1 0 Online: 2".to_owned(), "+2 1 new".to_owned()]));

    let sent = after(&mut h, |ctx| ctx.remove_sidebar(PlayerId(1)));
    assert_eq!((kinds(&sent), all::<RemoveObjective>(&sent)[0].objective_name.as_str()), (vec!["RemoveObjective"], "sapling"));
    assert!(after(&mut h, |ctx| ctx.remove_sidebar(PlayerId(1))).is_empty());
    assert_eq!(kinds(&show(&mut h, "Lobby", &["back"])), ["SetDisplayObjective", "SetScore"]);
}

#[test]
fn a_sidebar_shows_fifteen_lines_at_most_and_is_per_player() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    h.remote.run(|ctx| {
        let long = (0..20).fold(Sidebar::new("Long"), |board, n| board.line(format!("line {n}")));
        ctx.set_sidebar(PlayerId(1), &long);
        ctx.set_sidebar(PlayerId(9), &long);
    });
    let sent = h.tick_all();
    assert_eq!((scores(sent.to(1)).len(), scores(sent.to(1))[14].as_str()), (15, "+15 14 line 14"));
    assert!(sent.to(2).is_empty());
    // The lines past the fifteenth were never shown, so changing them sends nothing.
    h.remote.run(|ctx| ctx.set_sidebar(PlayerId(1), &(0..15).fold(Sidebar::new("Long"), |board, n| board.line(format!("line {n}")))));
    assert!(h.tick_all().to(1).is_empty());
}

#[test]
fn a_boss_bar_is_shown_changed_and_hidden() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let sent = after(&mut h, |ctx| ctx.set_boss_bar(PlayerId(1), &BossBar::new("sapling hub")));
    let expected = BossEvent {
        target_entity_id: 1,
        r#type: BossEventType::ShowBar,
        title: "sapling hub".into(),
        filtered_title: String::new(),
        progress: 1.0,
        color: BossEventColor::Purple,
        overlay: BossEventOverlay::Progress,
    };
    assert_eq!((kinds(&sent), &all::<BossEvent>(&sent)[0]), (vec!["BossEvent"], &expected));
    assert!(after(&mut h, |ctx| ctx.set_boss_bar(PlayerId(1), &BossBar::new("sapling hub"))).is_empty());

    let sent = after(&mut h, |ctx| ctx.set_boss_bar(PlayerId(1), &BossBar::new("restarting").progress(0.25).colour(BossBarColour::Red)));
    let changed = &all::<BossEvent>(&sent)[0];
    assert_eq!((changed.r#type, changed.title.as_str(), changed.progress, changed.color), (BossEventType::ShowBar, "restarting", 0.25, BossEventColor::Red));

    let sent = after(&mut h, |ctx| ctx.remove_boss_bar(PlayerId(1)));
    let hidden = &all::<BossEvent>(&sent)[0];
    assert_eq!((kinds(&sent), hidden.r#type, hidden.target_entity_id), (vec!["BossEvent"], BossEventType::HideBar, 1));
    assert!(after(&mut h, |ctx| ctx.remove_boss_bar(PlayerId(1))).is_empty());
}

#[test]
fn titles_action_bars_and_toasts_are_sent_as_bds_sends_them() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let sent = after(&mut h, |ctx| ctx.send_title(PlayerId(1), &Title::new("Welcome").subtitle("to the hub").times(10, 40, 10)));
    let titles = all::<SetTitle>(&sent);
    let shape = |t: &SetTitle| (t.r#type, t.text.clone(), [t.fade_in_time, t.stay_time, t.fade_out_time]);
    let expected = [
        (SetTitleType::SetDurations, String::new(), [10, 40, 10]),
        (SetTitleType::SetSubtitle, "to the hub".to_owned(), [-1; 3]),
        (SetTitleType::SetTitle, "Welcome".to_owned(), [-1; 3]),
    ];
    assert_eq!((sent.len(), titles.iter().map(shape).collect::<Vec<_>>()), (3, expected.to_vec()));
    assert!(titles.iter().all(|t| t.xuid.is_empty() && t.platform_online_id.is_empty() && t.filtered_message.is_empty()));

    let sent = after(&mut h, |ctx| ctx.send_title(PlayerId(1), &Title::new("Plain")));
    assert_eq!(all::<SetTitle>(&sent).iter().map(|t| t.r#type).collect::<Vec<_>>(), [SetTitleType::SetDurations, SetTitleType::SetTitle]);

    let sent = after(&mut h, |ctx| {
        ctx.send_actionbar(PlayerId(1), "above the hotbar");
        ctx.send_toast(PlayerId(1), "Achievement", "You joined");
    });
    assert_eq!(shape(&all::<SetTitle>(&sent)[0]), (SetTitleType::ActionBarMessage, "above the hotbar".to_owned(), [-1; 3]));
    assert_eq!(all::<ToastRequest>(&sent)[0], ToastRequest { title: "Achievement".into(), message: "You joined".into() });
}
