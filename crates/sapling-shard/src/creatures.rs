//! Mobs, spawned through the plugin API or kept by a world folder (birth.rs): bodies (body.rs) that fall,
//! collide and push each other (crowding.rs), stroll (strolling.rs), swim (swimming.rs, squids.rs), fly about (wings.rs), hunt if their kind does (hunting.rs) and are raised (husbandry.rs), shown as BDS announced their kind (`sapling_data::add_entity`) or
//! as they were stored; and the entity side of the handlers' context (backend.rs).
// TODO: no AI but strolling, swimming, flying about, hunting and raising; a spawned mob is
// saved with the least a compound needs, which BDS was not tried on.

mod afflictions;
mod armour_stands;
mod arrows;
mod attacks;
mod backend;
mod bats;
mod bees;
mod birth;
mod blast;
mod blows;
mod boosts;
mod breezes;
mod captains;
mod command_gear;
mod dolphins;
mod drinks;
mod drowned;
mod bullets;
mod charges;
mod creaking;
mod crowding;
pub(crate) use crowding::pushed_by_players;
mod death;
mod duels;
mod enchanted;
mod evented;
mod fireballs;
mod flight;
mod floating;
mod footfalls;
mod frogs;
mod gaze;
mod genes;
mod ghasts;
mod guardians;
mod frost;
mod harm;
mod hatching;
mod health;
mod heat;
mod hits;
mod hunting;
mod husbandry;
mod jockeys;
pub(crate) use jockeys::rein as jockey_rein;
mod killing;
mod landing;
mod legs;
mod looks;
mod omens;
mod paths;
mod piglins;
mod pigmen;
mod properties;
pub(crate) mod raids;
mod rams;
mod riders;
mod scooping;
mod shooting;
mod shown;
mod shulkers;
mod skeleton_traps;
mod slimes;
mod smash;
mod spawn_gear;
mod snow_golems;
mod spells;
mod splashes;
mod squids;
mod strolling;
mod stares;
mod sourced;
mod stomping;
mod strides;
mod struck;
mod struck_back;
mod stuck;
mod summoned;
mod swimming;
mod teleports;
mod ticked;
mod turning;
mod vexes;
mod villagers;
mod warden;
mod wings;
mod wither;
mod worn;
mod zombifying;

use bevy_ecs::prelude::*;

use sapling_world::Actor;

use crate::body::Body;
use birth::{Birth, born, spawn_entity};
use death::{die, leaves_loot};
use killing::kill;
use shown::{head, physics_of, reshow};
pub(crate) use birth::{fire_herd_event, from_storage, is_arriving, spawn, spawn_as};
pub(crate) use shown::{head as head_of, is_baby};
pub(crate) use properties::set as set_property;
pub(crate) use afflictions::{clouded, takes_potion};
pub(crate) use armour_stands::use_on as dress_stand;
pub(crate) use skeleton_traps::set_trap;
pub(crate) use landing::tell_after_gone;
pub(crate) use arrows::{ARROW, FIREBALL, Lands, Shot, first_move, spread};
pub(crate) use shooting::play_loads;
pub(crate) use flight::{Arrow, Launch, Loosed, Pickup, facing, launch, sound, summon as summon_shot};
pub(crate) use flight::vanish::vanish as vanish_shot;
pub(crate) use stuck::{from_storage as shot_from_storage, lying as lying_shots, shaking as shot_shaking, unstick};
pub(crate) use attacks::{attacked, hit_from_near};
pub(crate) use hits::channels;
pub(crate) use struck_back::{look_of, tell_owner};
pub(crate) use bees::{bees_in, hive_broken};
pub(crate) use creaking::{heart_noticed, hearts_arrived};
pub(crate) use blast::{Wreck, blast_particles, explode, explode_burning, explode_primed};
pub(crate) use enchanted::thorned;
pub(crate) use harm::{holds, in_rain, is_full_block, touches, wound};
pub(crate) use heat::{on_magma, softening};
pub(crate) use hatching::{hatch, make_home};
pub(crate) use health::{Cause, Ground, Health};
pub(crate) use hunting::{on_difficulty, sees};
pub(crate) use husbandry::{Raised, allays_hear_note, interact, kept as kept_raising, owner_hit, owner_hurt, shear_in, tame_by_riding, tame_by_upgrade, tie_to_fence};
pub(crate) use omens::{Omens, befall, infested_hurt};
pub(crate) use piglins::barter;
pub(crate) use scooping::{held_kind as bucketed_kind, release as release_from_bucket, scoop};
pub(crate) use strolling::is_under_way;
pub(crate) use struck::struck_by_lightning;
pub(crate) use teleports::landing as teleport_landing;
pub(crate) use villagers::{alarm as hear_bell, bargains, interact as trade_with, stop_trading, traded};
pub(crate) use warden::{darken_ahead as warden_darkens, hear as warden_hears, is_near as warden_near, summon_emerging as summon_warden};
pub(crate) use wither::{build as build_wither, hurt as wither_hurt, spares};
use crate::components::{InLayer, NetId, Tick};
use crate::{entities, mobs};


