//! Emotes passed on to those who see the player, and a creative pick block
//! (research/captures/bds-packets.txt).

use crate::common;

use acacia_proto::manual::Uuid;
use acacia_proto::packets::{Emote, EmoteFlags, EmoteList};
use common::{Harness, all};

const WAVE: &str = "4c8ae710-df2e-47cd-814d-cc7bf21a3d67";

#[test]
fn an_emote_and_an_emote_list_reach_the_watchers_not_the_player() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    (0..3).for_each(|_| drop(h.tick_all()));
    h.receive(1, &Emote { entity_id: 99, emote_id: WAVE.into(), emote_length_ticks: 60, xuid: String::new(), platform_id: String::new(), flags: EmoteFlags::Unknown(0) });
    h.receive(1, &EmoteList { player_id: 99, emote_pieces: vec![WAVE.parse::<Uuid>().unwrap()] });
    let sent = h.tick_all();
    assert!(all::<Emote>(sent.to(1)).is_empty() && all::<EmoteList>(sent.to(1)).is_empty());
    let seen = all::<Emote>(sent.to(2));
    assert_eq!(seen.iter().map(|emote| (emote.entity_id, emote.emote_id.as_str(), emote.flags)).collect::<Vec<_>>(), [(1, WAVE, EmoteFlags::ServerSide)]);
    assert_eq!(all::<EmoteList>(sent.to(2)).iter().map(|list| list.player_id).collect::<Vec<_>>(), [1]);
}

/// The same capture: a creative pick of a log put it in the first empty hotbar slot and selected
/// it; a second pick kept the slot.
#[test]
fn a_creative_pick_block_puts_the_block_in_the_hotbar_and_selects_it() {
    use acacia_proto::packets::{BlockPickRequest, PlayerHotbar};
    use sapling_api::{Block, GameMode, Item, PlayerId, Slot};
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_game_mode(PlayerId(1), GameMode::Creative);
        assert!(ctx.set_item(PlayerId(1), Slot::Main(0), Some(&Item::new("minecraft:stone", 1))));
        assert!(ctx.set_block([2, 65, 0], &Block::new("minecraft:oak_log")));
    });
    h.tick(1);
    let pick = BlockPickRequest { x: 2, y: 65, z: 0, add_user_data: false, selected_slot: 0 };
    for _ in 0..2 {
        h.receive(1, &pick);
        let sent = h.tick(1);
        assert_eq!(all::<PlayerHotbar>(&sent).iter().map(|hotbar| (hotbar.selected_slot, hotbar.select_slot)).collect::<Vec<_>>(), [(1, true)]);
    }
    h.remote.run(|ctx| assert_eq!(ctx.item(PlayerId(1), Slot::Main(1)).map(|item| item.name), Some("minecraft:oak_log".to_owned())));
    h.tick(1);
}
