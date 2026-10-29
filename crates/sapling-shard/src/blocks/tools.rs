//! What the item in a hand does to the block it is clicked on: a hoe tills, a shovel treads a
//! path, an axe strips and
//! scrapes, honeycomb waxes, shears carve, a bottle and a bucket empty and fill
//! (docs/research/block-uses.md).
// TODO: no handler is asked. Potions, dyes and powder snow in cauldrons are cauldrons.rs's.

use acacia_proto::packets::{LevelEvent, LevelEventEvent};
use acacia_proto::types::Vec3f;
use bytes::Bytes;
use sapling_api::{BlockPos, Face, GameMode, Item};
use sapling_world::BlockRegistry;

use super::copper::Copper;
use super::{Request, Site, level, send, with_property};
use crate::{drops, inventory};

/// What the cauldron's events carry: the water's colour.
const WATER: i32 = -1_087_090_343;
const CAULDRON_FULL: u8 = 6;
const BOTTLEFUL: u8 = 2;
/// A cauldron's lava filled and taken: events the protocol crate has no names for.
const FILL_LAVA: LevelEventEvent = LevelEventEvent::Unknown(3512);
const TAKE_LAVA: LevelEventEvent = LevelEventEvent::Unknown(3513);
const HONEY_FULL: &str = "5";
const ANCHOR_FULL: u8 = 4;

/// What the hand pays for a use, outside creative.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Cost {
    Wear,
    Spend,
    Becomes(&'static str),
}

