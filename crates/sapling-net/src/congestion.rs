use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use crate::event::ConnId;

/// The connections whose unsent backlog went over [`crate::NetConfig::congested_bytes`] and has
/// not drained below [`crate::NetConfig::drained_bytes`] since. The listener keeps it; the game
/// reads it through [`crate::Outbox::is_congested`] to hold back what can wait.
#[derive(Debug, Clone, Default)]
pub struct Congestion(Arc<Mutex<HashSet<ConnId>>>);

impl Congestion {
    pub fn contains(&self, conn: ConnId) -> bool {
        self.0.lock().expect("congestion lock").contains(&conn)
    }

    /// Marks or clears `conn`: the listener as backlogs grow and drain, a test by hand.
    pub fn set(&self, conn: ConnId, congested: bool) {
        let mut congested_conns = self.0.lock().expect("congestion lock");
        if congested {
            congested_conns.insert(conn);
        } else {
            congested_conns.remove(&conn);
        }
    }
}
