//! What the dragon probe prints of the packets it is sent (see bds_dragon_probe.rs).

use std::collections::HashMap;
use std::time::{Duration, Instant};

use acacia_bot::proto::packets::{AddEntity, MoveEntity, MoveEntityDelta, MovePlayer, MovePlayerMode, RemoveEntity, SetEntityData, SetEntityMotion, UpdateAttributes, UpdateBlock, UpdateSubchunkBlocks};
use acacia_bot::proto::types::{MetadataDictionaryItemKey, MetadataDictionaryItemValue, MetadataDictionaryItemValueDefault};
use acacia_bot::proto::{Packet, RawPacket};
use acacia_bot::{Bot, BotEvent};
use acacia_world::BlockRegistry;

/// Printed in full; other packets are tallied.
const DETAILED: &[&str] = &[
    "ChangeDimension", "PlayStatus", "SetSpawnPosition", "Respawn", "CommandOutput", "SetPlayerGameType", "CorrectPlayerMovePrediction", "LevelEvent", "LevelSoundEvent", "LevelEventGeneric",
    "EntityEvent", "SetHealth", "PlayerAction", "BossEvent", "ShowCredits", "SpawnParticleEffect", "AddItemEntity", "TakeItemEntity", "BlockActorData", "Animate", "AddPlayer",
    "RemoveEntity", "GameRulesChanged", "Disconnect", "SetEntityLink", "MobEffect", "ActorEvent", "PlaySound", "StopSound", "Explode", "BlockEntityData",
];
const CHARS: usize = 8000;
const ORB: &str = "minecraft:xp_orb";
/// Kinds whose every packet is printed.
const WATCHED: &[&str] = &["minecraft:ender_dragon", "minecraft:ender_crystal", "minecraft:dragon_fireball", "minecraft:area_effect_cloud", ORB];

pub struct Run {
    start: Instant,
    tally: Vec<(String, usize)>,
    /// Kinds by runtime and by unique id, as announced.
    kinds: HashMap<u64, String>,
    uniques: HashMap<i64, String>,
}

fn block_name(hash: u32) -> String {
    let registry = BlockRegistry::vanilla();
    registry.runtime_id_from_hash(hash).and_then(|id| registry.get(id)).map_or("?".into(), |s| format!("{}[{}]", s.name.trim_start_matches("minecraft:"), s.properties))
}

fn experience(entity: &AddEntity) -> Option<i32> {
    entity.metadata.iter().find(|item| item.key == MetadataDictionaryItemKey::ExperienceValue).and_then(|item| match item.value {
        MetadataDictionaryItemValue::Default(MetadataDictionaryItemValueDefault::Int(value)) => Some(value),
        _ => None,
    })
}

impl Run {
    pub fn new(start: Instant) -> Self {
        Self { start, tally: Vec::new(), kinds: HashMap::new(), uniques: HashMap::new() }
    }

    pub fn ms(&self) -> u128 {
        self.start.elapsed().as_millis()
    }

    fn flush(&mut self) {
        if !self.tally.is_empty() {
            let shown: Vec<String> = self.tally.drain(..).map(|(name, n)| format!("{name} x{n}")).collect();
            println!("{:>7} actor   .. {}", self.ms(), shown.join(", "));
        }
    }

    fn count(&mut self, name: String) {
        match self.tally.iter_mut().find(|(seen, _)| *seen == name) {
            Some((_, n)) => *n += 1,
            None => self.tally.push((name, 1)),
        }
    }

    fn print(&mut self, line: String) {
        self.flush();
        println!("{:>7} actor   {line}", self.ms());
    }

    fn watched(&self, id: u64, actor: &Bot) -> bool {
        id == actor.state().player.runtime_entity_id || self.kinds.get(&id).is_some_and(|kind| WATCHED.contains(&kind.as_str()))
    }

    fn short(&self, id: u64) -> String {
        self.kinds.get(&id).map_or("?".into(), |kind| kind.trim_start_matches("minecraft:").to_owned())
    }

