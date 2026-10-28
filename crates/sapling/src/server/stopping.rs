//! `/stop`'s request to stop the server, which [`crate::Server::stop_requested`] waits for.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

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
}
