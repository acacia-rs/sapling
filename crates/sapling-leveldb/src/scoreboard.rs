//! The world's scoreboard, under the key `scoreboard`, as BDS 1.26.5x keeps it (BDSRE: load
//! 1401ffab0, an identity 14266e220, an objective 142667ef0, a slot 142666a20; the drill world of
//! research/captures/bds-scoreboard.txt): `Criteria` (BDS leaves `dummy` out, so empty),
//! `DisplayObjectives`, `Entries` (one identity per holder with a score), `LastUniqueID` and
//! `Objectives`, each objective's `Scores` naming holders by their `ScoreboardId`. BDS writes every
//! compound's keys in name order.

use acacia_proto::nbt::{self, List, LittleEndian, Nbt, Str, Value};
use bytes::BytesMut;

use crate::Error;

pub(crate) const KEY: &[u8] = b"scoreboard";
const END: u8 = 0;
const COMPOUND: u8 = 10;

/// Who holds a score: a player and an entity by their `UniqueID`, a fake player by its name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ScoreIdentity {
    Player(i64),
    Entity(i64),
    Fake(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectiveRecord {
    pub name: String,
    pub display_name: String,
    pub criteria: String,
    /// `(ScoreboardId, score)`.
    pub scores: Vec<(i64, i32)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayRecord {
    /// `sidebar`, `list` or `belowname`.
    pub slot: String,
    pub objective: String,
    /// 0 ascending, 1 descending.
    pub sort_order: i8,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ScoreboardRecord {
    pub objectives: Vec<ObjectiveRecord>,
    pub displays: Vec<DisplayRecord>,
    /// `(ScoreboardId, who)`.
    pub entries: Vec<(i64, ScoreIdentity)>,
    /// The last `ScoreboardId` given out.
    pub last_unique_id: i64,
}

fn list(items: Vec<Value>) -> Value {
    Value::List(List { tag: if items.is_empty() { END } else { COMPOUND }, items })
}

fn compound(fields: Vec<(&str, Value)>) -> Value {
    Value::Compound(fields.into_iter().map(|(key, value)| (Str::from(key), value)).collect())
}

fn items<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    match value.get(key) {
        Some(Value::List(list)) => &list.items,
        _ => &[],
    }
}

fn text(value: &Value, key: &str) -> Option<String> {
    if let Some(Value::String(text)) = value.get(key) { Some(text.to_string()) } else { None }
}

fn long(value: &Value, key: &str) -> Option<i64> {
    if let Some(Value::Long(n)) = value.get(key) { Some(*n) } else { None }
}

fn identity(entry: &Value) -> Option<(i64, ScoreIdentity)> {
    let who = match entry.get("IdentityType") {
        Some(Value::Byte(1)) => ScoreIdentity::Player(long(entry, "PlayerId")?),
        Some(Value::Byte(2)) => ScoreIdentity::Entity(long(entry, "EntityID")?),
        Some(Value::Byte(3)) => ScoreIdentity::Fake(text(entry, "FakePlayerName")?),
        _ => return None,
    };
    Some((long(entry, "ScoreboardId")?, who))
}

fn objective(kept: &Value) -> Option<ObjectiveRecord> {
    let score = |kept: &Value| match kept.get("Score") {
        Some(Value::Int(score)) => Some((long(kept, "ScoreboardId")?, *score)),
        _ => None,
    };
    let name = text(kept, "Name")?;
    Some(ObjectiveRecord {
        display_name: text(kept, "DisplayName").unwrap_or_else(|| name.clone()),
        criteria: text(kept, "Criteria").unwrap_or_else(|| "dummy".into()),
        scores: items(kept, "Scores").iter().filter_map(score).collect(),
        name,
    })
}

fn display(kept: &Value) -> Option<DisplayRecord> {
    let sort_order = if let Some(Value::Byte(order)) = kept.get("SortOrder") { *order } else { 1 };
    Some(DisplayRecord { slot: text(kept, "Name")?, objective: text(kept, "ObjectiveName")?, sort_order })
}

/// What a record holds; parts it cannot read are left out.
pub fn decode_scoreboard(record: &[u8]) -> Result<ScoreboardRecord, Error> {
    let root = nbt::read::<LittleEndian>(&mut &record[..])?.value;
    Ok(ScoreboardRecord {
        objectives: items(&root, "Objectives").iter().filter_map(objective).collect(),
        displays: items(&root, "DisplayObjectives").iter().filter_map(display).collect(),
        entries: items(&root, "Entries").iter().filter_map(identity).collect(),
        last_unique_id: long(&root, "LastUniqueID").unwrap_or(0),
    })
}

fn entry((id, who): &(i64, ScoreIdentity)) -> Value {
    let (kind, key, value) = match who {
        ScoreIdentity::Player(unique) => (1, "PlayerId", Value::Long(*unique)),
        ScoreIdentity::Entity(unique) => (2, "EntityID", Value::Long(*unique)),
        ScoreIdentity::Fake(name) => (3, "FakePlayerName", Value::String(name.as_str().into())),
    };
    let mut fields = vec![(key, value), ("IdentityType", Value::Byte(kind)), ("ScoreboardId", Value::Long(*id))];
    fields.sort_by_key(|(key, _)| *key);
    compound(fields)
}

pub fn encode_scoreboard(board: &ScoreboardRecord) -> Vec<u8> {
    let objective = |kept: &ObjectiveRecord| {
        let scores = kept.scores.iter().map(|(id, score)| compound(vec![("Score", Value::Int(*score)), ("ScoreboardId", Value::Long(*id))])).collect();
        let named = |text: &str| Value::String(text.into());
        compound(vec![("Criteria", named(&kept.criteria)), ("DisplayName", named(&kept.display_name)), ("Name", named(&kept.name)), ("Scores", list(scores))])
    };
    let display = |kept: &DisplayRecord| {
        compound(vec![("Name", Value::String(kept.slot.as_str().into())), ("ObjectiveName", Value::String(kept.objective.as_str().into())), ("SortOrder", Value::Byte(kept.sort_order))])
    };
    let root = compound(vec![
        ("Criteria", list(Vec::new())),
        ("DisplayObjectives", list(board.displays.iter().map(display).collect())),
        ("Entries", list(board.entries.iter().map(entry).collect())),
        ("LastUniqueID", Value::Long(board.last_unique_id)),
        ("Objectives", list(board.objectives.iter().map(objective).collect())),
    ]);
    let mut out = BytesMut::new();
    nbt::write::<LittleEndian>(&mut out, &Nbt { name: String::new(), value: root });
    out.to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The board of research/captures/bds-scoreboard.txt's world as BDS left it, cut to one holder
    /// of each kind: what `records` printed of its `scoreboard` key, field by field.
    fn bds_board() -> Value {
        let score = |score: i32, id: i64| compound(vec![("Score", Value::Int(score)), ("ScoreboardId", Value::Long(id))]);
        let named = |text: &str| Value::String(text.into());
        compound(vec![
            ("Criteria", Value::List(List { tag: END, items: Vec::new() })),
            ("DisplayObjectives", Value::List(List { tag: COMPOUND, items: vec![compound(vec![("Name", named("sidebar")), ("ObjectiveName", named("obj")), ("SortOrder", Value::Byte(1))])] })),
            (
                "Entries",
                Value::List(List {
                    tag: COMPOUND,
                    items: vec![
                        compound(vec![("EntityID", Value::Long(-4294967282)), ("IdentityType", Value::Byte(2)), ("ScoreboardId", Value::Long(8))]),
                        compound(vec![("FakePlayerName", named("Newbie")), ("IdentityType", Value::Byte(3)), ("ScoreboardId", Value::Long(9))]),
                        compound(vec![("IdentityType", Value::Byte(1)), ("PlayerId", Value::Long(-4294967290)), ("ScoreboardId", Value::Long(3))]),
                    ],
                }),
            ),
            ("LastUniqueID", Value::Long(9)),
            (
                "Objectives",
                Value::List(List {
                    tag: COMPOUND,
                    items: vec![
                        compound(vec![("Criteria", named("dummy")), ("DisplayName", named("long")), ("Name", named("long")), ("Scores", Value::List(List { tag: END, items: Vec::new() }))]),
                        compound(vec![
                            ("Criteria", named("dummy")),
                            ("DisplayName", named("obj")),
                            ("Name", named("obj")),
                            ("Scores", Value::List(List { tag: COMPOUND, items: vec![score(16, 3), score(4, 8), score(3, 9)] })),
                        ]),
                    ],
                }),
            ),
        ])
    }

    #[test]
    fn bdss_layout_reads_and_is_written_back_as_it_was() {
        let mut bytes = BytesMut::new();
        nbt::write::<LittleEndian>(&mut bytes, &Nbt { name: String::new(), value: bds_board() });
        let board = decode_scoreboard(&bytes).unwrap();
        assert_eq!(board.entries, [(8, ScoreIdentity::Entity(-4294967282)), (9, ScoreIdentity::Fake("Newbie".into())), (3, ScoreIdentity::Player(-4294967290))]);
        assert_eq!((board.last_unique_id, board.displays[0].clone()), (9, DisplayRecord { slot: "sidebar".into(), objective: "obj".into(), sort_order: 1 }));
        assert_eq!((board.objectives[0].scores.len(), &board.objectives[1].scores), (0, &vec![(3, 16), (8, 4), (9, 3)]));
        assert_eq!(encode_scoreboard(&board), bytes.to_vec());
        assert_eq!(decode_scoreboard(&encode_scoreboard(&ScoreboardRecord::default())).unwrap(), ScoreboardRecord::default());
    }
}
