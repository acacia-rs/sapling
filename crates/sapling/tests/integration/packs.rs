//! Packs over a real connection: a client downloads the server's resource pack chunk by chunk and
//! checks its hash before joining, and an operator runs a behavior pack's functions, schedules one
//! and raises a script event, answered as BDS answered (research/captures/bds-functions.txt).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use acacia_bot::proto::Packet;
use acacia_bot::proto::packets::{CommandOutput, ResourcePackChunkData, ResourcePackDataInfo, ResourcePackStack, ResourcePacksInfo, StartGame, Text, TextContent};
use acacia_bot::{Bot, BotConfig, BotEvent};
use acacia_client::{Event, MemoryPackStore, PacketFilter};
use sapling::packs::{BehaviorPacks, ResourcePacks};
use sapling::{Priority, ScriptEvent};
use sapling_api::{Plugin, PluginBuilder};
use sha2::{Digest, Sha256};

use crate::common;
#[path = "common/travel.rs"]
mod travel;

use travel::{Operators, WAIT};

const RESOURCE_UUID: &str = "5d9a0a52-5d2f-4b47-9a43-34c0e2b1f001";

/// A pack folder `name` under `root` holding `files` and a manifest of `uuid` with a module of `kind`.
fn pack(root: &Path, name: &str, uuid: &str, kind: &str, files: &[(&str, Vec<u8>)]) -> PathBuf {
    let dir = root.join(name);
    let manifest = format!(r#"{{"format_version":2,"header":{{"name":"{name}","uuid":"{uuid}","version":[1,0,0]}},"modules":[{{"type":"{kind}","uuid":"{uuid}","version":[1,0,0]}}]}}"#);
    for (path, data) in std::iter::once(("manifest.json", manifest.into_bytes())).chain(files.iter().cloned()) {
        let file = dir.join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, data).unwrap();
    }
    dir
}

/// Bytes deflate cannot shrink much, so that the pack spans several 100 KiB chunks.
fn noise(len: usize) -> Vec<u8> {
    let mut state = 0x2545_f491_u32;
    (0..len).map(|_| {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        state as u8
    }).collect()
}

/// Tells everyone each script event, so that the bot sees the listener heard it.
struct ScriptEcho;

impl Plugin for ScriptEcho {
    fn build(&self, plugin: &mut PluginBuilder<'_>) {
        plugin.on::<ScriptEvent>(Priority::Normal, |ctx, event| ctx.broadcast(&format!("script {} {} from {}", event.id, event.message, event.sender.name())));
    }
}

fn describe(event: BotEvent) -> Option<String> {
    let BotEvent::Packet(raw) = event else { return None };
    Some(match raw.id {
        CommandOutput::ID => {
            let answer: CommandOutput = raw.decode().expect("decodes");
            let lines: Vec<String> = answer.output.iter().map(|line| format!("{} {:?}", line.message_id, line.parameters)).collect();
            format!("answer {} {}", lines.join(" | "), answer.success_count)
        }
        Text::ID => match raw.decode::<Text>().expect("decodes").content {
            TextContent::Announcement(said) => format!("said {}", said.message),
            TextContent::JsonWhisper(json) => format!("json {}", json.message.trim_end()),
            TextContent::Raw(raw) => format!("raw {}", raw.message),
            _ => return None,
        },
        _ => return None,
    })
}

