//! What a player's body is doing, as its flags and box height tell viewers.

use acacia_proto::packets::SetEntityData;
use acacia_proto::types::{EntityProperties, InputData, MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault, MetadataFlags1};

const STANDING_HEIGHT: f32 = 1.8;
const SNEAKING_HEIGHT: f32 = 1.49;
/// A glider's box, as BDS told it with the flag (research/captures/bds-glide.txt).
const GLIDING_HEIGHT: f32 = 0.6;

#[derive(Clone, Copy, PartialEq, Default)]
pub(crate) struct Pose {
    pub sneaking: bool,
    pub sprinting: bool,
    pub swimming: bool,
    /// On fire (vitals.rs).
    pub burning: bool,
    /// Using the item in its hand (vitals/eating.rs).
    pub using: bool,
    /// Flying on an elytra (gliding.rs), which the server starts and stops itself.
    pub gliding: bool,
}

impl Pose {
    /// The entity flags viewers get on top of the recorded ones.
    pub fn flags(self) -> MetadataFlags1 {
        let mut flags = MetadataFlags1::BREATHING;
        let shown = [
            (self.sneaking, MetadataFlags1::SNEAKING),
            (self.sprinting, MetadataFlags1::SPRINTING),
            (self.swimming, MetadataFlags1::SWIMMING),
            (self.burning, MetadataFlags1::ONFIRE),
            (self.using, MetadataFlags1::ACTION),
            (self.gliding, MetadataFlags1::GLIDING),
        ];
        for (on, flag) in shown {
            if on {
                flags.insert(flag);
            }
        }
        flags
    }

    fn height(self) -> f32 {
        match self {
            _ if self.gliding => GLIDING_HEIGHT,
            _ if self.sneaking => SNEAKING_HEIGHT,
            _ => STANDING_HEIGHT,
        }
    }

    pub(super) fn apply(&mut self, input: &[InputData]) {
        for flag in input {
            match flag {
                InputData::StartSneaking => self.sneaking = true,
                InputData::StopSneaking => self.sneaking = false,
                InputData::StartSprinting => self.sprinting = true,
                InputData::StopSprinting => self.sprinting = false,
                InputData::StartSwimming => self.swimming = true,
                InputData::StopSwimming => self.swimming = false,
                _ => {}
            }
        }
    }
}

/// The glide an input asks for: `Some(true)` to start, `Some(false)` to stop.
pub(super) fn glide_asked(input: &[InputData]) -> Option<bool> {
    input.iter().rev().find_map(|flag| match flag {
        InputData::StartGliding => Some(true),
        InputData::StopGliding => Some(false),
        _ => None,
    })
}

pub(crate) fn pose_packet(id: u64, pose: Pose, tick: u64) -> SetEntityData {
    let height = MetadataDictionaryItem {
        key: Key::BoundingboxHeight,
        r#type: MetadataDictionaryItemType::Float,
        legacy_type: 3,
        value: Value::Default(MetadataDictionaryItemValueDefault::Float(pose.height())),
    };
    let flags = sapling_data::player_metadata("", pose.flags()).into_iter().filter(|item| item.key == Key::Flags);
    SetEntityData { runtime_entity_id: id, metadata: flags.chain([height]).collect(), properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick }
}
