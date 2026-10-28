//! Custom content on the builder: the items and blocks plugins add ([`sapling_api::Plugin::content`]),
//! registered for the whole process before the shards start.

use std::sync::Arc;

use sapling_api::{Content, Plugin};
use sapling_shard::{ResourcePack, ResourcePacks};

use crate::packs::generated;

/// What every plugin adds, in the order they were added.
pub(super) fn gather(plugins: &[Arc<dyn Plugin>]) -> Content {
    let mut content = Content::default();
    plugins.iter().for_each(|plugin| plugin.content(&mut content));
    content
}

/// `packs` with the pack `content` needs at the bottom of the stack, under the server's own.
pub(super) fn with_generated_pack(packs: &Arc<ResourcePacks>, content: &Content) -> Arc<ResourcePacks> {
    let Some(pack) = generated::build(content) else { return packs.clone() };
    tracing::info!(version = %pack.version, items = content.items.len(), blocks = content.blocks.len(), "built the custom content's resource pack");
    let mut packs = (**packs).clone();
    packs.packs.retain(|kept| !kept.uuid().eq_ignore_ascii_case(pack.uuid));
    packs.packs.push(ResourcePack::new(pack.uuid, &pack.version, pack.zip, pack.unpacked).expect("a fixed uuid"));
    Arc::new(packs)
}

/// Gives each item its network id; a name vanilla has is left out, with a warning.
pub(super) fn register(content: &Content) {
    for item in &content.items {
        if sapling_data::register_item(item).is_none() {
            tracing::warn!(item = %item.identifier, "a custom item may not take a vanilla name; left out");
        }
    }
    for block in &content.blocks {
        if !sapling_shard::register_custom_block(block) {
            tracing::warn!(block = %block.identifier, "a custom block may not take a vanilla name; left out");
        }
    }
}
