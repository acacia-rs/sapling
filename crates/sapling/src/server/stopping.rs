//! `/stop`'s request to stop the server, which [`crate::Server::stop_requested`] waits for.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use sapling_shard::ShardHandle;
use tokio::sync::Notify;

use crate::Server;

/// Shared between the server and every shard's `/stop`.
#[derive(Clone, Default)]
pub(crate) struct StopRequest {
    asked: Arc<AtomicBool>,
    notify: Arc<Notify>,
}

impl StopRequest {
    pub fn ask(&self) {
        self.asked.store(true, Ordering::SeqCst);
        // A stored permit: a wait that starts later still wakes.
        self.notify.notify_one();
    }

    pub fn asked(&self) -> bool {
        self.asked.load(Ordering::SeqCst)
    }

    async fn wait(&self) {
        while !self.asked() {
            self.notify.notified().await;
        }
    }
}

impl Server {
    /// Resolves once `/stop` ran ([`crate::ServerBuilder::vanilla_commands`]); the server goes on
    /// until [`Server::shutdown`], so a binary waits on this beside Ctrl-C.
    pub async fn stop_requested(&self) {
        self.stop.wait().await;
    }

    /// Whether `/stop` ran.
    pub fn is_stop_requested(&self) -> bool {
        self.stop.asked()
    }

    /// Stops the game, disconnects every player and closes the socket, logging BDS's
    /// `Stopping server...` and `Quit correctly` (research/captures/bds-console-cmds.txt).
    pub async fn shutdown(self) {
        tracing::info!("Stopping server...");
        let Server { listener, mut shards, entry, watching, .. } = self;
        drop(watching);
        // The entry shard last: the others hand it their players' items as they stop.
        let last = shards.remove(entry);
        // Joining the game threads can take a tick; keep that off the async runtime.
        let _ = tokio::task::spawn_blocking(move || shards.into_iter().chain([last]).for_each(ShardHandle::stop)).await;
        listener.closed().await;
        tracing::info!("Quit correctly");
    }
}
