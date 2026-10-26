//! `/structure`, `/tickingarea`, `/place` and `/locate` run by a bot: each answer against
//! research/captures/bds-structures.txt, bds-ticking.txt, bds-place.txt and bds-locate.txt.

use crate::common;
#[path = "common/travel.rs"]
mod travel;

use acacia_bot::client::PacketFilter;
use acacia_bot::proto::Packet;
use acacia_bot::proto::packets::CommandOutput;
use acacia_bot::{Bot, BotConfig, BotEvent};
use travel::{Operators, WAIT};

/// Every line of the answer to `line`: key and parameters, then the success count.
async fn answer(bot: &mut Bot, line: &str) -> String {
    bot.client().command(line);
    let answered = tokio::time::timeout(WAIT, async {
        loop {
            match bot.next().await {
                Some(BotEvent::Packet(raw)) if raw.id == CommandOutput::ID => {
                    let answer: CommandOutput = raw.decode().expect("decodes");
                    let lines: Vec<String> = answer.output.iter().map(|message| format!("{} {:?}", message.message_id, message.parameters)).collect();
                    return format!("{} {}", lines.join(" | "), answer.success_count);
                }
                Some(BotEvent::Disconnected(reason)) => panic!("disconnected: {reason:?}"),
                Some(_) => {}
                None => panic!("the connection is gone"),
            }
        }
    });
    answered.await.unwrap_or_else(|_| panic!("/{line} answered in time"))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn structure_ticking_place_and_locate_answer_as_bds_answered() {
    let server = common::start(common::builder().handler(Operators).vanilla_commands()).await;
    let config = BotConfig { subscribe: PacketFilter::none().with(CommandOutput::ID), ..BotConfig::default() };
    let mut alice = common::spawn(common::client(&server, "Alice"), config).await;
    alice.wait_ticks(20).await.expect("connected");

    let steps: &[(&str, &str)] = &[
        ("fill ~3 ~5 ~3 ~5 ~7 ~5 stone", r#"commands.fill.success ["27"] 1"#),
        ("structure save s1 ~3 ~5 ~3 ~5 ~7 ~5", r#"commands.structure.save.success ["mystructure:s1"] 1"#),
        ("structure save big ~ ~ ~ ~64 ~ ~", r#"commands.structure.size.tooBig ["64", "384", "64", "65", "1", "1"] 0"#),
        ("structure save air ~3 ~20 ~3 ~5 ~22 ~5 false memory false", "commands.structure.empty.save [] 0"),
        ("structure load s1 ~-6 ~5 ~3 90_degrees x", r#"commands.structure.load.success ["s1"] 1"#),
        ("testforblocks ~3 ~5 ~3 ~5 ~7 ~5 ~-6 ~5 ~3", r#"commands.compare.success ["27"] 1"#),
        ("structure load nope ~-6 ~5 ~3", r#"commands.structure.notFound ["mystructure:nope"] 0"#),
        ("structure load s1 ~ 319 ~", r#"commands.structure.y.tooHigh ["317"] 0"#),
        ("structure load s1 ~-6 ~5 ~3 0_degrees none true true false 101", "commands.structure.invalid.integrity [] 0"),
        ("structure delete s1", r#"commands.structure.delete.success ["mystructure:s1"] 1"#),
        ("structure load s1 ~-6 ~5 ~3", r#"commands.structure.notFound ["mystructure:s1"] 0"#),
        ("tickingarea list", "commands.tickingarea.noneExist.currentDimension [] 0"),
        ("tickingarea add 0 0 0 20 0 20 farm", r#"commands.tickingarea-add-bounds.success ["0, 0, 0", "31, 0, 31"] | commands.tickingarea.inuse ["1", "10"] 1"#),
        ("tickingarea add 0 0 0 20 0 20 farm", r#"commands.tickingarea-add.conflictingname ["farm"] 0"#),
        ("tickingarea add 0 0 0 200 0 200",r#"commands.tickingarea-add.chunkfailure ["100"] 0"#),
        ("tickingarea add circle 40 0 40 5", r#"commands.tickingarea-add.radiusfailure ["4"] 0"#),
        ("tickingarea add circle 40 0 40 2 ring true", r#"commands.tickingarea-add-circle.preload.success ["40, 0, 40", "2"] | commands.tickingarea.inuse ["2", "10"] 1"#),
        (
            "tickingarea list",
            r#"§a%commands.tickingarea-list.success.currentDimension [] | - farm: 0 0 0 %commands.tickingarea-list.to 31 0 31
- ring (%commands.tickingarea-list.type.circle): 40 0 40 %commands.tickingarea-list.circle.radius: 2 %commands.tickingarea-list.chunks %commands.tickingarea-list.preload [] | commands.tickingarea.inuse ["2", "10"] 1"#,
        ),
        ("tickingarea preload farm", "- farm: 0 0 0 %commands.tickingarea-list.to 31 0 31\n [] | commands.tickingarea-preload.count [\"0\"] 1"),
        ("tickingarea remove 3 0 3", "commands.tickingarea-remove.success [] | - farm: 0 0 0 %commands.tickingarea-list.to 31 0 31\n [] | commands.tickingarea.inuse [\"1\", \"10\"] 1"),
        ("tickingarea remove nothing", r#"commands.tickingarea-remove.byname.failure ["nothing"] 0"#),
        ("tickingarea remove_all", "commands.tickingarea-remove_all.success [] | - ring (%commands.tickingarea-list.type.circle): 40 0 40 %commands.tickingarea-list.circle.radius: 2 %commands.tickingarea-list.chunks %commands.tickingarea-list.preload\n [] | commands.tickingarea.inuse [\"0\", \"10\"] 1"),
        ("place feature oak_tree_feature", r#"commands.generic.syntax ["e feature ", "oak_tree_feature", ""] 0"#),
        ("place feature minecraft:no_such_feature", r#"commands.generic.syntax ["e feature ", "minecraft:no_such_feature", ""] 0"#),
        ("place feature minecraft:oak_tree_feature ~2 400 ~2", "commands.place.invalidposition.error [] 0"),
        ("place jigsaw minecraft:trail_ruins/tower minecraft:road_anchor 30", "commands.place.maxdepth.error [] 0"),
        ("place jigsaw minecraft:no_such_pool minecraft:bottom 3", "commands.place.generation.error [] 0"),
        ("locate biome plains", r#"commands.generic.syntax ["ate biome ", "plains", ""] 0"#),
        ("locate structure village", "commands.locate.structure.fail.nostructurefound [] 0"),
    ];
    let mut mismatched = Vec::new();
    for (line, expected) in steps {
        let got = answer(&mut alice, line).await;
        if got != *expected {
            mismatched.push(format!("/{line}\n  got      {got}\n  expected {expected}"));
        }
    }
    assert!(mismatched.is_empty(), "{}", mismatched.join("\n"));
}
