use acacia_auth::ClientData;
use acacia_proto::packets::PlayerList;
use acacia_proto::types::PlayerRecordContent;
use acacia_proto::{GAME_VERSION, Packet, RawPacket};
use acacia_testserver::{Script, Step};

use super::*;

/// The skin BDS sent back for the bot of acacia-testserver's recording.
fn recorded() -> Skin {
    let list = Script::bds_spawn().steps.into_iter().find_map(|step| match step {
        Step::Send { packet, .. } => RawPacket::parse(packet).ok().filter(|p| p.id == PlayerList::ID),
        Step::Wait(_) => None,
    });
    let list: PlayerList = list.expect("a recorded PlayerList").decode().unwrap();
    match list.records.into_iter().next().map(|r| r.content) {
        Some(PlayerRecordContent::Add(add)) => add.skin_data,
        other => panic!("the recording starts with {other:?}"),
    }
}

fn claims(name: &str) -> Claims {
    ClientData::default_for(name, "127.0.0.1:19132", GAME_VERSION).skin
}

#[test]
fn claims_convert_as_bds_converts_them() {
    let (ours, theirs) = (convert(&claims("CacheBot")).expect("a valid skin"), recorded());
    assert_eq!(ours.skin_id, theirs.skin_id, "the recording's bot wore another skin: re-record or rename");
    let geometry = |skin: &Skin| serde_json::from_str::<Value>(&skin.geometry_data).unwrap();
    // The same JSON, not the same text: BDS pads fractions (0.082031250), serde_json does not.
    assert_eq!(geometry(&ours), geometry(&theirs));
    assert!(!ours.geometry_data.contains(char::is_whitespace) && ours.geometry_data.starts_with(&theirs.geometry_data[..40]));
    assert_eq!(Skin { geometry_data: String::new(), ..ours }, Skin { geometry_data: String::new(), ..theirs });
}

#[test]
fn a_skin_that_does_not_hold_together_is_replaced() {
    let good = claims("CacheBot");
    assert_ne!(wire_skin(&good).skin_id, plain().skin_id);
    let broken: [fn(&mut Claims); 6] = [
        |c| c.skin_image_width *= 2,
        |c| c.skin_data = "not base64!".into(),
        |c| c.skin_geometry = STANDARD.encode("{ not json"),
        |c| c.skin_id = String::new(),
        |c| (c.cape_image_width, c.cape_image_height) = (64, 32),
        |c| c.persona_pieces.push(serde_json::json!({ "PieceType": "persona_tail" })),
    ];
    for (i, break_it) in broken.iter().enumerate() {
        let mut claims = good.clone();
        break_it(&mut claims);
        assert_eq!(wire_skin(&claims), plain(), "case {i}");
    }
    let plain = plain();
    assert_eq!(plain.skin_data.data.len(), 64 * 64 * 4);
}

#[test]
fn colours_keep_their_bytes_in_wire_order() {
    assert_eq!(colour("#ff38281b"), 455_620_863);
    assert_eq!((colour("#0"), colour(""), colour("#zz")), (0, 0, 0));
}
