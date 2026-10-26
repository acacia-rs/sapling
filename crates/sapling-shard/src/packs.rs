//! Resource packs sent to clients as they join, in BDS's shapes (research/captures/bds-packs.txt):
//! ResourcePacksInfo lists them, a client asks for those it lacks (SendPacks), gets each one's
//! ResourcePackDataInfo and asks for every chunk, and once it has all it is sent the stack.

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use acacia_proto::manual::Uuid;
use acacia_proto::packets::{
    ResourcePackChunkData, ResourcePackChunkRequest, ResourcePackDataInfo, ResourcePackDataInfoPackType, ResourcePackStack, ResourcePacksInfo,
    ResourcePacksInfoWorldTemplate,
};
use acacia_proto::types::{ResourcePackIdVersionsItem, TexturePackInfosItem};
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sha2::{Digest, Sha256};

use crate::components::{Conn, Net, Outgoing, PacketQueues, Settings, Stage};

/// BDS's chunk: 100 KiB.
pub const CHUNK_SIZE: usize = 102_400;
/// Chunks sent to one player a tick while its connection keeps up (~4 MB/s).
const CHUNKS_PER_TICK: usize = 2;

/// A resource pack as clients download it: one zip.
#[derive(Clone)]
pub struct ResourcePack {
    uuid: String,
    version: String,
    archive: Bytes,
    sha256: [u8; 32],
    /// What ResourcePacksInfo gives as the pack's size: its files unpacked.
    unpacked: u64,
    content_key: String,
    cdn_url: String,
}

impl fmt::Debug for ResourcePack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResourcePack").field("id", &self.id()).field("bytes", &self.archive.len()).field("encrypted", &!self.content_key.is_empty()).field("cdn_url", &self.cdn_url).finish()
    }
}

impl ResourcePack {
    /// The pack `uuid` (lowercase) at `version` (`1.0.0`), as the zip `archive` whose files come to
    /// `unpacked` bytes. `None` for a uuid that is not one.
    pub fn new(uuid: &str, version: &str, archive: impl Into<Bytes>, unpacked: u64) -> Option<Self> {
        uuid.parse::<Uuid>().ok()?;
        let archive = archive.into();
        let sha256 = Sha256::digest(&archive).into();
        Some(Self { uuid: uuid.to_ascii_lowercase(), version: version.to_owned(), archive, sha256, unpacked, content_key: String::new(), cdn_url: String::new() })
    }

    /// The key an encrypted pack's `contents.json` was encrypted with, which clients need to read it.
    pub fn with_content_key(mut self, key: impl Into<String>) -> Self {
        self.content_key = key.into();
        self
    }

    /// Where clients download the pack instead of asking the server for it.
    pub fn with_cdn_url(mut self, url: impl Into<String>) -> Self {
        self.cdn_url = url.into();
        self
    }

    /// `<uuid>_<version>`, as clients ask for it.
    pub fn id(&self) -> String {
        format!("{}_{}", self.uuid, self.version)
    }

    pub fn uuid(&self) -> &str {
        &self.uuid
    }

    pub fn sha256(&self) -> [u8; 32] {
        self.sha256
    }

    fn chunk_count(&self) -> usize {
        self.archive.len().div_ceil(CHUNK_SIZE)
    }

    fn info(&self) -> TexturePackInfosItem {
        TexturePackInfosItem {
            uuid: self.uuid.parse().expect("checked in new"),
            version: self.version.clone(),
            size: self.unpacked,
            content_key: self.content_key.clone(),
            sub_pack_name: String::new(),
            // BDS names every pack's uuid here, encrypted or not.
            content_identity: self.uuid.clone(),
            has_scripts: false,
            addon_pack: false,
            rtx_enabled: false,
            cdn_url: self.cdn_url.clone(),
        }
    }
}

/// The packs every client is sent on joining, in the order of the stack: the first is on top.
#[derive(Debug, Clone, Default)]
pub struct ResourcePacks {
    pub packs: Vec<ResourcePack>,
    /// BDS's `texturepack-required`: a client that refuses them is disconnected.
    pub required: bool,
}

impl ResourcePacks {
    /// The pack a client names by `<uuid>_<version>` or by its uuid alone.
    fn find(&self, id: &str) -> Option<&ResourcePack> {
        let uuid = id.split_once('_').map_or(id, |(uuid, _)| uuid);
        self.packs.iter().find(|pack| pack.id() == id).or_else(|| self.packs.iter().find(|pack| pack.uuid.eq_ignore_ascii_case(uuid)))
    }

    /// The first packet after login.
    pub(crate) fn info(&self) -> ResourcePacksInfo {
        ResourcePacksInfo {
            must_accept: self.required && !self.packs.is_empty(),
            has_addons: false,
            has_scripts: false,
            disable_vibrant_visuals: false,
            world_template: ResourcePacksInfoWorldTemplate { uuid: Uuid::default(), version: "0.0.0".into() },
            texture_packs: self.packs.iter().map(ResourcePack::info).collect(),
        }
    }

