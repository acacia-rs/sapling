//! Breaking, placing and clicking blocks: what a client asks for is checked, offered to the
//! handlers and then either done for everyone who sees the place or undone on the client that
//! predicted it (docs/DESIGN.md "Step 6 as built").

mod buckets;
mod doors;
mod entities;
mod facing;
mod falling;
mod ground;
mod items;
mod liquid;
mod loot;
mod mining;
mod place;
mod plates;
mod pots;
mod rails;
mod send;
mod shapes;
mod sign_ink;
mod signs;
mod updates;
mod waterlog;

pub(crate) use doors::{bed_head, release_buttons, with as with_property};
pub(crate) use plates::press_plates;
pub(crate) use signs::dispatch_edits as dispatch_sign_edits;
pub(crate) use loot::Dice;
pub(crate) use liquid::tick as flow_liquids;
pub(crate) use mining::Breaking;
pub(crate) use send::{block_at, keep_entity, put_block, set, sound, to_viewers};

use std::collections::HashSet;

use acacia_proto::packets::{InventoryTransaction, PlayerAuthInput, PlayerAuthInputBlockActionItem};
use acacia_proto::types::{Action, TransactionTransactionData, TransactionUseItem, TransactionUseItemActionType};
use bevy_ecs::prelude::*;
use facing::Click;
use sapling_api::{Block, BlockPos, BreakEvent, Face, GameMode, InteractEvent, Player};
use sapling_data::EYE_HEIGHT;
use sapling_world::BlockRegistry;

use crate::components::{InLayer, Layer, Mode, PacketQueues, PerTick, Position, in_world_identity};
use crate::movement::Motion;
use crate::plugins::Plugins;
use crate::worlds;

/// Block actions looked at per player per tick; a client holding a break sends two or three.
const MAX_ACTIONS_PER_TICK: usize = 16;
/// Block clicks looked at per player per tick; more are dropped without an answer.
const MAX_CLICKS_PER_TICK: usize = 4;
/// Blocks from the eye to the middle of a block: BDS took a click at 6.4 and 12.2 (creative) and
/// refused 7.3 and 13.2.
const REACH: f32 = 7.0;
const CREATIVE_REACH: f32 = 13.0;
/// Blocks a click uses rather than builds on, by the end of their name. TODO: a per-block flag.
const USED: &[&str] = &[
    "door", "button", "lever", "fence_gate", "chest", "barrel", "shulker_box", "furnace", "smoker", "crafting_table", "anvil", "enchanting_table", "hopper",
    "dropper", "dispenser", "_sign", "noteblock", "repeater", "comparator", "brewing_stand", "beacon", "loom", "grindstone", "stonecutter_block", ":bed",
    "cartography_table", "smithing_table", "flower_pot",
];

enum Input {
    Actions(Vec<PlayerAuthInputBlockActionItem>),
    Click(TransactionUseItem),
}

/// What this tick's inputs asked of blocks, in arrival order, for [`dispatch`].
#[derive(Resource, Default)]
pub(crate) struct BlockInputs(Vec<(Entity, Input)>);

impl BlockInputs {
    /// Takes the block actions and the item use out of a player's input.
    pub fn take_from(&mut self, from: Entity, input: &mut PlayerAuthInput) {
        if let Some(mut actions) = input.block_action.take() {
            actions.truncate(MAX_ACTIONS_PER_TICK);
            self.0.push((from, Input::Actions(actions)));
        }
        if let Some(transaction) = input.transaction.take() {
            self.0.push((from, Input::Click(transaction.data)));
        }
    }
}

/// Where a request happens, once it passed the checks every request must.
struct Site {
    layer: Entity,
    mode: GameMode,
    sneaking: bool,
}

fn level(world: &World, layer: Entity) -> Option<&sapling_world::World> {
    world.get::<Layer>(layer).map(|layer| &layer.world)
}

