//! The player drills of bds_command_probe: what a drill's `!` lines make the actor do, and which
//! drill is in which file (harm and hunger: vitals_drill.rs; toil, armour, effects and beds:
//! armour_drill.rs; a zombie's and what food does besides feeding: hunt_drill.rs).

use std::time::Duration;

use acacia_bot::Bot;

#[path = "armour_drill.rs"]
mod armour;
#[path = "feedback_drill.rs"]
mod feedback;
#[path = "foes_drill.rs"]
mod foes;
#[path = "reach_drill.rs"]
mod reach;
#[path = "hunt_drill.rs"]
mod hunt;
#[path = "shove_drill.rs"]
mod shove;
#[path = "world_drill.rs"]
mod world;
#[path = "use_drill.rs"]
mod uses;
#[path = "trade_drill.rs"]
mod trades;
#[path = "vitals_drill.rs"]
mod vitals;

/// A player's eyes are this far over its feet.
const EYES: f32 = 1.62;

/// The block `offset` (three whole numbers) from the actor's feet.
fn from_feet(actor: &Bot, offset: &str) -> Option<[i32; 3]> {
    let offset: Vec<i32> = offset.split(' ').filter_map(|part| part.parse().ok()).collect();
    let feet = actor.movement().and_then(|movement| movement.position()).map(|at| at.map(|c| c.floor() as i32))?;
    let &[x, y, z] = &offset[..] else { return None };
    Some([feet[0] + x, feet[1] + y, feet[2] + z])
}

/// The face a `!click` or `!place` line names after its offset (`up` when it names none).
fn face_named(word: &str) -> Option<acacia_bot::interact::Face> {
    use acacia_bot::interact::Face;
    Some(match word {
        "down" => Face::Down, "up" => Face::Up,
        "north" => Face::North, "south" => Face::South,
        "west" => Face::West, "east" => Face::East,
        _ => return None,
    })
}

/// Does what a drill's `!` line asks of the actor: `!eat` holds use on what it holds, `!jump`,
/// `!walk` and `!sprint` hold those keys until `!stop`, `!break x y z` breaks, `!click x y z` clicks,
/// `!place x y z` puts what it holds on and `!sleep x y z` lies in the block that far from its feet,
/// `!wake` leaves its bed and `!close` its open screen; `!put`, `!take` and `!takeall` move items
/// in and out of the open container. False for any other line.
pub async fn act(actor: &mut Bot, line: &str) -> bool {
    let keys = |actor: &mut Bot, forward: f32, sprint: bool, jump: bool| {
        if let Some(controls) = actor.controls() {
            (controls.forward, controls.sprint, controls.jump) = (forward, sprint, jump);
        }
    };
    if let Some((places, offset)) = line.strip_prefix("!click ").map(|offset| (false, offset)).or(line.strip_prefix("!place ").map(|offset| (true, offset))) {
        let (offset, face) = offset.rsplit_once(' ').and_then(|(rest, word)| Some((rest, face_named(word)?))).unwrap_or((offset, acacia_bot::interact::Face::Up));
        if let Some(at) = from_feet(actor, offset) {
            match places {
                true => println!("## placed: {:?}", actor.place_block(at, face).await),
                false => println!("## clicked: {:?}", actor.use_item_on_block(at, face).await),
            }
        }
        return true;
    }
    // The bot turns its eyes to its feet plus the offset, and holds that look.
    if let Some(offset) = line.strip_prefix("!look ") {
        let offset: Vec<f32> = offset.split(' ').filter_map(|part| part.parse().ok()).collect();
        let feet = actor.movement().and_then(|movement| movement.position());
        if let (Some([x, y, z]), &[dx, dy, dz], Some(controls)) = (feet, &offset[..], actor.controls()) {
            controls.look_at([x, y + EYES, z], [x + dx, y + dy, z + dz]);
        }
        return true;
    }
    if let Some(at) = line.strip_prefix("!break ") {
        if let Some(at) = from_feet(actor, at) {
            println!("## broke: {:?}", actor.break_block(at).await);
        }
        return true;
    }
    // `!trade kind offer times`: trades with the newest mob of that kind it knows of.
    if let Some(words) = line.strip_prefix("!trade ") {
        let words: Vec<&str> = words.split(' ').collect();
        let (kind, offer, times) = (format!("minecraft:{}", words[0]), words.get(1).and_then(|w| w.parse().ok()).unwrap_or(0), words.get(2).and_then(|w| w.parse().ok()).unwrap_or(1));
        match actor.state().entities.iter().filter(|entity| entity.kind == kind).map(|entity| entity.runtime_id).max() {
            Some(id) => println!("## traded: {:?}", actor.trade(id, offer, times).await),
            None => println!("## no {kind} to trade with"),
        }
        return true;
    }
    if let Some(at) = line.strip_prefix("!sleep ") {
        if let Some(at) = from_feet(actor, at) {
            println!("## slept: {:?}", actor.sleep(at).await);
        }
        return true;
    }
    // `!put from to [count]`: main inventory slot `from` (all of it, or `count`) into the open
    // container's slot `to`; `!take slot` moves one of its stacks out, `!takeall` every one.
    if let Some(words) = line.strip_prefix("!put ") {
        use acacia_bot::items::SlotRef;
        let numbers: Vec<u8> = words.split(' ').filter_map(|word| word.parse().ok()).collect();
        if let [from, to, ref rest @ ..] = numbers[..] {
            let all = SlotRef::Main(from).stack(actor.state()).map_or(0, |stack| u8::try_from(stack.count).unwrap_or(u8::MAX));
            println!("## put: {:?}", actor.move_item(SlotRef::Main(from), SlotRef::Container(to), rest.first().copied().unwrap_or(all)).await);
        }
        return true;
    }
    if let Some(slot) = line.strip_prefix("!take ").and_then(|slot| slot.parse().ok()) {
        println!("## took: {:?}", actor.move_to_inventory(slot).await);
        return true;
    }
    match line {
        // The use goes on as the bot is polled after (`Bot::consume`).
        "!eat" => drop(tokio::time::timeout(Duration::from_millis(1), actor.consume()).await),
        "!wake" => println!("## woke: {:?}", actor.wake().await),
        "!close" => println!("## closed: {:?}", actor.close_container().await),
        "!takeall" => println!("## took all: {:?}", actor.take_all().await),
        "!jump" => keys(actor, 0.0, false, true),
        "!walk" => keys(actor, 1.0, false, false),
        "!sprint" => keys(actor, 1.0, true, false),
        "!leap" => keys(actor, 1.0, true, true),
        "!stop" => keys(actor, 0.0, false, false),
        _ => return false,
    }
    true
}

