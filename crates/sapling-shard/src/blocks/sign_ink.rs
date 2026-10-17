//! What honeycomb, glow ink, an ink sac or a dye does to a sign a player clicks with it, as BDS's
//! sign interaction (`148b55920`) does it before it would open the editor; and the sound a waxed
//! sign makes instead (docs/research/sign-dyes.md).

use acacia_proto::nbt::{Str, Value};
use acacia_proto::packets::{LevelEvent, LevelEventEvent, LevelSoundEvent};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, GameMode};

use super::send;
use crate::components::{Layer, Mode};
use crate::inventory::{self, Inventory, Place};

const WAXED_FAIL: &str = "block.sign.waxed_interact_fail";
/// Each dye's `SignTextColor` (BDS's table at `14a7716e0`, opaque ARGB): black writes the
/// default black.
const DYES: [(&str, i32); 16] = [
    ("minecraft:black_dye", -16_777_216),
    ("minecraft:red_dye", -5_231_066),
    ("minecraft:green_dye", -10_585_066),
    ("minecraft:brown_dye", -8_170_446),
    ("minecraft:blue_dye", -12_827_478),
    ("minecraft:purple_dye", -7_785_800),
    ("minecraft:cyan_dye", -15_295_332),
    ("minecraft:light_gray_dye", -6_447_721),
    ("minecraft:gray_dye", -12_103_854),
    ("minecraft:pink_dye", -816_214),
    ("minecraft:lime_dye", -8_337_633),
    ("minecraft:yellow_dye", -75_715),
    ("minecraft:light_blue_dye", -12_930_086),
    ("minecraft:magenta_dye", -3_715_395),
    ("minecraft:orange_dye", -425_955),
    ("minecraft:white_dye", -986_896),
];

fn get<'a>(fields: &'a [(Str, Value)], key: &str) -> Option<&'a Value> {
    fields.iter().find(|(name, _)| name == key).map(|(_, value)| value)
}

fn put(fields: &mut Vec<(Str, Value)>, key: &str, value: Value) {
    fields.retain(|(name, _)| name != key);
    fields.push((key.into(), value));
}

/// The sign entity `kept` with the item `item` applied to its side `side`, and the event that
/// shows it with whether it is at the block's middle; `None` when the item does nothing there.
fn applied(kept: &Value, side: &str, item: &str) -> Option<(Value, LevelEventEvent, bool)> {
    let Value::Compound(mut fields) = kept.clone() else { return None };
    if item == "minecraft:honeycomb" {
        put(&mut fields, "IsWaxed", Value::Byte(1));
        return Some((Value::Compound(fields), LevelEventEvent::ParticleWaxOn, true));
    }
    let Some(Value::Compound(mut text)) = get(&fields, side).cloned() else { return None };
    let has_words = matches!(get(&text, "Text"), Some(Value::String(words)) if !words.is_empty());
    let glows = matches!(get(&text, "IgnoreLighting"), Some(Value::Byte(1)));
    let event = match item {
        "minecraft:glow_ink_sac" if !glows => {
            put(&mut text, "IgnoreLighting", Value::Byte(1));
            LevelEventEvent::InkSackUsed
        }
        "minecraft:ink_sac" if glows => {
            put(&mut text, "IgnoreLighting", Value::Byte(0));
            LevelEventEvent::InkSackUsed
        }
        _ => {
            let (_, colour) = DYES.iter().find(|(dye, _)| *dye == item)?;
            if get(&text, "SignTextColor") == Some(&Value::Int(*colour)) {
                return None;
            }
            put(&mut text, "SignTextColor", Value::Int(*colour));
            LevelEventEvent::DyeUsed
        }
    };
    if !has_words {
        return None;
    }
    put(&mut text, "HideGlowOutline", Value::Byte(0));
    put(&mut fields, side, Value::Compound(text));
    Some((Value::Compound(fields), event, false))
}