/// A mob: spawned through the API, or kept by a world folder and dressed as it was stored
/// (`look`). Its [`Body`] and [`entities::Kind`] are beside it.
#[derive(Component)]
pub(crate) struct Creature {
    look: Option<Actor>,
}

impl Creature {
    /// One dressed as BDS announces its kind.
    pub(crate) fn plain() -> Self {
        Self { look: None }
    }

    /// Its stored look, where it has one, wears `body` in its body slot (`Armor`'s fifth entry).
    pub(crate) fn show_body(&mut self, body: Option<sapling_world::ItemStack>) {
        if let Some(look) = self.look.as_mut() {
            look.armor.resize(5, None);
            look.armor[4] = body;
        }
    }
}

/// A mob that died: the tick it goes in, [`DYING`] after its death (a wither's later, wither/dying.rs).
#[derive(Component)]
struct Dying(u64);

/// Whether the entity is a mob that died and is not gone yet.
pub(crate) fn is_dying(world: &World, entity: Entity) -> bool {
    world.get::<Dying>(entity).is_some()
}

/// Whether the entity is a mob that has not died.
pub(crate) fn is_alive(world: &World, entity: Entity) -> bool {
    world.get::<Creature>(entity).is_some() && world.get::<Dying>(entity).is_none()
}

/// Ticks from a mob's death to its going, as BDS was seen to (research/captures/bds-kill.txt).
const DYING: u64 = 21;
/// Killed, these go at once, without a death or loot, as BDS's armor stand.
const NO_DEATH: &[&str] = &["minecraft:armor_stand"];

/// Writes what mobs keep beyond their place and health (effects, a wither's fight) into what the
/// world folder keeps of them.
pub(crate) fn keep_state(world: &mut World) {
    afflictions::keep(world);
    wither::keep(world);
    piglins::keep(world);
    captains::keep(world);
    raids::keep_patrols(world);
    properties::keep(world);
    villagers::keep_pantries(world);
}

