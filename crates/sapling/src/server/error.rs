#[derive(Debug, thiserror::Error)]
pub enum StartError {
    #[error("binding the server socket: {0}")]
    Bind(#[from] std::io::Error),
    #[cfg(feature = "online")]
    #[error(transparent)]
    Keys(#[from] sapling_net::KeyError),
    #[error("online mode needs the `online` feature, which fetches the login signing keys")]
    OnlineUnavailable,
    #[error("two worlds are named {0:?}")]
    DuplicateWorld(String),
    #[error("players are to join the world {0:?}, which the server does not have")]
    UnknownWorld(String),
    #[error("shard {0} has no world, though a later shard has")]
    EmptyShard(usize),
    #[error("a server of several shards needs a handler per shard: add handlers with `handler_each`")]
    HandlerPerShard,
    #[error("allow-list is on, but the server keeps no allowlist: add one with `access`")]
    NoAllowlist,
}
