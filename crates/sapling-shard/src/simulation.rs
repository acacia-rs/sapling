//! The server's own step of each player input (acacia-physics) beside what the client reports.
//! Observe mode only logs where the two part; see docs/DESIGN.md "Server-side movement".

use acacia_physics::registry_world::{RegistryWorld, Terrain};
use acacia_physics::{Input, PlayerState, tick};
use acacia_proto::packets::PlayerAuthInput;
use acacia_proto::types::InputData;
use acacia_world::BlockAccess;
use bevy_ecs::prelude::*;
use sapling_world::ChunkPos;

use crate::components::Position;
use crate::config::MovementCheck;

/// What the server makes of a player's inputs, and how often it parted from the client.
#[derive(Component)]
pub(crate) struct Simulated {
    state: PlayerState,
    /// Gliding and swimming show only as start and stop edges.
    gliding: bool,
    swimming: bool,
    /// No input was stepped yet, or the last one was not: the next one starts from the client.
    fresh: bool,
    steps: u64,
    parted: u64,
    worst: f32,
}

impl Default for Simulated {
    fn default() -> Self {
        Self { state: PlayerState::new([0.0; 3]), gliding: false, swimming: false, fresh: true, steps: 0, parted: 0, worst: 0.0 }
    }
}

/// A shard's world as the physics reads it.
struct Blocks<'a>(&'a sapling_world::World);

impl BlockAccess for Blocks<'_> {
    fn block(&self, x: i32, y: i32, z: i32) -> u32 {
        self.0.block(x, y, z)
    }

    fn liquid(&self, x: i32, y: i32, z: i32) -> u32 {
        self.0.liquid(x, y, z)
    }
}

impl Terrain for Blocks<'_> {
    fn is_column_loaded(&self, cx: i32, cz: i32) -> bool {
        self.0.is_loaded(ChunkPos::new(cx, cz))
    }
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

    /// A throw the server sent: the client moves by it on its next input.
    pub fn knock(&mut self, velocity: [f32; 3]) {
        self.state.queue_knockback(velocity);
    }

    pub fn set_speed(&mut self, movement: f32) {
        self.state.set_movement_attribute(movement);
    }

    /// Steps one input over `world` and compares the feet with the client's; the distance where it
    /// is over `threshold`, after which the simulation goes on from the client's.
    pub fn step(&mut self, world: &sapling_world::World, input: &PlayerAuthInput, feet: [f32; 3], on_ground: bool, using_item: bool, threshold: f32) -> Option<f32> {
        let delta = [input.delta.x, input.delta.y, input.delta.z];
        let latch = |held: bool, start: InputData, stop: InputData| (held || input.input_data.contains(&start)) && !input.input_data.contains(&stop);
        self.gliding = latch(self.gliding, InputData::StartGliding, InputData::StopGliding);
        self.swimming = latch(self.swimming, InputData::StartSwimming, InputData::StopSwimming);
        if self.fresh {
            self.adopt(feet, delta, on_ground);
            return None;
        }
        let terrain = Blocks(world);
        let out = tick(&mut self.state, &input_of(input, self.gliding, self.swimming, using_item), &RegistryWorld { view: &terrain, registry: world.registry() });
        self.steps += 1;
        let apart = (0..3).map(|axis| (out.position[axis] - feet[axis]).powi(2)).sum::<f32>().sqrt();
        if apart <= threshold {
            return None;
        }
        self.parted += 1;
        self.worst = self.worst.max(apart);
        // The place, not the speed: a client that says it rises would be handed the speed to rise.
        self.adopt(feet, out.delta, out.on_ground);
        Some(apart)
    }
}

