//! Drowned that throw tridents. As the pack has it, 25 in 400 come `as_ranged_adult` (or the
//! baby's or jockey's like): those get `ranged_equipment`, a trident, and a `mode_switcher` that
//! throws beyond 5 off and bites within 3, a throw every 1 to 3 seconds within 10 (`ranged_attack`).
//! research/captures/bds-drowned.txt: shown holding the trident as it came; it swung, and the
//! trident flew a tick later at 3.94 a tick, heard `item.trident.throw` from the drowned; it kept
//! its own, and the thrown one stuck where it fell.
//! One in 12.5 also holds a nautilus shell in its offhand (`drowned_ranged_equipment.json`'s 8%).
//! research/captures/bds-drownedloot.txt, 40 armed drowned killed: 13 dropped their trident,
//! worn to a random `Damage` (32 to 248 of 250), and the 5 with shells all dropped them.
//! Both come from rolling that table, a quarter of tridents enchanted (`enchant_random_gear` 0.25
//! times the spot's multiplier; none seen on BDS, not drilled), shown, kept and dropped so.
// A viewer who comes later is shown the trident with the drowned.

use acacia_proto::nbt::Value;
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_world::ItemStack;

use super::arrows::{self, ARROW, Shot};
use super::hunting;
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{NetId, Position, Tick};
use crate::entities;

const DROWNED: &str = "minecraft:drowned";
const RANGED: &str = "minecraft:as_ranged_adult";
const RIDER: &str = "as_rider";
/// An unarmed drowned's events that add `melee_equipment`: a fishing rod at 3.75%, a shell at 8%
/// (research/captures/bds-f14cgear.txt: of 36, 2 rods and 6 shells, in the off hand).
const MELEE_EVENTS: [&str; 4] = ["entity_born", "as_adult", "as_baby", "as_baby_jockey"];
const MELEE_GEAR: &str = "loot_tables/entities/drowned_equipment.json";
const RANGED_GEAR: &str = "loot_tables/entities/drowned_ranged_equipment.json";
const TRIDENT_ITEM: &str = "minecraft:trident";
const SHELL: &str = "minecraft:nautilus_shell";
/// Of this many drowned, `ARMED` come armed so.
const ARMED: u32 = 25;
const OF: u32 = 400;
/// It throws beyond `THROWS_PAST`, bites within `BITES_IN`, keeps its way between, and throws
/// within `RANGE` every `EVERY` ticks and up to `MORE` more, its first a `SIGHTED` later: BDS's
/// throws came 48 to 84 ticks apart whatever the distance, the first 72 to 93 after it took its
/// prey (bds-s3drowned.txt, bds-q3drowned.txt), not the pack's 20 to 60 by distance.
const THROWS_PAST: f32 = 5.0;
const BITES_IN: f32 = 3.0;
const RANGE: f32 = 10.0;
const EVERY: u64 = 48;
const MORE: u32 = 36;
const SIGHTED: u64 = 20;
/// The pack's `thrown_trident`: power 4, gravity 0.1, 8 to whom it hits.
const TRIDENT: Shot = Shot {
    kind: "minecraft:thrown_trident",
    ahead: 0.0,
    held: 1.5,
    speed: 4.0,
    weight: 0.1,
    wet_drag: 0.99,
    harm: Some(sapling_data::shots::TRIDENT_HARM),
    sound: "item.trident.throw",
    heard: ["item.trident.hit", "item.trident.hit_ground"],
    size: [0.25, 0.35],
    ..ARROW
};

/// Of the dead, this fraction let go of their trident.
const DROPS_TRIDENT: f32 = 0.25;
const TRIDENT_LASTS: u32 = 250;

#[derive(Component, Default)]
pub(super) struct Thrower {
    /// Its trident's tag (its enchantments), if any.
    trident: Option<Value>,
    shell: bool,
    throwing: bool,
    ready: u64,
    /// The tick its swing's trident flies.
    loosing: Option<u64>,
    /// What it lets go of, chosen as it dies ([`choose_drops`]).
    drops: Vec<ItemStack>,
}

/// A drowned that comes: armed with a trident, shown in its hand, by the pack's odds or its spawn `event`.
pub(super) fn arm(world: &mut World, drowned: Entity, kind: &str, event: Option<&str>) {
    if kind != DROWNED {
        return;
    }
    // A rider's `drowned_rider_equipment` is a trident alone (bds-h3herds.txt).
    let rider = event.is_some_and(|event| event.trim_start_matches("minecraft:") == RIDER);
    let armed = rider || event == Some(RANGED) || event.is_none() && world.get_resource_or_insert_with(Dice::default).roll(OF) < ARMED;
    if !armed && event.is_none_or(|event| MELEE_EVENTS.contains(&event.trim_start_matches("minecraft:"))) {
        return super::spawn_gear::equip(world, drowned, MELEE_GEAR);
    }
    let Some(id) = world.get::<NetId>(drowned).map(|id| id.0).filter(|_| armed) else { return };
    let rolled = if rider { Vec::new() } else { super::spawn_gear::rolled(world, drowned, RANGED_GEAR) };
    let trident = rolled.iter().find(|stack| stack.name == TRIDENT_ITEM).and_then(|stack| stack.tag.clone());
    let shell = rolled.iter().any(|stack| stack.name == SHELL);
    world.entity_mut(drowned).insert(Thrower { trident, shell, ..Thrower::default() });
    super::reshow(world, drowned);
    let (held, Some(body)) = (held(world, drowned, id), world.get::<Body>(drowned).copied()) else { return };
    let layer = world.get::<crate::components::InLayer>(drowned).map(|layer| layer.0);
    if let Some(layer) = layer {
        entities::tell_viewers(world, drowned, layer, body.at.map(|c| c.floor() as i32), held);
    }
}

