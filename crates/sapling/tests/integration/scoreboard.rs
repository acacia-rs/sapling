//! `/scoreboard` and `/tag` run by a bot while a second one watches and a third joins late: the
//! answers and scoreboard packets, against research/captures/bds-scoreboard.txt.

use crate::common;
#[path = "common/travel.rs"]
mod travel;

use acacia_bot::client::PacketFilter;
use acacia_bot::proto::Packet;
use acacia_bot::proto::packets::{CommandOutput, RemoveObjective, SetDisplayObjective, SetScore, SetScoreEntriesItemContent as Content, Text, TextContent};
use acacia_bot::{Bot, BotConfig, BotEvent};
use travel::{Operators, WAIT};

const MARKER: &str = "time query daytime";
const MARKER_ANSWER: &str = "commands.time.query.daytime";

/// A packet of the drill in one line; players' entries leave out their wire id.
fn describe(event: BotEvent) -> Option<String> {
    let BotEvent::Packet(raw) = event else { return None };
    Some(match raw.id {
        CommandOutput::ID => {
            let answer: CommandOutput = raw.decode().expect("decodes");
            let lines: Vec<String> = answer.output.iter().map(|line| format!("{} {:?}", line.message_id, line.parameters)).collect();
            format!("answer {} {}", lines.join(" | "), answer.success_count)
        }
        Text::ID => match raw.decode::<Text>().expect("decodes").content {
            TextContent::JsonWhisper(json) => format!("json {}", json.message.trim_end()),
            _ => return None,
        },
        SetDisplayObjective::ID => {
            let shown: SetDisplayObjective = raw.decode().expect("decodes");
            format!("display {} {:?} {:?} {}", shown.display_slot, shown.objective_name, shown.display_name, shown.sort_order)
        }
        SetScore::ID => {
            let entries = raw.decode::<SetScore>().expect("decodes").entries.into_iter().map(|entry| match entry.content {
                Content::Remove(remove) => format!("{} remove {:?}", entry.scoreboard_id, remove.objective_name),
                Content::FakePlayer(fake) => format!("{} {} {} {} {}", entry.scoreboard_id, entry.entry_type_name, fake.objective_name, fake.score, fake.custom_name),
                Content::Default(actor) => format!("{} {} {} {}", entry.scoreboard_id, entry.entry_type_name, actor.objective_name, actor.score),
            });
            format!("score {}", entries.collect::<Vec<_>>().join(", "))
        }
        RemoveObjective::ID => format!("remove {}", raw.decode::<RemoveObjective>().expect("decodes").objective_name),
        _ => return None,
    })
}

async fn gather(bot: &mut Bot, done: impl Fn(&str) -> bool) -> Vec<String> {
    let mut lines = Vec::new();
    let gathered = tokio::time::timeout(WAIT, async {
        loop {
            match bot.next().await {
                Some(BotEvent::Disconnected(reason)) => panic!("disconnected: {reason:?}"),
                Some(event) => lines.extend(describe(event)),
                None => panic!("the connection is gone"),
            }
            if lines.last().is_some_and(|line| line.starts_with(&format!("answer {MARKER_ANSWER} "))) {
                lines.pop();
                return;
            }
            if lines.last().is_some_and(|line| done(line)) {
                return;
            }
        }
    });
    gathered.await.unwrap_or_else(|_| panic!("in time; so far {lines:?}"));
    lines
}

