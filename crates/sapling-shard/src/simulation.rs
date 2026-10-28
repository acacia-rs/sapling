//! The server's own step of each player input (acacia-physics) beside what the client reports:
//! logged where the two part (Observe), or the client corrected (Correct); see docs/DESIGN.md
//! "Server-side movement".

mod policy;

use acacia_physics::{Input, PlayerState, tick};
use acacia_proto::packets::PlayerAuthInput;
use acacia_proto::types::InputData;
use bevy_ecs::prelude::*;
use sapling_api::MovementPolicy;

pub(crate) use policy::Own;

use crate::components::Position;
use crate::config::MovementCheck;
use crate::terrain::physics_view;

/// What the server makes of a player's inputs, and how often it parted from the client.
#[derive(Component)]
pub(crate) struct Simulated {
    state: PlayerState,
    /// Gliding and swimming show only as start and stop edges.
    gliding: bool,
    swimming: bool,
    /// No input was stepped yet, or the last one was not: the next one starts from the client.
    fresh: bool,
    /// The input tick before which a corrected client is not judged: what it sent meanwhile was
    /// predicted before it heard.
    quiet_until: u64,
    steps: u64,
    parted: u64,
    corrected: u64,
    worst: f32,
}

impl Default for Simulated {
    fn default() -> Self {
        Self { state: PlayerState::new([0.0; 3]), gliding: false, swimming: false, fresh: true, quiet_until: 0, steps: 0, parted: 0, corrected: 0, worst: 0.0 }
    }
}

/// Where the server's step of one input left the player.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Stepped {
    pub feet: [f32; 3],
    pub delta: [f32; 3],
    pub on_ground: bool,
}

/// Whose place an input leaves the player at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Ruling {
    Client,
    /// The server's, told to the client as a correction where `tell`.
    Server { stepped: Stepped, tell: bool },
}

/// The held controls an input reports, as `controls_of` in acacia-bot's trace replay reads them.
fn input_of(input: &PlayerAuthInput, gliding: bool, swimming: bool, using_item: bool) -> Input {
    let has = |flag: InputData| input.input_data.contains(&flag);
    let axis = |pos: bool, neg: bool| f32::from(u8::from(pos)) - f32::from(u8::from(neg));
    Input {
        move_vector: [axis(has(InputData::Left), has(InputData::Right)), axis(has(InputData::Up), has(InputData::Down))],
        yaw: input.yaw,
        pitch: input.pitch,
        jump: has(InputData::JumpDown),
        sneak: has(InputData::SneakDown),
        want_down: has(InputData::WantDown),
        // BDS reads `Sprinting`, not `SprintDown`, as the sprint intent.
        sprint: has(InputData::Sprinting),
        swim: swimming,
        glide: gliding,
        using_item,
    }
}

impl Simulated {
    /// Takes the client's word for where the player is: after a teleport, and for what the
    /// simulation does not model (flight, spectators).
    pub fn adopt(&mut self, feet: [f32; 3], delta: [f32; 3], on_ground: bool) {
        self.state.apply_correction(feet, delta, on_ground);
        self.fresh = false;
    }

    /// Starts again from where the client next says it is.
    pub fn restart(&mut self) {
        self.fresh = true;
    }

    /// A throw the server sent: the client moves by it on its next input.
    pub fn knock(&mut self, velocity: [f32; 3]) {
        self.state.queue_knockback(velocity);
    }

    pub fn set_speed(&mut self, movement: f32) {
        self.state.set_movement_attribute(movement);
    }

    /// What the armour worn changes in a step: its Depth Strider, Soul Speed and Swift Sneak, and
    /// leather boots, which stand on powder snow.
    pub fn equip(&mut self, warding: &sapling_data::enchant::Warding, leather_boots: bool) {
        let equipment = &mut self.state.equipment;
        (equipment.depth_strider, equipment.soul_speed, equipment.swift_sneak) = (i32::from(warding.depth_strider), i32::from(warding.soul_speed), i32::from(warding.swift_sneak));
        equipment.leather_boots = leather_boots;
    }

    /// The freeze the server holds the player at: the step takes it rather than its own.
    pub fn freeze(&mut self, freeze: f32) {
        self.state.server_freeze = Some(freeze);
    }

