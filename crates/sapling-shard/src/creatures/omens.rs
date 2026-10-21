//! What the four newest effects do, as BDS's zombies and players were drilled
//! (research/captures/bds-effects3-findings.md, bds-wind-burst.txt, bds-player-omens-findings.md),
//! from a body's position (a mob's feet, a player's eyes): one that dies wind charged bursts at
//! its middle and pushes every living player by about `0.82 (1 - d / 9.8)` towards its eyes, d
//! from the position; one that dies weaving leaves 2 or 3 cobwebs about it, by the Java game's
//! rule; one that dies oozing leaves two slimes of the second size half a block up; one hurt while
//! infested lets out one or two silverfish a time in ten, at 0.3 within 90 degrees of its facing.
// TODO: the burst's fall-off is fitted to eight noisy samples and it pushes no mob or item; a
// slime of the second size does not split when it dies.

use acacia_proto::packets::{LevelEvent, LevelEventEvent, SetEntityData};
use acacia_proto::types::{EntityProperties, MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain, Vec3f};
use bevy_ecs::prelude::*;
use sapling_api::{Block, Effect};

use super::health::Health;
use super::{afflictions, arrows, legs};
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{InLayer, Layer, NetId, Position, in_world};
use crate::{blocks, entities, late_sounds, vitals};

/// The burst's push at its position, how far it reaches, and the height it pushes towards.
const PUSH: f32 = 0.82;
const REACH: f32 = 9.8;
const EYES: f32 = 1.62;
/// The burst's particle data for a mob.
const MOB_FLASH: i32 = 4;
/// Cells weaving tries, and the cobwebs it leaves: the first, and one more a time in two.
const WEB_TRIES: usize = 15;
/// Bedrock's name for a cobweb.
const WEB: &str = "minecraft:web";
const WEBS: usize = 2;
/// Oozing's slimes: how many, their size and health, and how far up their feet are.
const SLIMES: usize = 2;
const SLIME_SIZE: i32 = 2;
const SLIME_BOX: f32 = 1.04;
const SLIME_HEALTH: f32 = 4.0;
const SLIME_UP: f32 = 0.5;
/// Infested lets out silverfish this often a hurt, one and one more a time in two, at this speed,
/// this far either side of the body's facing.
const SILVERFISH_CHANCE: f32 = 0.1;
const SILVERFISH: u32 = 1;
const SILVERFISH_SPEED: f32 = 0.3;
const SILVERFISH_SPREAD: f32 = 90.0;

/// Which of the effects that act at death a dying body has.
#[derive(Clone, Copy)]
pub(crate) struct Omens {
    pub wind: bool,
    pub weave: bool,
    pub ooze: bool,
}

/// A mob's health the tick before, while it is infested.
#[derive(Component)]
pub(super) struct Watched(f32);

/// The mob, of the body `body`, died: its effects do what they do.
pub(super) fn died(world: &mut World, mob: Entity, layer: Entity, body: &Body) {
    let has = |effect| afflictions::level(world, mob, effect).is_some();
    let omens = Omens { wind: has(Effect::WindCharged), weave: has(Effect::Weaving), ooze: has(Effect::Oozing) };
    befall(world, layer, omens, body.at, body.size[1], MOB_FLASH);
}

/// A body at `at`, `tall` high, died with `omens`; `flash` is its burst's particle data.
pub(crate) fn befall(world: &mut World, layer: Entity, omens: Omens, at: [f32; 3], tall: f32, flash: i32) {
    if omens.wind {
        burst(world, layer, [at[0], at[1] + tall / 2.0, at[2]], at, flash);
    }
    if omens.weave {
        weave(world, layer, at.map(|c| c.floor() as i32));
    }
    if omens.ooze {
        for _ in 0..SLIMES {
            ooze(world, layer, [at[0], at[1] + SLIME_UP, at[2]]);
        }
    }
}

fn burst(world: &mut World, layer: Entity, middle: [f32; 3], from: [f32; 3], data: i32) {
    let [x, y, z] = middle;
    let flash = LevelEvent { event: LevelEventEvent::ParticleWindExplosion, position: Vec3f { x, y, z }, data };
    let cell = middle.map(|c| c.floor() as i32);
    crate::view::to_viewers(world, layer, cell, &[sapling_net::encode(&flash)]);
    late_sounds::play_later(world, 1, layer, cell, arrows::sound("wind_charge.burst", middle, ""));
    let players: Vec<(Entity, [f32; 3])> = in_world(world)
        .into_iter()
        .filter(|player| world.get::<InLayer>(*player).map(|on| on.0) == Some(layer) && !vitals::is_dead(world, *player))
        .filter_map(|player| Some((player, world.get::<Position>(player)?.feet)))
        .collect();
    for (player, them) in players {
        let far = (0..3).map(|axis| (them[axis] - from[axis]).powi(2)).sum::<f32>().sqrt();
        let towards: [f32; 3] = std::array::from_fn(|axis| them[axis] + if axis == 1 { EYES } else { 0.0 } - middle[axis]);
        let long = towards.iter().map(|c| c * c).sum::<f32>().sqrt().max(1e-4);
        if far < REACH {
            vitals::pushed(world, player, towards.map(|c| c / long * PUSH * (1.0 - far / REACH)));
        }
    }
}

