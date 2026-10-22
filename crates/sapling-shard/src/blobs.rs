//! The client blob cache (acacia `docs/research/blob-cache.md`): a client that reported the cache
//! gets hashes instead of section and biome bytes, says which hashes it lacks, and is sent those.
//! The bytes are the world's shared encodings; a player only holds references until it answers.

use std::collections::{HashMap, VecDeque};

use acacia_proto::packets::{ClientCacheBlobStatus, ClientCacheMissResponse, ClientCacheStatus};
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_world::Blob;

use crate::components::{Outgoing, PacketQueues, Settings};

/// Unanswered hashes past which a player gets no new columns: the client is not keeping up.
const BACKLOG: usize = 2048;
/// Unanswered hashes kept per player; the oldest are forgotten first.
const MAX_AWAITING: usize = 8192;

#[derive(Component, Default)]
pub(crate) struct BlobCache {
    enabled: bool,
    /// Blobs named to the client that it has not reported on yet.
    awaiting: HashMap<u64, Bytes>,
    order: VecDeque<u64>,
}

impl BlobCache {
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Whether blobs named to the client are still unanswered: sections it may not hold yet.
    pub fn awaits_any(&self) -> bool {
        !self.awaiting.is_empty()
    }

    pub fn is_backlogged(&self) -> bool {
        self.awaiting.len() >= BACKLOG
    }

    /// Lets go of every blob held for the client: they are of a world the player left. A status
    /// that still names one is answered without it.
    pub fn release(&mut self) {
        self.awaiting.clear();
        self.order.clear();
    }

    /// Remembers a blob whose hash is about to be sent.
    pub fn advertise(&mut self, blob: &Blob) {
        if self.awaiting.insert(blob.hash, blob.payload.clone()).is_none() {
            self.order.push_back(blob.hash);
        }
        while self.order.len() > MAX_AWAITING {
            let oldest = self.order.pop_front().expect("a non-empty queue");
            self.awaiting.remove(&oldest);
        }
    }

    /// The reply to a status: the missing blobs still held, in the order asked. Like BDS, one
    /// reply per status even when empty, and a blob is sent once: asked for again, it is left out.
    pub fn answer(&mut self, status: &ClientCacheBlobStatus) -> ClientCacheMissResponse {
        // Missing first: a hash named for several columns is listed missing once and, for its
        // other slots, under `have` in the same status.
        let take = |&hash: &u64| self.awaiting.remove(&hash).map(|payload| Blob { hash, payload });
        let blobs = status.missing.iter().filter_map(take).collect();
        for hash in &status.have {
            self.awaiting.remove(hash);
        }
        self.order.retain(|hash| self.awaiting.contains_key(hash));
        ClientCacheMissResponse { blobs }
    }
}

/// Whether the client keeps a blob cache; it says so once, right after login.
pub(crate) fn cache_status(mut packets: ResMut<PacketQueues>, settings: Res<Settings>, mut players: Query<&mut BlobCache>) {
    for (entity, status) in packets.take::<ClientCacheStatus>() {
        if let Ok(mut cache) = players.get_mut(entity) {
            cache.enabled = status.enabled && settings.0.blob_cache;
        }
    }
}

pub(crate) fn blob_status(mut packets: ResMut<PacketQueues>, mut players: Query<(&mut BlobCache, &mut Outgoing)>) {
    for (entity, status) in packets.take::<ClientCacheBlobStatus>() {
        let Ok((mut cache, mut out)) = players.get_mut(entity) else { continue };
        if cache.is_enabled() {
            out.push(&cache.answer(&status));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blob(hash: u64) -> Blob {
        Blob { hash, payload: Bytes::from(vec![hash as u8; 3]) }
    }

    #[test]
    fn missing_blobs_are_sent_once_and_held_ones_dropped() {
        let mut cache = BlobCache::default();
        [1, 2, 3, 2].iter().for_each(|&hash| cache.advertise(&blob(hash)));
        assert_eq!(cache.awaiting.len(), 3);

        let reply = cache.answer(&ClientCacheBlobStatus { missing: vec![3, 1, 9], have: vec![2, 2] });
        assert_eq!(reply.blobs, [blob(3), blob(1)]);
        assert!(cache.awaiting.is_empty() && cache.order.is_empty());
        cache.advertise(&blob(7));
        let both = cache.answer(&ClientCacheBlobStatus { missing: vec![7], have: vec![7, 7] });
        assert_eq!(both.blobs, [blob(7)], "missing in one slot and repeated under have");
        // Asked again (vanilla does, rarely): already sent, so left out; the reply still goes.
        assert!(cache.answer(&ClientCacheBlobStatus { missing: vec![3], have: vec![] }).blobs.is_empty());
    }

    #[test]
    fn a_silent_client_backs_up_then_loses_the_oldest() {
        let mut cache = BlobCache::default();
        (0..BACKLOG as u64 - 1).for_each(|hash| cache.advertise(&blob(hash)));
        assert!(!cache.is_backlogged());
        cache.advertise(&blob(u64::MAX));
        assert!(cache.is_backlogged());
        (BACKLOG as u64..MAX_AWAITING as u64 + 10).for_each(|hash| cache.advertise(&blob(hash)));
        assert_eq!((cache.awaiting.len(), cache.order.len()), (MAX_AWAITING, MAX_AWAITING));
        assert!(!cache.awaiting.contains_key(&0) && cache.awaiting.contains_key(&(MAX_AWAITING as u64)));
    }
}
