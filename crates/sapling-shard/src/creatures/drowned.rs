//! Drowned that throw tridents. As the pack has it, 25 in 400 come `as_ranged_adult` (or the
//! baby's or jockey's like): those get `ranged_equipment`, a trident, and a `mode_switcher` that
//! throws beyond 5 off and bites within 3, a throw every 1 to 3 seconds within 10 (`ranged_attack`).
//! research/captures/bds-drowned.txt: shown holding the trident as it came; it swung, and the
//! trident flew a tick later at 3.94 a tick, heard `item.trident.throw` from the drowned; it kept
//! its own, and the thrown one stuck where it fell.
//! One in 12.5 also holds a nautilus shell in its offhand (`drowned_ranged_equipment.json`'s 8%).
//! research/captures/bds-drownedloot.txt, 40 armed drowned killed: 13 dropped their trident,
//! worn to a random `Damage` (32 to 248 of 250), and the 5 with shells all dropped them.
// A viewer who comes later is shown the trident with the drowned.
// TODO: the table's enchanted tridents (`enchant_random_gear` 0.25) are not done; none were seen.

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

/// Of armed drowned, this fraction hold a shell too; of the dead, this fraction let go of their trident.
const SHELLED: f32 = 0.08;
const DROPS_TRIDENT: f32 = 0.25;
const TRIDENT_LASTS: u32 = 250;

#[derive(Component, Default)]
pub(super) struct Thrower {
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
    let Some(id) = world.get::<NetId>(drowned).map(|id| id.0).filter(|_| armed) else { return };
    let shell = !rider && world.get_resource_or_insert_with(Dice::default).fraction() < SHELLED;
    world.entity_mut(drowned).insert(Thrower { shell, ..Thrower::default() });
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
    let stack = |name| sapling_data::Stack { name, count: 1, damage: 0, block: None, tag: None };
    let hands = [Some(("minecraft:trident", false)), thrower.shell.then_some(("minecraft:nautilus_shell", true))];
    hands.into_iter().flatten().filter_map(|(name, offhand)| sapling_data::mob_hand(id, &stack(name), offhand)).map(|held| sapling_net::encode(&held)).collect()
}

/// Chooses, as an armed drowned dies, what it will let go of ([`let_go`]): its shell, and by chance
/// its trident if a player killed it, worn as Java's `Mob::dropCustomDeathLoot` wears a dropped
/// item. The trident it keeps, which BDS showed back in its hand at death (bds-drownedloot.txt).
pub(super) fn choose_drops(world: &mut World, drowned: Entity, by_player: bool) -> Option<ItemStack> {
    let shell = world.get::<Thrower>(drowned)?.shell;
    let mut dice = world.get_resource_or_insert_with(Dice::default);
    let item = |name: &str, tag| ItemStack { name: name.into(), count: 1, damage: 0, block: None, tag };
    let mut dropped: Vec<ItemStack> = shell.then(|| item("minecraft:nautilus_shell", None)).into_iter().collect();
    let trident_goes = by_player && dice.fraction() < DROPS_TRIDENT;
    if trident_goes {
        let spread = 1 + dice.roll(TRIDENT_LASTS - 3);
        let worn = TRIDENT_LASTS - dice.roll(spread);
        dropped.push(item("minecraft:trident", Some(Value::Compound(vec![("Damage".into(), Value::Int(worn as i32))]))));
    }
    world.get_mut::<Thrower>(drowned)?.drops = dropped;
    (!trident_goes).then(|| item("minecraft:trident", None))
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
