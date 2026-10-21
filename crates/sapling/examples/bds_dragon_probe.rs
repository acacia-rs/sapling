//! The dragon fight: an operator bot in the End while a watcher stays in the overworld. Prints the
//! actor's packets with the milliseconds since the drill started: the dragon's every move as a
//! `D` line (wire position, pitch, yaw, head yaw), crystals', orbs' and the fight's packets in
//! full, block changes by name, the rest tallied. The bot answers dimension changes and the
//! credits itself.
//! `cargo run -p sapling --example bds_dragon_probe -- <server> <drill>`; drills: shared/dragon_drill.rs.
#[path = "shared/dump.rs"]
#[allow(dead_code)]
mod dump;
#[path = "shared/dragon_drill.rs"]
mod dragon;
#[path = "shared/end_drill.rs"]
#[allow(dead_code)]
mod end;
#[path = "shared/dragon_show.rs"]
mod show;
#[path = "shared/probe.rs"]
#[allow(dead_code)]
mod probe;

use std::time::{Duration, Instant};

use acacia_bot::interact::Face;
use acacia_bot::{Bot, BotEvent};
use acacia_world::BlockAccess;
use dragon::Step;
use show::Run;

const CRYSTAL: &str = "minecraft:ender_crystal";
const DRAGON: &str = "minecraft:ender_dragon";

fn face(name: &str) -> Face {
    match name {
        "down" => Face::Down,
        "north" => Face::North,
        "south" => Face::South,
        "west" => Face::West,
        "east" => Face::East,
        _ => Face::Up,
    }
}

fn feet(actor: &Bot) -> Option<[f32; 3]> {
    actor.movement().and_then(|movement| movement.position())
}

/// The nearest tracked entity of `kind` to the actor's feet.
fn nearest(actor: &Bot, kind: &str) -> Option<(u64, [f32; 3])> {
    let at = feet(actor)?;
    let to = acacia_bot::proto::types::Vec3f { x: at[0], y: at[1], z: at[2] };
    actor.state().entities.nearest(&to, |entity| entity.kind == kind).map(|entity| (entity.runtime_id, [entity.position.x, entity.position.y, entity.position.z]))
}

/// The topmost cell named `block` in the actor's view of the column, within `ys`.
fn topmost(actor: &Bot, x: i32, z: i32, ys: std::ops::RangeInclusive<i32>, block: &str) -> Option<i32> {
    let world = actor.world()?;
    let (view, registry) = (world.view()?, world.registry()?);
    ys.rev().find(|y| registry.get(view.block(x, *y, z)).is_some_and(|state| state.name.ends_with(block)))
}

async fn hit(run: &mut Run, actor: &mut Bot, watcher: &mut Bot, id: u64) {
    let action = actor.attack(id);
    tokio::pin!(action);
    loop {
        tokio::select! {
            result = &mut action => break println!("## hit {id}: {result:?} at {}", run.ms()),
            event = watcher.next() => run.watcher_event(event),
        }
    }
}

async fn use_on(run: &mut Run, actor: &mut Bot, pos: [i32; 3], face: Face) {
    let result = actor.use_item_on_block(pos, face).await;
    println!("## used on {pos:?} {face:?}: {result:?} at {}", run.ms());
}

