//! What the others hear of a player's landing, a tick after it, as on BDS (research/captures/bds-hurt.txt,
//! bds-around.txt).

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::LevelSoundEvent;
use acacia_proto::types::InputData;
use common::{Harness, all, input_at};
use sapling_api::{GameMode, PlayerId};

fn ready() -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    h.remote.run(|ctx| ctx.set_game_mode(PlayerId(1), GameMode::Survival));
    h.tick_all();
    h
}

/// Alex comes down to 65 from each of `heights` in turn; what each player heard of it then and a tick later.
fn land(h: &mut Harness, heights: &[f32]) -> [Vec<(String, f32)>; 2] {
    for &height in heights {
        h.receive(1, &input_at([0.5, height, 0.5]));
        h.tick_all();
    }
    let mut landing = input_at([0.5, 65.0, 0.5]);
    (landing.input_data, landing.delta.y) = (vec![InputData::VerticalCollision], -1.0);
    h.receive(1, &landing);
    let sent: Vec<_> = (0..2).map(|_| h.tick_all()).collect();
    let heard = |player: u64| -> Vec<(String, f32)> {
        let packets: Vec<RawPacket> = sent.iter().flat_map(|sent| sent.to(player).to_vec()).collect();
        all::<LevelSoundEvent>(&packets).into_iter().filter(|sound| sound.entity_type == "minecraft:player").map(|sound| (sound.sound_id, sound.position.y)).collect()
    };
    [heard(1), heard(2)]
}

#[test]
fn others_hear_a_hurting_fall_from_where_its_last_tick_began_and_the_faller_does_not() {
    let mut h = ready();
    let [alex, steve] = land(&mut h, &[73.0, 77.0, 73.0, 69.0]);
    assert_eq!((alex, steve), (vec![], vec![("fall.big".to_owned(), 69.0), ("fall".to_owned(), 69.0)]));
}

#[test]
fn others_hear_a_hop_land() {
    let mut h = ready();
    let [alex, steve] = land(&mut h, &[65.5]);
    assert_eq!((alex, steve), (vec![], vec![("land".to_owned(), 65.5)]));
}
