//! A wither's death, as BDS's in the `witherdeath` drill (research/captures/bds-witherdeath.txt):
//! it hangs dying for 200 ticks, counting `WitherInvulnerableTicks` down from 199 and flashing
//! `WitherAerialAttack`: first 15 ticks in, then at each tick that the last gap less one divides
//! (28, 39, 48 ... 81, then every tick). `death.min.volume` is heard as it dies, `death` 80 ticks
//! in and `death.mid.volume` 140 in. A tick before its end it blows up with power 7 2.7 over its
//! feet and shows its death again; at the end it leaves its loot (a nether star) and goes.

use acacia_proto::packets::{EntityEvent, EntityEventEventId, LevelSoundEvent};
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValueDefault as Plain, Vec3f};
use bevy_ecs::prelude::*;

use super::super::death;
use super::{BLAST_UP, KIND, Wither, blow_up, tell};
use crate::body::Body;
use crate::components::{InLayer, NetId, Tick};
use crate::entities;
use crate::tags::UniqueId;

/// Ticks from its death to its going.
const LASTS: u64 = 200;
/// The tick of its first flash.
const FIRST_FLASH: u32 = 15;
/// Its sounds and the ticks into its death they are heard.
const SOUNDS: [(u64, &str); 3] = [(0, "death.min.volume"), (80, "death"), (140, "death.mid.volume")];
/// Its sounds are heard this far over its feet.
const HEARD_UP: f32 = 2.55;

/// A wither dying: the tick it died in, the gap to its next flash and what it shows now.
#[derive(Component)]
struct Fading {
    died: u64,
    gap: u32,
    aerial: bool,
}

/// The mob died: a wither begins its long death; the ticks till it goes, or `None` for any other.
pub(in crate::creatures) fn dies(world: &mut World, mob: Entity, layer: Entity, body: &Body) -> Option<u64> {
    world.get::<Wither>(mob)?;
    let died = world.resource::<Tick>().0;
    world.entity_mut(mob).insert(Fading { died, gap: FIRST_FLASH, aerial: true });
    heard(world, mob, layer, body, SOUNDS[0].1);
    Some(LASTS)
}

fn heard(world: &mut World, wither: Entity, layer: Entity, body: &Body, name: &str) {
    let unique = world.get::<UniqueId>(wither).map_or(-1, |id| id.0);
    let [x, y, z] = body.at;
    let sound = LevelSoundEvent { sound_id: name.into(), position: Vec3f { x, y: y + HEARD_UP, z }, extra_data: -1, entity_type: KIND.into(), is_baby_mob: false, is_global: false, entity_unique_id: unique, fire_at_position: None };
    entities::tell_viewers(world, wither, layer, body.at.map(|c| c.floor() as i32), vec![sapling_net::encode(&sound)]);
}

/// A tick of each dying wither's death.
pub(super) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let dying: Vec<(Entity, Entity, u64, Body, u64)> = world.query::<(Entity, &InLayer, &NetId, &Body, &Fading)>().iter(world).map(|(wither, layer, id, body, fading)| (wither, layer.0, id.0, *body, now - fading.died)).collect();
    for (wither, layer, id, body, into) in dying {
        if into == 0 {
            continue;
        }
        if into == LASTS {
            death::leave_remains(world, wither, layer, KIND, &body);
            continue;
        }
        let mut shown = vec![(Key::WitherInvulnerableTicks, Plain::Int((LASTS - into) as i32))];
        let mut fading = world.get_mut::<Fading>(wither).expect("queried above");
        if into % u64::from(fading.gap) == 0 {
            fading.aerial = !fading.aerial;
            fading.gap = (fading.gap - 1).max(1);
            shown.push((Key::WitherAerialAttack, Plain::Short(i16::from(fading.aerial))));
        }
        tell(world, wither, layer, shown);
        if let Some((_, name)) = SOUNDS.iter().find(|(at, _)| *at == into) {
            heard(world, wither, layer, &body, name);
        }
        if into == LASTS - 1 {
            blow_up(world, wither, layer, [body.at[0], body.at[1] + BLAST_UP, body.at[2]]);
            let death = EntityEvent { runtime_entity_id: id, event_id: EntityEventEventId::DeathAnimation, data: 0, fire_at_position: None };
            entities::tell_viewers(world, wither, layer, body.at.map(|c| c.floor() as i32), vec![sapling_net::encode(&death)]);
            death::empty_hands(world, wither, layer, &body);
        }
    }
}
