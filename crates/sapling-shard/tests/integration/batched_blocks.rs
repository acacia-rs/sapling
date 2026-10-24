//! Blocks a plugin sets in a batch, shown as BDS shows a `/fill` (research/captures/bds-batch.txt).

use crate::common;

use acacia_proto::packets::{UpdateBlock, UpdateSubchunkBlocks};
use common::{Harness, all};
use sapling_api::Block;

#[test]
fn a_batch_shows_a_sub_chunk_of_changes_in_one_packet_and_a_lone_change_alone() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| assert_eq!(ctx.batch_blocks(|ctx| (2..5).filter(|&x| ctx.update_block([x, 70, 3], &Block::new("stone"))).count()), 3));
    let sent = h.tick_all().to(1).to_vec();
    let batches = all::<UpdateSubchunkBlocks>(&sent);
    let shown: Vec<_> = batches.iter().map(|batch| (batch.blocks.len(), batch.blocks.iter().map(|block| block.flags).collect::<Vec<_>>())).collect();
    assert_eq!((shown, all::<UpdateBlock>(&sent).len()), (vec![(3, vec![2; 3])], 0));
    h.remote.run(|ctx| assert!(ctx.batch_blocks(|ctx| ctx.update_block([2, 70, 3], &Block::new("dirt")))));
    let sent = h.tick_all().to(1).to_vec();
    let alone: Vec<_> = all::<UpdateBlock>(&sent).iter().map(|update| (update.position.x, update.flags.0)).collect();
    assert_eq!((alone, all::<UpdateSubchunkBlocks>(&sent).len()), (vec![(2, 2)], 0));
}