    /// Prints one packet of the actor's.
    fn actor_packet(&mut self, actor: &Bot, packet: &RawPacket) {
        let name = acacia_bot::proto::packet_name(packet.id).unwrap_or("?");
        let full = || dump(packet);
        match packet.id {
            MoveEntityDelta::ID | MoveEntity::ID => {
                let id = if packet.id == MoveEntity::ID { packet.decode::<MoveEntity>().map(|p| p.runtime_entity_id) } else { packet.decode::<MoveEntityDelta>().map(|p| p.runtime_entity_id) };
                let Some(entity) = id.ok().filter(|id| self.watched(*id, actor)).and_then(|id| actor.state().entities.get(id)) else { return self.count(name.into()) };
                let line = format!("{} {} {:.5} {:.5} {:.5} p {:.2} y {:.2} h {:.2}{}", if packet.id == MoveEntity::ID { "DA" } else { "D" }, self.short(entity.runtime_id), entity.position.x, entity.position.y, entity.position.z, entity.pitch, entity.yaw, entity.head_yaw, if entity.on_ground { " ground" } else { "" });
                self.print(line);
            }
            SetEntityMotion::ID => match packet.decode::<SetEntityMotion>() {
                Ok(motion) if self.watched(motion.runtime_entity_id, actor) => {
                    let v = &motion.velocity;
                    let line = format!("DV {} {:.5} {:.5} {:.5}", self.short(motion.runtime_entity_id), v.x, v.y, v.z);
                    self.print(line);
                }
                _ => self.count(name.into()),
            },
            SetEntityData::ID => match packet.decode::<SetEntityData>() {
                Ok(data) if self.watched(data.runtime_entity_id, actor) => self.print(format!("SetEntityData {} {}", self.short(data.runtime_entity_id), full())),
                _ => self.count(name.into()),
            },
            UpdateAttributes::ID => match packet.decode::<UpdateAttributes>() {
                Ok(update) if self.watched(update.runtime_entity_id, actor) => {
                    let shown: Vec<String> = update.attributes.iter().map(|a| format!("{}={}/{}", a.name, a.current, a.max)).collect();
                    self.print(format!("UpdateAttributes {} {}", self.short(update.runtime_entity_id), shown.join(" ")));
                }
                _ => self.count(name.into()),
            },
            AddEntity::ID => {
                let Ok(entity) = packet.decode::<AddEntity>() else { return self.print("AddEntity <undecoded>".into()) };
                self.kinds.insert(entity.runtime_id, entity.entity_type.clone());
                self.uniques.insert(entity.unique_id, entity.entity_type.clone());
                if entity.entity_type == ORB {
                    let (p, v) = (&entity.position, &entity.velocity);
                    return self.print(format!("XP {} {:.4} {:.4} {:.4} v {:.4} {:.4} {:.4} worth {:?}", entity.runtime_id, p.x, p.y, p.z, v.x, v.y, v.z, experience(&entity)));
                }
                self.print(format!("AddEntity {}", full()));
            }
            RemoveEntity::ID => {
                let unique = packet.decode::<RemoveEntity>().map_or(0, |p| p.entity_id_self);
                match self.uniques.get(&unique).map(String::as_str) {
                    kind => self.print(format!("RemoveEntity {unique} {}", kind.unwrap_or("?"))),
                }
            }
            UpdateBlock::ID => {
                let Ok(update) = packet.decode::<UpdateBlock>() else { return };
                let p = &update.position;
                self.print(format!("UpdateBlock {} {} {} layer {} flags {:?} {}", p.x, p.y, p.z, update.layer, update.flags, block_name(update.block_runtime_id)));
            }
            UpdateSubchunkBlocks::ID => {
                let Ok(update) = packet.decode::<UpdateSubchunkBlocks>() else { return };
                let cells = |blocks: &[acacia_bot::proto::types::BlockUpdate]| blocks.iter().map(|b| format!("{} {} {} {} f{}", b.position.x, b.position.y, b.position.z, block_name(b.runtime_id), b.flags)).collect::<Vec<_>>().join("; ");
                self.print(format!("UpdateSubchunkBlocks at {} {} {}: {} | extra: {}", update.x, update.y, update.z, cells(&update.blocks), cells(&update.extra)));
            }
            MovePlayer::ID if packet.decode::<MovePlayer>().is_ok_and(|moved| moved.mode == MovePlayerMode::Normal) => self.count("MovePlayer[normal]".into()),
            MovePlayer::ID => self.print(format!("MovePlayer {}", full())),
            _ if DETAILED.contains(&name) => self.print(format!("{name} {}", full())),
            _ => self.count(name.into()),
        }
    }

    /// What the watcher hears that tells of the fight.
    pub fn watcher_event(&mut self, event: Option<BotEvent>) {
        let Some(BotEvent::Packet(packet)) = event else { return };
        let name = acacia_bot::proto::packet_name(packet.id).unwrap_or("?");
        if ["BossEvent", "ShowCredits", "Text", "LevelSoundEvent", "LevelEvent"].contains(&name) {
            self.flush();
            println!("{:>7} watcher {name} {}", self.ms(), dump(&packet));
        }
    }

    /// Prints what both bots are sent for `wait`; false once the actor is gone.
    pub async fn settle(&mut self, actor: &mut Bot, watcher: &mut Bot, wait: Duration) -> bool {
        let end = tokio::time::Instant::now() + wait;
        loop {
            tokio::select! {
                event = actor.next() => match event {
                    Some(BotEvent::Packet(packet)) => self.actor_packet(actor, &packet),
                    Some(BotEvent::Disconnected(reason)) => {
                        self.print(format!("disconnected: {reason:?}"));
                        return false;
                    }
                    Some(_) => {}
                    None => return false,
                },
                event = watcher.next() => self.watcher_event(event),
                _ = tokio::time::sleep_until(end) => break,
            }
        }
        self.flush();
        true
    }
}

fn dump(packet: &RawPacket) -> String {
    crate::dump::dump(packet).unwrap_or_default().chars().take(CHARS).collect()
}

/// Every entity the actor's client tracks: kind, id, wire position, health and flags.
pub fn entities(actor: &Bot) {
    let mut all: Vec<_> = actor.state().entities.iter().filter(|entity| !entity.is_player()).collect();
    all.sort_by_key(|entity| entity.runtime_id);
    println!("## entities: {}", all.len());
    for entity in all {
        let p = &entity.position;
        println!("##   {} {} at {:.3} {:.3} {:.3} yaw {:.1} health {:?}", entity.kind, entity.runtime_id, p.x, p.y, p.z, entity.yaw, entity.health());
    }
}
