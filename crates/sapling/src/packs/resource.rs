//! The resource packs joining clients are sent (sapling-shard `packs.rs` serves them).

use std::collections::HashMap;
use std::path::Path;

use super::source::{self, Source};
use super::{PackError, world_packs};

/// Bytes 4 to 8 of an encrypted pack's `contents.json`.
const ENCRYPTED_MAGIC: [u8; 4] = [0xfc, 0xb9, 0xcf, 0x9b];

/// Resource packs for clients to download as they join, in the order of the stack (the first
/// on top), and how: required or not, with the keys of encrypted packs and where a CDN has them.
///
/// ```no_run
/// # fn run() -> Result<(), sapling::packs::PackError> {
/// let packs = sapling::packs::ResourcePacks::new().folder("resource_packs")?.required(true);
/// let builder = sapling::Server::builder().resource_packs(packs);
/// # Ok(()) }
/// ```
#[derive(Default)]
pub struct ResourcePacks {
    packs: Vec<Source>,
    required: bool,
    keys: HashMap<String, String>,
    urls: HashMap<String, String>,
}

impl ResourcePacks {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds one pack: a folder with a `manifest.json`, or a `.mcpack` or `.zip` of one. A pack
    /// added twice (by uuid) is kept once, where it was first added.
    pub fn pack(self, path: impl AsRef<Path>) -> Result<Self, PackError> {
        Ok(self.with(vec![source::open(path.as_ref())?]))
    }

    /// Adds every pack in `folder`, by file name.
    pub fn folder(self, folder: impl AsRef<Path>) -> Result<Self, PackError> {
        Ok(self.with(source::scan(folder.as_ref())?))
    }

    /// Adds the packs a world folder applies, as BDS does: those its `world_resource_packs.json`
    /// names, in its order, from the world's `resource_packs` folder or `packs`.
    pub fn world(self, world: impl AsRef<Path>, packs: impl AsRef<Path>) -> Result<Self, PackError> {
        Ok(self.with(world_packs(world.as_ref(), "world_resource_packs.json", "resource_packs", packs.as_ref())?))
    }

    fn with(mut self, packs: Vec<Source>) -> Self {
        for pack in packs {
            if !pack.manifest.has_module("resources") {
                tracing::warn!(pack = pack.manifest.id(), "not a resource pack: no resources module");
            } else if self.packs.iter().any(|kept| kept.manifest.uuid == pack.manifest.uuid) {
                tracing::warn!(pack = pack.manifest.id(), "resource pack added twice; the first is kept");
            } else {
                self.packs.push(pack);
            }
        }
        self
    }

    /// Whether a client must take the packs to join, BDS's `texturepack-required` (default: no).
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// The key the encrypted pack `uuid` was encrypted with; clients need it to read the pack.
    pub fn content_key(mut self, uuid: &str, key: impl Into<String>) -> Self {
        self.keys.insert(uuid.to_ascii_lowercase(), key.into());
        self
    }

    /// Where clients download the pack `uuid` instead of from the server.
    pub fn cdn_url(mut self, uuid: &str, url: impl Into<String>) -> Self {
        self.urls.insert(uuid.to_ascii_lowercase(), url.into());
        self
    }

    /// `<uuid>_<version>` of each pack, in stack order.
    pub fn ids(&self) -> Vec<String> {
        self.packs.iter().map(|pack| pack.manifest.id()).collect()
    }

    pub(crate) fn build(self) -> sapling_shard::ResourcePacks {
        let mut packs = Vec::with_capacity(self.packs.len());
        for source in self.packs {
            let (uuid, version) = (source.manifest.uuid.clone(), source.manifest.version.clone());
            let encrypted = source.file("contents.json").and_then(|contents| contents.get(4..8)).is_some_and(|magic| magic == ENCRYPTED_MAGIC);
            let key = self.keys.get(&uuid).cloned().unwrap_or_default();
            if encrypted && key.is_empty() {
                tracing::warn!(pack = source.manifest.id(), "encrypted resource pack without a content key: clients cannot read it");
            }
            let unpacked = source.files.iter().map(|(_, data)| data.len() as u64).sum();
            let pack = sapling_shard::ResourcePack::new(&uuid, &version, source.into_zip(), unpacked).expect("the manifest's uuid was checked");
            packs.push(pack.with_content_key(key).with_cdn_url(self.urls.get(&uuid).cloned().unwrap_or_default()));
        }
        sapling_shard::ResourcePacks { packs, required: self.required }
    }
}
