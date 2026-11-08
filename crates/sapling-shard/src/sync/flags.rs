//! The flag word viewers are told of an entity the server moves: `MOVING`, gravity, fire, and a
//! mob's own flags.

use acacia_proto::packets::SetEntityData;
use acacia_proto::types::{EntityProperties, MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataFlags1 as Flags};
use bytes::Bytes;

use super::Told;

/// A tick's move, squared, past which the next tick shows `MOVING` (docs/research/moving-flag.md).
const MOVING: f32 = 0.001;

/// The metadata item that tells a whole flag word.
pub(crate) fn flags_item(word: Flags) -> MetadataDictionaryItem {
    MetadataDictionaryItem { key: Key::Flags, r#type: Type::Long, legacy_type: 7, value: Value::Flags(word) }
}

/// What tells viewers the entity `id`'s whole flag word, and nothing else.
pub(crate) fn flags_packet(id: u64, word: Flags) -> Bytes {
    sapling_net::encode(&SetEntityData { runtime_entity_id: id, metadata: vec![flags_item(word)], properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 })
}

/// The flags viewers were told the entity has, and whether the last tick moved far enough to show `MOVING`.
#[derive(Clone, Copy, Debug)]
pub(super) struct Shown {
    moving: bool,
    lifted: bool,
    burning: bool,
    moved_far: Option<bool>,
    /// A mob's: the flags it was announced with, and those set since (on fire, a fuse, a charge).
    mob: Option<[i64; 2]>,
    /// The whole word viewers were last told.
    told: i64,
}

impl Shown {
    fn word(&self) -> i64 {
        let bit = |on: bool, flag: Flags| if on { flag.0 } else { 0 };
        match self.mob {
            Some([announced, set]) => announced | set | bit(self.moving, Flags::MOVING),
            None => Flags::HAS_COLLISION.0 | bit(!self.lifted, Flags::AFFECTED_BY_GRAVITY) | bit(self.moving, Flags::MOVING) | bit(self.burning, Flags::ONFIRE),
        }
    }
}

impl Told {
    /// Tells viewers when the entity is moving, lifted and burning, as BDS tells them of items and
    /// falling blocks; `ticked` when its body already made a move, far enough to show, that viewers
    /// were not told of.
    pub fn flag_moving(&mut self, ticked: bool) {
        let shown = Shown { moving: false, lifted: false, burning: false, moved_far: ticked.then_some(true), mob: None, told: 0 };
        self.flags = Some(Shown { told: shown.word(), ..shown });
    }

    /// Tells viewers when the mob, announced with `announced`, is moving; its other flags go
    /// through [`Told::set_flag`] so that each word told has them all.
    pub fn flag_mob(&mut self, announced: Flags) {
        self.flags = Some(Shown { moving: false, lifted: false, burning: false, moved_far: None, mob: Some([announced.0, 0]), told: announced.0 });
    }

    /// Sets or clears `flag` of a mob's: the whole word to tell viewers now, which is then what
    /// they know. `None` for an entity that is no mob [`Told::flag_mob`] was called for.
    pub fn set_flag(&mut self, flag: Flags, on: bool) -> Option<Flags> {
        let shown = self.flags.as_mut()?;
        let [_, set] = shown.mob.as_mut()?;
        *set = if on { *set | flag.0 } else { *set & !flag.0 };
        shown.told = shown.word();
        Some(Flags(shown.told))
    }

    /// A mob's `flag` no longer counts among those it was announced with (a stored baby's, a
    /// tamed one's), so that [`Told::set_flag`] can clear it.
    pub fn forget_announced(&mut self, flag: Flags) {
        if let Some([announced, _]) = self.flags.as_mut().and_then(|shown| shown.mob.as_mut()) {
            *announced &= !flag.0;
        }
    }

    /// The whole word of a mob's flags its viewers know; `None` for an entity that is no mob.
    pub fn mob_flags(&self) -> Option<Flags> {
        self.flags.filter(|shown| shown.mob.is_some()).map(|shown| Flags(shown.told))
    }

    /// [`Told::set_flag`], `None` too where viewers already knew `flag` to be so.
    pub fn change_flag(&mut self, flag: Flags, on: bool) -> Option<Flags> {
        let told = self.flags?.told;
        if (told & flag.0 != 0) == on {
            return None;
        }
        self.set_flag(flag, on)
    }

    /// The entity's flag word when it changed, after a tick that moved it by `gone`, a liquid
    /// holding it up in place of gravity (`lifted`) and lava `burning` it. On BDS `MOVING` shows
    /// whether the tick before moved it far (docs/research/moving-flag.md); the rest go with the
    /// tick's move (research/captures/bds-water-flags.txt, bds-lava.txt).
    pub fn flags_word(&mut self, gone: [f32; 3], lifted: bool, burning: bool) -> Option<Flags> {
        let shown = self.flags.as_mut()?;
        let moved_far = gone.iter().map(|axis| axis * axis).sum::<f32>() > MOVING;
        shown.moving = std::mem::replace(&mut shown.moved_far, Some(moved_far)).unwrap_or(shown.moving);
        (shown.lifted, shown.burning) = (lifted, burning);
        if shown.word() == shown.told {
            return None;
        }
        shown.told = shown.word();
        Some(Flags(shown.told))
    }
}
