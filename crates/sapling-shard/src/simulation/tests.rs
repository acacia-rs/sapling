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

/// Thirty inputs of a client walking +z on the platform as the game steps it, each judged by `judge`.
fn walk(world: &sapling_world::World, mut judge: impl FnMut(&PlayerAuthInput, [f32; 3], bool, u64)) -> [f32; 3] {
    let terrain = crate::terrain::Blocks(world, &[]);
    let view = acacia_physics::registry_world::RegistryWorld { view: &terrain, registry: world.registry() };
    let mut client = PlayerState::new([0.5, 65.0, -6.5]);
    let walk = Input { move_vector: [0.0, 1.0], ..Input::default() };
    for at in 0..30 {
        let out = tick(&mut client, &walk, &view);
        judge(&reported(out.position, out.delta, vec![InputData::Up], at), out.position, out.on_ground, at);
    }
    client.eye_position()
}

#[test]
fn a_client_stepping_as_the_game_does_never_parts_and_a_flyer_does() {
    let world = platform();
    let mut server = Simulated::default();
    let eyes = walk(&world, |input, feet, on_ground, at| assert_eq!(server.step(&world, input, feet, on_ground, false, 0.01), None, "tick {at} at {feet:?}"));
    assert!(eyes[2] > -1.0, "it walked +z: {eyes:?}");

    let mut feet = [0.5, 65.0, 0.5];
    let parted = (0..10).filter(|&at| {
        feet[1] += 0.4;
        server.step(&world, &reported(feet, [0.0, 0.4, 0.0], Vec::new(), 30 + at), feet, false, false, 0.5).is_some()
    });
    assert!(parted.count() >= 5, "rising with nothing under it is not what the game does");
}

#[test]
fn a_flyer_is_corrected_then_left_alone_while_the_correction_is_in_flight() {
    let world = platform();
    let check = MovementCheck { policy: MovementPolicy::Correct, threshold: 0.5, correction_delay: 5 };
    let mut server = Simulated::default();
    walk(&world, |input, feet, on_ground, at| assert_eq!(server.rule(&world, input, feet, on_ground, false, &check), Ruling::Client, "tick {at} at {feet:?}"));

    let start = server.state.eye_position();
    let mut feet = [start[0], start[1] - EYE_HEIGHT, start[2]];
    let rulings: Vec<Ruling> = (0..12)
        .map(|at| {
            feet[1] += 0.4;
            server.rule(&world, &reported(feet, [0.0, 0.4, 0.0], Vec::new(), 30 + at), feet, false, false, &check)
        })
        .collect();
    let told: Vec<usize> = rulings.iter().enumerate().filter(|(_, ruling)| matches!(ruling, Ruling::Server { tell: true, .. })).map(|(at, _)| at).collect();
    assert_eq!(told[..2], [1, 6], "first once it is half a block up, again five inputs on: {rulings:?}");
    let held = rulings.iter().filter_map(|ruling| if let Ruling::Server { stepped, .. } = ruling { Some(stepped.feet[1]) } else { None });
    assert!(held.into_iter().all(|y| y < 65.1), "the server keeps it on the ground: {rulings:?}");
}