/// Carries out one step; how long its consequences are watched for.
async fn act(run: &mut Run, actor: &mut Bot, watcher: &mut Bot, line: &str) -> Duration {
    match dragon::step(line) {
        Step::Command(command, wait) => {
            actor.client().command(command);
            wait
        }
        Step::Wait(wait) => wait,
        Step::Where => {
            println!("## at {:?} dimension {}", feet(actor), actor.state().player.dimension);
            Duration::ZERO
        }
        Step::Box(corners) => {
            end::print_box(actor, corners);
            Duration::ZERO
        }
        Step::Top(x, z) => {
            end::print_top(actor, x, z);
            Duration::ZERO
        }
        Step::Spikes => {
            end::SPIKES.iter().for_each(|(x, z)| end::print_top(actor, *x, *z));
            Duration::ZERO
        }
        Step::Entities => {
            show::entities(actor);
            Duration::ZERO
        }
        Step::Use(pos, name, wait) => {
            use_on(run, actor, pos, face(name)).await;
            wait
        }
        Step::Hit(kind, wait) => {
            match nearest(actor, &format!("minecraft:{kind}")) {
                Some((id, _)) => hit(run, actor, watcher, id).await,
                None => println!("## no {kind} to hit"),
            }
            wait
        }
        Step::Crystal(wait) => {
            let Some((id, [x, y, z])) = nearest(actor, CRYSTAL) else {
                println!("## no crystal");
                return wait;
            };
            // Beside it on the spike's top, which is a block below the crystal's feet.
            let command = format!("tp @s {} {} {z}", x + 2.0, y - 1.0);
            println!("## crystal {id} at {:?}: {command}", [x, y, z]);
            actor.client().command(&command);
            run.settle(actor, watcher, Duration::from_millis(2500)).await;
            hit(run, actor, watcher, id).await;
            wait
        }
        Step::Enter(wait) => {
            match feet(actor).map(|[x, _, z]| [x.floor() as i32, z.floor() as i32]).and_then(|[x, z]| (-12..=12).flat_map(|dx| (-12..=12).map(move |dz| (x + dx, z + dz))).find_map(|(x, z)| topmost(actor, x, z, 40..=90, "end_portal").map(|y| [x, y, z]))) {
                Some([x, y, z]) => {
                    println!("## entering the portal at {x} {y} {z}");
                    actor.client().command(&format!("tp @s {} {y} {}", x as f32 + 0.5, z as f32 + 0.5));
                }
                None => println!("## no end portal near"),
            }
            wait
        }
        Step::Perch(wait) => {
            let sitting = |actor: &Bot| actor.state().entities.iter().any(|entity| entity.kind == DRAGON && entity.metadata.flags().contains(acacia_bot::proto::types::MetadataFlags1::SITTING));
            let end = Instant::now() + wait;
            while !sitting(actor) && Instant::now() < end {
                if !run.settle(actor, watcher, Duration::from_millis(100)).await {
                    break;
                }
            }
            println!("## perched: {} at {}", sitting(actor), run.ms());
            Duration::ZERO
        }
        Step::Near([dx, dy, dz], wait) => {
            match nearest(actor, DRAGON) {
                Some((_, [x, y, z])) => {
                    let command = format!("tp @s {} {} {}", x + dx as f32, y + dy as f32, z + dz as f32);
                    println!("## near the dragon at {:?}: {command}", [x, y, z]);
                    actor.client().command(&command);
                }
                None => println!("## no dragon"),
            }
            wait
        }
        Step::Summon(wait) => {
            for (x, z) in [(3, 0), (0, 3), (-3, 0), (0, -3)] {
                match topmost(actor, x, z, 40..=90, "bedrock") {
                    Some(y) => {
                        // Two blocks outside the ring, on the island.
                        actor.client().command(&format!("tp @s {} {} {}", (x * 5 / 3) as f32 + 0.5, y, (z * 5 / 3) as f32 + 0.5));
                        run.settle(actor, watcher, Duration::from_millis(1500)).await;
                        use_on(run, actor, [x, y, z], Face::Up).await
                    }
                    None => println!("## no bedrock at {x} {z}"),
                }
                run.settle(actor, watcher, Duration::from_millis(600)).await;
            }
            wait
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let server = args.next().ok_or("usage: bds_dragon_probe <server> <drill>")?;
    let name = args.next().unwrap_or_else(|| "dragon-watch".into());
    let lines = dragon::drill(&name).ok_or(format!("no drill {name}"))?;
    let (mut actor, mut watcher) = (probe::connect(&server, "DragonActor").await?, probe::connect(&server, "DragonWatcher").await?);
    watcher.wait_ticks(60).await?;
    actor.wait_ticks(60).await?;
    for bot in [&mut actor, &mut watcher] {
        while let Ok(Some(event)) = tokio::time::timeout(Duration::from_millis(5), bot.next()).await {
            if let BotEvent::Disconnected(reason) = event {
                return Err(format!("disconnected at the start: {reason:?}").into());
            }
        }
    }
    let mut run = Run::new(Instant::now());
    for line in &lines {
        println!("## {line} at {}", run.ms());
        let wait = act(&mut run, &mut actor, &mut watcher, line).await;
        if !run.settle(&mut actor, &mut watcher, wait).await {
            break;
        }
    }
    actor.disconnect().await;
    watcher.disconnect().await;
    Ok(())
}