/// What `movement::player_input` does with an input it accepted, the player now at `moved`: under
/// [`MovementCheck::Observe`], stepped over `world` and compared, or adopted where `world` is `None`
/// (a mode the physics does not model) or the input follows a teleport.
pub(crate) fn observe(simulated: &mut Simulated, check: MovementCheck, world: Option<&sapling_world::World>, input: &PlayerAuthInput, moved: &Position, eating: bool, player: u64) {
    let MovementCheck::Observe { threshold } = check else { return };
    let delta = [input.delta.x, input.delta.y, input.delta.z];
    match world.filter(|_| !input.input_data.contains(&InputData::HandledTeleport)) {
        Some(world) => {
            if let Some(apart) = simulated.step(world, input, moved.feet, moved.on_ground, eating, threshold) {
                tracing::debug!(player, tick = input.tick, apart, feet = ?moved.feet, flags = ?input.input_data, "movement parted from the server's step");
            }
        }
        None => simulated.adopt(moved.feet, delta, moved.on_ground),
    }
}

/// Logs how a leaving player's moves compared with the server's steps.
pub(crate) fn report(world: &World, player: Entity) {
    if let Some(simulated) = world.get::<Simulated>(player).filter(|simulated| simulated.steps > 0) {
        tracing::info!(steps = simulated.steps, parted = simulated.parted, worst = simulated.worst, "server-side movement observed");
    }
}

#[cfg(test)]
mod tests {
    use acacia_proto::packets::{PlayerAuthInputInputMode, PlayerAuthInputInteractionModel, PlayerAuthInputPlayMode};
    use acacia_proto::types::{Vec2f, Vec3f};
    use sapling_data::EYE_HEIGHT;

    use super::*;

    /// A void world with a 16x16 stone platform at y 64 around the origin.
    fn platform() -> sapling_world::World {
        let mut world = sapling_world::World::void();
        let stone = world.block_id("minecraft:stone").unwrap();
        world.fill([-8, 64, -8], [7, 64, 7], stone);
        world
    }

    fn reported(feet: [f32; 3], delta: [f32; 3], input_data: Vec<InputData>, tick: u64) -> PlayerAuthInput {
        let zero = || Vec2f { x: 0.0, z: 0.0 };
        let vec3 = |[x, y, z]: [f32; 3]| Vec3f { x, y, z };
        PlayerAuthInput {
            pitch: 0.0,
            yaw: 0.0,
            position: vec3([feet[0], feet[1] + EYE_HEIGHT, feet[2]]),
            move_vector: zero(),
            head_yaw: 0.0,
            input_data,
            input_mode: PlayerAuthInputInputMode::Mouse,
            play_mode: PlayerAuthInputPlayMode::Normal,
            interaction_model: PlayerAuthInputInteractionModel::Crosshair,
            interact_rotation: zero(),
            tick,
            delta: vec3(delta),
            transaction: None,
            item_stack_request: None,
            block_action: None,
            vehicle_rotation: None,
            predicted_vehicle: None,
            analogue_move_vector: zero(),
            camera_orientation: vec3([0.0; 3]),
            raw_move_vector: zero(),
        }
    }

    #[test]
    fn a_client_stepping_as_the_game_does_never_parts_and_a_flyer_does() {
        let world = platform();
        let terrain = Blocks(&world);
        let view = RegistryWorld { view: &terrain, registry: world.registry() };
        let mut client = PlayerState::new([0.5, 65.0, -6.5]);
        let mut server = Simulated::default();
        let walk = Input { move_vector: [0.0, 1.0], ..Input::default() };
        for at in 0..30 {
            let out = tick(&mut client, &walk, &view);
            let input = reported(out.position, out.delta, vec![InputData::Up], at);
            assert_eq!(server.step(&world, &input, out.position, out.on_ground, false, 0.01), None, "tick {at} at {:?}", out.position);
        }
        assert!(client.eye_position()[2] > -1.0, "it walked +z: {:?}", client.eye_position());

        let mut feet = [0.5, 65.0, 0.5];
        let parted = (0..10).filter(|&at| {
            feet[1] += 0.4;
            server.step(&world, &reported(feet, [0.0, 0.4, 0.0], Vec::new(), 30 + at), feet, false, false, 0.5).is_some()
        });
        assert!(parted.count() >= 5, "rising with nothing under it is not what the game does");
    }
}