    /// The stack, without the packs of a client that refused them.
    pub(crate) fn stack(&self, accepted: bool) -> ResourcePackStack {
        let listed = self.packs.iter().filter(|_| accepted);
        ResourcePackStack {
            must_accept: self.required && !self.packs.is_empty(),
            resource_packs: listed.map(|pack| ResourcePackIdVersionsItem { uuid: pack.uuid.clone(), version: pack.version.clone(), name: String::new() }).collect(),
            game_version: "*".into(),
            experiments: Vec::new(),
            experiments_previously_used: false,
            has_editor_packs: false,
        }
    }

    /// The data info of each pack `ids` asks for that the server has.
    pub(crate) fn data_infos(&self, ids: &[String]) -> Vec<ResourcePackDataInfo> {
        let data_info = |pack: &ResourcePack| ResourcePackDataInfo {
            pack_id: pack.id(),
            max_chunk_size: CHUNK_SIZE as u32,
            chunk_count: pack.chunk_count() as u32,
            size: pack.archive.len() as u64,
            hash: Bytes::copy_from_slice(&pack.sha256),
            is_premium: false,
            pack_type: ResourcePackDataInfoPackType::Resources,
        };
        ids.iter().filter_map(|id| self.find(id)).map(data_info).collect()
    }

    /// Chunk `index` of the pack `id`; `None` for a pack or chunk there is not.
    fn chunk(&self, id: &str, index: u32) -> Option<ResourcePackChunkData> {
        let pack = self.find(id)?;
        let start = index as usize * CHUNK_SIZE;
        if start >= pack.archive.len() {
            return None;
        }
        let payload = pack.archive.slice(start..(start + CHUNK_SIZE).min(pack.archive.len()));
        Some(ResourcePackChunkData { pack_id: pack.id(), chunk_index: index, progress: start as u64, payload })
    }
}

/// The chunks a joining player asked for and has not been sent yet, oldest first.
#[derive(Component, Default)]
pub(crate) struct Downloads(Vec<ResourcePackChunkRequest>);

/// Queues chunk requests of players still at the pack stage, and sends a few a tick to each whose
/// connection is not backed up: a pack is megabytes, and past 16 MiB unsent a player is kicked.
pub(crate) fn serve_chunks(mut commands: Commands, mut packets: ResMut<PacketQueues>, settings: Res<Settings>, net: Res<Net>, mut players: Query<(&Conn, &Stage, Option<&mut Downloads>, &mut Outgoing)>) {
    let mut asked: HashMap<Entity, Vec<ResourcePackChunkRequest>> = HashMap::new();
    for (entity, request) in packets.take::<ResourcePackChunkRequest>() {
        asked.entry(entity).or_default().push(request);
    }
    for (entity, requests) in asked {
        let Ok((_, Stage::PacksInfo, downloads, _)) = players.get_mut(entity) else { continue };
        match downloads {
            Some(mut downloads) => downloads.0.extend(requests),
            None => {
                commands.entity(entity).insert(Downloads(requests));
            }
        }
    }
    let packs: &Arc<ResourcePacks> = &settings.0.resource_packs;
    for (conn, stage, downloads, mut out) in &mut players {
        let Some(mut downloads) = downloads.filter(|d| !d.0.is_empty()) else { continue };
        if *stage != Stage::PacksInfo {
            downloads.0.clear();
            continue;
        }
        if net.0.is_congested(conn.0) {
            continue;
        }
        let due = downloads.0.len().min(CHUNKS_PER_TICK);
        for request in downloads.0.drain(..due) {
            match packs.chunk(&request.pack_id, request.chunk_index) {
                Some(chunk) => out.push(&chunk),
                None => tracing::debug!(pack = request.pack_id, chunk = request.chunk_index, "a chunk of no pack asked for"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const UUID: &str = "5d9a0a52-5d2f-4b47-9a43-34c0e2b1f001";

    #[test]
    fn a_pack_is_cut_into_bds_chunks() {
        let pack = ResourcePack::new(UUID, "1.0.0", vec![7u8; CHUNK_SIZE * 2 + 5], 9).unwrap();
        let packs = ResourcePacks { packs: vec![pack], required: true };
        let infos = packs.data_infos(&[format!("{UUID}_1.0.0"), "nope_1.0.0".into()]);
        assert_eq!((infos.len(), infos[0].chunk_count, infos[0].size), (1, 3, CHUNK_SIZE as u64 * 2 + 5));
        let last = packs.chunk(UUID, 2).unwrap();
        assert_eq!((last.progress, last.payload.len()), (CHUNK_SIZE as u64 * 2, 5));
        assert!(packs.chunk(UUID, 3).is_none());
        assert_eq!(packs.info().texture_packs[0].size, 9);
        assert!(ResourcePack::new("not-a-uuid", "1.0.0", Vec::new(), 0).is_none());
    }
}
