//! Custom content over a real connection: a plugin's item and block reach a joining client in
//! ItemRegistry and StartGame, the client downloads the pack the server built for them, and an
//! operator `/give`s the item.

use crate::common;

use std::sync::Arc;

use acacia_bot::proto::Packet;
use acacia_bot::proto::packets::{CommandOutput, ItemRegistry, ResourcePackChunkData, ResourcePackDataInfo, ResourcePacksInfo, StartGame};
use acacia_bot::{Bot, BotConfig};
use acacia_client::{Event, MemoryPackStore, PacketFilter};
use sapling::{Content, CustomBlock, CustomItem, Plugin, StateValues};
use sapling_api::PluginBuilder;

#[path = "common/travel.rs"]
mod travel;

use travel::{Operators, WAIT, run};

const RUBY: &str = "cc_test:ruby";
const LAMP: &str = "cc_test:lamp";

struct Gems;

impl Plugin for Gems {
    fn build(&self, _: &mut PluginBuilder<'_>) {}

    fn content(&self, content: &mut Content) {
        content.item(CustomItem::new(RUBY, "Ruby", b"ruby texture".to_vec()).max_stack(16));
        content.block(CustomBlock::new(LAMP, "Lamp", b"lamp texture".to_vec()).light_emission(12).state("cc_test:lit", StateValues::Bool));
    }
}

fn held(bot: &Bot, name: &str) -> u32 {
    let state = bot.state();
    state.inventory.main.iter().filter(|stack| !stack.is_empty() && state.items.name(stack.network_id) == Some(name)).map(|stack| u32::from(stack.count)).sum()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_client_is_told_the_custom_content_downloads_its_pack_and_is_given_the_item() {
    let builder = common::builder().handler_each(|| Operators).vanilla_commands().plugin(Gems);
    let server = common::start(builder).await;

    let client = common::client(&server, "Packer").filter(PacketFilter::all()).pack_store(Arc::new(MemoryPackStore::default()));
    let mut client = client.connect().await.expect("the client gets through the packs");
    let (mut info, mut data_info, mut chunks, mut start, mut items) = (None, None, Vec::new(), None, None);
    let joined = tokio::time::timeout(WAIT, async {
        while let Some(event) = client.recv().await {
            let Event::Packet(raw) = event else { continue };
            match raw.id {
                ResourcePacksInfo::ID => info = Some(raw.decode::<ResourcePacksInfo>().unwrap()),
                ResourcePackDataInfo::ID => data_info = Some(raw.decode::<ResourcePackDataInfo>().unwrap()),
                ResourcePackChunkData::ID => chunks.push(raw.decode::<ResourcePackChunkData>().unwrap()),
                StartGame::ID => start = Some(raw.decode::<StartGame>().unwrap()),
                ItemRegistry::ID => {
                    items = Some(raw.decode::<ItemRegistry>().unwrap());
                    return;
                }
                _ => {}
            }
        }
    });
    joined.await.expect("ItemRegistry after the packs");
    let (info, data_info) = (info.unwrap(), data_info.expect("the generated pack is downloaded"));
    assert_eq!(info.texture_packs.len(), 1, "the generated pack, the server having none of its own");
    assert_eq!(data_info.pack_id, format!("{}_{}", info.texture_packs[0].content_identity, info.texture_packs[0].version));
    chunks.sort_by_key(|chunk| chunk.chunk_index);
    let zip: Vec<u8> = chunks.iter().flat_map(|chunk| chunk.payload.to_vec()).collect();
    let has = |name: &str| zip.windows(name.len()).any(|window| window == name.as_bytes());
    assert!(has("textures/items/cc_test_ruby.png") && has("textures/item_texture.json") && has("texts/en_US.lang"), "the pack holds the item's texture and names");
    assert!(has("textures/blocks/cc_test_lamp.png") && has("textures/terrain_texture.json") && has("blocks.json"), "and the block's");

    let items = items.unwrap();
    let ruby = items.itemstates.iter().find(|item| item.name == RUBY).expect("the ruby in ItemRegistry");
    assert!(ruby.component_based);
    let _ = start.expect("StartGame");
    client.close();

    let config = BotConfig { subscribe: PacketFilter::none().with(CommandOutput::ID), ..BotConfig::default() };
    let mut alice = common::spawn(common::client(&server, "Alice"), config).await;
    alice.wait_ticks(20).await.expect("connected");
    assert_eq!(run(&mut alice, &format!("give @s {RUBY} 20")).await, ("commands.give.success".to_owned(), true));
    alice.wait_ticks(10).await.expect("connected");
    assert_eq!(held(&alice, RUBY), 20, "the bot read the ruby's id off ItemRegistry");
    alice.disconnect().await;
    server.shutdown().await;
}
