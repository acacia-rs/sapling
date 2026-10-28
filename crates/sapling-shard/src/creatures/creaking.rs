//! The creaking, as BDS's was in the `beasts` and `creaking` drills
//! (research/captures/bds-wander-findings.md, "Creaking"): it strolls until a survival player
//! looks at it, takes that player as its prey (`provoked` in mob-traits.tsv) and stands frozen,
//! heard `stare` every 3 ticks, while anyone looks at it; looked away from, it hunts its prey and
//! hits it every 2 seconds (hunting.rs).
//! One a heart called is tied to it (heart.rs).
// TODO: a look is the Java game's (`isLookingAtMe`: within 60 degrees of its middle, in sight);
// a summoned one is the same by day and by night (BDS's was drilled at both). Its prey let go is
// not told as BDS's was (`deactivate`, state neutral), BDS's Flags beside a freeze are not sent, and
// one shown anew has the flags it was announced with.

mod heart;

pub(crate) use heart::{arrived as hearts_arrived, noticed as heart_noticed};
pub(super) use heart::{shrugs_off, tick as tick_hearts};

use acacia_proto::types::{MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataFlags2};
use bevy_ecs::prelude::*;

use crate::body::Body;
use crate::components::{InLayer, Position, Tick, in_world};
use crate::{entities, late_sounds, vitals};
use super::{hunting, legs, properties};

const KIND: &str = "minecraft:creaking";
/// A look at its middle within this cosine, from up to this far, freezes it.
const LOOK: f32 = 0.5;
const SEEN_FROM: f32 = 16.0;
const STARES: u64 = 3;
/// Its stare is heard from its head.
const HEAD: f32 = 0.85;
/// Its `creaking_state` and extended flags as BDS told them with it (bds-creaking.txt): looked at
/// with the flag `OBSERVED`, looked away from without.
const OBSERVED_STATE: i32 = 1;
const UNOBSERVED: i32 = 2;
const CALM: i64 = 36028797052518400;
const OBSERVED: i64 = 1 << 54;

fn flags_extended(flags: i64) -> MetadataDictionaryItem {
    MetadataDictionaryItem { key: Key::FlagsExtended, r#type: Type::Long, legacy_type: 7, value: Value::FlagsExtended(MetadataFlags2(flags)) }
}

#[derive(Component, Default)]
pub(super) struct Creaking {
    /// The tick it froze in, while it is frozen.
    frozen: Option<u64>,
    bound: Option<heart::Bound>,
}

pub(super) fn arm(world: &mut World, entity: Entity, kind: &str) {
    if kind == KIND {
        world.entity_mut(entity).insert(Creaking::default());
    }
}

pub(super) fn is_frozen(world: &World, entity: Entity) -> bool {
    world.get::<Creaking>(entity).is_some_and(|creaking| creaking.frozen.is_some()) || heart::is_crumbling(world, entity)
}

/// Whether the player at `position` looks at the point `at`, `SEEN_FROM` off at most.
fn looks_at(position: &Position, at: [f32; 3]) -> bool {
    let eyes = [position.feet[0], position.feet[1] + sapling_data::EYE_HEIGHT, position.feet[2]];
    let towards: [f32; 3] = std::array::from_fn(|axis| at[axis] - eyes[axis]);
    let far = towards.iter().map(|c| c * c).sum::<f32>().sqrt();
    let (yaw, pitch) = (position.yaw.to_radians(), position.pitch.to_radians());
    let look = [-yaw.sin() * pitch.cos(), -pitch.sin(), yaw.cos() * pitch.cos()];
    far > 1e-4 && far <= SEEN_FROM && (0..3).map(|axis| look[axis] * towards[axis] / far).sum::<f32>() > LOOK
}

/// Before its move: looked at, it takes the looker as its prey if it has none and stands; looked
/// away from, it thaws and is steered on as any hunter. True while it stands frozen.
pub(super) fn steer(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str) -> bool {
    let (Some(creaking), Some(body)) = (world.get::<Creaking>(entity), world.get::<Body>(entity).copied()) else { return false };
    let frozen = creaking.frozen;
    if heart::is_crumbling(world, entity) {
        legs::rest(world, entity);
        return true;
    }
    let now = world.resource::<Tick>().0;
    let middle = [body.at[0], body.at[1] + body.size[1] / 2.0, body.at[2]];
    let looker = in_world(world).into_iter().find(|player| {
        world.get::<InLayer>(*player).map(|layer| layer.0) == Some(layer)
            && vitals::is_mortal(world, *player)
            && world.get::<Position>(*player).is_some_and(|position| looks_at(position, middle) && hunting::in_sight(world, layer, &body, position.feet))
    });
    let cell = body.at.map(|c| c.floor() as i32);
    let Some(player) = looker else {
        if frozen.is_some() {
            entities::tell_viewers(world, entity, layer, cell, vec![hunting::sound("unfreeze", id, kind, &body, 0.0)]);
            world.get_mut::<Creaking>(entity).expect("read above").frozen = None;
            properties::set(world, entity, heart::STATE, UNOBSERVED, vec![flags_extended(CALM)]);
        }
        return false;
    };
    let since = frozen.unwrap_or(now);
    if frozen.is_none() {
        let woke = hunting::prey_at(world, entity).is_none();
        if woke {
            hunting::provoke(world, entity, layer, id, kind, &body, player);
        }
        late_sounds::play_later(world, 1, layer, cell, hunting::sound(if woke { "activate" } else { "freeze" }, id, kind, &body, 0.0));
        world.get_mut::<Creaking>(entity).expect("read above").frozen = Some(now);
        let mut told = vec![flags_extended(CALM | OBSERVED)];
        told.extend(woke.then(|| properties::ambient_sound("undefined")));
        properties::set(world, entity, heart::STATE, OBSERVED_STATE, told);
    }
    if (now - since).is_multiple_of(STARES) {
        late_sounds::play_later(world, 1, layer, cell, hunting::sound("stare", id, kind, &body, HEAD));
    }
    legs::rest(world, entity);
    true
}
