//! A mob is announced to a later viewer as it is now: BDS's `AddEntity` then carries the whole flag word
//! the mob's viewers were last told, BREATHING, MOVING, a charge, invisibility and a name included
//! (research/captures/bds-lateeffects.txt, bds-bg2names.txt). A mob new this tick is announced as made,
//! its first tick's flags told after (entities/tell.rs).

use acacia_proto::RawPacket;
use acacia_proto::packets::AddEntity;
use acacia_proto::types::MetadataDictionaryItemValue as Value;
use bytes::Bytes;

use crate::sync::Told;

/// `announcement` with its `AddEntity`'s flags the word `told` holds now.
pub(super) fn as_now(mut announcement: Vec<Bytes>, told: Option<&Told>) -> Vec<Bytes> {
    let Some(word) = told.and_then(Told::mob_flags) else { return announcement };
    let Some(mut added) = announcement.first().and_then(|first| RawPacket::parse(first.clone()).ok()?.decode::<AddEntity>().ok()) else { return announcement };
    let Some(flags) = added.metadata.iter_mut().find_map(|item| if let Value::Flags(flags) = &mut item.value { Some(flags) } else { None }) else { return announcement };
    if flags.0 != word.0 {
        *flags = word;
        announcement[0] = sapling_net::encode(&added);
    }
    announcement
}
