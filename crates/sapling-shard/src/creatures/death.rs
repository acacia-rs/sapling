//! A mob's death: what it lets go of, what its viewers are told, and the loot it leaves.

use acacia_proto::packets::MobEquipment;
use acacia_proto::types::WindowID;
use bevy_ecs::prelude::*;
use sapling_api::Item;
use sapling_world::ItemStack;

use super::health::Health;
use super::{DYING, Dying, attacks, hunting, omens, wither};
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{InLayer, Layer, NetId, Tick};
use crate::{drops, entities, inventory};

const MOB_LOOT: &str = "doMobLoot";
const SADDLE: &str = "minecraft:saddle";
/// How far above its feet a mob's loot appears (where the wire puts it), as BDS was seen to
/// (research/captures/bds-kill.txt).
const LOOT_LIFT: f32 = 0.075;
/// How far above its feet a mob's own gear lands, tossed as it dies (research/captures/bds-ad3drops.txt).
const GEAR_LIFT: f32 = 0.125;

/// What a mob that died leaves, tossed where it is the tick after its death, as BDS's loot, a
/// mount's chest and saddle came (research/captures/bds-bodies.txt, bds-chestdie.txt).
#[derive(Component)]
pub(super) struct Remains {
    due: u64,
    stacks: Vec<ItemStack>,
}

/// The mob, seen to die, is dead where it is, no longer kept by the world folder, and leaves its
/// loot, cooked if it was on fire, a tick later ([`Remains`]). Its viewers see both its hands
/// emptied after its death, as BDS's (research/captures/bds-reach.txt). A wither does both at
/// the end of its long death.
pub(super) fn die(world: &mut World, entity: Entity, layer: Entity, kind: &str, body: &Body) {
    if crate::dragon::dies(world, entity) {
        return;
    }
    let now = world.resource::<Tick>().0;
    let lasts = wither::dies(world, entity, layer, body);
    let gear = (lasts.is_none() && leaves_loot(world, layer)).then(|| gear_to_let_go(world, entity, kind));
    if let Some(id) = world.get::<NetId>(entity).map(|id| id.0) {
        hunting::let_go(world, entity, layer, id, body);
        if lasts.is_none() {
            let kept = gear.as_ref().is_none_or(|(_, main_went)| !main_went).then(|| super::command_gear::main_hand(world, entity)).flatten();
            show_dead_hands(world, entity, layer, body, kept);
        }
    }
    omens::died(world, entity, layer, body);
    feed_catalyst(world, entity, layer, kind, body);
    let spilled = crate::riding::spilled(world, entity);
    let unsaddled = spilled.iter().any(|stack| stack.name == SADDLE);
    crate::riding::mouth_at_death(world, entity, layer, kind, body, unsaddled);
    // research/captures/bds-e1loot.txt: a tamed horse left its leather, armour, then saddle the tick
    // after; with doMobLoot off its saddle, then armour, in the tick it died.
    let mut stacks = Vec::new();
    if !leaves_loot(world, layer) {
        let [x, y, z] = body.at;
        for stack in spilled.into_iter().chain(crate::riding::looted_armor(world, entity)) {
            drops::toss(world, layer, stack, [x, y + LOOT_LIFT, z]);
        }
    } else {
        if let Some((gear, _)) = gear {
            let [x, y, z] = body.at;
            for stack in gear {
                drops::toss(world, layer, stack, [x, y + GEAR_LIFT, z]);
            }
            stacks.extend(remains(world, entity, layer, kind));
        }
        stacks.extend(spilled);
    }
    world.entity_mut(entity).insert((Dying(now + lasts.unwrap_or(DYING)), Remains { due: now + 1, stacks })).remove::<entities::Stored>();
    if let Some(mut health) = world.get_mut::<Health>(entity) {
        health.end();
    }
}

/// Tosses the [`Remains`] that are due where their mobs lie.
pub(super) fn leave_due(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let due: Vec<(Entity, Entity, [f32; 3])> =
        world.query::<(Entity, &InLayer, &Body, &Remains)>().iter(world).filter(|(.., remains)| now >= remains.due).map(|(entity, layer, body, _)| (entity, layer.0, body.at)).collect();
    for (entity, layer, [x, y, z]) in due {
        let Some(remains) = world.entity_mut(entity).take::<Remains>() else { continue };
        for stack in remains.stacks {
            drops::toss(world, layer, stack, [x, y + LOOT_LIFT, z]);
        }
    }
}