/// Moves every spawned mob a tick and hurts it by where it is; a dead one goes once it has died.
pub(crate) fn tick(world: &mut World) {
    let ticked = ticked::Ticked::of(world);
    flight::tick(world);
    afflictions::tick(world, &ticked);
    omens::tick(world);
    spells::tick(world);
    bullets::tick(world);
    shulkers::tick(world);
    charges::tick(world);
    fireballs::tick(world);
    wither::tick(world);
    bees::tick(world);
    guardians::tick(world);
    snow_golems::tick(world);
    piglins::tick(world);
    husbandry::tick(world);
    crowding::tick(world);
    warden::tick(world);
    raids::tick(world);
    villagers::spawn_golems(world);
    villagers::spawn_cats(world);
    armour_stands::tick(world);
    zombifying::tick(world);
    turning::tick(world);
    skeleton_traps::tick(world);
    creaking::tick_hearts(world);
    death::leave_due(world);
    let now = world.resource::<Tick>().0;
    let dead: Vec<Entity> = world.query::<(Entity, &Dying)>().iter(world).filter(|(_, dying)| now >= dying.0).map(|(entity, _)| entity).collect();
    for entity in dead {
        attacks::leave_experience(world, entity);
        slimes::split(world, entity);
        mobs::remove(world, entity);
    }
    let all: Vec<(Entity, Entity, u64, String)> =
        world.query_filtered::<(Entity, &InLayer, &NetId, &entities::Kind), With<Creature>>().iter(world).map(|(entity, layer, id, kind)| (entity, layer.0, id.0, kind.0.clone())).collect();
    for (entity, layer, id, kind) in all {
        // The fight moves its dragon (dragon.rs); a mob far from every player waits (ticked.rs).
        if kind == crate::dragon::DRAGON && world.get::<crate::dragon::Dragon>(entity).is_some() || !ticked.ticks(world, entity, layer) {
            continue;
        }
        if world.get::<Dying>(entity).is_none() {
            crate::spawning::count_idle(world, entity, layer, &kind);
        }
        death::hear_hurt(world, entity, layer);
        // A rider is carried to its seat, a vehicle its driver's client moves put where it says (riding.rs).
        // A seated mob is still harmed where its seat is: BDS's pig in a boat under a roof held its
        // breath and choked (research/captures/bds-ai1boat.txt). A seated hunter hunts (jockeys.rs).
        let seated = world.get::<crate::riding::Riding>(entity).is_some();
        let ridden = seated || crate::riding::is_driven(world, entity);
        let Some(start) = world.get::<Body>(entity).copied().filter(|_| !ridden) else {
            let Some(body) = world.get::<Body>(entity).copied().filter(|_| seated) else { continue };
            let dying = world.get::<Dying>(entity).is_some();
            if !dying && crate::rules::on(world, layer, sapling_data::MOB_AI) {
                jockeys::think(world, entity, layer, id, &kind, &body);
            }
            if harm::tick(world, entity, layer, id, &kind, &body, &body, dying) {
                die(world, entity, layer, &kind, &body);
            }
            continue;
        };
        // With `sapling:mobAi` off a mob still falls, is pushed and is hurt, but decides nothing.
        let thinks = world.get::<Dying>(entity).is_none() && crate::rules::on(world, layer, sapling_data::MOB_AI);
        if thinks {
            stares::watch(world, entity, layer, id, &kind, &start);
            drinks::tick(world, entity, layer, id, &start, hunting::prey_at(world, entity));
            teleports::tick(world, entity, layer, hunting::prey_at(world, entity));
            if villagers::tick(world, entity, layer) {
                continue;
            }
        }
        // A teleport starts the tick afresh where it landed: no fall from where it left.
        let Some(start) = world.get::<Body>(entity).copied() else { continue };
        if thinks && !legs::ridden(world, entity, layer, &kind, &start) && !wither::steer(world, entity, layer) && !rams::steer(world, entity, layer, id, &kind) && !creaking::steer(world, entity, layer, id, &kind) && !warden::steer(world, entity, layer, id, &kind) && !hunting::steer(world, entity, layer, id, &kind) && !raids::steer(world, entity, layer, &kind) && !stomping::steer(world, entity, layer, &kind) && !snow_golems::steer(world, entity, layer, id, &kind) && !piglins::steer(world, entity, layer, id, &kind) && !husbandry::steer(world, entity, layer, &kind) && !villagers::steer(world, entity, layer)
            && !frogs::steer(world, entity, layer, id) && !swimming::steer(world, entity, layer, &kind) && !squids::steer(world, entity, layer)&& !bats::steer(world, entity, layer, &kind) && !wings::steer(world, entity, layer, &kind)
        {
            let traits = sapling_data::mob_traits(&kind);
            let strolls = world.resource::<crate::components::Settings>().0.mobs_stroll;
            if strolls {
                let (strolls, panics) = genes::paces(world, entity, traits.strolls.or(traits.hunts.map(|hunts| hunts.speed)), traits.panics);
                strolling::steer(world, entity, layer, strolls, panics, traits.burns_in_sun);
                gaze::tick(world, entity, layer, &kind, &start);
            }
        }
        if thinks {
            floating::tick(world, entity, layer, &kind, &start);
            slimes::tick(world, entity, layer, id, &kind, &start);
        }
        afflictions::float(world, entity);
        let stepped = if hits::thrown_now(world, entity) { world.get::<Body>(entity).map(|body| (*body, Vec::new())) } else { let physics = wings::physics_showing_gravity(world, entity, &kind, physics_of(&kind)); entities::step(world, entity, layer, id, physics, 0.0) };
        let Some((body, packets)) = stepped else { continue };
        entities::tell_viewers(world, entity, layer, body.at.map(|c| c.floor() as i32), packets.clone());
        crate::blocks::tread_turtle_eggs(world, layer, &kind, body.at, body.on_ground, body.on_ground && !start.on_ground);
        if body.on_ground {
            crate::blocks::tread_ore(world, layer, body.at);
        }
        footfalls::step(world, entity, layer, &start, &body);
        if thinks {
            if !creaking::is_frozen(world, entity) && !warden::is_busy(world, entity) {
                hunting::strike(world, entity, layer, id, &kind, &body);
            }
            rams::strike(world, entity, layer, id, &kind, &body);
            drowned::throw(world, entity, layer, id, &kind, &body);
            slimes::after_move(world, entity, layer, id, &kind, &start, &body);
            husbandry::after_move(world, entity, layer, id, &kind, &body);
            if blast::tick(world, entity, layer, id, &kind, &body) {
                continue;
            }
        }
        let before = world.get::<Health>(entity).map(|health| (health.left(), health.is_burning()));
        if harm::tick(world, entity, layer, id, &kind, &start, &body, world.get::<Dying>(entity).is_some()) {
            die(world, entity, layer, &kind, &body);
        }
        // A new mob is announced as it was made, its first tick told after (bds-firstdata.txt: a
        // zombie summoned in the sun shown unlit, then lit); later viewers get it from its second.
        let health = world.get::<Health>(entity);
        let changed = !packets.is_empty() || health.map(|health| (health.left(), health.is_burning())) != before;
        if !entities::is_new(world, entity) && (changed || entities::was_new(world, entity)) {
            reshow(world, entity);
        }
    }
}
