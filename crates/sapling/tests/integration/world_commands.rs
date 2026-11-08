//! The spawn, server, region, item and selecting commands run by a bot with a second one online:
//! each answer against research/captures/bds-worldcmds.txt and bds-itemcmds.txt.

use crate::common;
#[path = "common/travel.rs"]
mod travel;

use acacia_bot::client::PacketFilter;
use acacia_bot::proto::Packet;
use acacia_bot::proto::packets::CommandOutput;
use acacia_bot::{Bot, BotConfig, BotEvent};
use travel::{Operators, WAIT};

/// The first line of the answer to `line`: its key, parameters and success count.
async fn answer(bot: &mut Bot, line: &str) -> String {
    bot.client().command(line);
    let answered = tokio::time::timeout(WAIT, async {
        loop {
            match bot.next().await {
                Some(BotEvent::Packet(raw)) if raw.id == CommandOutput::ID => {
                    let answer: CommandOutput = raw.decode().expect("decodes");
                    let first = &answer.output[0];
                    return format!("{} {:?} {}", first.message_id, first.parameters, answer.success_count);
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
async fn world_and_item_commands_answer_as_bds_answered() {
    let server = common::start(common::builder().handler(Operators).vanilla_commands()).await;
    let config = || BotConfig { subscribe: PacketFilter::none().with(CommandOutput::ID), ..BotConfig::default() };
    let mut alice = common::spawn(common::client(&server, "Alice"), config()).await;
    let mut bob = common::spawn(common::client(&server, "Bob"), config()).await;
    alice.wait_ticks(20).await.expect("connected");
    // The fixture's plains spawn cows about the players, which the `/damage` below must not find.
    for line in ["gamerule domobspawning false", "kill @e[type=!player]"] {
        answer(&mut alice, line).await;
    }

    let steps: &[(&str, &str)] = &[
        ("setworldspawn 10 70 10", r#"commands.setworldspawn.success ["10", "70", "10"] 1"#),
        ("setworldspawn 0 400 0", r#"commands.setworldspawn.success ["0", "319", "0"] 1"#),
        ("spawnpoint @s 5 70 5", r#"commands.spawnpoint.success.single ["Alice", "5", "70", "5"] 1"#),
        ("spawnpoint @a 3 500 3", r#"commands.spawnpoint.success.multiple.specific ["Alice, Bob", "3", "500", "3"] 2"#),
        ("spawnpoint @a[name=nobody]", "commands.generic.noTargetMatch [] 0"),
        ("clearspawnpoint @a", r#"commands.clearspawnpoint.success.multiple ["Alice, Bob"] 2"#),
        ("clearspawnpoint", r#"commands.clearspawnpoint.success.single ["Alice"] 1"#),
        ("difficulty h", r#"commands.difficulty.success ["HARD"] 1"#),
        ("difficulty 0", r#"commands.difficulty.success ["PEACEFUL"] 1"#),
        ("difficulty 4", r#"commands.generic.parameter.invalid ["4"] 0"#),
        ("testfor @a", r#"commands.testfor.success ["Alice, Bob"] 2"#),
        ("testfor nobody", "commands.generic.noTargetMatch [] 0"),
        ("defaultgamemode creative", r#"commands.defaultgamemode.success ["%createWorldScreen.gameMode.creative"] 1"#),
        ("setmaxplayers -3", r#"commands.setmaxplayers.success ["2"] 1"#),
        ("transfer nobody 127.0.0.1 19132", "commands.generic.player.notFound [] 0"),
        ("transfer Bob 127.0.0.1 99999", "commands.transferserver.invalid.port [] 0"),
        ("fill ~3 ~5 ~3 ~5 ~7 ~5 stone hollow", r#"commands.fill.success ["26"] 1"#),
        ("fill ~3 ~5 ~3 ~5 ~7 ~5 stone hollow", r#"commands.fill.success ["0"] 0"#),
        ("fill ~3 ~5 ~3 ~5 ~7 ~5 dirt keep", r#"commands.fill.success ["1"] 1"#),
        ("fill ~3 ~5 ~3 ~5 ~7 ~5 glass replace stone", r#"commands.fill.success ["26"] 1"#),
        ("fill ~ ~ ~ ~40 ~40 ~40 stone outline", r#"commands.fill.tooManyBlocks ["68921", "32768"] 0"#),
        // bds-tickedge.txt: six chunks off is past `tick-distance` 4's region.
        ("fill ~96 ~5 ~ ~96 ~5 ~ stone", "commands.fill.outOfWorld [] 0"),
        ("setblock ~96 ~5 ~ stone", "commands.setblock.outOfWorld [] 0"),
        ("testforblock ~96 ~5 ~ stone", "commands.testforblock.outOfWorld [] 0"),
        ("clone ~ ~5 ~ ~ ~5 ~ ~96 ~5 ~", "commands.generic.outOfWorld [] 0"),
        ("clone ~3 ~5 ~3 ~5 ~7 ~5 ~10 ~5 ~3", r#"commands.clone.success ["27"] 1"#),
        ("testforblocks ~3 ~5 ~3 ~5 ~7 ~5 ~10 ~5 ~3", r#"commands.compare.success ["27"] 1"#),
        ("testforblocks ~3 ~5 ~3 ~5 ~7 ~5 ~10 ~5 ~3 masked", r#"commands.compare.success ["27"] 1"#),
        ("clone ~3 ~5 ~3 ~5 ~7 ~5 ~4 ~5 ~3", "commands.clone.noOverlap [] 0"),
        ("clone ~3 ~5 ~3 ~5 ~7 ~5 ~3 ~5 ~20 filtered normal oak_log", "commands.clone.failed [] 0"),
        ("clone ~3 ~5 ~3 ~5 ~7 ~5 ~3 ~5 ~20 filtered move dirt", r#"commands.clone.success ["1"] 1"#),
        ("testforblocks ~3 ~5 ~3 ~5 ~7 ~5 ~10 ~5 ~3", "commands.compare.failed [] 0"),
        ("clear", r#"commands.clear.failure.no.items ["Alice"] 0"#),
        ("give @s stone 0", r#"commands.generic.num.tooSmall ["0", "1"] 0"#),
        ("give @s apple 1 -1", r#"commands.generic.num.tooSmall ["-1", "0"] 0"#),
        (r#"give @s apple 1 0 {"bogus":{}}"#, "Component 'bogus' is not a supported component. [] 0"),
        (r#"give @s apple 1 0 {"minecraft:keep_on_death":{}}"#, r#"commands.give.success ["Apple", "1", "Alice"] 1"#),
        ("give @s wool 3 14", r#"commands.give.success ["Red Wool", "3", "Alice"] 1"#),
        ("clear @s wool 14 1", r#"commands.clear.success ["Alice", "1"] 1"#),
        ("clear @s red_wool -1 0", r#"commands.clear.testing ["Alice", "2"] 1"#),
        ("clear @a red_wool", r#"commands.clear.success ["Alice", "2"] 1"#),
        ("clear @s apple", r#"commands.clear.success ["Alice", "1"] 1"#),
        ("clear @s apple 0 0", r#"commands.clear.failure.no.items ["Alice"] 0"#),
        ("give @s shulker_box 1 14", r#"commands.give.success ["Red Shulker Box", "1", "Alice"] 1"#),
        ("give @s coral_block 1 14", r#"commands.give.success ["Dead Tube Coral Block", "1", "Alice"] 1"#),
        ("give @s ender_pearl 3000", r#"commands.give.success ["Ender Pearl", "3000", "Alice"] 1"#),
        ("clear", r#"commands.clear.success ["Alice", "546"] 1"#),
        ("give @s diamond_sword", r#"commands.give.success ["Diamond Sword", "1", "Alice"] 1"#),
        ("enchant @s sharpness", r#"commands.enchant.success ["Alice"] 1"#),
        ("enchant @s sharpness 6", r#"commands.generic.num.tooBig ["6", "5"] 0"#),
        ("enchant @s smite", r#"commands.enchant.cantEnchant ["Alice"] 0"#),
        ("enchant @s protection", r#"commands.enchant.cantEnchant ["Alice"] 0"#),
        ("enchant @s 12 2", r#"commands.enchant.success ["Alice"] 1"#),
        ("enchant @s knockback 1", r#"commands.enchant.cantEnchant ["Alice"] 0"#),
        ("enchant @s 99", r#"commands.enchant.notFound ["99"] 0"#),
        ("enchant Bob sharpness", r#"commands.enchant.noItem ["Bob"] 0"#),
        (r#"give @s dirt 1 0 {"minecraft:can_place_on":{"blocks":["nosuch"]}}"#, "Component 'minecraft:can_place_on' could not be updated. [] 0"),
        (r#"give @s dirt 1 0 {"minecraft:can_place_on":{"blocks":"stone"}}"#, "Component 'minecraft:can_place_on' could not be updated. [] 0"),
        (r#"give @s dirt 1 0 {"can_destroy":{"blocks":["dirt"]}}"#, r#"commands.give.success ["Dirt", "1", "Alice"] 1"#),
        // bds-k1cmds.txt.
        ("replaceitem entity @s slot.hotbar 0 stone 0", r#"commands.generic.num.tooSmall ["0", "1"] 0"#),
        ("replaceitem entity @s slot.hotbar 0 stone 65", r#"commands.generic.num.tooBig ["65", "64"] 0"#),
        ("replaceitem entity @s slot.hotbar 0 wool 1 5", r#"commands.replaceitem.success.entity ["slot.hotbar", "0", "Alice", "1", "Lime Wool"] 1"#),
        ("replaceitem entity @s slot.hotbar 0 wool 1 20", r#"commands.replaceitem.success.entity ["slot.hotbar", "0", "Alice", "1", "White Wool"] 1"#),
        ("spreadplayers ~ ~ -1 5 @a", r#"commands.generic.num.tooSmall ["-1.00", "0.00"] 0"#),
        ("spreadplayers ~ ~ 10 1 @a", r#"commands.generic.num.tooSmall ["1.00", "11.00"] 0"#),
        ("spreadplayers ~ ~ 1 5 nobody", "commands.generic.noTargetMatch [] 0"),
        // An instant effect's seconds are its ticks (bds-raid.txt, bds-effects.txt).
        ("effect @s saturation 1000 0 true", r#"commands.effect.success ["%potion.saturation", "0", "Alice", "50"] 1"#),
        ("effect @s instant_health 1 0", r#"commands.effect.success ["%potion.heal", "0", "Alice", "0"] 1"#),
    ];
    for (line, expected) in steps {
        assert_eq!(answer(&mut alice, line).await, *expected, "/{line}");
    }
    // Still peaceful, from `difficulty 0` (bds-server.txt).
    assert_eq!(answer(&mut alice, "summon zombie").await, "commands.summon.failed [] 0");
    let v2 = answer(&mut alice, "summon zombie_villager_v2 ~ ~ ~").await;
    assert!(v2.starts_with("commands.generic.syntax [") && v2.contains(r#""zombie_villager_v2""#), "{v2}");
    let pearl = answer(&mut alice, "summon ender_pearl ~ ~ ~").await;
    assert!(pearl.starts_with("commands.generic.syntax [") && pearl.contains(r#""ender_pearl""#), "bds-s7shots.txt: {pearl}");
    // bds-raidhero.txt: none matched answers noTargetMatch; each one hurt is counted.
    assert_eq!(answer(&mut alice, "damage @e[type=cow] 1").await, "commands.generic.noTargetMatch [] 0");
    for _ in 0..2 {
        assert_eq!(answer(&mut alice, "summon cow ~ ~ ~3").await, "commands.summon.success [] 1");
    }
    assert_eq!(answer(&mut alice, "damage @e[type=cow] 1 entity_attack entity @s").await, r#"commands.damage.success ["Cow, Cow"] 2"#);
    // bds-ad3reload.txt: a name tag after the event, which `name=` finds and the answer names.
    assert_eq!(answer(&mut alice, "summon cow ~ ~ ~3 0 0 minecraft:entity_spawned cow1").await, "commands.summon.success [] 1");
    assert_eq!(answer(&mut alice, "kill @e[type=cow,name=!cow1]").await, r#"commands.kill.successful ["Cow, Cow"] 2"#);
    assert_eq!(answer(&mut alice, "kill @e[type=cow,name=cow1]").await, r#"commands.kill.successful ["cow1"] 1"#);
    assert_eq!(answer(&mut alice, "kill @e[name=cow1]").await, "commands.generic.noTargetMatch [] 0");
    // bds-bg2names.txt: BDS's other overload, the name before the position.
    assert_eq!(answer(&mut alice, "summon villager v ~ ~ ~3").await, "commands.summon.success [] 1");
    // bds-bg2names.txt: other answers name it too.
    assert_eq!(answer(&mut alice, "effect @e[name=v] slowness 10 0 true").await, r#"commands.effect.success ["%potion.moveSlowdown", "0", "v", "10"] 1"#);
    assert_eq!(answer(&mut alice, "kill @e[name=v]").await, r#"commands.kill.successful ["v"] 1"#);
    // bds-bb1event.txt: kinds listed once each, the event by the pack's own name.
    for kind in ["sheep", "sheep", "cow"] {
        assert_eq!(answer(&mut alice, &format!("summon {kind} ~ ~ ~3")).await, "commands.summon.success [] 1");
    }
    let events: &[(&str, &str)] = &[
        ("event entity @e[type=sheep] minecraft:entity_born", r#"commands.event.success ["minecraft:entity_born", "minecraft:sheep"] 1"#),
        ("event entity @e[type=sheep] entity_born", r#"commands.event.error.failed ["entity_born", "minecraft:sheep"] 0"#),
        ("event entity @e[type=sheep] spawn_baby", r#"commands.event.success ["spawn_baby", "minecraft:sheep"] 1"#),
        ("event entity @e[type=cow] minecraft:temperate_color", r#"commands.event.error.failed ["minecraft:temperate_color", "minecraft:cow"] 0"#),
        ("event entity @s minecraft:entity_born", r#"commands.event.error.failed ["minecraft:entity_born", "minecraft:player"] 0"#),
        ("event entity @s minecraft:gain_raid_omen", r#"commands.event.success ["minecraft:gain_raid_omen", "minecraft:player"] 1"#),
        ("event entity nobody minecraft:entity_born", "commands.generic.noTargetMatch [] 0"),
        ("event entity @e[type=sheep]", r#"commands.generic.syntax ["ype=sheep]", "", ""] 0"#),
    ];
    for (line, expected) in events {
        assert_eq!(answer(&mut alice, line).await, *expected, "/{line}");
    }
    let seed = answer(&mut alice, "seed").await;
    assert!(seed.starts_with("commands.seed.success [") && seed.ends_with("] 1"), "{seed}");
    let version = answer(&mut alice, "version").await;
    assert!(version.starts_with("sapling ") && version.ends_with("[] 1"), "{version}");
    let spread = answer(&mut alice, "spreadplayers ~ ~ 1 3 @a").await;
    assert!(spread.starts_with(r#"commands.spreadplayers.success.players ["2", "#), "{spread}");
    alice.disconnect().await;
    bob.disconnect().await;
    server.shutdown().await;
}

/// bds-s3misc.txt: `tp` counts each one moved, and the one gone to once more; a number written with
/// a point is not centred (a boat summoned at 2.0 stayed at 2.0).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tp_counts_whom_it_moves_and_centres_only_whole_numbers() {
    let server = common::start(common::builder().handler(Operators).vanilla_commands()).await;
    let config = || BotConfig { subscribe: PacketFilter::none().with(CommandOutput::ID), ..BotConfig::default() };
    let mut alice = common::spawn(common::client(&server, "Alice"), config()).await;
    let mut bob = common::spawn(common::client(&server, "Bob"), config()).await;
    alice.wait_ticks(20).await.expect("connected");
    let steps: &[(&str, &str)] = &[
        ("tp @a 3 70 3", r#"commands.tp.success.coordinates ["Alice, Bob", "3.50", "70.00", "3.50"] 2"#),
        ("tp @a @s", r#"commands.tp.success ["Alice, Bob", "Alice"] 3"#),
        ("tp @a 3.0 70 -2.0", r#"commands.tp.success.coordinates ["Alice, Bob", "3.00", "70.00", "-2.00"] 2"#),
    ];
    for (line, expected) in steps {
        assert_eq!(answer(&mut alice, line).await, *expected, "/{line}");
    }
    alice.disconnect().await;
    bob.disconnect().await;
    server.shutdown().await;
}