fn at([x, y, z]: BlockPos, middle: bool) -> Vec3f {
    let offset = if middle { 0.5 } else { 0.0 };
    Vec3f { x: x as f32 + offset, y: y as f32 + offset, z: z as f32 + offset }
}

/// Lets those who see a waxed sign hear that it will not change.
pub(super) fn refuse_waxed(world: &mut World, layer: Entity, pos: BlockPos) {
    let sound = LevelSoundEvent {
        sound_id: WAXED_FAIL.into(),
        position: at(pos, false),
        extra_data: -1,
        entity_type: String::new(),
        is_baby_mob: false,
        is_global: false,
        entity_unique_id: -1,
        fire_at_position: None,
    };
    send::to_viewers(world, layer, pos, &[sapling_net::encode(&sound)]);
}

/// Applies what a player holds to the side `side` (`FrontText` or `BackText`) of the sign at
/// `pos`, for everyone who sees it, and takes one of it outside creative mode; false when it does
/// nothing there, and the editor is to open instead.
pub(super) fn apply(world: &mut World, player: Entity, layer: Entity, pos: BlockPos, side: &str) -> bool {
    let item = world.get::<Inventory>(player).and_then(|inventory| inventory.in_hand().as_ref()).map(|held| held.stack.name.clone());
    let kept = world.get::<Layer>(layer).and_then(|layer| layer.world.block_entity(pos)).map(|kept| kept.nbt.clone());
    let Some((entity, event, middle)) = item.zip(kept).and_then(|(item, kept)| applied(&kept, side, &item)) else { return false };
    send::keep_entity(world, layer, pos, &entity);
    let shown = sapling_net::encode(&LevelEvent { event, position: at(pos, middle), data: 0 });
    send::to_viewers(world, layer, pos, &[shown]);
    if world.get::<Mode>(player).is_none_or(|mode| mode.0 != GameMode::Creative) {
        inventory::change(world, player, |inventory| inventory.take(Place::Main(inventory.selected.into()), 1).is_ok());
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sign(words: &str) -> Value {
        let side = Value::Compound(vec![("SignTextColor".into(), Value::Int(-16_777_216)), ("IgnoreLighting".into(), Value::Byte(0)), ("Text".into(), Value::String(words.into()))]);
        Value::Compound(vec![("FrontText".into(), side), ("IsWaxed".into(), Value::Byte(0))])
    }

    fn front(entity: &Value, key: &str) -> Option<Value> {
        entity.get("FrontText")?.get(key).cloned()
    }

    #[test]
    fn dyes_and_ink_change_a_written_side_and_wax_the_whole_sign() {
        let (red, event, _) = applied(&sign("hi"), "FrontText", "minecraft:red_dye").expect("dyed");
        assert_eq!((front(&red, "SignTextColor"), event), (Some(Value::Int(-5_231_066)), LevelEventEvent::DyeUsed));
        let (glowing, _, _) = applied(&red, "FrontText", "minecraft:glow_ink_sac").expect("glows");
        assert_eq!(front(&glowing, "IgnoreLighting"), Some(Value::Byte(1)));
        assert!(applied(&glowing, "FrontText", "minecraft:glow_ink_sac").is_none(), "already glowing");
        assert!(applied(&red, "FrontText", "minecraft:red_dye").is_none(), "already red");
        assert!(applied(&sign(""), "FrontText", "minecraft:red_dye").is_none(), "nothing written");
        assert!(applied(&sign("hi"), "FrontText", "minecraft:black_dye").is_none(), "black is the default");
        let (waxed, event, middle) = applied(&sign(""), "FrontText", "minecraft:honeycomb").expect("waxed");
        assert_eq!((waxed.get("IsWaxed"), event, middle), (Some(&Value::Byte(1)), LevelEventEvent::ParticleWaxOn, true));
        assert!(applied(&sign("hi"), "FrontText", "minecraft:lapis_lazuli").is_none());
    }
}
