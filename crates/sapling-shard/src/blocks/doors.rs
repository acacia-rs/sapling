//! Blocks a hand moves (doors, trapdoors, fence gates and levers open and shut, buttons go in
//! for a second) and the second cell of doors and beds, which is placed, opened and broken with
//! the first.
// TODO: no pressure plates, no redstone, nobody sleeps in a bed, a gate does not
// swing away from its user, and `door.open`/`door.close` are named by the pattern of the other
// sounds: the server's binary holds those as plain strings, but not these two.

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};
use sapling_world::BlockRegistry;

use super::{level, send};
use crate::components::Tick;

const OPEN: &str = "open_bit";
const UPPER: &str = "upper_block_bit";
const PRESSED: &str = "button_pressed_bit";
const HEAD: &str = "head_piece_bit";
const HINGE: &str = "door_hinge_bit";
/// The step a bed's `direction` takes from foot to head: 0 south, 1 west, 2 north, 3 east.
const AHEAD: [[i32; 2]; 4] = [[0, 1], [-1, 0], [0, -1], [1, 0]];
/// The step to the left of a look into each quarter, numbered the same way.
const LEFT: [[i32; 2]; 4] = [[1, 0], [0, 1], [-1, 0], [0, -1]];
/// Ticks a button stays in (BDS `ButtonBlock::buttonPressed`): the stone ones, and every other.
const STONE_PRESS_TICKS: u64 = 20;
const WOODEN_PRESS_TICKS: u64 = 30;
const STONE_BUTTONS: &[&str] = &[":stone_button", "polished_blackstone_button"];

fn property<'a>(registry: &'a BlockRegistry, id: u32, key: &str) -> Option<&'a str> {
    registry.get(id)?.properties.split(',').filter_map(|pair| pair.split_once('=')).find(|(name, _)| *name == key).map(|(_, value)| value)
}

/// The state of `id`'s block with one property set; `None` when the block lacks it.
pub(crate) fn with(registry: &BlockRegistry, id: u32, key: &str, value: &str) -> Option<u32> {
    let state = registry.get(id)?;
    property(registry, id, key)?;
    let set = |pair: &str| if pair.split_once('=').is_some_and(|(name, _)| name == key) { format!("{key}={value}") } else { pair.to_owned() };
    registry.find(state.name, &state.properties.split(',').map(set).collect::<Vec<_>>().join(","))
}

/// The cell a bed's head lies in, seen from its foot.
fn ahead(registry: &BlockRegistry, id: u32, [x, y, z]: BlockPos, steps: i32) -> Option<BlockPos> {
    let [across, along] = *AHEAD.get(property(registry, id, "direction")?.parse::<usize>().ok()?)?;
    Some([x + across * steps, y, z + along * steps])
}

/// Where the other cell of the two-cell block at `pos` is, when it is there.
fn other_half(world: &World, layer: Entity, pos: BlockPos) -> Option<BlockPos> {
    let level = level(world, layer)?;
    let (registry, id) = (level.registry(), level.block(pos[0], pos[1], pos[2]));
    let other = match (property(registry, id, UPPER), property(registry, id, HEAD)) {
        (Some(upper), _) => if upper == "1" { Face::Down.neighbour(pos) } else { Face::Up.neighbour(pos) },
        (None, Some(head)) => ahead(registry, id, pos, if head == "1" { -1 } else { 1 })?,
        (None, None) => return None,
    };
    let same = level.in_height(other[1]) && registry.get(level.block(other[0], other[1], other[2]))?.name == registry.get(id)?.name;
    same.then_some(other)
}

/// The second cell of the two-cell block `id` whose first goes to `pos`, and what fills it: a
/// door's upper half above, a bed's head ahead of its foot.
pub(super) fn second_cell(registry: &BlockRegistry, id: u32, pos: BlockPos) -> Option<(BlockPos, u32)> {
    if let Some(upper) = with(registry, id, UPPER, "1") {
        return Some((Face::Up.neighbour(pos), upper));
    }
    Some((ahead(registry, id, pos, 1)?, with(registry, id, HEAD, "1")?))
}

/// Fills the second cell of a two-cell block just placed at `pos`; where that cell is.
pub(super) fn place_second_cell(world: &mut World, layer: Entity, pos: BlockPos) -> Option<BlockPos> {
    let level = level(world, layer)?;
    let (cell, state) = second_cell(level.registry(), level.block(pos[0], pos[1], pos[2]), pos)?;
    send::set(world, layer, cell, state).then_some(cell)
}

/// The door `id` about to stand at `pos`, placed by a look into `quarter`, hinged on the right
/// where the same door stands to its left or only its right has a solid block to hang on: the
/// Java game's `DoorBlock.getHinge` without its tie-break by where the click fell
/// (docs/research/block-rules.md). Any other block is left as it is.
pub(super) fn hinged(level: &sapling_world::World, id: u32, pos: BlockPos, quarter: usize) -> u32 {
    let (registry, [x, z]) = (level.registry(), LEFT[quarter % 4]);
    let beside = |side: i32| registry.get(level.block(pos[0] + side * x, pos[1], pos[2] + side * z));
    let is_pair = beside(1).map(|state| state.name) == registry.get(id).map(|state| state.name);
    let hangs_right = beside(-1).is_some_and(|state| state.is_solid()) && !beside(1).is_some_and(|state| state.is_solid());
    Some(id).filter(|_| is_pair || hangs_right).and_then(|id| with(registry, id, HINGE, "1")).unwrap_or(id)
}

