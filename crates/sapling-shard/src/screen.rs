//! The packets of `sapling_api::ScreenAction`, the input locks and `/playanimation`, shaped as
//! BDS 1.26.52 sent them (sapling's research/captures/bds-screen.txt, bds-controls.txt,
//! bds-music.txt).

use std::collections::HashSet;

use acacia_proto::nbt::{Nbt, Value as NbtValue};
use acacia_proto::packets::{
    AnimateEntity, LevelEventGeneric, CameraInstruction, CameraInstructionFade as Fade, CameraInstructionFadeTime as FadeTime, CameraInstructionFov as Fov, CameraInstructionInstructionSet as Set, CameraInstructionInstructionSetEaseData as Ease, CameraShake, CameraShakeAction, PlayerFog,
    SetHud, SetHudVisibility, UnlockedRecipes, UnlockedRecipesUnlockType as Unlock, UpdateClientInputLocks,
};
use acacia_proto::types::{EaseType, Element, InputLockFlags, Vec2f, Vec3f};
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_api::{Animation, Camera, EntityId, Music, PlayerId, RecipeChange, ScreenAction};

use crate::body::Body;
use crate::components::{InLayer, NetId, Outgoing, Position};
use crate::handlers::Ctx;
use crate::hud::Hud;
use crate::inventory::RecipeBook;
use crate::{entities, view};

fn vec3([x, y, z]: [f32; 3]) -> Vec3f {
    Vec3f { x, y, z }
}

