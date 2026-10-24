//! A player's requests of blocks: breaks and clicks, checked, offered to the handlers and then
//! done or undone (placing: place.rs; buckets: buckets.rs).

use std::collections::HashSet;

use acacia_proto::packets::{InventoryTransaction, PlayerAuthInput, PlayerAuthInputBlockActionItem};
use acacia_proto::types::{Action, TransactionTransactionData, TransactionTransactionDataItemUseOnEntityActionType as OnEntity, TransactionUseItem, TransactionUseItemActionType};
use bevy_ecs::prelude::*;
use sapling_api::{Block, BlockPos, BreakEvent, Face, GameMode, InteractEvent, Player};
use sapling_world::BlockRegistry;

use super::facing::{self, Click};
use super::{after_break, doors, level, loot, mining, place, pots, reaches, send, signs, updates};
use crate::components::{InLayer, Mode, PacketQueues, PerTick, Position, in_world_identity};
use crate::movement::Motion;
use crate::{actions, plugins, worlds};

/// Block actions looked at per player per tick; a client holding a break sends two or three.
const MAX_ACTIONS_PER_TICK: usize = 16;
/// Block clicks looked at per player per tick; more are dropped without an answer.
const MAX_CLICKS_PER_TICK: usize = 4;
/// Blocks a click uses rather than builds on, by the end of their name. TODO: a per-block flag.
const USED: &[&str] = &["daylight_detector", "daylight_detector_inverted", 
    "door", "button", "lever", "fence_gate", "chest", "barrel", "shulker_box", "furnace", "smoker", "crafting_table", "anvil", "enchanting_table", "hopper",
    "dropper", "dispenser", "_sign", "noteblock", "repeater", "comparator", "brewing_stand", "beacon", "loom", "grindstone", "stonecutter_block", ":bed",
    "cartography_table", "smithing_table", "flower_pot", "jukebox", ":frame", "glow_frame", "decorated_pot",
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
pub(super) struct Site {
    pub layer: Entity,
    pub mode: GameMode,
    pub sneaking: bool,
}

/// The site of a request at `pos`, unless it is outside the world's height, in a column that is
/// not loaded or further from the player's eyes than its game mode reaches.
pub(super) fn site(world: &World, player: Entity, pos: BlockPos) -> Option<Site> {
    let (layer, mode) = (world.get::<InLayer>(player)?.0, world.get::<Mode>(player)?.0);
    let level = level(world, layer)?;
    let known = level.in_height(pos[1]) && level.is_loaded(send::column(pos));
    let sneaking = world.get::<Motion>(player).is_some_and(|motion| motion.pose.sneaking);
    (known && reaches(world, player, pos)).then_some(Site { layer, mode, sneaking })
}

fn is_used(name: &str) -> bool {
    USED.iter().any(|end| name.ends_with(end))
}

/// What a click on the block `name` at `pos` does beyond moving it by hand: a container opens,
/// a bed is slept in, a sign is written on, a pot takes or gives a plant, a campfire takes food.
/// False when it does none of these.
fn use_block(world: &mut World, player: Entity, layer: Entity, pos: BlockPos, name: &str) -> bool {
    let uses: [fn(&mut World, Entity, Entity, BlockPos, &str) -> bool; 8] = [
        crate::containers::open,
        crate::sleep::lie_down,
        signs::edit_by_click,
        pots::use_pot,
        crate::campfires::cook,
        super::handled::use_by_hand,
        super::jukebox::use_jukebox,
        super::holders::use_holder,
    ];
    uses.iter().any(|used| used(world, player, layer, pos, name))
}

/// Runs this tick's block requests through the checks and the handlers.
pub(crate) fn dispatch(world: &mut World) {
    let mut inputs = std::mem::take(&mut world.resource_mut::<BlockInputs>().0);
    let transactions = world.resource_mut::<PacketQueues>().take::<InventoryTransaction>();
    let (mut hits, mut uses) = (Vec::new(), Vec::new());
    inputs.extend(transactions.into_iter().filter_map(|(from, packet)| match packet.transaction.transaction_data {
        TransactionTransactionData::ItemUse(click) => Some((from, Input::Click(click))),
        TransactionTransactionData::ItemRelease(_) => {
            crate::vitals::eating::stop(world, from);
            None
        }
        TransactionTransactionData::ItemUseOnEntity(on) => {
            let clicks = if on.action_type == OnEntity::Attack { &mut hits } else { &mut uses };
            clicks.push((from, on.entity_runtime_id));
            None
        }
        _ => None,
    }));
    let hits = actions::allowed_hits(world, hits);
    crate::creatures::attacked(world, hits);
    actions::interacted(world, uses);
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
        let mut request = Request { world: &mut *world, entity, player: &player };
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
                    TransactionUseItemActionType::ClickAir if actions::may_use_item(request.world, entity, request.player) => {
                        if !crate::eyes::throw(request.world, entity) {
                            crate::vitals::eating::start(request.world, entity);
                        }
                    }
                    _ => {}
                }
            }
            Input::Click(_) => {}
        }
    }
}