/// What shows an armed drowned's trident in its hand, and its shell.
pub(super) fn held(world: &World, drowned: Entity, id: u64) -> Vec<Bytes> {
    let Some(thrower) = world.get::<Thrower>(drowned) else { return Vec::new() };
    let stack = |name, tag| sapling_data::Stack { name, count: 1, damage: 0, block: None, tag };
    let hands = [Some((TRIDENT_ITEM, thrower.trident.as_ref(), false)), thrower.shell.then_some((SHELL, None, true))];
    hands.into_iter().flatten().filter_map(|(name, tag, offhand)| sapling_data::mob_hand(id, &stack(name, tag), offhand)).map(|held| sapling_net::encode(&held)).collect()
}

/// Chooses, as an armed drowned dies, what it will let go of ([`let_go`]): its shell, and by chance
/// its trident if a player killed it, worn as Java's `Mob::dropCustomDeathLoot` wears a dropped
/// item. The trident it keeps, which BDS showed back in its hand at death (bds-drownedloot.txt).
pub(super) fn choose_drops(world: &mut World, drowned: Entity, by_player: bool) -> Option<ItemStack> {
    let (shell, mut tag) = world.get::<Thrower>(drowned).map(|thrower| (thrower.shell, thrower.trident.clone()))?;
    let mut dice = world.get_resource_or_insert_with(Dice::default);
    let item = |name: &str, tag| ItemStack { name: name.into(), count: 1, damage: 0, block: None, tag };
    let mut dropped: Vec<ItemStack> = shell.then(|| item(SHELL, None)).into_iter().collect();
    let trident_goes = by_player && dice.fraction() < DROPS_TRIDENT;
    if trident_goes {
        let spread = 1 + dice.roll(TRIDENT_LASTS - 3);
        crate::inventory::set_damage(&mut tag, (TRIDENT_LASTS - dice.roll(spread)) as i32);
        dropped.push(item(TRIDENT_ITEM, tag.clone()));
    }
    world.get_mut::<Thrower>(drowned)?.drops = dropped;
    (!trident_goes).then(|| item(TRIDENT_ITEM, tag))
}

/// What [`choose_drops`] chose for the dead drowned to let go of.
pub(super) fn let_go(world: &mut World, drowned: Entity) -> Vec<ItemStack> {
    world.get_mut::<Thrower>(drowned).map(|mut thrower| std::mem::take(&mut thrower.drops)).unwrap_or_default()
}

fn wait(world: &mut World) -> u64 {
    EVERY + u64::from(world.get_resource_or_insert_with(Dice::default).roll(MORE + 1))
}

/// After its move: an armed drowned's throw, a tick after its swing.
pub(super) fn throw(world: &mut World, drowned: Entity, layer: Entity, id: u64, kind: &str, body: &Body) -> Option<()> {
    let now = world.resource::<Tick>().0;
    let feet = hunting::quarry(world, drowned).and_then(|(prey, _)| world.get::<Position>(prey)).map(|at| at.feet);
    let thrower = world.get::<Thrower>(drowned)?;
    let (throwing, ready, loosing) = (thrower.throwing, thrower.ready, thrower.loosing);
    if let (Some(due), Some(feet)) = (loosing, feet)
        && now >= due
    {
        world.get_mut::<Thrower>(drowned)?.loosing = None;
        arrows::loose(world, layer, drowned, kind, body, feet, None, TRIDENT);
    }
    let feet = feet?;
    let far = (feet[0] - body.at[0]).hypot(feet[2] - body.at[2]);
    let throwing = if far > THROWS_PAST { true } else if far < BITES_IN { false } else { throwing };
    world.get_mut::<Thrower>(drowned)?.throwing = throwing;
    if throwing && ready == 0 {
        world.get_mut::<Thrower>(drowned)?.ready = now + SIGHTED + wait(world);
        return Some(());
    }
    if throwing && far <= RANGE && now >= ready {
        let wait = wait(world);
        let mut thrower = world.get_mut::<Thrower>(drowned)?;
        (thrower.ready, thrower.loosing) = (now + wait, Some(now + 1));
        hunting::swing(world, drowned, layer, id, body);
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// bds-drownedloot.txt: 13 of 40 tridents dropped, worn; every shell dropped.
    #[test]
    fn a_dead_drowned_drops_its_shell_and_a_quarter_of_the_time_its_worn_trident() {
        let mut world = World::new();
        let drowned = world.spawn(Thrower { shell: true, ..Thrower::default() }).id();
        let mut die = |by_player| {
            let kept = choose_drops(&mut world, drowned, by_player);
            let dropped = let_go(&mut world, drowned);
            assert_eq!(kept.is_some(), !dropped.iter().any(|stack| stack.name == "minecraft:trident"), "kept or dropped, not both");
            dropped
        };
        let deaths: Vec<Vec<ItemStack>> = (0..400).map(|_| die(true)).collect();
        assert!(deaths.iter().all(|dropped| dropped[0].name == "minecraft:nautilus_shell"));
        let worn = |stack: &ItemStack| match stack.tag.as_ref()?.get("Damage") {
            Some(Value::Int(worn)) => Some(*worn),
            _ => None,
        };
        let tridents: Vec<i32> = deaths.iter().filter_map(|dropped| worn(dropped.get(1)?)).collect();
        assert!((70..130).contains(&tridents.len()), "{}", tridents.len());
        assert!(tridents.iter().all(|worn| (4..=250).contains(worn)));
        assert_eq!(die(false).len(), 1, "only a player's kill drops the trident");
    }
}
