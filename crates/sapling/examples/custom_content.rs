//! A plugin's own item and block: a ruby that stacks to 16 and a lamp that glows when lit, on a
//! stone platform. The server builds the resource pack they need and sends it to joining
//! clients. `/give @s custom_content:ruby` and `/give @s custom_content:lamp` (players are
//! operators in creative mode here).
//! `cargo run -p sapling --example custom_content -- [port] [--offline]` (default port 19132).

use std::io::Write;

use flate2::{Compression, Crc, write::ZlibEncoder};
use sapling::{
    Content, CreativeCategory, CustomBlock, CustomItem, GameMode, Handler, Permission, Permutation, Player, Plugin, PluginBuilder, Server, ShardCtx, StateValues,
};

const RUBY: &str = "custom_content:ruby";
const LAMP: &str = "custom_content:lamp";
const LIT: &str = "custom_content:lit";

struct Showcase;

struct Operators;

impl Handler for Operators {
    fn on_join(&mut self, ctx: &mut ShardCtx<'_>, player: &Player) {
        ctx.set_permission(player.id, Permission::Operator);
    }
}

impl Plugin for Showcase {
    fn build(&self, _: &mut PluginBuilder<'_>) {}

    fn content(&self, content: &mut Content) {
        content.item(CustomItem::new(RUBY, "Ruby", square([200, 20, 60, 255], [120, 0, 30, 255])).max_stack(16).category(CreativeCategory::Items));
        let lamp = CustomBlock::new(LAMP, "Lamp", square([250, 210, 90, 255], [140, 90, 20, 255]))
            .state(LIT, StateValues::Bool)
            .light_dampening(0)
            .break_seconds(Some(0.5))
            .permutation(Permutation { light_emission: Some(15), ..Permutation::when(format!("q.block_state('{LIT}') == true")) });
        content.block(lamp);
    }
}

/// A 16x16 PNG: `fill` framed by `edge`.
fn square(fill: [u8; 4], edge: [u8; 4]) -> Vec<u8> {
    let mut pixels = Vec::new();
    for y in 0..16 {
        pixels.push(0);
        (0..16).for_each(|x| pixels.extend(if [x, y].iter().any(|&c| c == 0 || c == 15) { edge } else { fill }));
    }
    let mut zlib = ZlibEncoder::new(Vec::new(), Compression::default());
    zlib.write_all(&pixels).expect("into memory");
    let header = [&16u32.to_be_bytes()[..], &16u32.to_be_bytes(), &[8, 6, 0, 0, 0]].concat();
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    for (kind, data) in [(b"IHDR", header), (b"IDAT", zlib.finish().expect("into memory")), (b"IEND", Vec::new())] {
        let mut crc = Crc::new();
        crc.update(kind);
        crc.update(&data);
        png.extend((data.len() as u32).to_be_bytes());
        png.extend(kind);
        png.extend(&data);
        png.extend(crc.sum().to_be_bytes());
    }
    png
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into())).init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let port = args.iter().find_map(|arg| arg.parse().ok()).unwrap_or(19132);
    let offline = args.iter().any(|arg| arg == "--offline");
    let builder = Server::builder().bind(([0, 0, 0, 0], port).into()).name("sapling custom content").online_mode(!offline).console(true);
    let server = builder.game_mode(GameMode::Creative).handler_each(|| Operators).vanilla_commands().plugin(Showcase).start().await?;
    tokio::select! {
        signal = tokio::signal::ctrl_c() => signal?,
        () = server.stop_requested() => {}
    }
    server.shutdown().await;
    Ok(())
}
