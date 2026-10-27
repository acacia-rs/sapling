//! `/loot`, `/camera`, `/camerashake`, `/fog`, `/hud`, `/recipe`, `/inputpermission` and
//! `/mobevent` run by a bot with a second one online: each answer against
//! research/captures/bds-lootcmd2.txt, bds-screen.txt, bds-controls.txt and bds-controls2.txt.

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
async fn client_commands_answer_as_bds_answered() {
    let server = common::start(common::builder().handler(Operators).vanilla_commands()).await;
    let config = || BotConfig { subscribe: PacketFilter::none().with(CommandOutput::ID), ..BotConfig::default() };
    let mut alice = common::spawn(common::client(&server, "Alice"), config()).await;
    let _bob = common::spawn(common::client(&server, "Bob"), config()).await;
    alice.wait_ticks(20).await.expect("connected");

    let steps: &[(&str, &str)] = &[
        ("loot spawn ~2 ~ ~ loot entities/chicken", r#"commands.generic.syntax ["t entities", "/", "chicken"] 0"#),
        ("loot spawn ~2 ~ ~ loot nosuch", r#"commands.loot.failure.invalidLootTable ["loot_tables/nosuch.json"] 0"#),
        ("loot spawn ~2 ~ ~ loot \"entities/armor_stand\"", r#"commands.loot.success ["0"] 1"#),
        ("loot spawn ~2 ~ ~ kill @s", r#"commands.loot.success ["0"] 1"#),
        ("loot spawn ~2 ~ ~ kill @e[type=sheep,r=1]", r#"commands.generic.noTargetMatch [] 0"#),
        ("loot spawn ~2 ~ ~ mine ~2 10000 ~2", r#"commands.loot.success ["0"] 1"#),
        ("loot replace entity @s slot.hotbar 9 loot \"entities/chicken\"", r#"commands.loot.failure.invalidSlotIdForSlotType ["9", "0", "8", "slot.hotbar"] 0"#),
        ("camerashake add @s 5 1 positional", r#"commands.generic.num.tooBig ["5.00", "4.00"] 0"#),
        ("camerashake stop", r#"commands.screenshake.successStop ["Alice, Bob"] 2"#),
        ("camera @s set nosuch", "commands.camera.unknown.preset [] 0"),
        ("camera @s set minecraft:free rot 100 0", "commands.camera.invalid.pitch [] 0"),
        ("camera @s fade time 11 2 3", "commands.camera.invalid.time [] 0"),
        ("camera @s set minecraft:free ease 2 in_out_sine pos ~ ~5 ~ rot 30 90", r#"commands.camera.success ["Alice"] 1"#),
        ("fog @s push minecraft:fog_hell probe", r#"commands.fog.success.push ["Alice", "probe"] 1"#),
        ("fog @s pop nothing", r#"commands.fog.invalidUserId ["Alice", "nothing"] 0"#),
        ("fog @s remove probe", r#"commands.fog.success.remove ["Alice", "probe"] 1"#),
        ("hud @s hide banana", r#"commands.generic.syntax ["d @s hide ", "banana", ""] 0"#),
        ("hud @s hide hotbar", "commands.hud.success [] 1"),
        ("recipe give @s minecraft:stick", r#"commands.recipes.unlockedRecipe ["Alice", "minecraft:stick"] 1"#),
        ("recipe give @s minecraft:stick", r#"commands.recipes.playerHasRecipe ["Alice", "minecraft:stick"] 0"#),
        ("recipe give @a minecraft:stick", r#"commands.recipes.playerHasRecipe ["Alice", "minecraft:stick"] | commands.recipes.unlockedRecipe ["Bob", "minecraft:stick"] 1"#),
        ("recipe take @s minecraft:WorkBench_recipeId_from_oak", r#"commands.recipes.removedRecipe ["Alice", "minecraft:WorkBench_recipeId_from_oak"] 1"#),
        ("recipe give @s nosuch", "commands.recipes.notFound [] 0"),
        ("recipe give nobody minecraft:stick", "commands.generic.noTargetMatch [] | commands.generic.player.notFound [] 0"),
        ("inputpermission set @s movement disabled", r#"Movement input disabled for Alice   ["1"] 1"#),
        ("inputpermission query @a movement", r#"<Movement>: 1 enabled | 1 disabled ["Bob"] 1"#),
        ("inputpermission query @s movement disabled", r#"<Movement>: 1 disabled ["Alice"] 1"#),
        ("inputpermission query @a movement disabled", r#"<Movement>: 1 disabled ["Alice"] 0"#),
        ("inputpermission set @a mount enabled", r#"Mount  input enabled for 2 players   ["2"] 1"#),
        ("inputpermission set @s banana enabled", r#"commands.generic.syntax ["on set @s ", "banana", " enabled"] 0"#),
        ("mobevent minecraft:pillager_patrols_event false", r#"commands.mobevent.eventSetToFalse ["%entity.pillager.name", "minecraft:pillager_patrols_event"] 1"#),
        ("mobevent minecraft:pillager_patrols_event", r#"commands.mobevent.eventIsFalse ["%entity.pillager.name", "minecraft:pillager_patrols_event"] 1"#),
        ("mobevent events_enabled", "commands.mobevent.eventsEnabledIsTrue [] 1"),
        ("music volume 5", r#"commands.generic.num.tooBig ["5.00", "1.00"] 0"#),
        ("music volume 0.3", r#"commands.music.success.volumeAction ["0.30"] 1"#),
    ];
    for (line, expected) in steps {
        assert_eq!(answer(&mut alice, line).await, *expected, "/{line}");
    }
}
