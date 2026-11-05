//! The `/mobevent` switches under the key `mobevents`: a compound of one byte per switch, as a BDS
//! 1.26.5x save holds it (`events_enabled`, `minecraft:ender_dragon_event`,
//! `minecraft:pillager_patrols_event`, `minecraft:wandering_trader_event`, each 1 when on).

use acacia_proto::nbt::{self, LittleEndian, Value};

use crate::{Db, Error};

const KEY: &[u8] = b"mobevents";

/// The switches `db` keeps, by name; `None` for a world that kept none.
pub fn load_mob_events(db: &Db) -> Result<Option<Vec<(String, bool)>>, Error> {
    let Some(record) = db.get(KEY)? else { return Ok(None) };
    let Value::Compound(fields) = nbt::read::<LittleEndian>(&mut &record[..])?.value else { return Ok(None) };
    Ok(Some(fields.into_iter().filter_map(|(name, value)| if let Value::Byte(on) = value { Some((name.to_string(), on != 0)) } else { None }).collect()))
}

/// Keeps `switches` in `db`, sorted by name as BDS writes them.
pub fn store_mob_events(db: &Db, switches: &[(String, bool)]) -> Result<(), Error> {
    let mut switches = switches.to_vec();
    switches.sort();
    let fields = switches.into_iter().map(|(name, on)| (name.into(), Value::Byte(i8::from(on)))).collect();
    db.put(KEY, &crate::actors::written(Value::Compound(fields)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switches_read_back_as_kept() {
        let folder = std::env::temp_dir().join(format!("sapling-mobevents-{}", std::process::id()));
        let db = Db::create(&folder).unwrap();
        assert_eq!(load_mob_events(&db).unwrap(), None);
        let kept = vec![("minecraft:pillager_patrols_event".to_string(), false), ("events_enabled".to_string(), true)];
        store_mob_events(&db, &kept).unwrap();
        let read = load_mob_events(&db).unwrap().unwrap();
        assert_eq!(read, [("events_enabled".to_string(), true), ("minecraft:pillager_patrols_event".to_string(), false)]);
        drop(db);
        let _ = std::fs::remove_dir_all(folder);
    }
}
