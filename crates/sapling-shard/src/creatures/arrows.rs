//! What mobs shoot. Arrows, as BDS's skeleton shot them (docs/research/mob-attacks.md,
//! "Skeletons"): loosed at its prey, one flies by its own drag and weight, sticks in the block
//! it meets and hurts the player it meets, with its archer's effect if it has one. And a
//! blaze's small fireballs ("The blaze"), which fly straight, burn whom they hit and are gone
//! where they land. And a llama's spit ("Hit back by day"), which falls and throws nobody, and
//! a witch's potions ("The witch"), which break where they land (`splashes`).
// TODO: an arrow is not picked up, burns nothing and goes after a minute; a mob it hits is not
// thrown; its
// aim and its spread are the Java game's, fitted to no more than the speeds BDS showed.

use acacia_proto::packets::{EntityEvent, EntityEventEventId, LevelSoundEvent};
use acacia_proto::types::{MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain, MetadataFlags1, Vec3f};
use bevy_ecs::prelude::*;
use sapling_data::AddEntityOptions;

use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{InLayer, Layer, NetId, Position, Tick, in_world};
use crate::sync::Told;
use super::health::{Cause, Health};
use super::{Creature, Dying, harm};
use crate::{entities, late_sounds, mobs, vitals};

/// A kind of thing shot.
#[derive(Clone, Copy)]
pub(super) struct Shot {
    kind: &'static str,
    /// It leaves this far before its archer and this high up it, at this speed.
    ahead: f32,
    held: f32,
    speed: f32,
    /// It is aimed this high up its prey, raised by this much of the way there.
    aimed: f32,
    loft: f32,
    /// A tick keeps this much of its speed and adds this much fall.
    drag: f32,
    weight: f32,
    /// What it costs, or its speed twice over, rounded up.
    harm: Option<f32>,
    /// It stays in the block it meets, and it sets whom it hits on fire for this many ticks.
    sticks: bool,
    burns: u16,
    /// It throws the player it hits as a hit does.
    throws: bool,
    sound: &'static str,
    /// Each part of its speed is off by up to this.
    spread: f32,
    /// Its rise is this much a block of the way to its prey, less this, where it is not aimed.
    arc: Option<[f32; 2]>,
    /// A thrown potion's data value: it breaks where it lands (`splashes`).
    potion: Option<i16>,
}

const ARROW: Shot = Shot { kind: "minecraft:arrow", ahead: 1.0, held: 1.61, speed: 1.6, aimed: 0.6, loft: 0.2, drag: 0.99, weight: 0.05, harm: None, sticks: true, burns: 0, throws: true, sound: "bow", spread: SPREAD, arc: None, potion: None };
/// research/captures/bds-blaze.txt: from 1.4 up the blaze, 1.15 to 1.42 a tick and no slower
/// after; bds-kinds2.txt: 5 off the player it hit, which burnt five seconds.
const FIREBALL: Shot = Shot { kind: "minecraft:small_fireball", ahead: 0.0, held: 1.4, speed: 1.3, aimed: 0.9, loft: 0.0, drag: 1.0, weight: 0.0, harm: Some(5.0), sticks: false, burns: 100, sound: "shoot", ..ARROW };
/// research/captures/bds-provoke2.txt: from 1.58 up the llama at 1.5 a tick, falling 0.06 a tick
/// more; 1 off the player it hit, which it did not throw. Aimed as the Java game's.
const SPIT: Shot = Shot { kind: "minecraft:llama_spit", ahead: 0.0, held: 1.58, speed: 1.5, weight: 0.06, harm: Some(1.0), sticks: false, throws: false, sound: "shoot", ..ARROW };
/// research/captures/bds-witch.txt: from 1.415 up the witch and 1.15 before it at 0.75 a tick and
/// no spread, its rise fitted to three throws 4.3, 6.2 and 8.2 off.
const POTION: Shot = Shot { kind: "minecraft:splash_potion", ahead: 1.15, held: 1.415, speed: 0.75, sticks: false, sound: "throw", spread: 0.0, arc: Some([0.0748, 0.2607]), potion: Some(0), ..ARROW };

/// What a kind shoots, by the name of it in `mob-traits.tsv` (`shot=`); arrows where none is named.
pub(super) fn shot_named(name: Option<&str>) -> Shot {
    [FIREBALL, SPIT, POTION].into_iter().find(|shot| shot.kind.strip_prefix("minecraft:") == name).unwrap_or(ARROW)
}

impl Shot {
    pub(super) fn is_potion(&self) -> bool {
        self.potion.is_some()
    }

    /// The same, a thrown potion of the data value `data`.
    pub(super) fn of_potion(self, data: i16) -> Self {
        Self { potion: Some(data), ..self }
    }
}

const SIZE: [f32; 2] = [0.25, 0.25];
/// Each part of a shot's speed is off by up to this.
const SPREAD: f32 = 0.075;
/// An arrow costs its speed times this, rounded up.
const HARM: f32 = 2.0;
/// One that does not stick is gone after this many ticks in the air.
const FLIES: u64 = 100;
const PLAYER: [f32; 2] = [0.6, 1.8];
/// Ticks one lies stuck before it goes.
const LIES: u64 = 1200;
const SHAKES: i32 = 7;