/// One player's request while it is being answered.
pub(super) struct Request<'a> {
    pub world: &'a mut World,
    pub entity: Entity,
    pub player: &'a Player,
}

impl Request<'_> {
    pub(super) fn resync(&mut self, positions: &[BlockPos]) {
        send::resync(self.world, self.entity, positions);
    }

    /// The registry and the layer-0 block at `pos`.
    fn block(&self, site: &Site, [x, y, z]: BlockPos) -> Option<(&BlockRegistry, u32)> {
        level(self.world, site.layer).map(|level| (level.registry(), level.block(x, y, z)))
    }

    /// A hit that starts a break: viewers see fragments fly off the face, as from BDS.
    fn punch(&mut self, pos: BlockPos, face: Option<Face>) {
        let (Some(site), Some(face)) = (site(self.world, self.entity, pos), face) else { return };
        if site.mode != GameMode::Spectator && super::holders::hit(self.world, site.layer, pos) {
            return;
        }
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
        let name = block.name.clone();
        let by = mining::breaker(self.world, self.entity, &mining);
        let mut drops = Vec::new();
        if site.mode == GameMode::Survival {
            drops = loot::thrown(self.world, &block.name, &block.properties, &by);
            loot::add_entity_loot(self.world, site.layer, pos, &mut drops);
        }
        let allowed = site.mode == GameMode::Creative || site.mode == GameMode::Survival && took_its_time;
        let event = plugins::offer(self.world,BreakEvent::new(self.player.clone(), pos, block, drops, allowed));
        match !event.is_cancelled() && self.remove(&site, pos) {
            true => {
                loot::spill(self.world, site.layer, pos, &event.drops);
                if site.mode == GameMode::Survival {
                    after_break::leave(self.world, site.layer, pos, &name, &by);
                }
                if site.mode == GameMode::Survival && mining.hardness > 0.0 {
                    crate::inventory::wear_tool_in_hand(self.world, self.entity);
                }
                if by.harvests {
                    crate::vitals::tire(self.world, self.entity, crate::vitals::BREAK_TIRES);
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
        // BDS sends the fragments before the block; the cell is loaded and not air, so it changes.
        send::to_viewers(self.world, site.layer, [x, y, z], &[particles]);
        if !send::set(self.world, site.layer, [x, y, z], air) {
            return false;
        }
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
        let held = self.held_item(site.mode, click).filter(|_| builds);
        if held.as_ref().is_some_and(|held| self.use_tool(&site, pos, face, held) || self.stack(&site, pos, face, held)) {
            return self.resync(&[pos, target]);
        }
        if held.is_some_and(|held| self.use_bucket(&site, pos, face, &held)) {
            return;
        }
        let Some((registry, clicked)) = self.block(&site, pos) else { return };
        let in_hand = place::held_block(self.world, self.entity, registry, face).filter(|_| site.mode != GameMode::Creative);
        let (yaw, feet) = self.world.get::<Position>(self.entity).map_or((0.0, [0.0; 3]), |position| (position.yaw, position.feet));
        let is_near = (feet[0] - target[0] as f32 - 0.5).abs() < 2.0 && (feet[2] - target[2] as f32 - 0.5).abs() < 2.0;
        let eyes_over = is_near.then(|| feet[1] + facing::EYES - target[1] as f32);
        match in_hand.or_else(|| place::claimed_block(registry, &click.held_item, face)).filter(|_| builds) {
            Some(id) => self.place(&site, pos, &Click { face, height: click.click_pos.y, yaw, eyes_over }, id, in_hand.is_some()),
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

    /// Allowed, the block is used ([`use_block`]) or, if it moves by hand, moved (doors.rs). A
    /// click that does neither shows the client the block and its neighbours as they are (a door
    /// is two blocks), which undoes what it predicted; BDS sent nothing more after a lever, a door,
    /// a chest or a bed it used, a block in hand or not (research/captures/bds-held-use.txt).
    fn interact(&mut self, site: &Site, pos: BlockPos, face: Face, block: Block) {
        let name = block.name.clone();
        let event = plugins::offer(self.world,InteractEvent::new(self.player.clone(), pos, face, block, site.mode != GameMode::Spectator));
        let used = !event.is_cancelled() && (use_block(self.world, self.entity, site.layer, pos, &name) || doors::use_by_hand(self.world, site.layer, pos, &name));
        if !used {
            self.resync(&[pos, face.neighbour(pos), Face::Up.neighbour(pos), Face::Down.neighbour(pos)]);
        }
    }
}
