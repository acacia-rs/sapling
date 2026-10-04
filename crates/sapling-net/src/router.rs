//! The directory: which shard owns each connection. Network-free. The network task hands every
//! event to the owner's inbox and moves a connection when its owner says so; the old owner gets
//! `Rerouted` behind the last event it was sent, so it knows what to forward (docs/DESIGN.md
//! "Step 5 as built").

use std::collections::HashMap;

use tokio::sync::mpsc;

use crate::event::{ConnId, NetEvent};

pub struct Router {
    inboxes: Vec<mpsc::UnboundedSender<NetEvent>>,
    routes: HashMap<ConnId, usize>,
    /// The shard new connections join.
    entry: usize,
}

impl Router {
    pub fn new(inboxes: Vec<mpsc::UnboundedSender<NetEvent>>, entry: usize) -> Self {
        assert!(entry < inboxes.len(), "the entry shard is one of the shards");
        Self { inboxes, routes: HashMap::new(), entry }
    }

    /// False once every shard dropped its inbox: nobody is left to play for.
    pub fn is_open(&self) -> bool {
        self.inboxes.iter().any(|inbox| !inbox.is_closed())
    }

    /// The shard a connection's events go to.
    pub fn owner(&self, conn: ConnId) -> Option<usize> {
        self.routes.get(&conn).copied()
    }

    fn send(&self, shard: usize, event: NetEvent) -> bool {
        self.inboxes.get(shard).is_some_and(|inbox| inbox.send(event).is_ok())
    }

    /// Hands an event to the shard that owns its connection. False when that shard is gone, or
    /// the connection is not known: the caller closes a connection nobody plays for.
    pub fn deliver(&mut self, event: NetEvent) -> bool {
        let shard = match &event {
            NetEvent::Joined { conn, .. } => {
                self.routes.insert(*conn, self.entry);
                Some(self.entry)
            }
            NetEvent::Packet { conn, .. } | NetEvent::Rerouted { conn } => self.owner(*conn),
            NetEvent::Left { conn, .. } => self.routes.remove(conn),
        };
        shard.is_some_and(|shard| self.send(shard, event))
    }

    /// Moves a connection from shard `from` to shard `to` and tells `from` it has seen the last
    /// of it. A connection that ended meanwhile is reported as left to `to`, which was promised
    /// the player. False when `to` is gone and the connection must be closed.
    pub fn reroute(&mut self, conn: ConnId, from: usize, to: usize) -> bool {
        let delivered = match self.routes.get_mut(&conn) {
            Some(owner) => {
                *owner = to;
                self.inboxes.get(to).is_some_and(|inbox| !inbox.is_closed())
            }
            None => {
                self.send(to, NetEvent::Left { conn, reason: "left during a transfer".into() });
                true
            }
        };
        self.send(from, NetEvent::Rerouted { conn });
        delivered
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn left(conn: u64) -> NetEvent {
        NetEvent::Left { conn: ConnId(conn), reason: String::new() }
    }

    fn joined(router: &mut Router, conn: u64) {
        // A `Joined` needs a verified login; the route is what these tests are about.
        router.routes.insert(ConnId(conn), router.entry);
    }

    #[test]
    fn a_rerouted_connection_reaches_its_new_shard_and_the_old_one_is_told_last() {
        let (a, mut inbox_a) = mpsc::unbounded_channel();
        let (b, mut inbox_b) = mpsc::unbounded_channel();
        let mut router = Router::new(vec![a, b], 0);
        joined(&mut router, 7);
        assert!(router.reroute(ConnId(7), 0, 1));
        assert!(matches!(inbox_a.try_recv(), Ok(NetEvent::Rerouted { conn }) if conn == ConnId(7)));
        assert_eq!(router.owner(ConnId(7)), Some(1));
        assert!(router.deliver(left(7)) && inbox_a.try_recv().is_err());
        assert!(matches!(inbox_b.try_recv(), Ok(NetEvent::Left { .. })));
        assert_eq!(router.owner(ConnId(7)), None);
    }

    #[test]
    fn a_connection_that_ended_before_its_reroute_is_reported_to_the_new_shard() {
        let (a, mut inbox_a) = mpsc::unbounded_channel();
        let (b, mut inbox_b) = mpsc::unbounded_channel();
        let mut router = Router::new(vec![a, b], 0);
        assert!(router.reroute(ConnId(3), 0, 1));
        assert!(matches!(inbox_b.try_recv(), Ok(NetEvent::Left { conn, .. }) if conn == ConnId(3)));
        assert!(matches!(inbox_a.try_recv(), Ok(NetEvent::Rerouted { .. })));
    }

    #[test]
    fn a_shard_that_is_gone_refuses_events_and_reroutes() {
        let (a, _inbox_a) = mpsc::unbounded_channel();
        let (b, inbox_b) = mpsc::unbounded_channel();
        let mut router = Router::new(vec![a, b], 1);
        joined(&mut router, 1);
        drop(inbox_b);
        assert!(router.is_open() && !router.deliver(NetEvent::Rerouted { conn: ConnId(1) }));
        router.routes.insert(ConnId(2), 0);
        assert!(!router.reroute(ConnId(2), 0, 1), "the caller closes the connection");
        assert!(!router.deliver(NetEvent::Rerouted { conn: ConnId(9) }), "unknown connections have no owner");
    }
}