/// A sculk catalyst near the mob takes its experience, and it leaves none. BDS counts every death
/// as a player's kill for it (144879f70 sets the last-hurt-by-player time), so the mob's worth is
/// rolled now when no player slew it.
fn feed_catalyst(world: &mut World, entity: Entity, layer: Entity, kind: &str, body: &Body) {
    let slain = world.get::<attacks::Slain>(entity).map(|slain| slain.0);
    let reward = |world: &mut World| Some(slain.unwrap_or_else(|| attacks::experience_reward(world, entity, kind)));
    if crate::blocks::mob_died(world, layer, body.at, reward)
        && let Some(mut slain) = world.get_mut::<attacks::Slain>(entity)
    {
        slain.0 = 0;
    }
}

/// Its viewers see both its hands emptied, then what it wears.
pub(super) fn empty_hands(world: &mut World, entity: Entity, layer: Entity, body: &Body) {
    show_dead_hands(world, entity, layer, body, None);
}

/// Its viewers see its main hand hold `kept` (what it did not let go of: BDS re-sent a piglin's
/// undropped crossbow, research/captures/bds-ah3nugget.txt) or nothing, its offhand emptied, then
/// what it wears.
fn show_dead_hands(world: &mut World, entity: Entity, layer: Entity, body: &Body, kept: Option<ItemStack>) {
    let Some(id) = world.get::<NetId>(entity).map(|id| id.0) else { return };
    let empty = |slot: u8, window_id| MobEquipment { runtime_entity_id: id, item: sapling_data::no_item(), slot, selected_slot: slot, window_id };
    let main = kept.and_then(|stack| sapling_data::mob_hand(id, &inventory::stack(&stack), false)).unwrap_or_else(|| empty(0, WindowID::Inventory));
    let shown = vec![sapling_net::encode(&main), sapling_net::encode(&empty(1, WindowID::Offhand)), super::worn::shown_dead(world, entity, id)];
    entities::tell_viewers(world, entity, layer, body.at.map(|c| c.floor() as i32), shown);
}

/// The dead mob's loot, tossed now.
pub(super) fn leave_remains(world: &mut World, entity: Entity, layer: Entity, kind: &str, body: &Body) {
    let [x, y, z] = body.at;
    for stack in remains(world, entity, layer, kind) {
        drops::toss(world, layer, stack, [x, y + LOOT_LIFT, z]);
    }
}

/// The dead mob's loot, where the world's rules let it leave any.
fn remains(world: &mut World, entity: Entity, layer: Entity, kind: &str) -> Vec<ItemStack> {
    let on_fire = world.get::<Health>(entity).is_some_and(|health| health.is_burning());
    let looting = world.get::<attacks::Slain>(entity).map(|slain| slain.1);
    let by_player = looting.is_some();
    let killer = looting.map_or(sapling_data::Killer::None, |looting| sapling_data::Killer::Player { looting });
    if !leaves_loot(world, layer) || crate::riding::saddled_bare(world, entity) {
        return Vec::new();
    }
    // BDS dropped a pig's or strider's saddle before the rest (research/captures/bds-saddleevents.txt).
    let mut stacks: Vec<_> = crate::riding::looted_saddle(world, entity, kind).into_iter().collect();
    stacks.extend(loot(world, layer, kind, on_fire, killer));
    stacks.extend(crate::riding::looted_armor(world, entity));
    stacks.extend(super::drowned::let_go(world, entity, by_player));
    stacks.extend(super::captains::let_go(world, entity, by_player));
    stacks
}

/// The gear the dying mob lets go of, to be tossed in its death's tick (BDS's swords had lower ids
/// than all the loot of the pigmen killed with them, research/captures/bds-ad3drops.txt), and
/// whether its main hand's item went with it.
fn gear_to_let_go(world: &mut World, entity: Entity, kind: &str) -> (Vec<ItemStack>, bool) {
    let looting = world.get::<attacks::Slain>(entity).map(|slain| slain.1);
    let (mut stacks, main_went) = super::spawn_gear::let_go(world, entity, kind, looting);
    stacks.extend(super::piglins::picked_gold(world, entity));
    (stacks, main_went)
}

/// Whether the world's `doMobLoot` lets the dead leave loot and experience.
pub(super) fn leaves_loot(world: &World, layer: Entity) -> bool {
    world.get::<Layer>(layer).is_some_and(|layer| layer.world.level().rules.on(MOB_LOOT))
}

fn loot(world: &mut World, layer: Entity, kind: &str, on_fire: bool, killer: sapling_data::Killer) -> Vec<ItemStack> {
    let loot = {
        let mut dice = world.get_resource_or_insert_with(Dice::default);
        sapling_data::mob_loot(kind, on_fire, killer, &mut || dice.fraction())
    };
    let Some(registry) = world.get::<Layer>(layer).map(|layer| layer.world.registry()) else { return Vec::new() };
    loot.into_iter().filter_map(|(name, count)| inventory::stored(&Item::new(name, count.min(64) as u8), registry)).collect()
}
