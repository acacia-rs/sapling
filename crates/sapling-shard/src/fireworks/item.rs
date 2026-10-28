//! A firework rocket's and a firework star's item tag, as Bedrock keeps them: a rocket's
//! `Fireworks` holds its `Flight` and its `Explosions`, a star's `FireworksItem` one explosion
//! and its `customColor`.

use acacia_proto::nbt::{List, Value};
use sapling_world::ItemStack;

pub(crate) const ROCKET: &str = "minecraft:firework_rocket";
pub(crate) const STAR: &str = "minecraft:firework_star";
const FIREWORKS: &str = "Fireworks";
const FLIGHT: &str = "Flight";
const EXPLOSIONS: &str = "Explosions";
const STAR_TAG: &str = "FireworksItem";
const CUSTOM_COLOUR: &str = "customColor";
const COMPOUND: u8 = 10;

/// One burst of a rocket: its shape (0 a small ball, 1 a large one, 2 a star, 3 a creeper, 4 a
/// burst), its colours and fade colours as dye colour numbers, and whether it twinkles and trails.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Explosion {
    pub shape: u8,
    pub colours: Vec<u8>,
    pub fades: Vec<u8>,
    pub flicker: bool,
    pub trail: bool,
}

fn byte(value: Option<&Value>) -> i8 {
    match value {
        Some(Value::Byte(byte)) => *byte,
        _ => 0,
    }
}

fn bytes(value: Option<&Value>) -> Vec<u8> {
    match value {
        Some(Value::ByteArray(bytes)) => bytes.clone(),
        _ => Vec::new(),
    }
}

impl Explosion {
    fn read(value: &Value) -> Self {
        Self {
            shape: byte(value.get("FireworkType")) as u8,
            colours: bytes(value.get("FireworkColor")),
            fades: bytes(value.get("FireworkFade")),
            flicker: byte(value.get("FireworkFlicker")) != 0,
            trail: byte(value.get("FireworkTrail")) != 0,
        }
    }

    fn written(&self) -> Value {
        Value::Compound(vec![
            ("FireworkColor".into(), Value::ByteArray(self.colours.clone())),
            ("FireworkFade".into(), Value::ByteArray(self.fades.clone())),
            ("FireworkFlicker".into(), Value::Byte(self.flicker.into())),
            ("FireworkTrail".into(), Value::Byte(self.trail.into())),
            ("FireworkType".into(), Value::Byte(self.shape as i8)),
        ])
    }
}

/// A rocket's `Flight`: how many gunpowder it was made with.
pub(crate) fn flight(stack: &ItemStack) -> u8 {
    byte(stack.tag.as_ref().and_then(|tag| tag.get(FIREWORKS)).and_then(|fireworks| fireworks.get(FLIGHT))).max(0) as u8
}

/// A rocket's bursts.
pub(crate) fn explosions(stack: &ItemStack) -> Vec<Explosion> {
    match stack.tag.as_ref().and_then(|tag| tag.get(FIREWORKS)).and_then(|fireworks| fireworks.get(EXPLOSIONS)) {
        Some(Value::List(list)) => list.items.iter().map(Explosion::read).collect(),
        _ => Vec::new(),
    }
}

/// A star's burst.
pub(crate) fn star(stack: &ItemStack) -> Option<Explosion> {
    stack.tag.as_ref()?.get(STAR_TAG).map(Explosion::read)
}

/// The tag of a rocket of `flight` with `bursts`.
pub(crate) fn rocket_tag(flight: u8, bursts: &[Explosion]) -> Value {
    let list = Value::List(List { tag: COMPOUND, items: bursts.iter().map(Explosion::written).collect() });
    Value::Compound(vec![(FIREWORKS.into(), Value::Compound(vec![(EXPLOSIONS.into(), list), (FLIGHT.into(), Value::Byte(flight as i8))]))])
}

/// The tag of a star of `burst`, its colour the mean of its colours' (`customColor`, ARGB).
pub(crate) fn star_tag(burst: &Explosion) -> Value {
    let mut fields = vec![(STAR_TAG.into(), burst.written())];
    if let Some(colour) = mean_colour(&burst.colours) {
        fields.push((CUSTOM_COLOUR.into(), Value::Int(colour)));
    }
    Value::Compound(fields)
}

/// The dye colours' RGB, by dye colour number (the Java game's `DyeColor` diffuse colours).
pub(super) const RGB: [u32; 16] = [
    0x1D1D21, 0xB02E26, 0x5E7C16, 0x835432, 0x3C44AA, 0x8932B8, 0x169C9C, 0x9D9D97, 0x474F52, 0xF38BAA, 0x80C71F, 0xFED83D, 0x3AB3DA, 0xC74EBD, 0xF9801D, 0xF9FFFE,
];

/// The dyes by dye colour number, and the older items that dye as one.
const DYES: [&str; 16] = ["black", "red", "green", "brown", "blue", "purple", "cyan", "light_gray", "gray", "pink", "lime", "yellow", "light_blue", "magenta", "orange", "white"];
const OLD_DYES: &[(&str, u8)] = &[("minecraft:ink_sac", 0), ("minecraft:cocoa_beans", 3), ("minecraft:lapis_lazuli", 4), ("minecraft:bone_meal", 15)];

/// The dye colour number of a dye item, banners' numbering too.
pub(crate) fn dye(name: &str) -> Option<u8> {
    let named = name.strip_prefix("minecraft:").and_then(|bare| bare.strip_suffix("_dye")).and_then(|colour| DYES.iter().position(|dye| *dye == colour));
    named.map(|colour| colour as u8).or_else(|| OLD_DYES.iter().find(|(old, _)| *old == name).map(|(_, colour)| *colour))
}

fn mean_colour(colours: &[u8]) -> Option<i32> {
    let known: Vec<u32> = colours.iter().filter_map(|colour| RGB.get(usize::from(*colour)).copied()).collect();
    if known.is_empty() {
        return None;
    }
    let mean = |shift: u32| known.iter().map(|rgb| (rgb >> shift) & 0xFF).sum::<u32>() / known.len() as u32;
    Some((0xFF00_0000 | mean(16) << 16 | mean(8) << 8 | mean(0)) as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rocket_reads_back_what_it_was_made_with() {
        let burst = Explosion { shape: 3, colours: vec![1, 4], fades: vec![15], flicker: true, trail: false };
        let rocket = ItemStack { name: ROCKET.into(), count: 3, damage: 0, block: None, tag: Some(rocket_tag(2, std::slice::from_ref(&burst))) };
        assert_eq!((flight(&rocket), explosions(&rocket)), (2, vec![burst]));
        let plain = ItemStack { tag: None, ..rocket };
        assert_eq!((flight(&plain), explosions(&plain).len()), (0, 0));
    }

    #[test]
    fn a_star_is_coloured_by_the_mean_of_its_dyes() {
        assert_eq!(mean_colour(&[15]), Some(0xFFF9_FFFE_u32 as i32));
        assert_eq!(mean_colour(&[]), None);
        let tag = star_tag(&Explosion { colours: vec![1], ..Explosion::default() });
        let star_stack = ItemStack { name: STAR.into(), count: 1, damage: 0, block: None, tag: Some(tag) };
        assert_eq!(star(&star_stack).map(|burst| burst.colours), Some(vec![1]));
    }
}
