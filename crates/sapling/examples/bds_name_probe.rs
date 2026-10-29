//! The names BDS gives items and entities in its answers: `/give` of every item it registers, and of
//! each data value of the items named by theirs (potions, tipped arrows), then `/summon` and `/kill` of
//! every entity kind it lists. Prints `item <identifier> <data> <name>` and `entity <identifier> <name>`
//! lines, tab-separated: crates/sapling-data/data/names.tsv (its README, "Provenance").
//! `cargo run -p sapling --example bds_name_probe -- <server> > names.tsv`

use std::time::Duration;

use acacia_bot::client::{Client, PacketFilter};
use acacia_bot::proto::{Packet, packets};
use acacia_bot::{Bot, BotConfig, BotEvent};

/// Items whose name follows their data value, with the highest value tried.
const BY_DATA: &[(&str, u16)] = &[("minecraft:potion", 46), ("minecraft:splash_potion", 46), ("minecraft:lingering_potion", 46), ("minecraft:arrow", 47), ("minecraft:banner", 15)];
/// Network ids scanned for items; BDS's registry fits well inside.
const IDS: std::ops::Range<i32> = -2048..4096;
/// Gives between clearing the inventory.
const CLEAR_EVERY: usize = 30;
const ANSWER: Duration = Duration::from_secs(3);

/// The first line of the answer to `line`: its key and parameters.
async fn ask(bot: &mut Bot, line: &str) -> Option<(String, Vec<String>)> {
    bot.client().command(line);
    let answer = tokio::time::timeout(ANSWER, async {
        loop {
            match bot.next().await? {
                BotEvent::Packet(packet) if packet.id == packets::CommandOutput::ID => return packet.decode::<packets::CommandOutput>().ok(),
                BotEvent::Disconnected(_) => return None,
                _ => {}
            }
        }
    });
    let output = answer.await.ok()??;
    output.output.into_iter().next().map(|line| (line.message_id, line.parameters))
}

/// The `minecraft:` names in BDS's entity identifier list, as its NBT holds them.
fn entity_kinds() -> Vec<String> {
    let raw = sapling_data::registry_packets()[2].to_vec();
    let text = String::from_utf8_lossy(&raw);
    let mut kinds: Vec<String> = text.match_indices("minecraft:").filter_map(|(at, _)| {
        let name: String = text[at..].chars().take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, ':' | '_' | '.')).collect();
        (name.len() > "minecraft:".len()).then_some(name)
    }).collect();
    kinds.sort();
    kinds.dedup();
    kinds
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = std::env::args().nth(1).ok_or("usage: bds_name_probe <server>")?;
    let config = BotConfig { subscribe: PacketFilter::all(), ..BotConfig::default() };
    let mut bot = Bot::connect(Client::builder(server.as_str()).offline("NameProbe"), config).await?;
    bot.wait_ticks(60).await?;
    ask(&mut bot, "gamemode creative @s").await;
    let items: Vec<String> = IDS.filter_map(|id| bot.state().items.name(id).map(str::to_owned)).collect();
    let mut given = 0;
    for item in &items {
        let data: &[u16] = &BY_DATA.iter().find(|(name, _)| name == item).map_or(vec![0], |(_, most)| (0..=*most).collect::<Vec<u16>>());
        for &value in data {
            if let Some((key, parameters)) = ask(&mut bot, &format!("give @s {item} 1 {value}")).await
                && key == "commands.give.success"
            {
                println!("item\t{item}\t{value}\t{}", parameters[0]);
            }
            given += 1;
            if given % CLEAR_EVERY == 0 {
                ask(&mut bot, "clear @s").await;
            }
        }
    }
    for kind in entity_kinds() {
        if ask(&mut bot, &format!("summon {kind} ~ ~ ~6")).await.is_some_and(|(key, _)| key == "commands.summon.success")
            && let Some((key, parameters)) = ask(&mut bot, &format!("kill @e[type={kind}]")).await
            && key == "commands.kill.successful"
        {
            // One summoned, one name: what was killed beside it (its loot, a rider) comes after.
            println!("entity\t{kind}\t{}", parameters[0].split(", ").next().unwrap_or_default());
        }
        ask(&mut bot, "kill @e[type=item]").await;
    }
    bot.disconnect().await;
    Ok(())
}
