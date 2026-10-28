//! Powder snow in a cauldron, as BDS ran it (research/captures/bds-cauldrons3.txt): its bucket
//! fills an empty cauldron, or one of powder snow, to the brim with event 3514 at the full
//! surface, and bursts one of anything else (`CauldronExplode` at the floor, the hand a bucket);
//! a bucket takes a full one with 3515; any bottle poured on it bursts it, unheard. Rain and snow
//! fill a cauldron by BDS's decompiled `CauldronBlock::handlePrecipitation` (141a46850).

use acacia_proto::nbt::Value;
use acacia_proto::packets::LevelEventEvent;
use sapling_api::BlockPos;
use sapling_world::{ItemStack, World as Level};

use super::{BOTTLES, CAULDRON, Cost, FULL, Held, Outcome, burst, stack, with_contents};

pub(super) const BUCKET: &str = "minecraft:powder_snow_bucket";
const FILL: LevelEventEvent = LevelEventEvent::Unknown(3514);
const TAKE: LevelEventEvent = LevelEventEvent::Unknown(3515);
/// What the snow events carry: plain water's colour, as BDS sent.
const COLOUR: i32 = super::WATER;
/// Snow, not rain, fills a cauldron colder than this.
const SNOWS_UNDER: f32 = 0.15;

/// What a click with `hand` does to a cauldron of powder snow, or what the snow bucket does to any.
pub(super) fn change(hand: &ItemStack, held: Held) -> Option<Outcome> {
    let name = hand.name.as_str();
    if name == BUCKET {
        let bucket = Cost::Becomes(stack("minecraft:bucket", 0));
        return Some(if held.level == 0 || held.snow {
            let next = Held { level: FULL, potion: None, colour: None, snow: true };
            Outcome { next, told: Some((FILL, FULL, COLOUR)), sound: None, cost: bucket }
        } else {
            burst(held, None, bucket)
        });
    }
    match name {
        "minecraft:bucket" if held.level == FULL => {
            let next = Held { level: 0, potion: None, colour: None, snow: false };
            Some(Outcome { next, told: Some((TAKE, FULL, COLOUR)), sound: None, cost: Cost::Becomes(stack(BUCKET, 0)) })
        }
        _ if BOTTLES.contains(&name) && held.level > 0 => Some(burst(held, None, Cost::Becomes(stack("minecraft:glass_bottle", 0)))),
        _ => None,
    }
}

/// The cauldron at `pos` a level fuller of the rain or snow falling on it: snow where it is
/// colder than 0.15, as powder snow; none into lava, a potion, dyed water, a full cauldron, or
/// snow into water and rain into powder snow.
pub(in crate::blocks) fn fallen_in(level: &Level, [x, y, z]: BlockPos) -> Option<u32> {
    let state = level.registry().get(level.block(x, y, z)).filter(|state| state.name == CAULDRON)?;
    let filled: u8 = state.property("fill_level")?.parse().ok()?;
    let liquid = state.property("cauldron_liquid")?;
    let snows = level.temperature(x, y, z).is_some_and(|t| t < SNOWS_UNDER);
    let entity = level.block_entity([x, y, z]);
    let potion = entity.is_some_and(|entity| matches!(entity.nbt.get("PotionId"), Some(Value::Short(id)) if *id >= 0));
    let dyed = entity.is_some_and(|entity| entity.nbt.get("CustomColor").is_some());
    let unlike = filled > 0 && (potion || dyed || liquid == "lava" || (liquid == "powder_snow") != snows);
    let open = level.light_at(x, y + 1, z).is_some_and(|(sky, _)| sky == 15);
    (filled < FULL && !unlike && open).then(|| with_contents(level.registry(), level.block(x, y, z), filled + 1, snows)).flatten()
}