/// The site of a request at `pos`, unless it is outside the world's height, in a column that is
/// not loaded or further from the player's eyes than its game mode reaches.
fn site(world: &World, player: Entity, pos: BlockPos) -> Option<Site> {
    let (layer, mode) = (world.get::<InLayer>(player)?.0, world.get::<Mode>(player)?.0);
    let level = level(world, layer)?;
    let known = level.in_height(pos[1]) && level.is_loaded(send::column(pos));
    let sneaking = world.get::<Motion>(player).is_some_and(|motion| motion.pose.sneaking);
    (known && reaches(world, player, pos)).then_some(Site { layer, mode, sneaking })
}

/// Whether the middle of the block at `pos` is within the player's reach of its eyes.
pub(crate) fn reaches(world: &World, player: Entity, pos: BlockPos) -> bool {
    let (Some(position), Some(mode)) = (world.get::<Position>(player), world.get::<Mode>(player)) else { return false };
    let eye = [position.feet[0], position.feet[1] + EYE_HEIGHT, position.feet[2]];
    let distance: f32 = (0..3).map(|axis| (pos[axis] as f32 + 0.5 - eye[axis]).powi(2)).sum();
    let reach = if mode.0 == GameMode::Creative { CREATIVE_REACH } else { REACH };
    distance <= reach * reach
}

fn is_used(name: &str) -> bool {
    USED.iter().any(|end| name.ends_with(end))
}

/// What a click on the block `name` at `pos` does beyond moving it by hand: a container opens,
/// a bed is slept in, a sign is written on, a pot takes or gives a plant, a campfire takes food.
/// False when it does none of these.
fn use_block(world: &mut World, player: Entity, layer: Entity, pos: BlockPos, name: &str) -> bool {
    let uses: [fn(&mut World, Entity, Entity, BlockPos, &str) -> bool; 5] =
        [crate::containers::open, crate::sleep::lie_down, signs::edit_by_click, pots::use_pot, crate::campfires::cook];
    uses.iter().any(|used| used(world, player, layer, pos, name))
}

/// Moves falling blocks and starts the falls that are due, then settles what they changed.
pub(crate) fn fall_blocks(world: &mut World) {
    for (layer, pos) in falling::tick(world) {
        updates::settle(world, layer, pos);
    }
}

/// Runs this tick's block requests through the checks and the handlers.
pub(crate) fn dispatch(world: &mut World, plugins: &mut Plugins) {
    let mut inputs = std::mem::take(&mut world.resource_mut::<BlockInputs>().0);
    let transactions = world.resource_mut::<PacketQueues>().take::<InventoryTransaction>();
    inputs.extend(transactions.into_iter().filter_map(|(from, packet)| match packet.transaction.transaction_data {
        TransactionTransactionData::ItemUse(click) => Some((from, Input::Click(click))),
        _ => None,
    }));
    let (mut actions, mut clicks) = (PerTick::new(MAX_ACTIONS_PER_TICK), PerTick::new(MAX_CLICKS_PER_TICK));
    // A client names a finished break twice: as a block action and as an item use.
    let mut broken = HashSet::new();
    for (entity, input) in inputs {
        let Some(player) = in_world_identity(world, entity) else { continue };
        // See `movement::player_input`: until a teleport is followed, clicks are from the old place.
        if matches!(input, Input::Click(_)) && world.get::<Motion>(entity).is_some_and(Motion::awaits_teleport) {
            continue;
        }
        worlds::focus_on(world, entity);
        let mut request = Request { world: &mut *world, plugins: &mut *plugins, entity, player: &player };
        match input {
            Input::Actions(items) => {
                for item in items.iter().filter(|_| actions.allow(entity)) {
                    let pos = [item.position.x, item.position.y, item.position.z];
                    match item.action {
                        Action::StartBreak => request.punch(pos, u32::try_from(item.face).ok().and_then(Face::from_index)),
                        Action::PredictBreak if broken.insert((entity, pos)) => request.break_block(pos),
                        Action::AbortBreak => request.abort_break(),
                        _ => {}
                    }
                }
            }
            Input::Click(click) if clicks.allow(entity) => {
                let pos = [click.block_position.x, click.block_position.y, click.block_position.z];
                match click.action_type {
                    TransactionUseItemActionType::ClickBlock => request.click(pos, Face::from_index(u32::from(click.face)), &click),
                    TransactionUseItemActionType::BreakBlock if broken.insert((entity, pos)) => request.break_block(pos),
                    _ => {}
                }
            }
            Input::Click(_) => {}
        }
    }
}

