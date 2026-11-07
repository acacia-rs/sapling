//! An allay bringing what it collected (collecting.rs) to a note block it heard
//! (`behavior.go_and_give_items_to_noteblock`, priority 3) or else to its owner
//! (`go_and_give_items_to_owner`, 4), both at `run_speed` 8, and keeping near that note block
//! (`stay_near_noteblock`, 5: from 16 off until 4), as BDS's did (research/captures/bds-ap3give.txt,
//! bds-ap3note.txt). It hears a note block played within 16 (its `vibration_listener`) and keeps it
//! 30 s (BDS `+0x54` seconds, decompiled 0x5743700): BDS's took the dirt it threw there up again
//! and threw it back until then. Before its move, within 3 of the owner's eyes or the block's
//! centre it throws one item a tick at that point a block up (BDS 0x5745670), from 0.6654 over its
//! feet, and may take nothing up for 3 s (the pack's `pickup_item_delay`).

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_world::ItemStack;

use super::{ALLAY, Errands, errands, shown};
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{InLayer, Layer, Position, Tick};
use crate::creatures::wings;
use crate::{drops, mobs};
use crate::entities::Kind;

const HEARS: f32 = 16.0;
const REMEMBERS: u64 = 600;
const SPEED: f32 = 8.0;
const THROWS: f32 = 3.0;
const RESTS: u64 = 60;
const STAY_FROM: f32 = 16.0;
const FOLLOW_FROM: f32 = 16.0;
const FOLLOW_UNTIL: f32 = 4.0;
const STAY_UNTIL: f32 = 4.0;
/// The thrown item's box's middle over its feet, as BDS's was shown (bds-ap3give.txt).
const THROWN_FROM: f32 = 0.6654;
/// The throw's speed across and up along its line.
const THROW_SPEED: [f32; 2] = [0.2, 0.3];
/// BDS plays `item_thrown` when the level's `currentTick` is a multiple of this, nine times in ten
/// (husbandry.md "Allays collecting and giving back").
const SOUND_EVERY: i64 = 7;
const SOUND_CHANCE: f32 = 0.9;

fn centre(cell: BlockPos) -> [f32; 3] {
    cell.map(|c| c as f32 + 0.5)
}

/// Allays within [`HEARS`] of the note block at `at` played, that `heard` (no wool between), keep it.
pub(crate) fn hear_note(world: &mut World, layer: Entity, at: [f32; 3], heard: impl Fn(&World, BlockPos) -> bool) {
    let cell = at.map(|c| c.floor() as i32);
    let near: Vec<Entity> = world
        .query::<(Entity, &Kind, &Body, &InLayer)>()
        .iter(world)
        .filter(|(_, kind, body, in_layer)| kind.0 == ALLAY && in_layer.0 == layer && super::super::distance(body.at, at) <= HEARS)
        .filter(|(_, _, body, _)| heard(world, body.at.map(|c| c.floor() as i32)))
        .map(|(allay, ..)| allay)
        .collect();
    let now = world.resource::<Tick>().0;
    for allay in near {
        errands(world, allay).note = Some((layer, cell, now));
    }
}

/// Before its move: what it carries goes to the note block, else its owner; with nothing, it keeps
/// near the note block. False where it does none of these.
pub(super) fn steer(world: &mut World, allay: Entity, layer: Entity, body: &Body) -> bool {
    let now = world.resource::<Tick>().0;
    let Some(errands) = world.get::<Errands>(allay) else { return false };
    let note = errands.note.filter(|(heard_in, _, heard)| *heard_in == layer && now < heard + REMEMBERS).map(|(_, cell, _)| centre(cell));
    let owner = errands.owner.and_then(|owner| world.get::<InLayer>(owner).filter(|in_layer| in_layer.0 == layer).and(world.get::<Position>(owner)));
    let owner = owner.map(|position| [position.feet[0], position.feet[1] + sapling_data::EYE_HEIGHT, position.feet[2]]);
    let (carries, staying) = (errands.carried.is_some(), errands.staying);
    if let Some(to) = note.or(owner).filter(|_| carries) {
        if super::super::distance(body.at, to) < THROWS {
            throw(world, allay, layer, body, to, now);
            return true;
        }
        return wings::fly_at_times(world, allay, ALLAY, to, SPEED);
    }
    let stays = note.filter(|to| {
        let far = super::super::distance(body.at, *to);
        far > STAY_UNTIL && (staying || far > STAY_FROM)
    });
    super::errands(world, allay).staying = stays.is_some();
    if let Some(to) = stays {
        return wings::fly_at_times(world, allay, ALLAY, to, SPEED);
    }
    follow(world, allay, body, owner)
}

/// `follow_owner`: from over [`FOLLOW_FROM`] off its owner's eyes it flies at them until within
/// [`FOLLOW_UNTIL`], as BDS's crossed 30 blocks at about 0.56 a tick (research/captures/bds-aq3allay.txt).
fn follow(world: &mut World, allay: Entity, body: &Body, owner: Option<[f32; 3]>) -> bool {
    let Some(to) = owner else { return false };
    let far = super::super::distance(body.at, to);
    let mut errands = super::errands(world, allay);
    errands.following = far > FOLLOW_UNTIL && (errands.following || far > FOLLOW_FROM);
    errands.following && wings::fly_at_times(world, allay, ALLAY, to, SPEED)
}

/// Throws one of what it carries at `to` a block up, and rests.
fn throw(world: &mut World, allay: Entity, layer: Entity, body: &Body, to: [f32; 3], now: u64) {
    let mut errands = errands(world, allay);
    let Some(carried) = errands.carried.take() else { return };
    if carried.count > 1 {
        errands.carried = Some(ItemStack { count: carried.count - 1, ..carried.clone() });
    }
    errands.rests_until = now + RESTS;
    super::keep_carried(world, allay);
    let aim = [to[0] - body.at[0], to[1] + 1.0 - body.at[1], to[2] - body.at[2]];
    let long = aim.iter().map(|c| c * c).sum::<f32>().sqrt();
    let [across, up] = THROW_SPEED;
    let motion = if long < 1e-4 { [0.0; 3] } else { [aim[0] / long * across, aim[1] / long * up, aim[2] / long * across] };
    let from = [body.at[0], body.at[1] + THROWN_FROM, body.at[2]];
    // BDS showed the item, then the sound, then the item's metadata (bds-ap3note.txt).
    let begun = drops::hand_over(world, layer, ItemStack { count: 1, ..carried }, from, motion).and_then(|item| mobs::at_once::show_first(world, item));
    let age = world.get::<Layer>(layer).map_or(0, |level| level.world.level().age);
    if age % SOUND_EVERY == 0 && world.get_resource_or_insert_with(Dice::default).fraction() < SOUND_CHANCE {
        shown::sound(world, allay, "item_thrown");
    }
    if let Some(begun) = begun {
        mobs::at_once::finish(world, begun);
    }
}
