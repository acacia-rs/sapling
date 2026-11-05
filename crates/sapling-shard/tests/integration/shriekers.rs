//! A shrieker that can summon, stood on: its warnings and the warden (blocks/sculk/shrieks.rs),
//! against research/captures/bds-t2warden.txt.

use crate::common;

use acacia_proto::packets::{AddEntity, LevelEventGeneric, LevelSoundEvent, MobEffect};
use common::{Harness, all};
use sapling_api::Block;

/// `ParticlesSculkShriek` 2035, zigzagged.
const SHRIEKED: u32 = 2035 << 1;
const DARKNESS: i32 = 30;

#[test]
fn a_shrieker_stood_on_warns_three_times_200_ticks_apart_then_a_warden_emerges() {
    // The fourth answer's darkness comes twice, the shrieker's and the warden's, as on BDS.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_difficulty(sapling_api::Difficulty::Easy);
        assert!(ctx.set_block([0, 64, 0], &Block::new("sculk_shrieker").with("active=0,can_summon=1")));
    });
    let (mut seen, mut heard): (Vec<(usize, String)>, Vec<usize>) = (Vec::new(), Vec::new());
    for tick in 0..760 {
        let sent = h.tick_all().to(1).to_vec();
        seen.extend(all::<LevelEventGeneric>(&sent).iter().filter(|event| event.event_id == SHRIEKED).map(|_| (tick, "shriek".to_owned())));
        seen.extend(all::<MobEffect>(&sent).iter().filter(|effect| effect.runtime_entity_id == 1 && effect.effect_id == DARKNESS && effect.duration == 260).map(|_| (tick, "darkness".to_owned())));
        seen.extend(all::<LevelSoundEvent>(&sent).iter().filter(|sound| sound.sound_id.starts_with("nearby_")).map(|sound| (tick, sound.sound_id.clone())));
        heard.extend(all::<LevelSoundEvent>(&sent).iter().filter(|sound| sound.sound_id == "block.sculk_shrieker.shriek" && sound.position.y == 64.0).map(|_| tick));
        seen.extend(all::<AddEntity>(&sent).iter().filter(|added| added.entity_type == "minecraft:warden").map(|_| (tick, "warden".to_owned())));
        seen.extend(all::<LevelSoundEvent>(&sent).iter().filter(|sound| sound.sound_id == "agitated" && sound.entity_unique_id == -1).map(|_| (tick, "agitated".to_owned())));
        seen.extend(all::<LevelSoundEvent>(&sent).iter().filter(|sound| sound.sound_id == "listening" && sound.entity_type == "minecraft:warden").map(|_| (tick, "listening".to_owned())));
        assert!(!all::<LevelSoundEvent>(&sent).iter().any(|sound| sound.sound_id == "spawn" && sound.entity_type == "minecraft:warden"), "an emerging warden is not heard to spawn");
    }
    let names: Vec<&str> = seen.iter().map(|(_, name)| name.as_str()).collect();
    assert_eq!(names, ["shriek", "darkness", "nearby_close", "shriek", "darkness", "nearby_closer", "shriek", "darkness", "nearby_closest", "shriek", "darkness", "darkness", "warden", "agitated", "listening"], "{seen:?}");
    // bds-t2warden.txt: the warden, and the shrieker falling quiet, a tick after the darkness.
    assert_eq!(seen[12].0 - seen[10].0, 1, "{seen:?}");
    assert_eq!(seen[11].0, seen[10].0, "the warden's own darkness with the shrieker's: {seen:?}");
    let shrieks: Vec<usize> = seen.iter().filter(|(_, name)| name == "shriek").map(|(tick, _)| *tick).collect();
    assert!(shrieks.windows(2).all(|pair| pair[1] - pair[0] == 200), "{shrieks:?}");
    assert_eq!(heard, shrieks.iter().map(|tick| tick + 1).collect::<Vec<_>>(), "heard from the block's corner a tick on");
    assert_eq!((seen[1].0 - seen[0].0, seen[2].0 - seen[0].0), (90, 91), "darkness as the shriek ends, the warning a tick on");
}