fn camera(set: &Camera) -> CameraInstruction {
    let ease_data = set.ease.map(|(kind, duration)| Ease { r#type: EaseType::from_raw(kind.into()), duration });
    let rotation = set.rotation.map(|[pitch, yaw]| Vec2f { x: pitch, z: yaw });
    let instruction = Set {
        runtime_id: set.preset as i32,
        ease_data,
        position: set.position.map(vec3),
        rotation,
        facing: set.facing.map(vec3),
        offset: None,
        entity_offset: None,
        default: set.default.then_some(true),
        remove_ignore_starting_values: set.default,
    };
    CameraInstruction { instruction_set: Some(instruction), ..cleared(None) }
}

fn cleared(clear: Option<bool>) -> CameraInstruction {
    CameraInstruction { instruction_set: None, clear, fade: None, target: None, remove_target: None, fov: None, spline: None, attach_to_entity: None, detach_from_entity: None }
}

fn fade(seconds: Option<[f32; 3]>, colour: Option<[f32; 3]>) -> CameraInstruction {
    let time = seconds.map(|[fade_in_duration, wait_duration, fade_out_duration]| FadeTime { fade_in_duration, wait_duration, fade_out_duration });
    CameraInstruction { fade: Some(Fade { time, color_rgb: colour.map(vec3) }), ..cleared(None) }
}

/// BDS's `fov_clear` is a zero field of view, eased `linear`, with `clear` set.
fn fov(degrees: f32, ease_time: f32, ease: &str, clear: bool) -> CameraInstruction {
    CameraInstruction { fov: Some(Fov { field_of_view: degrees, ease_time, ease_type: ease.to_owned(), clear }), ..cleared(None) }
}

/// BDS's music events (bds-music.txt), shifted as acacia-proto writes the id.
const QUEUE_TRACK: u32 = 1900 << 1;
const PLAY_TRACK: u32 = 1901 << 1;
const STOP_MUSIC: u32 = 1902 << 1;
const MUSIC_VOLUME: u32 = 1903 << 1;

/// `/music`'s event, its fields in BDS's (alphabetical) order.
fn music_event(music: &Music) -> Option<LevelEventGeneric> {
    let float = |name: &str, value: f32| Nbt { name: name.to_owned(), value: NbtValue::Float(value) };
    let (event_id, nbt) = match music {
        Music::Play { track, volume, fade_seconds, looped, queued } => {
            let fields = vec![
                float("fadeSeconds", *fade_seconds),
                Nbt { name: "repeatMode".into(), value: NbtValue::Byte(i8::from(*looped)) },
                Nbt { name: "trackName".into(), value: NbtValue::String(track.as_str().into()) },
                float("volume", *volume),
            ];
            (if *queued { QUEUE_TRACK } else { PLAY_TRACK }, fields)
        }
        Music::Stop { fade_seconds } => (STOP_MUSIC, vec![float("fadeSeconds", *fade_seconds)]),
        Music::Volume(volume) => (MUSIC_VOLUME, vec![float("volume", *volume)]),
        _ => return None,
    };
    Some(LevelEventGeneric { event_id, nbt })
}

/// The packet of `action`.
pub(crate) fn packet(action: &ScreenAction) -> Option<Bytes> {
    // BDS writes a HUD element and the visibility zigzagged, which acacia-proto reads unsigned.
    let zigzag = |value: i32| i64::from(value) * 2;
    Some(match action {
        ScreenAction::Shake { intensity, seconds, rotational } => sapling_net::encode(&CameraShake { intensity: *intensity, duration: *seconds, r#type: u8::from(*rotational), action: CameraShakeAction::Add }),
        ScreenAction::StopShake => sapling_net::encode(&CameraShake { intensity: 0.0, duration: 0.0, r#type: 0, action: CameraShakeAction::Stop }),
        ScreenAction::Fog(stack) => sapling_net::encode(&PlayerFog { stack: stack.clone() }),
        ScreenAction::Hud { elements, hide } => {
            let elements = elements.iter().map(|element| Element::from_raw(zigzag(*element))).collect();
            sapling_net::encode(&SetHud { elements, visibility: SetHudVisibility::from_raw(zigzag(if *hide { 0 } else { 1 })) })
        }
        ScreenAction::Camera(set) => sapling_net::encode(&camera(set)),
        ScreenAction::ClearCamera => sapling_net::encode(&cleared(Some(true))),
        ScreenAction::CameraFade { seconds, colour } => sapling_net::encode(&fade(*seconds, *colour)),
        ScreenAction::CameraFov { degrees, ease_seconds, ease } => sapling_net::encode(&fov(*degrees, *ease_seconds, ease, false)),
        ScreenAction::ClearCameraFov => sapling_net::encode(&fov(0.0, 0.0, "linear", true)),
        ScreenAction::Recipes { ids, change } => {
            let unlock_type = match change {
                RecipeChange::Initial => Unlock::InitiallyUnlocked,
                RecipeChange::Learnt => Unlock::NewlyUnlocked,
                RecipeChange::Forgotten => Unlock::RemoveUnlocked,
                RecipeChange::AllForgotten => Unlock::RemoveAllUnlocked,
                _ => return None,
            };
            sapling_net::encode(&UnlockedRecipes { unlock_type, recipes: ids.clone() })
        }
        ScreenAction::Music(music) => sapling_net::encode(&music_event(music)?),
        _ => return None,
    })
}

impl Ctx<'_> {
    pub(crate) fn act_on_screen(&mut self, player: PlayerId, action: &ScreenAction) {
        if let (ScreenAction::Recipes { ids, change }, Some(entity)) = (action, self.entity(player)) {
            if let Some(mut book) = self.world.get_mut::<RecipeBook>(entity) {
                book.changed(ids, *change);
            }
        }
        if let (Some(packet), Some(mut out)) = (packet(action), self.out(player)) {
            out.0.push(packet);
        }
    }

    pub(crate) fn locked_inputs(&self, player: PlayerId) -> u32 {
        self.entity(player).and_then(|entity| self.world.get::<Hud>(entity)).map_or(0, |hud| hud.input_locks)
    }

    /// Tells the client only of a change, as BDS did for a lock set twice.
    pub(crate) fn lock_inputs(&mut self, player: PlayerId, locks: u32) {
        let Some(mut hud) = self.entity(player).and_then(|entity| self.world.get_mut::<Hud>(entity)) else { return };
        if std::mem::replace(&mut hud.input_locks, locks) == locks {
            return;
        }
        if let Some(mut out) = self.out(player) {
            out.push(&UpdateClientInputLocks { locks: InputLockFlags(locks) });
        }
    }

    /// One AnimateEntity naming every target, to everyone who sees any of them.
    pub(crate) fn play_animation(&mut self, players: &[PlayerId], others: &[EntityId], animation: &Animation) {
        let mut targets: Vec<Entity> = players.iter().filter_map(|player| self.entity(*player)).collect();
        targets.extend(others.iter().filter_map(|id| entities::find(self.world, *id)));
        let runtime_entity_ids = targets.iter().filter_map(|target| self.world.get::<NetId>(*target)).map(|id| id.0).collect();
        let packet = sapling_net::encode(&AnimateEntity {
            animation: animation.name.clone(),
            next_state: animation.next_state.clone(),
            stop_condition: animation.stop_expression.clone(),
            stop_condition_version: 1,
            controller: animation.controller.clone(),
            blend_out_time: animation.blend_out_seconds,
            runtime_entity_ids,
        });
        let mut told = HashSet::new();
        for target in targets {
            let feet = self.world.get::<Body>(target).map(|body| body.at).or_else(|| self.world.get::<Position>(target).map(|at| at.feet));
            let (Some(layer), Some(feet)) = (self.world.get::<InLayer>(target).map(|layer| layer.0), feet) else { continue };
            let viewers = view::viewers(self.world, layer, feet.map(|axis| axis.floor() as i32));
            for viewer in viewers.into_iter().filter(|viewer| told.insert(*viewer)) {
                if let Some(mut out) = self.world.get_mut::<Outgoing>(viewer) {
                    out.0.push(packet.clone());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use acacia_proto::Packet;
    use bytes::BytesMut;

    use super::*;

    fn hex(instruction: &CameraInstruction) -> String {
        let mut body = BytesMut::new();
        instruction.encode(&mut body);
        body.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    /// BDS 1.26.52's bodies, research/captures/bds-commands-findings.md.
    #[test]
    fn fade_and_fov_are_bds_bodies() {
        assert_eq!(hex(&fade(None, None)), "0000010000000000000000");
        assert_eq!(hex(&fade(Some([1.0, 2.0, 3.0]), None)), "000001010000803f000000400000404000000000000000");
        assert_eq!(hex(&fade(None, Some([0.0, 1.0, 0.0]))), "0000010001000000000000803f00000000000000000000");
        assert_eq!(hex(&fov(60.0, 2.0, "linear", false)), "0000000000010000704200000040066c696e65617200000000");
        assert_eq!(hex(&fov(0.0, 0.0, "linear", true)), "0000000000010000000000000000066c696e65617201000000");
    }
}
