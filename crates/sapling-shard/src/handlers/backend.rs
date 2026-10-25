//! What a [`sapling_api::ShardCtx`] asks of the shard, answered from the ECS world.

use std::any::TypeId;

use acacia_proto::packets::{SetDifficulty, UpdatePlayerGameType};
use bevy_ecs::prelude::*;
use sapling_api::{Backend, Block, BlockPos, BossBar, Difficulty, FormCallback, GameMode, Item, Permission, Player, PlayerId, Sidebar, Slot, State, Title};
use sapling_net::ConnId;

use super::Ctx;
use crate::commands::Access;
use crate::components::{Identity, Layer, Mode, Net, NetId, Players, Position, Settings, Tick, in_world};
use crate::inventory::Inventory;
use crate::plugins::{Deferred, PluginState};
use crate::{blocks, chat, clock, containers, forms, hud, inventory, join, movement, worlds};

impl Backend for Ctx<'_> {
    fn tick(&self) -> u64 {
        self.world.resource::<Tick>().0
    }

    fn player_count(&self) -> usize {
        self.world.resource::<Players>().in_world
    }

    fn players(&self) -> Vec<Player> {
        in_world(self.world).into_iter().filter_map(|entity| self.world.get::<Identity>(entity).map(|i| i.0.clone())).collect()
    }

    fn position(&self, player: PlayerId) -> Option<sapling_api::Position> {
        let position = self.world.get::<Position>(self.entity(player)?)?;
        Some(sapling_api::Position::new(position.feet, position.yaw, position.pitch))
    }

    fn spawn(&self) -> [f32; 3] {
        self.layer().spawn
    }

    fn set_spawn(&mut self, feet: [f32; 3]) {
        let layer = worlds::current(self.world);
        let Some(mut layer) = self.world.get_mut::<Layer>(layer).filter(|_| feet.iter().all(|c| c.is_finite())) else { return };
        let block = feet.map(|c| c.floor() as i32);
        layer.spawn = feet;
        layer.world.level_mut().spawn = Some(block);
        // TODO: only the players of the world in scope.
        chat::send_to_all(self.world, &sapling_net::encode(&sapling_data::world_spawn(block)));
    }

    fn time(&self) -> i64 {
        self.layer().world.level().time
    }

    fn set_time(&mut self, time: i64) {
        let layer = worlds::current(self.world);
        clock::set(self.world, layer, time);
    }

    fn difficulty(&self) -> Difficulty {
        self.world.resource::<Settings>().0.difficulty
    }

    fn set_difficulty(&mut self, difficulty: Difficulty) {
        self.world.resource_mut::<Settings>().0.difficulty = difficulty;
        chat::send_to_all(self.world, &sapling_net::encode(&SetDifficulty { difficulty: difficulty as u32 }));
    }

    fn block(&self, pos: BlockPos) -> Option<Block> {
        blocks::block_at(self.world, worlds::current(self.world), pos)
    }

    fn power_at(&self, pos: BlockPos) -> u8 {
        blocks::power_at(self.world, worlds::current(self.world), pos)
    }

    fn set_block(&mut self, pos: BlockPos, block: &Block) -> bool {
        let layer = worlds::current(self.world);
        blocks::put_block(self.world, layer, pos, block).is_some()
    }

    fn update_block(&mut self, pos: BlockPos, block: &Block) -> bool {
        let layer = worlds::current(self.world);
        blocks::update_block(self.world, layer, pos, block)
    }

    fn break_block(&mut self, pos: BlockPos) -> bool {
        let layer = worlds::current(self.world);
        blocks::break_block(self.world, layer, pos)
    }

    fn set_container_item(&mut self, pos: BlockPos, slot: usize, item: Option<&Item>) -> bool {
        let layer = worlds::current(self.world);
        let stack = match item.map(|item| inventory::stored(item, self.layer().world.registry())) {
            Some(None) => return false,
            stack => stack.flatten(),
        };
        let put = |world: &mut World, slots: &mut Vec<Option<inventory::Held>>| {
            let fits = slot < slots.len();
            if fits {
                slots[slot] = stack.map(|stack| containers::new_held(world, stack));
            }
            fits
        };
        containers::with_slots(self.world, layer, pos, put).unwrap_or(false)
    }

    fn container_size(&mut self, pos: BlockPos) -> Option<usize> {
        let layer = worlds::current(self.world);
        containers::with_slots(self.world, layer, pos, |_, slots| slots.len())
    }

    fn kick(&mut self, player: PlayerId, message: &str) {
        self.world.resource::<Net>().0.kick(ConnId::new(player.0), message);
    }

    fn broadcast(&mut self, message: &str) {
        chat::send_to_all(self.world, &chat::raw_packet(message));
    }

    fn send_message(&mut self, player: PlayerId, message: &str) {
        if let Some(mut out) = self.out(player) {
            out.0.push(chat::raw_packet(message));
        }
    }

    fn send_translation(&mut self, player: PlayerId, key: &str, parameters: &[String]) {
        if let Some(mut out) = self.out(player) {
            out.0.push(chat::json_packet(vec![chat::translation(key, parameters)]));
        }
    }

    fn teleport(&mut self, player: PlayerId, feet: [f32; 3]) {
        if let Some(entity) = self.entity(player).filter(|_| feet.iter().all(|c| c.is_finite())) {
            movement::teleport(self.world, entity, feet);
        }
    }

    fn look(&mut self, player: PlayerId, yaw: f32, pitch: f32) {
        if let Some(mut position) = self.entity(player).filter(|_| yaw.is_finite() && pitch.is_finite()).and_then(|entity| self.world.get_mut::<Position>(entity)) {
            (position.yaw, position.head_yaw, position.pitch) = (yaw, yaw, pitch);
        }
    }

    fn dragon_fight(&self) -> Option<sapling_api::DragonFight> {
        crate::dragon::describe(self.world, crate::worlds::current(self.world))
    }

    fn permission(&self, player: PlayerId) -> Option<Permission> {
        self.world.get::<Access>(self.entity(player)?).map(|access| access.permission)
    }

    fn set_permission(&mut self, player: PlayerId, permission: Permission) {
        // The command list follows at the end of the tick (commands::announce).
        if let Some(mut access) = self.entity(player).and_then(|entity| self.world.get_mut::<Access>(entity)) {
            access.permission = permission;
        }
    }

    fn game_mode(&self, player: PlayerId) -> Option<GameMode> {
        self.world.get::<Mode>(self.entity(player)?).map(|mode| mode.0)
    }

    fn set_game_mode(&mut self, player: PlayerId, mode: GameMode) {
        let Some(entity) = self.entity(player) else { return };
        let (tick, Some(id)) = (self.tick(), self.world.get::<NetId>(entity).copied()) else { return };
        let Some(mut current) = self.world.get_mut::<Mode>(entity) else { return };
        current.0 = mode;
        // BDS's `/gamemode` sends this to the player itself; the others learn whom to show how.
        let packet = UpdatePlayerGameType { gamemode: join::wire_game_mode(mode), player_unique_id: id.0 as i64, tick };
        chat::send_to_all(self.world, &sapling_net::encode(&packet));
    }

    fn item(&self, player: PlayerId, slot: Slot) -> Option<Item> {
        inventory::item_at(self.world, self.entity(player)?, slot)
    }

    fn set_item(&mut self, player: PlayerId, slot: Slot, item: Option<&Item>) -> bool {
        let Some(entity) = self.entity(player) else { return false };
        inventory::set_item(self.world, entity, slot, item)
    }

    fn give(&mut self, player: PlayerId, item: &Item) -> u8 {
        let stack = self.entity(player).zip(inventory::stored(item, self.layer().world.registry()));
        stack.and_then(|(entity, stack)| inventory::give_and_pop(self.world, entity, &stack)).unwrap_or(item.count)
    }

    fn clear_inventory(&mut self, player: PlayerId) -> u32 {
        let Some(entity) = self.entity(player) else { return 0 };
        inventory::change(self.world, entity, |inventory| inventory.clear()).unwrap_or(0)
    }

    fn held_slot(&self, player: PlayerId) -> Option<u8> {
        self.world.get::<Inventory>(self.entity(player)?).map(|inventory| inventory.selected)
    }

    fn send_form(&mut self, player: PlayerId, json: String, on_answer: FormCallback) {
        if let Some(entity) = self.entity(player) {
            forms::send(self.world, entity, json, on_answer);
        }
    }

    fn set_sidebar(&mut self, player: PlayerId, sidebar: Option<&Sidebar>) {
        if let Some(entity) = self.entity(player) {
            hud::set_sidebar(self.world, entity, sidebar);
        }
    }

    fn set_boss_bar(&mut self, player: PlayerId, bar: Option<&BossBar>) {
        if let Some(entity) = self.entity(player) {
            hud::set_boss_bar(self.world, entity, bar);
        }
    }

    fn send_title(&mut self, player: PlayerId, title: &Title) {
        if let Some(mut out) = self.out(player) {
            hud::send_title(&mut out, title);
        }
    }

    fn send_actionbar(&mut self, player: PlayerId, text: &str) {
        if let Some(mut out) = self.out(player) {
            hud::send_actionbar(&mut out, text);
        }
    }

    fn send_toast(&mut self, player: PlayerId, title: &str, message: &str) {
        if let Some(mut out) = self.out(player) {
            hud::send_toast(&mut out, title, message);
        }
    }

    fn state(&mut self, key: TypeId, init: &dyn Fn() -> State) -> &mut State {
        self.world.resource_mut::<PluginState>().into_inner().0.entry(key).or_insert_with(init)
    }

    fn defer(&mut self, change: sapling_api::ListenerChange) {
        self.world.resource::<Deferred>().push(change);
    }
}
