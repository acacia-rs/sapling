//! `/execute` run by a bot with a second one online: each answer against
//! research/captures/bds-execute.txt, bds-execute-if.txt and bds-execute-run.txt.

use crate::common;
#[path = "common/travel.rs"]
mod travel;

use acacia_bot::client::PacketFilter;
use acacia_bot::proto::Packet;
use acacia_bot::proto::packets::CommandOutput;
use acacia_bot::{Bot, BotConfig, BotEvent};
use travel::{Operators, WAIT};

/// Every line of the answer to `line`, `key [parameters]` joined by ` | `, and its success count.
async fn answer(bot: &mut Bot, line: &str) -> String {
    bot.client().command(line);
    let answered = tokio::time::timeout(WAIT, async {
        loop {
            match bot.next().await {
                Some(BotEvent::Packet(raw)) if raw.id == CommandOutput::ID => {
                    let answer: CommandOutput = raw.decode().expect("decodes");
                    let lines: Vec<String> = answer.output.iter().map(|line| format!("{} {:?}", line.message_id, line.parameters)).collect();
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
async fn execute_answers_as_bds_answered() {
    let server = common::start(common::builder().handler(Operators).vanilla_commands()).await;
    let config = || BotConfig { subscribe: PacketFilter::none().with(CommandOutput::ID), ..BotConfig::default() };
    let mut alice = common::spawn(common::client(&server, "Alice"), config()).await;
    let mut bob = common::spawn(common::client(&server, "Bob"), config()).await;
    alice.wait_ticks(20).await.expect("connected");

    let false_condition = |mode: &str, kind: &str| format!(r#"commands.execute.falseCondition ["{mode}", "{kind}"]"#);
    let steps: Vec<(&str, String)> = vec![
        ("execute", r#"commands.generic.syntax ["/execute", "", ""] 0"#.into()),
        ("execute as @s run banana", r#"commands.generic.syntax ["as @s run ", "banana", ""] 0"#.into()),
        ("execute run gamemode banana", r#"commands.generic.syntax [" gamemode ", "banana", ""] 0"#.into()),
        ("execute as nobody run say x", "commands.generic.noTargetMatch [] 0".into()),
        ("execute align banana run say x", "commands.execute.align.invalidInput [] 0".into()),
        ("execute as @a run testfor @s", r#"commands.testfor.success ["Alice"] | commands.testfor.success ["Bob"] 2"#.into()),
        (
            "execute as @a run testfor @e[type=wither]",
            r#"commands.generic.noTargetMatch [] | commands.execute.failed ["testfor", "Alice"] | commands.generic.noTargetMatch [] | commands.execute.failed ["testfor", "Bob"] 0"#.into(),
        ),
        ("execute run execute run banana", r#"commands.generic.syntax ["ecute run ", "banana", ""] 0"#.into()),
        ("execute run execute if entity @e[type=wither]", format!(r#"{} | commands.execute.failed ["execute", "Alice"] 0"#, false_condition("if", "entity"))),
        ("execute run op Bob", r#"commands.generic.error.permissions ["op"] | commands.execute.failed ["op", "Alice"] 0"#.into()),
        ("summon armor_stand ~3 ~ ~", "commands.summon.success [] 1".into()),
        ("summon armor_stand ~3 ~ ~3", "commands.summon.success [] 1".into()),
        ("execute if entity @e[type=armor_stand]", "commands.execute.trueCondition [] 1".into()),
        ("execute unless entity @e[type=armor_stand]", format!("{} 0", false_condition("unless", "entity"))),
        ("execute as @e[type=armor_stand] if entity @s[type=armor_stand]", "commands.execute.trueCondition [] | commands.execute.trueCondition [] 2".into()),
        ("execute as @e[type=armor_stand] run testfor @s", r#"commands.testfor.success ["Armor Stand"] | commands.testfor.success ["Armor Stand"] 2"#.into()),
        ("execute positioned ~5 ~ ~ run setblock ~ ~ ~ glass", "commands.setblock.success [] 1".into()),
        ("execute if block ~5 ~ ~ glass", "commands.execute.trueCondition [] 1".into()),
        ("execute if block ~5 ~ ~ stone", format!("{} 0", false_condition("if", "block"))),
        ("execute if block 0 10000 0 air", format!(r#"commands.execute.outRangedDetectPosition ["0", "10000", "0"] | {} 0"#, false_condition("if", "block"))),
        ("execute if blocks ~5 ~ ~ ~5 ~ ~ ~5 ~ ~ all", r#"commands.execute.trueConditionWithCount ["1"] 1"#.into()),
        ("execute if blocks ~ ~ ~ ~40 ~40 ~40 ~ ~ ~ all", format!(r#"commands.execute.ifUnlessBLocks.tooManyBlocks ["32768", "64000"] | {} 0"#, false_condition("if", "blocks"))),
        ("execute rotated 90 0 run setblock ^ ^ ^2 red_wool", "commands.setblock.success [] 1".into()),
        ("execute if block ~-2 ~ ~ red_wool", "commands.execute.trueCondition [] 1".into()),
        ("scoreboard objectives add s dummy", r#"commands.scoreboard.objectives.add.success ["s"] 1"#.into()),
        ("scoreboard players set @s s 5", r#"commands.scoreboard.players.set.success ["s", "Alice", "5"] 1"#.into()),
        ("scoreboard players set fake s 3", r#"commands.scoreboard.players.set.success ["s", "fake", "3"] 1"#.into()),
        ("execute if score @s s matches !1..4", "commands.execute.trueCondition [] 1".into()),
        ("execute if score @s s > fake s", "commands.execute.trueCondition [] 1".into()),
        ("execute if score @s s = fake s", format!("{} 0", false_condition("if", "score"))),
        ("execute if score nobody s matches 1", format!(r#"commands.execute.ifUnlessScore.targets.empty ["nobody"] | {} 0"#, false_condition("if", "score"))),
        ("execute if score @s nosuch matches 1", format!(r#"commands.execute.ifUnlessScore.objectiveNotFound ["nosuch"] | {} 0"#, false_condition("if", "score"))),
        (
            "execute unless score @e[type=armor_stand] s matches 1",
            format!(r#"commands.execute.ifUnlessScore.targets.tooMany ["2"] | commands.selector.error.moreThanOne [] | {} 0"#, false_condition("unless", "score")),
        ),
        ("kill @e[type=armor_stand]", r#"commands.kill.successful ["Armor Stand, Armor Stand"] 2"#.into()),
    ];
    for (line, expected) in &steps {
        assert_eq!(answer(&mut alice, line).await, *expected, "/{line}");
    }
    let aligned = answer(&mut alice, "execute positioned ~0.3 ~ ~0.3 align xz positioned ~0.25 ~ ~0.25 run tp @s ~ ~ ~").await;
    assert!(aligned.starts_with(r#"commands.tp.success.coordinates ["Alice", "#) && aligned.matches(".25\"").count() + aligned.matches(".75\"").count() == 2, "{aligned}");
    alice.disconnect().await;
    bob.disconnect().await;
    server.shutdown().await;
}