/// Takes the other cell of a two-cell block out of the world; call before the cell at `pos` goes.
pub(super) fn remove_other_half(world: &mut World, layer: Entity, pos: BlockPos) {
    let (Some(other), Some(air)) = (other_half(world, layer, pos), level(world, layer).map(|level| level.registry().air_id())) else { return };
    send::set(world, layer, other, air);
}

/// Pressed buttons and when they come out again.
#[derive(Resource, Default)]
pub(crate) struct Pressed(Vec<(u64, Entity, BlockPos)>);

/// Lets those who see the block at `pos` hear it make the sound `name`.
fn play(world: &mut World, layer: Entity, pos: BlockPos, name: &str) {
    let Some(level) = level(world, layer) else { return };
    let heard = send::sound(name, level.registry(), pos, level.block(pos[0], pos[1], pos[2]));
    send::to_viewers(world, layer, pos, &[heard]);
}

/// Sets a button in or out, with its click.
fn set_pressed(world: &mut World, layer: Entity, [x, y, z]: BlockPos, pressed: bool) -> bool {
    let Some(level) = level(world, layer) else { return false };
    let Some(id) = with(level.registry(), level.block(x, y, z), PRESSED, if pressed { "1" } else { "0" }) else { return false };
    let set = send::set(world, layer, [x, y, z], id);
    if set {
        play(world, layer, [x, y, z], if pressed { "button.click_on" } else { "button.click_off" });
    }
    set
}

/// Pushes the button `name` at `pos` in until [`release_buttons`] lets it out; a button that
/// is in stays in no longer for it.
fn press(world: &mut World, layer: Entity, pos: BlockPos, name: &str) {
    let is_out = level(world, layer).is_some_and(|level| property(level.registry(), level.block(pos[0], pos[1], pos[2]), PRESSED) == Some("0"));
    if is_out && set_pressed(world, layer, pos, true) {
        let ticks = if STONE_BUTTONS.iter().any(|stone| name.ends_with(stone)) { STONE_PRESS_TICKS } else { WOODEN_PRESS_TICKS };
        let until = world.resource::<Tick>().0 + ticks;
        world.get_resource_or_insert_with(Pressed::default).0.push((until, layer, pos));
    }
}

/// Lets out the buttons whose time is up.
pub(crate) fn release_buttons(world: &mut World) {
    let tick = world.resource::<Tick>().0;
    let Some(mut pressed) = world.get_resource_mut::<Pressed>().filter(|pressed| pressed.0.iter().any(|(until, ..)| *until <= tick)) else { return };
    let (due, held): (Vec<_>, Vec<_>) = std::mem::take(&mut pressed.0).into_iter().partition(|(until, ..)| *until <= tick);
    pressed.0 = held;
    for (_, layer, pos) in due {
        set_pressed(world, layer, pos, false);
    }
}

/// What a hand does to the block `name` at `pos`: a door, trapdoor, fence gate or lever opens
/// or shuts, a button goes in. Iron does not move by hand.
pub(super) fn use_by_hand(world: &mut World, layer: Entity, pos: BlockPos, name: &str) {
    if name.contains("iron_") {
        return;
    }
    // What it sounds like to open and to shut; a lever clicks like a button (BDS `LeverBlock::toggle`).
    let sounds = match name {
        _ if name.ends_with("button") => return press(world, layer, pos, name),
        _ if name.ends_with("trapdoor") => ["trapdoor.open", "trapdoor.close"],
        _ if name.ends_with("door") => ["door.open", "door.close"],
        _ if name.ends_with("fence_gate") => ["fence_gate.open", "fence_gate.close"],
        _ if name.ends_with("lever") => ["button.click_on", "button.click_off"],
        _ => return,
    };
    if let Some(now_open) = toggle(world, layer, pos) {
        play(world, layer, pos, sounds[usize::from(!now_open)]);
    }
}

/// Opens or shuts the block at `pos`, both cells of a door; whether it is open now, or `None`
/// for a block that does neither.
fn toggle(world: &mut World, layer: Entity, pos: BlockPos) -> Option<bool> {
    let cells = [Some(pos), other_half(world, layer, pos)];
    let mut now_open = None;
    for [x, y, z] in cells.into_iter().flatten() {
        let level = level(world, layer)?;
        let (registry, id) = (level.registry(), level.block(x, y, z));
        let opens = property(registry, id, OPEN).map(|open| open == "0");
        let Some((id, opens)) = opens.and_then(|opens| Some((with(registry, id, OPEN, if opens { "1" } else { "0" })?, opens))) else { continue };
        if send::set(world, layer, [x, y, z], id) {
            now_open = Some(opens);
        }
    }
    now_open
}