async fn run(bot: &mut Bot, line: &str) -> Vec<String> {
    bot.client().command(line);
    bot.client().command(MARKER);
    gather(bot, |_| false).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scores_and_tags_answer_and_show_as_on_bds() {
    let server = common::start(crate::common::builder().handler(Operators).vanilla_commands()).await;
    let subscribe = [CommandOutput::ID, Text::ID, SetDisplayObjective::ID, SetScore::ID, RemoveObjective::ID].into_iter().fold(PacketFilter::none(), PacketFilter::with);
    let config = || BotConfig { subscribe: subscribe.clone(), ..BotConfig::default() };
    let mut alice = common::spawn(common::client(&server, "Alice"), config()).await;
    let mut bob = common::spawn(common::client(&server, "Bob"), config()).await;
    alice.wait_ticks(20).await.expect("connected");

    let first: [(&str, &[&str]); 9] = [
        ("scoreboard objectives add obj dummy", &[r#"answer commands.scoreboard.objectives.add.success ["obj"] 1"#]),
        ("scoreboard objectives add obj dummy", &[r#"answer commands.scoreboard.objectives.add.alreadyExists ["obj"] 0"#]),
        ("scoreboard objectives setdisplay sidebar obj", &[r#"display sidebar "obj" "obj" 1"#, r#"answer commands.scoreboard.objectives.setdisplay.successSet ["sidebar", "obj"] 1"#]),
        ("scoreboard players set @s obj 7", &["score 1 changeplayer obj 7", r#"answer commands.scoreboard.players.set.success ["obj", "Alice", "7"] 1"#]),
        ("scoreboard players set Fake obj 3", &["score 2 changefakeplayer obj 3 Fake", r#"answer commands.scoreboard.players.set.success ["obj", "Fake", "3"] 1"#]),
        ("scoreboard players add @s obj 5", &["score 1 changeplayer obj 12", r#"answer commands.scoreboard.players.add.success ["5", "obj", "Alice", "12"] 1"#]),
        ("scoreboard players operation @s obj %= Fake obj", &["score 1 changeplayer obj 0", r#"answer commands.scoreboard.players.set.success ["obj", "Alice", "0"] 1"#]),
        ("scoreboard players set nosuch obj banana", &[r#"answer commands.generic.syntax ["osuch obj ", "banana", ""] 0"#]),
        ("scoreboard players test @s nosuch 1", &[r#"answer commands.scoreboard.objectiveNotFound ["nosuch"] 0"#]),
    ];
    for (line, expected) in first {
        assert_eq!(run(&mut alice, line).await, expected, "/{line}");
    }

    // One who joins now is shown the sidebar with its scores.
    let mut carol = common::spawn(common::client(&server, "Carol"), config()).await;
    let shown = gather(&mut carol, |line| line.starts_with("score")).await;
    assert_eq!(shown, [r#"display sidebar "obj" "obj" 1"#, "score 2 changefakeplayer obj 3 Fake, 1 changeplayer obj 0"]);

    let score_parts = r#"{"rawtext":[{"score":{"name":"Fake","objective":"obj"}},{"text":"|"},{"score":{"name":"*","objective":"obj"}},{"text":"|"},{"score":{"name":"@s","objective":"none"}}]}"#;
    let second: [(&str, &[&str]); 10] = [
        ("scoreboard players test Fake obj 3 3", &[r#"answer commands.scoreboard.players.test.success ["3", "3", "3"] 1"#]),
        ("scoreboard players list", &[r#"answer §a%commands.scoreboard.players.list.count ["2"] | %%s, %%s ["Fake", "Alice"] 1"#]),
        ("scoreboard players list Fake", &[r#"answer §a%commands.scoreboard.players.list.player.count ["1", "Fake"] | commands.scoreboard.players.list.player.entry ["3", "obj", "obj"] 1"#]),
        ("scoreboard players list Nobody", &[r#"answer commands.scoreboard.players.list.player.empty ["Nobody"] 0"#]),
        (&format!("tellraw @s {score_parts}"), &[r#"json {"rawtext":[{"text":"3"},{"text":"|"},{"text":"0"},{"text":"|"}]}"#]),
        ("tag @a[scores={obj=0}] add zero", &[r#"answer commands.tag.add.success.single ["zero", "Alice"] 1"#]),
        ("tag @s add zero", &["answer commands.tag.add.failed [] 0"]),
        ("tag @s list", &[r#"answer commands.tag.list.single.success ["Alice", "1", "§azero§r"] 1"#]),
        ("scoreboard players reset Fake", &[r#"score 2 remove Some("obj")"#, r#"answer commands.scoreboard.players.reset.success ["Fake"] 1"#]),
        ("scoreboard objectives remove obj", &["remove obj", r#"answer commands.scoreboard.objectives.remove.success ["obj"] 1"#]),
    ];
    for (line, expected) in second {
        assert_eq!(run(&mut alice, line).await, expected, "/{line}");
    }
    assert_eq!(run(&mut alice, "scoreboard objectives list").await, ["answer commands.scoreboard.objectives.list.empty [] 0"]);

    // The watcher saw the board change, and nothing that was Alice's alone.
    let watched = gather(&mut bob, |line| line == "remove obj").await;
    assert_eq!(watched, [
        r#"display sidebar "obj" "obj" 1"#,
        "score 1 changeplayer obj 7",
        "score 2 changefakeplayer obj 3 Fake",
        "score 1 changeplayer obj 12",
        "score 1 changeplayer obj 0",
        r#"score 2 remove Some("obj")"#,
        "remove obj",
    ]);
    for mut bot in [alice, bob, carol] {
        bot.disconnect().await;
    }
    server.shutdown().await;
}
