//! A skin changed in game: shown to everyone and to who joins later; refused by a plugin, or
//! malformed, nobody else sees it.

use crate::common;

use std::sync::{Arc, Mutex};

use acacia_proto::RawPacket;
use acacia_proto::packets::{PlayerList, PlayerSkin};
use acacia_proto::types::{PlayerRecordContent, Skin};
use common::{Harness, all};
use sapling_api::{Plugin, PluginBuilder, Priority, SkinChangeEvent};

/// Notes each change, and refuses it.
#[derive(Clone, Default)]
struct Veto(Arc<Mutex<Vec<String>>>);

impl Plugin for Veto {
    fn build(&self, plugin: &mut PluginBuilder<'_>) {
        let log = self.0.clone();
        plugin.on::<SkinChangeEvent>(Priority::Normal, move |_, event| {
            log.lock().unwrap().push(format!("{} {}", event.player().name, event.skin_id()));
            event.cancel();
        });
    }
}

/// The skin `name`'s list entry among `packets` gives it.
fn listed(packets: &[RawPacket], name: &str) -> Option<Skin> {
    all::<PlayerList>(packets).into_iter().flat_map(|list| list.records).find_map(|record| match record.content {
        PlayerRecordContent::Add(add) if add.username == name => Some(add.skin_data),
        _ => None,
    })
}

/// Alex and Steve in the world, and Alex's skin as Steve was shown it.
fn two(veto: Option<&Veto>) -> (Harness, Skin, acacia_proto::manual::Uuid) {
    let mut h = Harness::new();
    if let Some(veto) = veto {
        h.shard.add_plugin(veto);
    }
    h.enter(1, "Alex");
    let sent = h.enter(2, "Steve");
    let list = all::<PlayerList>(sent.to(2)).into_iter().flat_map(|list| list.records).find_map(|record| match record.content {
        PlayerRecordContent::Add(add) if add.username == "Alex" => Some((add.skin_data, add.uuid)),
        _ => None,
    });
    let (skin, uuid) = list.expect("Steve lists Alex");
    (h, skin, uuid)
}

fn change(uuid: acacia_proto::manual::Uuid, skin: Skin) -> PlayerSkin {
    PlayerSkin { uuid, skin, skin_name: "new".into(), old_skin_name: "old".into() }
}

#[test]
fn a_changed_skin_is_shown_to_everyone_and_kept_for_who_comes_later() {
    let (mut h, skin, uuid) = two(None);
    h.receive(1, &change(uuid, Skin { skin_id: "custom-1".into(), ..skin }));
    let sent = h.tick_all();
    for conn in [1, 2] {
        let shown = all::<PlayerSkin>(sent.to(conn));
        assert_eq!(shown.iter().map(|skin| (skin.uuid, skin.skin.skin_id.as_str())).collect::<Vec<_>>(), [(uuid, "custom-1")], "to {conn}");
    }
    let later = h.enter(3, "Kai");
    assert_eq!(listed(later.to(3), "Alex").map(|skin| skin.skin_id), Some("custom-1".into()));
}

#[test]
fn a_refused_or_malformed_skin_is_not_shown() {
    let veto = Veto::default();
    let (mut h, skin, uuid) = two(Some(&veto));
    let old = skin.skin_id.clone();
    h.receive(1, &change(uuid, Skin { skin_id: "custom-1".into(), ..skin.clone() }));
    let sent = h.tick_all();
    assert!(all::<PlayerSkin>(sent.to(2)).is_empty());
    assert_eq!(all::<PlayerSkin>(sent.to(1)).iter().map(|skin| skin.skin.skin_id.clone()).collect::<Vec<_>>(), [old], "Alex is shown its own again");
    assert_eq!(*veto.0.lock().unwrap(), ["Alex custom-1"]);

    let mut torn = skin;
    torn.skin_data.width += 1;
    h.receive(1, &change(uuid, torn));
    let sent = h.tick_all();
    assert!(all::<PlayerSkin>(sent.to(1)).is_empty() && all::<PlayerSkin>(sent.to(2)).is_empty());
    assert_eq!(veto.0.lock().unwrap().len(), 1, "a malformed skin is not offered");
}