/// One player's request while it is being answered.
struct Request<'a> {
    world: &'a mut World,
    plugins: &'a mut Plugins,
    entity: Entity,
    player: &'a Player,
}

impl Request<'_> {
    fn resync(&mut self, positions: &[BlockPos]) {
        send::resync(self.world, self.entity, positions);
    }

    /// The registry and the layer-0 block at `pos`.
    fn block(&self, site: &Site, [x, y, z]: BlockPos) -> Option<(&BlockRegistry, u32)> {
        level(self.world, site.layer).map(|level| (level.registry(), level.block(x, y, z)))
    }

    /// A hit that starts a break: viewers see fragments fly off the face, as from BDS.
    fn punch(&mut self, pos: BlockPos, face: Option<Face>) {
        let (Some(site), Some(face)) = (site(self.world, self.entity, pos), face) else { return };
        let Some((registry, id)) = self.block(&site, pos) else { return };
        let Some(state) = registry.get(id).filter(|state| site.mode != GameMode::Spectator && !state.is_air() && !state.is_liquid()) else { return };
        let (packet, hardness) = (send::punch_particles(registry, pos, face, id), state.mining);
        send::to_viewers(self.world, site.layer, pos, &[packet]);
        if site.mode == GameMode::Survival {
            mining::start(self.world, self.entity, site.layer, pos, &hardness);
        }
    }

    fn abort_break(&mut self) {
        if let Some(layer) = self.world.get::<InLayer>(self.entity).map(|layer| layer.0) {
            mining::abort(self.world, self.entity, layer);
        }
    }

    /// The client finished breaking a block. In survival the event starts refused unless the
    /// break was started and took its time (mining.rs); a handler may still allow it.
    fn break_block(&mut self, pos: BlockPos) {
        let above = Face::Up.neighbour(pos);
        let took_its_time = mining::finish(self.world, self.entity, pos);
        let Some(site) = site(self.world, self.entity, pos) else { return self.resync(&[pos, above]) };
        let Some((registry, id)) = self.block(&site, pos) else { return };
        let Some(state) = registry.get(id).filter(|state| !state.is_air() && !state.is_liquid()) else { return self.resync(&[pos, above]) };
        let (block, mining) = (send::describe(registry, id), state.mining);
        let by = mining::breaker(self.world, self.entity, &mining);
        let mut drops = Vec::new();
        if site.mode == GameMode::Survival {
            drops = loot::thrown(self.world, &block.name, &block.properties, &by);
            loot::add_entity_loot(self.world, site.layer, pos, &mut drops);
        }
        let allowed = site.mode == GameMode::Creative || site.mode == GameMode::Survival && took_its_time;
        let (event, player) = (BreakEvent::new(pos, block, drops, allowed), self.player);
        let event = self.plugins.offer(self.world, event, |handler, ctx, event| handler.on_break(ctx, player, event));
        match !event.is_cancelled() && self.remove(&site, pos) {
            true => {
                loot::spill(self.world, site.layer, pos, &event.drops);
                if site.mode == GameMode::Survival && mining.hardness > 0.0 {
                    crate::inventory::wear_tool_in_hand(self.world, self.entity);
                }
            }
            false => self.resync(&[pos, above]),
        }
    }

    /// Takes the block at `pos` out of the world: what is left is the water it stood in, or air.
    fn remove(&mut self, site: &Site, [x, y, z]: BlockPos) -> bool {
        let Some(level) = level(self.world, site.layer) else { return false };
        let (id, air) = (level.block(x, y, z), level.registry().air_id());
        let particles = send::destroy_particles(level.registry(), [x, y, z], id);
        doors::remove_other_half(self.world, site.layer, [x, y, z]);
        crate::containers::spill(self.world, site.layer, [x, y, z], site.mode == GameMode::Creative);
        if !send::set(self.world, site.layer, [x, y, z], air) {
            return false;
        }
        send::to_viewers(self.world, site.layer, [x, y, z], &[particles]);
        updates::settle(self.world, site.layer, [x, y, z]);
        true
    }

    /// A click on `face` of the block at `pos`: a placement if the player holds a block and the
    /// clicked block is not one a click uses, an interaction otherwise. The block is the one in
    /// the server's copy of the hand; in creative mode, where any block is to be had, and where
    /// the hand holds none, it is the one the client names.
    fn click(&mut self, pos: BlockPos, face: Option<Face>, click: &TransactionUseItem) {
        let Some(face) = face else { return self.resync(&[pos]) };
        let target = face.neighbour(pos);
        let Some(site) = site(self.world, self.entity, pos) else { return self.resync(&[pos, target]) };
        let Some((registry, clicked)) = self.block(&site, pos) else { return };
        let Some(name) = registry.get(clicked).filter(|state| !state.is_air()).map(|state| state.name) else { return self.resync(&[pos, target]) };
        // The clicked block's own use comes first, unless the player sneaks.
        let builds = site.sneaking || !is_used(name);
        if builds && self.held_item(site.mode, click).is_some_and(|held| self.use_bucket(&site, pos, face, &held)) {
            return;
        }
        let Some((registry, clicked)) = self.block(&site, pos) else { return };
        let in_hand = place::held_block(self.world, self.entity, registry, face).filter(|_| site.mode != GameMode::Creative);
        let (yaw, pitch) = self.world.get::<Position>(self.entity).map_or((0.0, 0.0), |position| (position.yaw, position.pitch));
        match in_hand.or_else(|| place::claimed_block(registry, &click.held_item, face)).filter(|_| builds) {
            Some(id) => self.place(&site, pos, &Click { face, height: click.click_pos.y, yaw, pitch }, id, in_hand.is_some()),
            None => self.interact(&site, pos, face, send::describe(registry, clicked)),
        }
    }

    /// The name of the item in the server's copy of the hand; in creative mode, where any item is
    /// to be had, the one the client names when the hand holds none.
    fn held_item(&self, mode: GameMode, click: &TransactionUseItem) -> Option<String> {
        let held = self.world.get::<crate::inventory::Inventory>(self.entity).and_then(|inventory| inventory.in_hand().as_ref()).map(|held| held.stack.name.clone());
        let claimed = || sapling_data::item_name(click.held_item.network_id.into()).filter(|_| mode == GameMode::Creative && click.held_item.count > 0).map(str::to_owned);
        held.or_else(claimed)
    }

    /// Allowed, the block is used ([`use_block`]) or, if it moves by hand, moved (doors.rs).
    /// Either way the client is shown the block and its neighbours as they are (a door is two
    /// blocks), which undoes what it predicted.
    fn interact(&mut self, site: &Site, pos: BlockPos, face: Face, block: Block) {
        let name = block.name.clone();
        let (event, player) = (InteractEvent::new(pos, face, block, site.mode != GameMode::Spectator), self.player);
        let event = self.plugins.offer(self.world, event, |handler, ctx, event| handler.on_interact(ctx, player, event));
        if !event.is_cancelled() && !use_block(self.world, self.entity, site.layer, pos, &name) {
            doors::use_by_hand(self.world, site.layer, pos, &name);
        }
        self.resync(&[pos, face.neighbour(pos), Face::Up.neighbour(pos), Face::Down.neighbour(pos)]);
    }
}
