//! Group X of the fourteenth wave: sculk hearing a summoned TNT's fuse and the items a blast hurts, the
//! nearest of a tick's vibrations winning (research/captures/bds-b1sculk.txt).

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::nbt::Value as Nbt;
use acacia_proto::packets::LevelSoundEvent;
use common::{Harness, all};
use common::asking::ask;
use common::shooting::{ALEX, ready};
use sapling_api::{Block, GameMode, Item};

fn ticks(h: &mut Harness, ticks: usize) {
    (0..ticks).for_each(|_| drop(h.tick_all()));
}

fn told(h: &mut Harness, ticks: usize) -> Vec<RawPacket> {
    (0..ticks).flat_map(|_| h.tick_all().to(1).to_vec()).collect()
}

fn took_off(sent: &[RawPacket]) -> bool {
    all::<LevelSoundEvent>(sent).iter().any(|sound| sound.sound_id == "takeoff" && sound.entity_type == "minecraft:bat")
}

/// bds-x2bat.txt: a hanging bat kept still with a player 4.1 off and let go, heard `takeoff`, with one 3.2 off.
#[test]
fn a_hanging_bat_lets_go_when_a_player_comes_within_4() {
    let mut h = ready(GameMode::Creative, &[], 0.0);
    ask(&mut h, |ctx| {
        ctx.set_game_mode(ALEX, GameMode::Creative);
        assert!(ctx.set_block([4, 67, 0], &Block::new("minecraft:stone")));
        ctx.teleport(ALEX, [-0.5, 65.0, 0.5]);
        ctx.summon_as(None, "minecraft:bat", [4.5, 66.0, 0.5], 0.0, 0.0, None).expect("a bat");
    });
    assert!(!took_off(&told(&mut h, 20)), "4.1 off it hangs on");
    ask(&mut h, |ctx| ctx.teleport(ALEX, [1.5, 65.0, 0.5]));
    assert!(took_off(&told(&mut h, 5)), "3.2 off it lets go");
}

fn heard(h: &mut Harness, sensor: [i32; 3]) -> Option<Nbt> {
    let data = ask(h, move |ctx| ctx.block_entity(sensor).and_then(|data| data.get::<Nbt>().cloned()))?;
    data.get("VibrationListener").and_then(|listener| listener.get("event")).cloned()
}

/// bds-b1sculk.txt: a TNT summoned 7 off was heard (`prime_fuse`), then its blast behind an obsidian wall
/// hurt stacks lying 3 off the sensor, which lay on, and those were what the sensor heard, not the blast.
#[test]
fn a_summoned_tnt_is_heard_and_then_the_items_its_blast_hurt_over_the_blast() {
    let mut h = ready(GameMode::Creative, &[], 0.0);
    let sensor = [0, 65, 0];
    ask(&mut h, move |ctx| {
        ctx.set_game_mode(ALEX, GameMode::Creative);
        assert!(ctx.set_block(sensor, &Block::new("minecraft:sculk_sensor")));
        for y in 65..=67 {
            for z in -2..=2 {
                assert!(ctx.set_block([4, y, z], &Block::new("minecraft:obsidian")));
            }
        }
        assert!(ctx.drop_item(&Item::new("minecraft:feather", 1), [3.5, 65.0, 0.5]));
    });
    ticks(&mut h, 20);
    assert!(ask(&mut h, |ctx| ctx.spawn_entity("minecraft:tnt", [7.5, 65.0, 0.5]).is_some()));
    ticks(&mut h, 12);
    assert_eq!(heard(&mut h, sensor), Some(Nbt::Int(39)), "prime_fuse");
    ticks(&mut h, 80);
    assert_eq!(heard(&mut h, sensor), Some(Nbt::Int(17)), "entity_damage, the nearer");
    let feathers = ask(&mut h, |ctx| ctx.entities().into_iter().filter(|entity| entity.item.as_ref().is_some_and(|item| item.name == "minecraft:feather")).count());
    assert_eq!(feathers, 1, "the hidden stack took 1 of its 5");
}
