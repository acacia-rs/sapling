//! Names the block states behind network hashes, as captures print them (`block_runtime_id: ...`).
//! `cargo run -p sapling --example block_names -- 2150698529 2273676414`
use acacia_world::BlockRegistry;

fn main() {
    let registry = BlockRegistry::vanilla();
    let wanted: Vec<u32> = std::env::args().skip(1).filter_map(|arg| arg.parse().ok()).collect();
    for hash in wanted {
        let state = (0..registry.len() as u32).find(|&id| registry.network_hash(id) == hash).and_then(|id| registry.get(id));
        match state {
            Some(state) => println!("{hash} {} [{}]", state.name, state.properties),
            None => println!("{hash} ?"),
        }
    }
}