#[derive(Component)]
pub(super) struct Arrow {
    /// The kind that shot it, named if it kills, and the mob itself, which it passes through.
    archer: String,
    by: Entity,
    /// The effect it gives the player or mob it hits, and for how many ticks (a stray's, a bogged's).
    gives: Option<(&'static str, u32)>,
    shot: Shot,
    born: u64,
    stuck: Option<u64>,
}

pub(super) fn sound(name: &str, [x, y, z]: [f32; 3], of: &str) -> bytes::Bytes {
    sapling_net::encode(&LevelSoundEvent { sound_id: name.into(), position: Vec3f { x, y, z }, extra_data: -1, entity_type: of.into(), is_baby_mob: false, is_global: false, entity_unique_id: -1, fire_at_position: None })
}

/// A mob of `kind` with the body `archer` looses a `shot` at the player whose feet are `prey`.
pub(super) fn loose(world: &mut World, layer: Entity, by: Entity, kind: &str, archer: &Body, prey: [f32; 3], gives: Option<(&'static str, u32)>, shot: Shot) {
    let (dx, dz) = (prey[0] - archer.at[0], prey[2] - archer.at[2]);
    let far = dx.hypot(dz).max(1e-4);
    let from = [archer.at[0] + dx / far * shot.ahead, archer.at[1] + shot.held, archer.at[2] + dz / far * shot.ahead];
    let aim = match shot.arc {
        Some([per_block, less]) => [dx / far, per_block * far - less, dz / far],
        None => [prey[0] - from[0], prey[1] + shot.aimed - from[1] + far * shot.loft, prey[2] - from[2]],
    };
    let long = aim.iter().map(|c| c * c).sum::<f32>().sqrt().max(1e-4);
    let mut dice = world.get_resource_or_insert_with(Dice::default);
    let motion = aim.map(|c| c / long * shot.speed + (dice.fraction() + dice.fraction() - 1.0) * shot.spread);
    let (yaw, pitch) = ((-motion[0]).atan2(motion[2]).to_degrees(), motion[1].atan2(motion[0].hypot(motion[2])).to_degrees());
    let body = Body::new(from, motion, SIZE);
    let announce = |id: u64| {
        let options = AddEntityOptions { id, identifier: shot.kind, feet: from, yaw, pitch, variant: 0, mark_variant: 0, color: 0, baby: false, flags: MetadataFlags1(0), health: None, name: "" };
        let mut shown = sapling_data::add_entity(&options);
        shown.velocity = Vec3f { x: motion[0], y: motion[1], z: motion[2] };
        if let Some(data) = shot.potion {
            // The recorded potion already carries one, of 0.
            shown.metadata.retain(|item| item.key != Key::PotionAuxValue);
            shown.metadata.push(MetadataDictionaryItem { key: Key::PotionAuxValue, r#type: Type::Short, legacy_type: 1, value: Value::Default(Plain::Short(data)) });
        }
        Some(vec![sapling_net::encode(&shown)])
    };
    let born = world.resource::<Tick>().0;
    let Some(arrow) = entities::spawn(world, layer, shot.kind, body, from, announce) else { return };
    world.entity_mut(arrow).insert(Arrow { archer: kind.to_owned(), by, gives, shot, born, stuck: None });
    let middle = [archer.at[0], archer.at[1] + archer.size[1] / 2.0, archer.at[2]];
    late_sounds::play_later(world, 0, layer, archer.at.map(|c| c.floor() as i32), sound(shot.sound, middle, kind));
}

/// The player in `layer` whose box the arrow's, its bottom middle at `at`, reaches into: BDS's
/// flew over a player 8 blocks off, a hand above its head.
fn met(world: &World, layer: Entity, at: [f32; 3]) -> Option<Entity> {
    in_world(world).into_iter().find(|player| {
        let Some(feet) = world.get::<Position>(*player).map(|position| position.feet) else { return false };
        let across = (PLAYER[0] + SIZE[0]) / 2.0;
        world.get::<InLayer>(*player).map(|layer| layer.0) == Some(layer) && vitals::is_mortal(world, *player) && (at[0] - feet[0]).abs() < across && (at[2] - feet[2]).abs() < across && at[1] + SIZE[1] > feet[1] && at[1] < feet[1] + PLAYER[1]
    })
}

/// The living mob of `layer` whose box the arrow's, its bottom middle at `at`, reaches into.
fn met_mob(world: &mut World, layer: Entity, by: Entity, at: [f32; 3]) -> Option<Entity> {
    let mut mobs = world.query_filtered::<(Entity, &InLayer, &Body), (With<Creature>, With<Health>, Without<Dying>)>();
    mobs.iter(world).filter(|(mob, ..)| *mob != by).find(|(_, in_layer, body)| {
        let across = (body.size[0] + SIZE[0]) / 2.0;
        in_layer.0 == layer && (at[0] - body.at[0]).abs() < across && (at[2] - body.at[2]).abs() < across && at[1] + SIZE[1] > body.at[1] && at[1] < body.at[1] + body.size[1]
    }).map(|(mob, ..)| mob)
}

/// A tick of one flying arrow: whether it is gone.
fn fly(world: &mut World, arrow: Entity, layer: Entity, id: u64, now: u64) -> Option<()> {
    let mut body = *world.get::<Body>(arrow)?;
    let (shot, born, by) = world.get::<Arrow>(arrow).map(|arrow| (arrow.shot, arrow.born, arrow.by))?;
    if !shot.sticks && now >= born + FLIES {
        mobs::remove(world, arrow);
        return None;
    }
    let motion = [body.motion[0] * shot.drag, body.motion[1] * shot.drag - shot.weight, body.motion[2] * shot.drag];
    let speed = motion.iter().map(|c| c * c).sum::<f32>().sqrt();
    // Looked at a quarter of a block at a time, so that it passes through nobody.
    let looks = (speed / 0.25).ceil().max(1.0) as usize;
    // The first it meets on its way, player or mob.
    let on_way = |look: usize| -> [f32; 3] { std::array::from_fn(|axis| body.at[axis] + motion[axis] * look as f32 / looks as f32) };
    let met = (1..=looks).find_map(|look| met(world, layer, on_way(look)).map(|player| (look, player, true)).or_else(|| met_mob(world, layer, by, on_way(look)).map(|mob| (look, mob, false))));
    let behind: [f32; 3] = std::array::from_fn(|axis| body.at[axis] - motion[axis]);
    if let (Some(data), Some((_, struck, _))) = (shot.potion, met) {
        super::splashes::burst(world, layer, body.at, behind, data, Some(struck));
        mobs::remove(world, arrow);
        return None;
    }
    if let Some((_, struck, is_player)) = met {
        let cost = shot.harm.unwrap_or((speed * HARM).ceil());
        match is_player {
            true => {
                let (archer, gives) = world.get::<Arrow>(arrow).map(|arrow| (arrow.archer.clone(), arrow.gives))?;
                // One proof against fire lost nothing to a fireball on BDS (bds-blaze.txt).
                let landed = !(shot.burns > 0 && vitals::is_fireproof(world, struck)) && vitals::shot(world, struck, cost, &archer, body.at, shot.throws);
                if let Some((effect, ticks)) = gives.filter(|_| landed) {
                    vitals::afflict(world, struck, effect, ticks);
                }
                if landed && shot.burns > 0 {
                    vitals::set_alight(world, struck, shot.burns);
                }
            }
            false => {
                let gives = world.get::<Arrow>(arrow).and_then(|arrow| arrow.gives);
                if harm::wound(world, struck, layer, cost, Cause::Projectile).is_some()
                    && let Some((effect, ticks)) = gives
                {
                    super::afflictions::afflict(world, struck, effect, ticks);
                }
            }
        }
        if shot.sticks {
            late_sounds::play_later(world, 1, layer, body.at.map(|c| c.floor() as i32), sound("bow.hit", body.at, ""));
        }
        mobs::remove(world, arrow);
        return None;
    }
    let before = body.at;
    body.motion = motion;
    body.shove(&world.get::<Layer>(layer)?.world, motion);
    let stopped = (0..3).any(|axis| (body.at[axis] - before[axis] - motion[axis]).abs() > 1e-4);
    if stopped && !shot.sticks {
        if let Some(data) = shot.potion {
            super::splashes::burst(world, layer, body.at, behind, data, None);
        }
        mobs::remove(world, arrow);
        return None;
    }
    if stopped {
        body.motion = [0.0; 3];
        world.get_mut::<Arrow>(arrow)?.stuck = Some(now);
    }
    *world.get_mut::<Body>(arrow)? = body;
    let mut packets = world.get_mut::<Told>(arrow)?.update(id, body.at, body.motion, stopped);
    if stopped {
        packets.push(sapling_net::encode(&EntityEvent { runtime_entity_id: id, event_id: EntityEventEventId::ArrowShake, data: SHAKES, fire_at_position: None }));
        packets.push(sound("bow.hit", body.at, ""));
    }
    entities::tell_viewers(world, arrow, layer, body.at.map(|c| c.floor() as i32), packets);
    Some(())
}

/// Moves every arrow a tick; one that lay long enough goes.
pub(super) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let arrows: Vec<(Entity, Entity, u64, Option<u64>)> = world.query::<(Entity, &InLayer, &NetId, &Arrow)>().iter(world).map(|(arrow, layer, id, state)| (arrow, layer.0, id.0, state.stuck)).collect();
    for (arrow, layer, id, stuck) in arrows {
        match stuck {
            Some(since) if now >= since + LIES => mobs::remove(world, arrow),
            Some(_) => {}
            None => drop(fly(world, arrow, layer, id, now)),
        }
    }
}