/// Where a change is told from.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Told {
    /// A sound, with the new block in it or not.
    Sound(&'static str, [f32; 3], bool),
    /// A sound naming the block `id` there was.
    SoundOf(&'static str, [f32; 3], u32),
    Event(LevelEventEvent, [f32; 3], i32),
}

#[derive(Debug, PartialEq)]
struct Change {
    id: u32,
    told: Told,
    /// Stacks of one thrown out of the block.
    leaves: (&'static str, u8),
    cost: Cost,
}

const MIDDLE: [f32; 3] = [0.5; 3];
const CORNER: [f32; 3] = [0.0; 3];
const FAR_CORNER: [f32; 3] = [1.0; 3];

/// What clicking `held` on `face` of the block `id` makes of it.
fn change(registry: &BlockRegistry, held: &str, id: u32, face: Face) -> Option<Change> {
    let state = registry.get(id)?;
    let (bare, properties) = (state.name.strip_prefix("minecraft:")?, state.properties);
    let get = |key: &str| properties.split(',').filter_map(|pair| pair.split_once('=')).find(|(k, _)| *k == key).map(|(_, value)| value);
    let set = |key: &str, value: &str| with_property(registry, id, key, value);
    let sound = |name| Told::Sound(name, MIDDLE, false);
    let done = |id, told, cost| Some(Change { id, told, leaves: ("", 0), cost });
    let event = |event| Told::Event(event, MIDDLE, 0);
    let is_hive = matches!(bare, "beehive" | "bee_nest") && get("honey_level") == Some(HONEY_FULL);
    let level: u8 = get("fill_level").and_then(|level| level.parse().ok()).unwrap_or(0);
    let surface = |level: u8| [0.5, 0.375 + f32::from(level) / 8.0, 0.5];
    match held.strip_prefix("minecraft:")? {
        // research/captures/bds-bkuses.txt: dirt and hanging roots, as the Java game.
        hoe if hoe.ends_with("_hoe") && bare == "dirt_with_roots" => {
            Some(Change { id: registry.find("minecraft:dirt", "")?, told: Told::Sound("item.use.on", MIDDLE, true), leaves: ("minecraft:hanging_roots", 1), cost: Cost::Wear })
        }
        hoe if hoe.ends_with("_hoe") => {
            let tilled = match bare {
                "dirt" | "grass_block" | "grass_path" => registry.find("minecraft:farmland", "moisturized_amount=0"),
                "coarse_dirt" => registry.find("minecraft:dirt", ""),
                _ => None,
            };
            done(tilled?, Told::Sound("item.use.on", MIDDLE, true), Cost::Wear)
        }
        shovel if shovel.ends_with("_shovel") && bare.ends_with("campfire") && get("extinguished") == Some("0") => done(set("extinguished", "1")?, Told::Sound("extinguish.fire", CORNER, false), Cost::Wear),
        // research/captures/bds-bkuses.txt: a layer of snow, shovelled off, leaves a snowball.
        shovel if shovel.ends_with("_shovel") && bare == "snow_layer" && get("height") == Some("0") => {
            Some(Change { id: registry.air_id(), told: Told::SoundOf("item.use.on", MIDDLE, id), leaves: ("minecraft:snowball", 1), cost: Cost::Wear })
        }
        shovel if shovel.ends_with("_shovel") && matches!(bare, "grass_block" | "dirt" | "coarse_dirt" | "podzol" | "mycelium" | "dirt_with_roots") => {
            done(registry.find("minecraft:grass_path", "")?, Told::Sound("item.use.on", MIDDLE, true), Cost::Wear)
        }
        axe if axe.ends_with("_axe") => {
            match Copper::of(bare) {
                Some(copper) if copper.waxed => return done(Copper { waxed: false, ..copper }.state(registry, id)?, event(LevelEventEvent::ParticleWaxOff), Cost::Wear),
                Some(copper) if copper.stage > 0 => return done(Copper { stage: copper.stage - 1, ..copper }.state(registry, id)?, event(LevelEventEvent::ParticleScrape), Cost::Wear),
                _ => {}
            }
            let stripped = registry.find(&format!("minecraft:stripped_{bare}"), properties).filter(|_| !bare.starts_with("stripped_"))?;
            done(stripped, Told::Sound("item.use.on", MIDDLE, true), Cost::Wear)
        }
        "honeycomb" => {
            let copper = Copper::of(bare).filter(|copper| !copper.waxed)?;
            done(Copper { waxed: true, ..copper }.state(registry, id)?, event(LevelEventEvent::ParticleWaxOn), Cost::Spend)
        }
        "shears" if bare == "pumpkin" => {
            // Carved from above it faced south whichever way the carver looked; that a side's
            // carving faces out of that side is the Java game's rule, not tried on BDS.
            let faces = match face {
                Face::North => "north",
                Face::East => "east",
                Face::West => "west",
                _ => "south",
            };
            let carved = registry.find("minecraft:carved_pumpkin", &format!("minecraft:cardinal_direction={faces}"))?;
            Some(Change { id: carved, told: sound("pumpkin.carve"), leaves: ("minecraft:pumpkin_seeds", 1), cost: Cost::Wear })
        }
        "shears" if is_hive => Some(Change { id: set("honey_level", "0")?, told: Told::Sound("block.beehive.shear", FAR_CORNER, false), leaves: ("minecraft:honeycomb", 3), cost: Cost::Wear }),
        "glass_bottle" if is_hive => done(set("honey_level", "0")?, Told::Sound("bucket.fill.water", FAR_CORNER, false), Cost::Becomes("minecraft:honey_bottle")),
        "glass_bottle" if bare == "cauldron" && get("cauldron_liquid") == Some("water") && level >= BOTTLEFUL => {
            done(set("fill_level", &(level - BOTTLEFUL).to_string())?, Told::Event(LevelEventEvent::CauldronTakeWater, surface(level), WATER), Cost::Becomes("minecraft:potion"))
        }
        "water_bucket" if bare == "cauldron" && level < CAULDRON_FULL => {
            let full = registry.find("minecraft:cauldron", &format!("cauldron_liquid=water,fill_level={CAULDRON_FULL}"))?;
            done(full, Told::Event(LevelEventEvent::CauldronFillWater, surface(CAULDRON_FULL), WATER), Cost::Becomes("minecraft:bucket"))
        }
        "bucket" if bare == "cauldron" && level == CAULDRON_FULL => {
            let (event, filled) = if get("cauldron_liquid") == Some("lava") { (TAKE_LAVA, "minecraft:lava_bucket") } else { (LevelEventEvent::CauldronTakeWater, "minecraft:water_bucket") };
            let empty = registry.find("minecraft:cauldron", "cauldron_liquid=water,fill_level=0")?;
            done(empty, Told::Event(event, surface(CAULDRON_FULL), WATER), Cost::Becomes(filled))
        }
        "lava_bucket" if bare == "cauldron" && level == 0 => {
            let full = registry.find("minecraft:cauldron", &format!("cauldron_liquid=lava,fill_level={CAULDRON_FULL}"))?;
            done(full, Told::Event(FILL_LAVA, surface(CAULDRON_FULL), WATER), Cost::Becomes("minecraft:bucket"))
        }
        "potion" if bare == "cauldron" && get("cauldron_liquid") == Some("water") && level < CAULDRON_FULL => {
            let fuller = (level + BOTTLEFUL).min(CAULDRON_FULL);
            done(set("fill_level", &fuller.to_string())?, Told::Event(LevelEventEvent::CauldronFillWater, surface(fuller), WATER), Cost::Becomes("minecraft:glass_bottle"))
        }
        "potion" if matches!(bare, "dirt" | "coarse_dirt" | "dirt_with_roots") => done(registry.find("minecraft:mud", "")?, Told::Sound("bottle.empty", CORNER, false), Cost::Becomes("minecraft:glass_bottle")),
        candle if candle.ends_with("candle") && bare == "cake" && get("bite_counter") == Some("0") => {
            done(registry.find(&format!("minecraft:{candle}_cake"), "lit=0")?, Told::Sound("cake.add_candle", CORNER, false), Cost::Spend)
        }
        "ender_eye" if bare == "end_portal_frame" && get("end_portal_eye_bit") == Some("0") => done(set("end_portal_eye_bit", "1")?, Told::Sound("block.end_portal_frame.fill", [0.5, 1.0, 0.5], false), Cost::Spend),
        "flint_and_steel" if bare.ends_with("campfire") && get("extinguished") == Some("1") => done(set("extinguished", "0")?, Told::Sound("ignite", CORNER, false), Cost::Wear),
        "glowstone" if bare == "respawn_anchor" => {
            let charge: u8 = get("respawn_anchor_charge")?.parse().ok()?;
            done(set("respawn_anchor_charge", &(charge + 1).to_string()).filter(|_| charge < ANCHOR_FULL)?, sound("respawn_anchor.charge"), Cost::Spend)
        }
        "flint_and_steel" if bare.ends_with("candle") && get("lit") == Some("0") => done(set("lit", "1")?, Told::Sound("ignite", CORNER, false), Cost::Wear),
        _ => None,
    }
}

fn telling(registry: &BlockRegistry, pos: BlockPos, change: &Change) -> Bytes {
    let at = |offset: [f32; 3]| Vec3f { x: pos[0] as f32 + offset[0], y: pos[1] as f32 + offset[1], z: pos[2] as f32 + offset[2] };
    match change.told {
        Told::Sound(name, offset, names_block) => send::noise(name, at(offset), if names_block { registry.network_hash(change.id) as i32 } else { -1 }),
        Told::SoundOf(name, offset, id) => send::noise(name, at(offset), registry.network_hash(id) as i32),
        Told::Event(event, offset, data) => sapling_net::encode(&LevelEvent { event, position: at(offset), data }),
    }
}

const FLINT_AND_STEEL: &str = "minecraft:flint_and_steel";
const FIRE_CHARGE: &str = "minecraft:fire_charge";
const ENDER_EYE: &str = "minecraft:ender_eye";
const END_CRYSTAL: &str = "minecraft:end_crystal";
/// What BDS sent with a fire charge's sound.
const CHARGE_SOUND: i32 = 78642;

impl Request<'_> {
    /// A click with `held` in the hand on the block at `pos`: false when it does nothing to it.
    /// A hoe and a shovel want air over the ground they work.
    pub(super) fn use_tool(&mut self, site: &Site, pos: BlockPos, face: Face, held: &str) -> bool {
        if super::cauldrons::use_on(self.world, self.entity, site.layer, pos, site.mode == GameMode::Creative) {
            return true;
        }
        if let Some(spent) = super::composter::fill(self.world, site.layer, pos, held) {
            if spent && site.mode != GameMode::Creative {
                inventory::change(self.world, self.entity, |inventory| inventory.take(inventory::Place::Main(inventory.selected.into()), 1).is_ok());
            }
            return true;
        }
        if held == "minecraft:bone_meal" && super::bonemeal::feed(self.world, site.layer, pos, site.mode == GameMode::Creative) {
            if site.mode != GameMode::Creative {
                inventory::change(self.world, self.entity, |inventory| inventory.take(inventory::Place::Main(inventory.selected.into()), 1).is_ok());
            }
            return true;
        }
        if let Some(spent) = crate::spawners::use_item(self.world, self.entity, site.layer, pos, held) {
            if spent && site.mode != GameMode::Creative {
                inventory::change(self.world, self.entity, |inventory| inventory.take(inventory::Place::Main(inventory.selected.into()), 1).is_ok());
            }
            return true;
        }
        if crate::spawning::hatch(self.world, site.layer, held, face.neighbour(pos)) {
            if site.mode != GameMode::Creative {
                inventory::change(self.world, self.entity, |inventory| inventory.take(inventory::Place::Main(inventory.selected.into()), 1).is_ok());
            }
            return true;
        }
        if held == crate::paintings::PAINTING && crate::paintings::hang(self.world, site.layer, pos, face) {
            if site.mode != GameMode::Creative {
                inventory::change(self.world, self.entity, |inventory| inventory.take(inventory::Place::Main(inventory.selected.into()), 1).is_ok());
            }
            return true;
        }
        if held == END_CRYSTAL && crate::dragon::place_crystal(self.world, site.layer, pos) {
            if site.mode != GameMode::Creative {
                inventory::change(self.world, self.entity, |inventory| inventory.take(inventory::Place::Main(inventory.selected.into()), 1).is_ok());
            }
            return true;
        }
        if held == crate::fireworks::ROCKET && crate::fireworks::launch_held(self.world, self.entity, pos, face) {
            if site.mode != GameMode::Creative {
                inventory::change(self.world, self.entity, |inventory| inventory.take(inventory::Place::Main(inventory.selected.into()), 1).is_ok());
            }
            return true;
        }
        if super::lodestone::bind(self.world, self.entity, site.layer, pos, held) {
            return true;
        }
        if held == FLINT_AND_STEEL && crate::tnt::light(self.world, site.layer, pos) {
            if site.mode != GameMode::Creative {
                inventory::wear_tool_in_hand(self.world, self.entity);
            }
            return true;
        }
        let Some(level) = level(self.world, site.layer) else { return false };
        let (registry, [x, y, z]) = (level.registry(), pos);
        let covered = (held.ends_with("_hoe") || held.ends_with("_shovel")) && !registry.get(level.block(x, y + 1, z)).is_some_and(|above| above.is_air());
        let Some(change) = change(registry, held, level.block(x, y, z), face).filter(|_| !covered) else { return (held == FLINT_AND_STEEL || held == FIRE_CHARGE) && self.strike(site, face.neighbour(pos), held) };
        let (told, is_event) = (telling(registry, pos, &change), matches!(change.told, Told::Event(..)));
        let left = inventory::stored(&Item::new(change.leaves.0.to_owned(), 1), registry);
        let copper = registry.get(change.id).and_then(|state| Copper::of(state.name)).filter(|copper| copper.is_door());
        // BDS tells an event before the block and a sound after it.
        if is_event {
            send::to_viewers(self.world, site.layer, pos, std::slice::from_ref(&told));
        }
        if held == ENDER_EYE {
            super::end_portal::fill(self.world, site.layer, pos, change.id);
        } else if let Some(copper) = copper {
            super::copper::turn(self.world, site.layer, pos, copper);
        } else {
            send::set(self.world, site.layer, pos, change.id);
        }
        for stack in left.iter().flat_map(|stack| std::iter::repeat_n(stack, change.leaves.1.into())) {
            drops::spill(self.world, site.layer, stack.clone(), pos);
        }
        if !is_event {
            send::to_viewers(self.world, site.layer, pos, &[told]);
        }
        // A bottle is heard besides the cauldron's own event.
        match change.cost {
            Cost::Becomes("minecraft:potion") => self.hear_at(site, pos, "bottle.fill"),
            Cost::Becomes("minecraft:glass_bottle") if is_event => self.hear_at(site, pos, "bottle.empty"),
            _ => {}
        }
        if site.mode != GameMode::Creative {
            match change.cost {
                Cost::Wear => inventory::wear_tool_in_hand(self.world, self.entity),
                Cost::Spend => drop(inventory::change(self.world, self.entity, |inventory| inventory.take(inventory::Place::Main(inventory.selected.into()), 1).is_ok())),
                Cost::Becomes(into) => self.swap_held(into),
            }
        }
        true
    }

    /// Flint and steel, or a fire charge, lights the cell `into` (fire.rs), with BDS's sound of each.
    fn strike(&mut self, site: &Site, into: BlockPos, held: &str) -> bool {
        if !super::fire::light(self.world, site.layer, into) {
            return false;
        }
        if held == FIRE_CHARGE {
            let [x, y, z] = into.map(|c| c as f32);
            let shot = LevelEvent { event: LevelEventEvent::SoundGhastShoot, position: Vec3f { x, y, z }, data: CHARGE_SOUND };
            send::to_viewers(self.world, site.layer, into, &[sapling_net::encode(&shot)]);
        } else {
            self.hear_at(site, into, "ignite");
        }
        if site.mode != GameMode::Creative {
            match held {
                FIRE_CHARGE => drop(inventory::change(self.world, self.entity, |inventory| inventory.take(inventory::Place::Main(inventory.selected.into()), 1).is_ok())),
                _ => inventory::wear_tool_in_hand(self.world, self.entity),
            }
        }
        true
    }

    fn hear_at(&mut self, site: &Site, pos: BlockPos, name: &str) {
        let [x, y, z] = pos.map(|c| c as f32 + 0.5);
        send::to_viewers(self.world, site.layer, pos, &[send::noise(name, Vec3f { x, y, z }, -1)]);
    }
}

#[cfg(test)]
mod tests;