pub const CLOCK_SHOWN: &[&str] = &["UpdateAttributes", "SyncWorldClocks", "SetTime", "Text", "LevelSoundEvent", "LevelEvent"];
pub const SHOOT_SHOWN: &[&str] = &["UpdateAttributes", "EntityEvent", "SetEntityData", "LevelSoundEvent", "SetEntityMotion", "RemoveEntity", "AddEntity", "MoveEntityDelta", "MobEffect", "MobEquipment"];
pub const HURT_SHOWN: &[&str] = &[
    "UpdateAttributes", "EntityEvent", "SetEntityData", "SetHealth", "Respawn", "LevelSoundEvent", "LevelEvent", "MovePlayer", "SetEntityMotion", "Text", "DeathInfo", "PlayerAction", "Animate",
    "MobEffect", "SetPlayerGameType", "UpdatePlayerGameType", "RemoveEntity", "AddPlayer", "AddItemEntity", "AddEntity", "InventoryContent", "InventorySlot", "MobEquipment", "MobArmorEquipment",
    "SetSpawnPosition", "CorrectPlayerMovePrediction", "PlayStatus", "ChangeDimension", "SpawnExperienceOrb", "CompletedUsingItem", "ItemStackResponse",
];
const WAKE_SHOWN: &[&str] = &["UpdateBlock", "SetEntityData", "MovePlayer", "PlayerAction", "Animate", "Text", "SetSpawnPosition", "LevelSoundEvent", "CorrectPlayerMovePrediction"];