fn weave(world: &mut World, layer: Entity, at: [i32; 3]) {
    let (tries, count): (Vec<[i32; 3]>, usize) = {
        let mut dice = world.get_resource_or_insert_with(Dice::default);
        let count = WEBS + dice.roll(2) as usize;
        ((0..WEB_TRIES).map(|_| std::array::from_fn(|axis| at[axis] + dice.roll(3) as i32 - 1)).collect(), count)
    };
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return };
    let solid = |[x, y, z]: [i32; 3]| level.in_height(y) && level.registry().get(level.block(x, y, z)).is_some_and(|state| state.is_full_cube());
    let open = |[x, y, z]: [i32; 3]| level.in_height(y) && level.registry().get(level.block(x, y, z)).is_some_and(|state| state.name == "minecraft:air");
    let mut webs: Vec<[i32; 3]> = Vec::new();
    for cell in tries {
        if webs.len() < count && !webs.contains(&cell) && open(cell) && solid([cell[0], cell[1] - 1, cell[2]]) {
            webs.push(cell);
        }
    }
    for [x, y, z] in webs {
        let spun = LevelEvent { event: LevelEventEvent::AnimationSpawnCobweb, position: Vec3f { x: x as f32, y: y as f32, z: z as f32 }, data: 0 };
        crate::view::to_viewers(world, layer, [x, y, z], &[sapling_net::encode(&spun)]);
        blocks::update_block(world, layer, [x, y, z], &Block::new(WEB));
    }
}

fn ooze(world: &mut World, layer: Entity, feet: [f32; 3]) {
    let Some(slime) = super::spawn(world, layer, "minecraft:slime", feet).and_then(|id| entities::find(world, id)) else { return };
    world.entity_mut(slime).insert(super::genes::Gene(SLIME_SIZE));
    if let Some(mut body) = world.get_mut::<Body>(slime) {
        body.size = [SLIME_BOX; 2];
    }
    if let Some(mut health) = world.get_mut::<Health>(slime) {
        health.grow(SLIME_HEALTH);
    }
    let Some(id) = world.get::<NetId>(slime).map(|id| id.0) else { return };
    let metadata = vec![
        MetadataDictionaryItem { key: Key::Variant, r#type: Type::Int, legacy_type: 2, value: Value::Default(Plain::Int(SLIME_SIZE)) },
        MetadataDictionaryItem { key: Key::BoundingboxWidth, r#type: Type::Float, legacy_type: 3, value: Value::Default(Plain::Float(SLIME_BOX)) },
        MetadataDictionaryItem { key: Key::BoundingboxHeight, r#type: Type::Float, legacy_type: 3, value: Value::Default(Plain::Float(SLIME_BOX)) },
    ];
    let sized = SetEntityData { runtime_entity_id: id, metadata, properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 };
    entities::tell_viewers(world, slime, layer, feet.map(|c| c.floor() as i32), vec![sapling_net::encode(&sized)]);
}

/// A tick of every infested mob: one hurt since the last may let out silverfish.
pub(super) fn tick(world: &mut World) {
    let watched: Vec<(Entity, Entity, f32, Option<f32>, Body)> = world
        .query::<(Entity, &InLayer, &Health, Option<&Watched>, &Body)>()
        .iter(world)
        .map(|(mob, layer, health, watched, body)| (mob, layer.0, health.left(), watched.map(|watched| watched.0), *body))
        .collect();
    for (mob, layer, left, before, body) in watched {
        if afflictions::level(world, mob, Effect::Infested).is_none() {
            if before.is_some() {
                world.entity_mut(mob).remove::<Watched>();
            }
            continue;
        }
        world.entity_mut(mob).insert(Watched(left));
        if before.is_some_and(|before| left < before && left > 0.0) {
            let facing = world.get::<legs::Legs>(mob).map_or(0.0, |legs| legs.yaw);
            infested_hurt(world, layer, [body.at[0], body.at[1] + body.size[1] / 2.0, body.at[2]], facing);
        }
    }
}

/// An infested body facing `facing` was hurt: a time in ten, silverfish come out at `from`.
pub(crate) fn infested_hurt(world: &mut World, layer: Entity, from: [f32; 3], facing: f32) {
    let count = {
        let mut dice = world.get_resource_or_insert_with(Dice::default);
        if dice.fraction() >= SILVERFISH_CHANCE {
            return;
        }
        SILVERFISH + dice.roll(2)
    };
    for _ in 0..count {
        infest(world, layer, from, facing);
    }
}

fn infest(world: &mut World, layer: Entity, from: [f32; 3], facing: f32) {
    let Some(silverfish) = super::spawn(world, layer, "minecraft:silverfish", from).and_then(|id| entities::find(world, id)) else { return };
    let off = (world.get_resource_or_insert_with(Dice::default).fraction() * 2.0 - 1.0) * SILVERFISH_SPREAD;
    let yaw = (facing + off).to_radians();
    if let Some(mut body) = world.get_mut::<Body>(silverfish) {
        body.motion = [-yaw.sin() * SILVERFISH_SPEED, 0.0, yaw.cos() * SILVERFISH_SPEED];
    }
}
