//! Which goal the dragon follows, as BDS's goal selector picks among a group's: the running goal
//! goes on while it may, unless one before it in its group can start; then the first that can.
//! The goals themselves: flying.rs and sitting.rs.

use bevy_ecs::prelude::*;
use sapling_api::GameMode;

use super::{Flight, Goal, Group, flying, sitting};
use crate::components::{InLayer, Layer, Mode, Position, in_world};
use crate::dragon::random::Random;

/// What a goal works with, and what it asks for: the dragon's death at the portal.
pub(crate) struct Ctx<'w> {
    pub world: &'w mut World,
    pub layer: Entity,
    pub dragon: Entity,
    /// Where it was at the tick's start, before its stored motion carried it.
    pub from: [f32; 3],
    pub at: [f32; 3],
    pub crystals: u32,
    pub random: &'w mut Random,
    pub dead: bool,
}

impl Ctx<'_> {
    /// Tells the dragon's viewers what `packet` makes of its id.
    pub fn tell_dragon<P: acacia_proto::Packet>(&mut self, packet: impl FnOnce(u64) -> P) {
        let Some(id) = self.world.get::<crate::components::NetId>(self.dragon).map(|id| id.0) else { return };
        let told = sapling_net::encode(&packet(id));
        crate::entities::tell_viewers(self.world, self.dragon, self.layer, self.at.map(|c| c.floor() as i32), vec![told]);
    }

    /// Its growl, as heard from its head: two over its feet as BDS placed it on the portal.
    pub fn growl_at_head(&mut self) {
        let [x, y, z] = self.at;
        self.tell_dragon(|id| crate::dragon::fly::sound("mad", id, [x, y + 2.0, z]));
    }

    /// The heightmap's y over the island's middle: the first air over the pillar.
    pub fn portal_y(&self) -> i32 {
        self.world.get::<Layer>(self.layer).and_then(|level| level.world.surface(0, 0)).map_or(64, |y| y + 1)
    }

    /// The nearest player the dragon may attack (survival or adventure) within `reach` of `at`'s
    /// cell middle, and where its feet are.
    pub fn nearest_player(&self, at: [f32; 3], reach: f32) -> Option<(Entity, [f32; 3])> {
        let centre = at.map(|c| c.floor() + 0.5);
        let attackable = |player: &Entity| {
            self.world.get::<InLayer>(*player).is_some_and(|in_layer| in_layer.0 == self.layer)
                && self.world.get::<Mode>(*player).is_some_and(|mode| matches!(mode.0, GameMode::Survival | GameMode::Adventure))
                && !crate::vitals::is_dead(self.world, *player)
        };
        in_world(self.world)
            .into_iter()
            .filter(attackable)
            .filter_map(|player| Some((player, self.world.get::<Position>(player)?.feet)))
            .map(|(player, feet)| (player, feet, (0..3).map(|axis| (feet[axis] - centre[axis]).powi(2)).sum::<f32>()))
            .filter(|(.., far)| *far <= reach * reach)
            .min_by(|a, b| a.2.total_cmp(&b.2))
            .map(|(player, feet, _)| (player, feet))
    }

    /// Where a player's feet are, while it is in the End and alive.
    pub fn feet_of(&self, player: Entity) -> Option<[f32; 3]> {
        let here = self.world.get::<InLayer>(player).is_some_and(|in_layer| in_layer.0 == self.layer) && !crate::vitals::is_dead(self.world, player);
        here.then(|| self.world.get::<Position>(player).map(|position| position.feet)).flatten()
    }
}

/// The dragon's events: its goal stops and the group's start over.
pub(super) fn land(flight: &mut Flight) {
    stop(flight);
    flight.group = Group::Sitting;
}

pub(crate) fn take_off(flight: &mut Flight) {
    stop(flight);
    flight.group = Group::Flying;
}

pub(crate) fn begin_dying(flight: &mut Flight) {
    stop(flight);
    flight.group = Group::Dying;
}

fn stop(flight: &mut Flight) {
    if let Some(goal) = flight.goal.take() {
        match goal {
            Goal::Holding | Goal::Landing | Goal::Takeoff => (flight.motion.target, flight.path) = (None, None),
            Goal::Strafe => flying::stop_strafe(flight),
            Goal::Charge => flying::stop_charge(flight),
            Goal::Flaming => sitting::stop_flaming(flight),
            Goal::Scanning => flight.motion.target = None,
            Goal::Death => flight.motion.air_speed = 0.6,
        }
    }
}

fn can_use(cx: &Ctx<'_>, flight: &Flight, goal: Goal) -> bool {
    let prey = flight.prey.is_some_and(|prey| cx.feet_of(prey).is_some());
    match goal {
        Goal::Takeoff | Goal::Scanning => flight.sitting && !prey,
        Goal::Charge => flight.sitting && prey,
        Goal::Strafe => !flight.sitting && prey,
        Goal::Holding | Goal::Landing => !flight.sitting && !prey,
        Goal::Flaming => prey,
        Goal::Death => true,
    }
}

fn can_continue(cx: &mut Ctx<'_>, flight: &mut Flight, goal: Goal) -> bool {
    match goal {
        Goal::Holding => flight.prey.is_none(),
        Goal::Takeoff => flying::takeoff_goes_on(cx, flight),
        Goal::Strafe => flying::strafe_goes_on(cx, flight),
        Goal::Charge => flying::charge_goes_on(flight),
        Goal::Landing => sitting::landing_goes_on(flight),
        Goal::Scanning => sitting::scanning_goes_on(cx, flight),
        Goal::Flaming => sitting::flaming_goes_on(flight),
        Goal::Death => true,
    }
}

fn start(cx: &mut Ctx<'_>, flight: &mut Flight, goal: Goal) {
    flight.goal = Some(goal);
    match goal {
        Goal::Holding => {
            (flight.motion.target, flight.path, flight.prey) = (None, None, None);
            flight.sit(false);
        }
        Goal::Takeoff => flying::start_takeoff(flight),
        Goal::Strafe => flying::start_strafe(cx, flight),
        Goal::Charge => flying::start_charge(cx, flight),
        Goal::Landing => sitting::start_landing(flight),
        Goal::Scanning => sitting::start_scanning(flight),
        Goal::Flaming => sitting::start_flaming(flight),
        Goal::Death => sitting::start_death(flight),
    }
}

/// A tick of the dragon's goals: the selector, then the goal it follows.
pub(crate) fn think(cx: &mut Ctx<'_>, flight: &mut Flight) {
    if flight.goal.is_some_and(|goal| !can_continue(cx, flight, goal)) {
        stop(flight);
    }
    let goals = Goal::of(flight.group);
    let current = flight.goal.and_then(|goal| goals.iter().position(|g| *g == goal)).unwrap_or(goals.len());
    if let Some(&better) = goals[..current].iter().find(|goal| can_use(cx, flight, **goal)) {
        stop(flight);
        start(cx, flight, better);
    }
    match flight.goal {
        Some(Goal::Holding) => flying::holding(cx, flight),
        Some(Goal::Takeoff) => flying::takeoff(cx, flight),
        Some(Goal::Strafe) => flying::strafe(cx, flight),
        Some(Goal::Charge) => flying::charge(cx, flight),
        Some(Goal::Landing) => sitting::landing(cx, flight),
        Some(Goal::Scanning) => sitting::scanning(cx, flight),
        Some(Goal::Flaming) => sitting::flaming(cx, flight),
        Some(Goal::Death) => sitting::death(cx, flight),
        None => {}
    }
}
