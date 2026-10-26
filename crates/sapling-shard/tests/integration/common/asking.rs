//! Questions put to the shard through its context, answered by the next tick.

use std::sync::{Arc, Mutex};

use sapling_api::ShardCtx;

use super::Harness;

/// What `ask` makes of the context in the next tick.
pub fn ask<T: Send + 'static>(h: &mut Harness, ask: impl FnOnce(&mut ShardCtx<'_>) -> T + Send + 'static) -> T {
    let found = Arc::new(Mutex::new(None));
    let out = found.clone();
    h.remote.run(move |ctx| *out.lock().unwrap() = Some(ask(ctx)));
    h.tick_all();
    found.lock().unwrap().take().expect("the handler ran")
}
