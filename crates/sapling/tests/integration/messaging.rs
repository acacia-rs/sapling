//! `/me`, `/tellraw`, `/title`, `/titleraw`, `/playsound`, `/stopsound` and `/particle` run by a
//! bot while a second one watches: what each is sent, against research/captures/bds-messaging.txt.

use crate::common;
#[path = "common/travel.rs"]
mod travel;

use acacia_bot::client::PacketFilter;
use acacia_bot::proto::Packet;
use acacia_bot::proto::packets::{CommandOutput, PlaySound, SetTitle, SpawnParticleEffect, StopSound, Text, TextContent};
use acacia_bot::{Bot, BotConfig, BotEvent};
use travel::{Operators, WAIT};

/// Answered after every line, so that a line with no answer of its own still ends.
const MARKER: &str = "time query daytime";
const MARKER_ANSWER: &str = "commands.time.query.daytime";

/// A packet of the drill in one line; `None` for the rest (an operator's echo).
fn describe(event: BotEvent) -> Option<String> {
    let BotEvent::Packet(raw) = event else { return None };
    Some(match raw.id {
        CommandOutput::ID => {
            let answer: CommandOutput = raw.decode().expect("decodes");
            let line = &answer.output[0];
            format!("answer {} {:?} {}", line.message_id, line.parameters, answer.success_count)
        }
        Text::ID => match raw.decode::<Text>().expect("decodes").content {
            TextContent::Chat(chat) => format!("chat {:?} {}", chat.source_name, chat.message),
            TextContent::JsonWhisper(json) => format!("json {}", json.message.trim_end()),
            _ => return None,
        },
        SetTitle::ID => {
            let title: SetTitle = raw.decode().expect("decodes");
            format!("title {:?} {} {:?}", title.r#type, title.text.trim_end(), [title.fade_in_time, title.stay_time, title.fade_out_time])
        }
        PlaySound::ID => {
            let sound: PlaySound = raw.decode().expect("decodes");
            let at = sound.coordinates;
            format!("sound {} {:?} {} {}", sound.name, [at.x, at.y, at.z], sound.volume, sound.pitch)
        }
        StopSound::ID => {
            let stop: StopSound = raw.decode().expect("decodes");
            format!("stop {:?} {}", stop.name, stop.stop_all)
        }
        SpawnParticleEffect::ID => {
            let particle: SpawnParticleEffect = raw.decode().expect("decodes");
            format!("particle {} {:?} {}", particle.particle_name, [particle.position.x, particle.position.y, particle.position.z], particle.dimension)
        }
        _ => return None,
    })
}

/// What the bot is sent until the marker's answer, or until `done` says a line ends it.
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
async fn messages_titles_sounds_and_particles_go_out_as_bds_sent_them() {
    let server = common::start(crate::common::builder().handler(Operators).vanilla_commands()).await;
    let subscribe = [CommandOutput::ID, Text::ID, SetTitle::ID, PlaySound::ID, StopSound::ID, SpawnParticleEffect::ID].into_iter().fold(PacketFilter::none(), PacketFilter::with);
    let config = || BotConfig { subscribe: subscribe.clone(), ..BotConfig::default() };
    let mut alice = common::spawn(common::client(&server, "Alice"), config()).await;
    let mut bob = common::spawn(common::client(&server, "Bob"), config()).await;
    alice.wait_ticks(20).await.expect("connected");

    let hi = r#"json {"rawtext":[{"text":"hi "},{"text":"Alice, Bob"},{"translate":"%%s","with":{"rawtext":[{"text":"x"}]}}]}"#;
    let particle = "particle minecraft:heart_particle [21.25, 74.0, 205.25] 0";
    let steps: [(&str, &[&str]); 18] = [
        ("me waves", &[r#"chat "" * Alice waves"#]),
        ("me", &["answer commands.generic.invalidMessage [] 0"]),
        (r#"tellraw @a {"rawtext":[{"text":"hi "},{"selector":"@a"},{"selector":"@e[type=cow]"},{"score":{"name":"@s","objective":"o"}},{"translate":"%%s","with":["x"]}]}"#, &[hi]),
        (r#"tellraw @a {"rawtext":[{"text":5}]}"#, &["answer commands.tellraw.error.textNotString [] 0"]),
        ("tellraw @a not json", &[r#"answer commands.generic.syntax ["ellraw @a ", "not", " json"] 0"#]),
        (r#"tellraw nobody {"rawtext":[]}"#, &["answer commands.generic.noTargetMatch [] 0"]),
        ("title @a title Hello there", &["title SetTitle Hello there [-1, -1, -1]", r#"answer commands.title.success ["Alice, Bob"] 2"#]),
        ("title @a times 5 40 10", &["title SetDurations  [5, 40, 10]", r#"answer commands.title.success ["Alice, Bob"] 2"#]),
        ("title @s clear", &["title Clear  [-1, -1, -1]", r#"answer commands.title.success ["Alice"] 1"#]),
        ("title @s reset", &["title Reset  [-1, -1, -1]", r#"answer commands.title.success ["Alice"] 1"#]),
        ("title @s title", &["answer commands.generic.invalidMessage [] 0"]),
        (
            r#"titleraw @s actionbar {"rawtext":[{"selector":"@s"}]}"#,
            &[r#"title ActionBarMessageJson {"rawtext":[{"text":"Alice"}]} [-1, -1, -1]"#, r#"answer commands.titleraw.success ["Alice"] 1"#],
        ),
        ("playsound random.orb @s 21.25 72 205.25 2 0.5", &["sound random.orb [170, 576, 1642] 2 0.5", r#"answer commands.playsound.success.single ["random.orb", "Alice"] 1"#]),
        ("playsound random.orb @a 121.25 72 205.25", &["answer commands.playsound.failed [] 0"]),
        ("stopsound @s random.orb", &[r#"stop "random.orb" false"#, r#"answer commands.stopsound.success ["random.orb"] 1"#]),
        ("stopsound @s", &[r#"stop "" true"#, "answer commands.stopsound.success.all [] 1"]),
        ("particle minecraft:heart_particle 21.25 74 205.25", &[particle, r#"answer commands.spawnParticleEmitter.success ["minecraft:heart_particle"] 1"#]),
        ("playsound random.orb nobody", &["answer commands.generic.noTargetMatch [] 0"]),
    ];
    for (line, expected) in steps {
        assert_eq!(run(&mut alice, line).await, expected, "/{line}");
    }

    // The watcher was sent what was for everyone, and nothing that was Alice's alone.
    alice.client().command(r#"tellraw Bob {"rawtext":[{"text":"end"}]}"#);
    let end = r#"json {"rawtext":[{"text":"end"}]}"#;
    let watched = gather(&mut bob, |line| line == end).await;
    assert_eq!(watched, [r#"chat "" * Alice waves"#, hi, "title SetTitle Hello there [-1, -1, -1]", "title SetDurations  [5, 40, 10]", particle, end]);
    alice.disconnect().await;
    bob.disconnect().await;
    server.shutdown().await;
}