/// What the bot is told until a line `done` accepts, that line included.
async fn until(bot: &mut Bot, done: impl Fn(&str) -> bool) -> Vec<String> {
    let mut lines = Vec::new();
    let gathered = tokio::time::timeout(WAIT, async {
        loop {
            match bot.next().await {
                Some(BotEvent::Disconnected(reason)) => panic!("disconnected: {reason:?}"),
                Some(event) => lines.extend(describe(event)),
                None => panic!("the connection is gone"),
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
    until(bot, |line| line.starts_with("answer ")).await
}

/// The fake player `ticks`' score in `tk`, which the tick function adds one to.
async fn tick_runs(bot: &mut Bot) -> u32 {
    let ticks = run(bot, "scoreboard players list ticks").await;
    ticks[0].split('"').nth(5).and_then(|n| n.parse().ok()).unwrap_or_else(|| panic!("{ticks:?}"))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_client_downloads_the_packs_and_an_operator_runs_functions() {
    let root = std::env::temp_dir().join(format!("sapling-packs-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let resources = pack(&root, "probe_rp", RESOURCE_UUID, "resources", &[("padding.bin", noise(250_000))]);
    let functions: [(&str, Vec<u8>); 6] = [
        ("functions/probe/hello.mcfunction", b"# a comment\n\nsay hello\nscoreboard players add @s fn 1\n".to_vec()),
        ("functions/probe/nested.mcfunction", b"function probe/hello\nsay after\n".to_vec()),
        ("functions/probe/bad.mcfunction", b"notacommand\nsay bad\n".to_vec()),
        ("functions/probe/self.mcfunction", b"scoreboard players add @s loops 1\nfunction probe/self\n".to_vec()),
        ("functions/probe/later.mcfunction", b"say later\n".to_vec()),
        ("functions/tick.json", br#"{"values":["probe/tick"]}"#.to_vec()),
    ];
    let mut functions = functions.to_vec();
    functions.push(("functions/probe/tick.mcfunction", b"scoreboard players add ticks tk 1\n".to_vec()));
    let behavior = pack(&root, "probe_bp", "5d9a0a52-5d2f-4b47-9a43-34c0e2b1f003", "data", &functions);
    let resource_packs = ResourcePacks::new().pack(&resources).unwrap().required(true);
    let behavior_packs = BehaviorPacks::new().pack(&behavior).unwrap();
    std::fs::remove_dir_all(&root).unwrap();
    let builder = common::builder().handler(Operators).vanilla_commands().resource_packs(resource_packs).behavior_packs(behavior_packs).plugin(ScriptEcho);
    let server = common::start(builder).await;

    // A fresh client: told of the pack, it asks for it, every chunk at once, and checks the hash.
    let client = common::client(&server, "Packer").filter(PacketFilter::all()).pack_store(Arc::new(MemoryPackStore::default()));
    let mut client = client.connect().await.expect("the client gets through the packs");
    let (mut info, mut data_info, mut chunks, mut stack) = (None, None, Vec::new(), None);
    let joined = tokio::time::timeout(WAIT, async {
        while let Some(event) = client.recv().await {
            let Event::Packet(raw) = event else { continue };
            match raw.id {
                ResourcePacksInfo::ID => info = Some(raw.decode::<ResourcePacksInfo>().unwrap()),
                ResourcePackDataInfo::ID => data_info = Some(raw.decode::<ResourcePackDataInfo>().unwrap()),
                ResourcePackChunkData::ID => chunks.push(raw.decode::<ResourcePackChunkData>().unwrap()),
                ResourcePackStack::ID => stack = Some(raw.decode::<ResourcePackStack>().unwrap()),
                StartGame::ID => return,
                _ => {}
            }
        }
    });
    joined.await.expect("StartGame after the packs");
    let (info, data_info, stack) = (info.unwrap(), data_info.unwrap(), stack.unwrap());
    assert!(info.must_accept && stack.must_accept);
    assert_eq!((info.texture_packs.len(), info.texture_packs[0].content_identity.as_str()), (1, RESOURCE_UUID));
    assert_eq!((data_info.pack_id.as_str(), data_info.max_chunk_size), (format!("{RESOURCE_UUID}_1.0.0").as_str(), 102_400));
    assert_eq!(chunks.len() as u32, data_info.chunk_count);
    assert!(data_info.chunk_count >= 3, "{} chunks", data_info.chunk_count);
    chunks.sort_by_key(|chunk| chunk.chunk_index);
    assert!(chunks.iter().all(|chunk| chunk.progress == u64::from(chunk.chunk_index) * 102_400));
    let bytes: Vec<u8> = chunks.iter().flat_map(|chunk| chunk.payload.to_vec()).collect();
    assert_eq!((bytes.len() as u64, &Sha256::digest(&bytes)[..]), (data_info.size, &data_info.hash[..]));
    assert_eq!(stack.resource_packs.iter().map(|pack| pack.uuid.as_str()).collect::<Vec<_>>(), [RESOURCE_UUID]);
    client.close();

    let config = BotConfig { subscribe: [CommandOutput::ID, Text::ID].into_iter().fold(PacketFilter::none(), PacketFilter::with), ..BotConfig::default() };
    let mut alice = common::spawn(common::client(&server, "Alice"), config).await;
    alice.wait_ticks(20).await.expect("connected");
    let answered: [(&str, &[&str]); 7] = [
        ("scoreboard objectives add tk dummy", &[r#"answer commands.scoreboard.objectives.add.success ["tk"] 1"#]),
        ("function probe/hello", &["said [Alice] hello", r#"answer commands.function.success ["2"] 1"#]),
        ("function probe/nested", &["said [Alice] hello", "said [Alice] after", r#"answer commands.function.success ["4"] 1"#]),
        ("function probe/bad", &[r#"answer commands.function.functionNameNotFound ["probe/bad"] 0"#]),
        ("function PROBE/HELLO", &[r#"answer commands.function.functionNameNotFound ["PROBE/HELLO"] 0"#]),
        ("gamerule functioncommandlimit 5", &[r#"answer commands.gamerule.success ["functioncommandlimit", "5"] 1"#]),
        ("function probe/self", &[r#"answer commands.function.success ["5"] 1"#]),
    ];
    for (line, expected) in answered {
        assert_eq!(run(&mut alice, line).await, expected, "/{line}");
    }
    let before = tick_runs(&mut alice).await;
    alice.wait_ticks(20).await.expect("connected");
    let after = tick_runs(&mut alice).await;
    assert!(after - before >= 10, "the tick function ran {} times in 20 ticks", after - before);

    let queued = run(&mut alice, "schedule delay add probe/later 10").await;
    assert!(queued[0].starts_with(r#"answer commands.schedule.delay.functionQueued ["probe/later", "10", "#), "{queued:?}");
    let ran = until(&mut alice, |line| line.starts_with("json")).await;
    let echo = r#"json {"rawtext":[{"text":"§7§o["},{"translate":"commands.origin.server"},{"text":": "},{"translate":"commands.schedule.functionRan","with":{"rawtext":[{"text":"1"},{"text":"probe/later"}]}},{"text":"]"}]}"#;
    assert_eq!(ran, ["said [Server] later", echo]);
    assert_eq!(run(&mut alice, "schedule delay clear probe/later").await, [r#"answer commands.schedule.clear.failure ["probe/later"] 0"#]);

    assert_eq!(run(&mut alice, "scriptevent hi there").await, ["answer commands.scriptevent.invalidNamespace [] 0"]);
    assert_eq!(run(&mut alice, "scriptevent ns:hi hello world").await, [r#"answer Script event %%s has been sent ["ns:hi"] 1"#]);
    assert_eq!(until(&mut alice, |line| line.starts_with("raw script")).await, ["raw script ns:hi hello world from Alice"]);
    alice.disconnect().await;
    server.shutdown().await;
}
