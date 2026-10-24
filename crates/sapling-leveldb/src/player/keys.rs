//! Where a player's record lies. `player_<id>` maps the player's id (its UUID as text) to the key
//! of its `player_server_` compound, as BDS and dragonfly keep them (research/worlds-and-ecs.md);
//! a world BDS saved may name any key there. Sapling once kept players under its own
//! `sapling_player_<id>`: that record is read while no vanilla one exists, and dropped at the
//! player's next save.

use acacia_proto::nbt::{self, LittleEndian, Nbt, Value};
use acacia_world::BlockRegistry;
use bytes::BytesMut;

use super::PlayerData;
use crate::{Batch, Db, Error};

const MAPPING: &str = "player_";
const SERVER: &str = "player_server_";
const LEGACY: &str = "sapling_player_";

fn key(prefix: &str, id: &str) -> Vec<u8> {
    [prefix, id].concat().into_bytes()
}

/// The key `player_<id>` names, if it is there.
fn mapped(db: &Db, id: &str) -> Result<Option<Vec<u8>>, Error> {
    let Some(record) = db.get(&key(MAPPING, id))? else { return Ok(None) };
    match nbt::read::<LittleEndian>(&mut &record[..])?.value.get("ServerId") {
        Some(Value::String(server)) if !server.is_empty() => Ok(Some(server.as_bytes().to_vec())),
        _ => Err(Error::Player("a player mapping without a ServerId")),
    }
}

/// A new mapping: dragonfly's fields, the player's id standing for both of its identities.
fn mapping(id: &str, server: &[u8]) -> Vec<u8> {
    let server = String::from_utf8_lossy(server);
    let fields = vec![("MsaId".into(), Value::String(id.into())), ("SelfSignedId".into(), Value::String(id.into())), ("ServerId".into(), Value::String(server.as_ref().into()))];
    let mut out = BytesMut::new();
    nbt::write::<LittleEndian>(&mut out, &Nbt { name: String::new(), value: Value::Compound(fields) });
    out.to_vec()
}

/// What was kept of player `id`; `None` for a first visit.
pub(crate) fn load(db: &Db, id: &str) -> Result<Option<PlayerData>, Error> {
    let server = mapped(db, id)?.unwrap_or_else(|| key(SERVER, id));
    if let Some(record) = db.get(&server)? {
        return PlayerData::decode(&record).map(Some);
    }
    db.get(&key(LEGACY, id))?.map(|record| PlayerData::decode_legacy(&record)).transpose()
}

/// Keeps `player` over the record already there, mapping a player seen for the first time and
/// dropping a legacy record, in one batch.
pub(crate) fn store(db: &Db, id: &str, player: &PlayerData, registry: &BlockRegistry) -> Result<(), Error> {
    let known = mapped(db, id)?;
    let server = known.clone().unwrap_or_else(|| key(SERVER, id));
    let record = player.encode_over(registry, db.get(&server)?.as_deref())?;
    let mut batch = Batch::default();
    if known.is_none() {
        batch.put(&key(MAPPING, id), &mapping(id, &server));
    }
    batch.put(&server, &record);
    batch.delete(&key(LEGACY, id));
    db.write(batch)
}
