//! Entity data told among packets, less the first data that follows each announcement
//! (src/entities/first_data.rs), as BDS sends it too.

use acacia_proto::packets::{AddEntity, SetEntityData};
use acacia_proto::{Packet, RawPacket};

/// The `SetEntityData` among `packets` but each announced entity's first after its `AddEntity`.
#[allow(dead_code)]
pub fn changes(packets: &[RawPacket]) -> Vec<SetEntityData> {
    let mut announced = Vec::new();
    let mut changes = Vec::new();
    for packet in packets {
        if packet.id == AddEntity::ID {
            announced.push(packet.decode::<AddEntity>().unwrap().runtime_id);
        } else if packet.id == SetEntityData::ID {
            let data: SetEntityData = packet.decode().unwrap();
            match announced.iter().position(|id| *id == data.runtime_entity_id) {
                Some(at) => {
                    announced.swap_remove(at);
                }
                None => changes.push(data),
            }
        }
    }
    changes
}