    /// Steps one input over `world`; `None` for the first after a restart, which takes the
    /// client's place.
    fn advance(&mut self, world: &sapling_world::World, input: &PlayerAuthInput, feet: [f32; 3], on_ground: bool, using_item: bool) -> Option<Stepped> {
        let latch = |held: bool, start: InputData, stop: InputData| (held || input.input_data.contains(&start)) && !input.input_data.contains(&stop);
        self.gliding = latch(self.gliding, InputData::StartGliding, InputData::StopGliding);
        self.swimming = latch(self.swimming, InputData::StartSwimming, InputData::StopSwimming);
        if self.fresh {
            self.adopt(feet, [input.delta.x, input.delta.y, input.delta.z], on_ground);
            return None;
        }
        let out = physics_view(world, |view| tick(&mut self.state, &input_of(input, self.gliding, self.swimming, using_item), view));
        self.steps += 1;
        Some(Stepped { feet: out.position, delta: out.delta, on_ground: out.on_ground })
    }

    /// How far the client's `feet` are from the server's step, where that is past `threshold`.
    fn parts(&mut self, stepped: &Stepped, feet: [f32; 3], threshold: f32) -> Option<f32> {
        let apart = (0..3).map(|axis| (stepped.feet[axis] - feet[axis]).powi(2)).sum::<f32>().sqrt();
        (apart > threshold).then(|| {
            self.parted += 1;
            self.worst = self.worst.max(apart);
            apart
        })
    }

    /// Steps one input over `world` and compares the feet with the client's; the distance where it
    /// is over `threshold`, after which the simulation goes on from the client's.
    pub fn step(&mut self, world: &sapling_world::World, input: &PlayerAuthInput, feet: [f32; 3], on_ground: bool, using_item: bool, threshold: f32) -> Option<f32> {
        let stepped = self.advance(world, input, feet, on_ground, using_item)?;
        let apart = self.parts(&stepped, feet, threshold)?;
        // The place, not the speed: a client that says it rises would be handed the speed to rise.
        self.adopt(feet, stepped.delta, stepped.on_ground);
        Some(apart)
    }

    /// Steps one input as [`Simulated::step`], but the server's place stands where the client
    /// strays past `check.threshold`, and while the correction is in flight. Inside the threshold
    /// the client's place stands and the server keeps its own, so no stray adds up unseen.
    pub fn rule(&mut self, world: &sapling_world::World, input: &PlayerAuthInput, feet: [f32; 3], on_ground: bool, using_item: bool, check: &MovementCheck) -> Ruling {
        let Some(stepped) = self.advance(world, input, feet, on_ground, using_item) else { return Ruling::Client };
        if input.tick < self.quiet_until {
            return Ruling::Server { stepped, tell: false };
        }
        if self.parts(&stepped, feet, check.threshold).is_none() {
            return Ruling::Client;
        }
        self.corrected += 1;
        self.quiet_until = input.tick + check.correction_delay;
        Ruling::Server { stepped, tell: true }
    }
}

/// What `movement::player_input` makes of an input it accepted, the client saying its player is
/// at `moved`: stepped over `world` under `policy`, or adopted where `world` is `None` (a mode the
/// physics does not model) or the input follows a teleport.
pub(crate) fn judge(simulated: &mut Simulated, check: &MovementCheck, policy: MovementPolicy, world: Option<&sapling_world::World>, input: &PlayerAuthInput, moved: &Position, eating: bool, player: u64) -> Ruling {
    if policy == MovementPolicy::Trust {
        return Ruling::Client;
    }
    let Some(world) = world.filter(|_| !input.input_data.contains(&InputData::HandledTeleport)) else {
        simulated.adopt(moved.feet, [input.delta.x, input.delta.y, input.delta.z], moved.on_ground);
        return Ruling::Client;
    };
    if policy == MovementPolicy::Correct {
        let ruling = simulated.rule(world, input, moved.feet, moved.on_ground, eating, check);
        if let Ruling::Server { stepped, tell: true } = ruling {
            tracing::debug!(player, tick = input.tick, feet = ?moved.feet, server = ?stepped.feet, flags = ?input.input_data, "movement corrected");
        }
        return ruling;
    }
    if let Some(apart) = simulated.step(world, input, moved.feet, moved.on_ground, eating, check.threshold) {
        tracing::debug!(player, tick = input.tick, apart, feet = ?moved.feet, flags = ?input.input_data, "movement parted from the server's step");
    }
    Ruling::Client
}

/// Logs how a leaving player's moves compared with the server's steps.
pub(crate) fn report(world: &World, player: Entity) {
    if let Some(simulated) = world.get::<Simulated>(player).filter(|simulated| simulated.steps > 0) {
        tracing::info!(steps = simulated.steps, parted = simulated.parted, corrected = simulated.corrected, worst = simulated.worst, "server-side movement observed");
    }
}

#[cfg(test)]
mod tests;
