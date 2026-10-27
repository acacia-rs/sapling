//! A spectator takes up nothing lying about (research/captures/bds-spectate.txt).

use crate::common;

use acacia_proto::packets::TakeItemEntity;
use common::{Harness, all};
use sapling_api::{GameMode, Item, PlayerId};

const ALEX: PlayerId = PlayerId(1);

#[test]
fn a_spectator_leaves_items_where_they_lie_until_it_plays_again() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_game_mode(ALEX, GameMode::Spectator);
        assert!(ctx.drop_item(&Item::new("minecraft:stone", 3), [0.5, 65.0, 0.5]));
    });
    assert!((0..60).all(|_| all::<TakeItemEntity>(h.tick_all().to(1)).is_empty()), "not taken by a spectator");
    h.remote.run(|ctx| ctx.set_game_mode(ALEX, GameMode::Survival));
    assert!((0..5).any(|_| !all::<TakeItemEntity>(h.tick_all().to(1)).is_empty()), "taken in survival");
}