/// The lines and shown packets of the drill `name`, if it is one of these.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "hurt" => (vitals::hurt(), HURT_SHOWN),
        "smother" => (vitals::smother(), HURT_SHOWN),
        "hunger" => (vitals::hunger(), HURT_SHOWN),
        "eat" => (vitals::eat(), HURT_SHOWN),
        "tire" => (vitals::tire(), HURT_SHOWN),
        "bite" => (hunt::bite(), hunt::BITE_SHOWN),
        "slain" => (hunt::slain(), hunt::BITE_SHOWN),
        "chase" => (hunt::chase(), hunt::BITE_SHOWN),
        "sick" => (hunt::sick(), HURT_SHOWN),
        "worn" => (hunt::worn(), HURT_SHOWN),
        "kinds" => (hunt::kinds(), hunt::KINDS_SHOWN),
        "stroll" => (hunt::stroll(), hunt::KINDS_SHOWN),
        "flee" => (hunt::flee(), hunt::KINDS_SHOWN),
        "around" => (hunt::around(), hunt::BITE_SHOWN),
        "drink" => (hunt::drink(), HURT_SHOWN),
        "more" => (hunt::more(), HURT_SHOWN),
        "shove" => (shove::shove(), shove::SHOVE_SHOWN),
        "byhand" => (shove::byhand(), shove::BYHAND_SHOWN),
        "bedspawn" => (shove::bedspawn(), shove::BED_SHOWN),
        "boom" => (shove::boom(), shove::BOOM_SHOWN),
        "shoot" => (shove::shoot(), shove::BOOM_SHOWN),
        "wear" => (shove::wear(), shove::WEAR_SHOWN),
        "provoke" => (shove::provoke(), hunt::KINDS_SHOWN),
        "provoke2" => (shove::provoke2(), hunt::KINDS_SHOWN),
        "provoke3" => (shove::provoke3(), hunt::KINDS_SHOWN),
        "witch" => (foes::witch(), hunt::SPLASHES_SHOWN),
        "ender" => (foes::ender(), hunt::SPLASHES_SHOWN),
        "goat" => (foes::goat(), hunt::KINDS_SHOWN),
        "goat-cows" => (foes::goat_cows(), hunt::KINDS_SHOWN),
        "mob-effects" => (foes::mob_effects(), hunt::KINDS_SHOWN),
        "mob-effects2" => (foes::mob_effects2(), hunt::KINDS_SHOWN),
        "effects3" => (foes::effects3(), shove::BOOM_SHOWN),
        "wind-burst" => (foes::wind_burst(), shove::BOOM_SHOWN),
        "player-omens" => (foes::player_omens(), foes::OMENS_SHOWN),
        "evoker" => (foes::evoker(), hunt::SPLASHES_SHOWN),
        "shulker" => (foes::shulker(), hunt::SPLASHES_SHOWN),
        "ghast" => (foes::ghast(), shove::BOOM_SHOWN),
        "breeze" => (foes::breeze(), shove::BOOM_SHOWN),
        "vex" => (foes::vex(), shove::BOOM_SHOWN),
        "stare" => (foes::stare(), hunt::SPLASHES_SHOWN),
        "dolphin" => (foes::dolphin(), hunt::KINDS_SHOWN),
        "pandas" => (foes::pandas(), hunt::KINDS_SHOWN),
        "witch-drinks" => (foes::witch_drinks(), hunt::SPLASHES_SHOWN),
        "fx" => (shove::fx(), HURT_SHOWN),
        "drink2" => (shove::drink2(), HURT_SHOWN),
        "shoot2" => (shove::shoot2(), SHOOT_SHOWN),
        "golem" => (shove::golem(), SHOOT_SHOWN),
        "clock" => (world::clock(), CLOCK_SHOWN),
        "sounds" => (world::sounds(), world::SOUNDS_SHOWN),
        "shut" => (reach::shut(), reach::HELD_USE_SHOWN),
        "chest-moves" => (reach::chest_moves(), reach::MOVES_SHOWN),
        "loot" => (world::loot(), world::LOOT_SHOWN),
        "loot2" => (world::loot2(), world::LOOT2_SHOWN),
        "names" => (world::names(), &[]),
        "sun" => (world::sun(), hunt::KINDS_SHOWN),
        "weather" => (world::weather(), world::WEATHER_SHOWN),
        "feedback" => (feedback::feedback(), feedback::FEEDBACK_SHOWN),
        "reach-close" => (reach::reach_close(), reach::REACH_SHOWN),
        "lids" => (reach::lids(), reach::REACH_SHOWN),
        "held-use" => (reach::held_use(), reach::HELD_USE_SHOWN),
        "trapdoors" => (reach::trapdoors(), world::PLACING_SHOWN),
        "place-faces" => (reach::place_faces(), &["UpdateBlock", "InventorySlot", "InventoryContent", "ItemStackResponse", "PlayerHotbar", "MobEquipment"]),
        "messaging" => (feedback::MESSAGING.iter().map(|line| line.to_string()).collect(), feedback::MESSAGING_SHOWN),
        "scoreboard" => (feedback::SCOREBOARD.iter().map(|line| line.to_string()).collect(), feedback::SCOREBOARD_SHOWN),
        "scoreboard2" => (feedback::SCOREBOARD2.iter().map(|line| line.to_string()).collect(), feedback::SCOREBOARD_SHOWN),
        "spiders" => (world::spiders(), hunt::KINDS_SHOWN),
        "job" => (trades::job(), trades::TRADE_SHOWN),
        "trade" => (trades::trade(), trades::TRADE_SHOWN),
        "kinds2" => (world::kinds2(), hunt::KINDS_SHOWN),
        "pushed" => (world::pushed(), world::PUSHED_SHOWN),
        "pvp" => (world::pvp(), HURT_SHOWN),
        "blaze" => (world::blaze(), SHOOT_SHOWN),
        "duel" => (world::duel(), HURT_SHOWN),
        "placing" => (world::placing(), world::PLACING_SHOWN),
        "item-names" => (world::item_names(), world::ITEM_NAMES_SHOWN),
        "placing2" => (world::placing2(), world::PLACING_SHOWN),
        "daylight" => (world::daylight(), world::PLACING_SHOWN),
        "toil" => (armour::toil(), HURT_SHOWN),
        "armour" => (armour::armour(), HURT_SHOWN),
        "effects" => (armour::effects(), HURT_SHOWN),
        "wake" => (armour::wake(), WAKE_SHOWN),
        _ => return uses::named(name),
    })
}
