//! The entries of the access lists and their JSON: `allowlist.json` and `permissions.json` as BDS
//! writes them (its bedrock_server_how_to.html); the ban files are sapling's own (see `mod.rs`).

use std::net::IpAddr;

use serde_json::{Map, Value, json};

/// One player `allowlist.json` lets in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllowEntry {
    pub name: String,
    /// Empty until the player first joins online: BDS fills it in then, and matches by it after.
    pub xuid: String,
    /// Joins a full server.
    pub ignores_player_limit: bool,
}

impl AllowEntry {
    pub fn named(name: impl Into<String>) -> Self {
        Self { name: name.into(), xuid: String::new(), ignores_player_limit: false }
    }
}

/// A role in `permissions.json`. sapling has no visitors: a visitor plays as a member.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Visitor,
    Member,
    Operator,
}

impl Role {
    fn word(self) -> &'static str {
        match self {
            Self::Visitor => "visitor",
            Self::Member => "member",
            Self::Operator => "operator",
        }
    }
}

/// One XUID's role in `permissions.json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grant {
    pub xuid: String,
    pub role: Role,
}

/// What every ban carries. Times are Unix seconds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Terms {
    pub reason: String,
    /// Who banned: an operator's name, `Server` or a plugin's.
    pub source: String,
    pub created: u64,
    /// `None`: until pardoned.
    pub expires: Option<u64>,
}

impl Terms {
    /// Starting now, for `lasts` (`None`: for good).
    pub fn new(reason: impl Into<String>, source: impl Into<String>, lasts: Option<std::time::Duration>) -> Self {
        let created = super::unix_now();
        Self { reason: reason.into(), source: source.into(), created, expires: lasts.map(|lasts| created + lasts.as_secs()) }
    }

    pub fn in_force(&self, now: u64) -> bool {
        self.expires.is_none_or(|expires| now < expires)
    }

    /// The banned player's disconnect screen, in Java's wording: BDS has no bans to copy.
    pub(crate) fn screen(&self, now: u64) -> String {
        let mut screen = format!("You are banned from this server.\nReason: {}", self.reason);
        if let Some(expires) = self.expires {
            let minutes = expires.saturating_sub(now).div_ceil(60).max(1);
            let left = match minutes {
                ..120 => format!("{minutes} minutes"),
                ..2880 => format!("{} hours", minutes.div_ceil(60)),
                _ => format!("{} days", minutes.div_ceil(1440)),
            };
            screen += &format!("\nYour ban ends in {left}.");
        }
        screen
    }
}

/// A player banned by name and, once known, XUID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ban {
    pub name: String,
    /// Empty when the player was banned offline or by a name never seen.
    pub xuid: String,
    pub terms: Terms,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpBan {
    pub ip: IpAddr,
    pub terms: Terms,
}

/// Whether an entry for `entry_name`/`entry_xuid` is this player: by XUID where both have one (a
/// gamertag can change), else by name in any case.
pub(crate) fn same_player(entry_name: &str, entry_xuid: &str, name: &str, xuid: &str) -> bool {
    if !entry_xuid.is_empty() && !xuid.is_empty() { entry_xuid == xuid } else { entry_name.eq_ignore_ascii_case(name) }
}

/// An entry of one of the JSON arrays the lists are kept in.
pub(crate) trait Entry: Sized {
    fn to_json(&self) -> Value;
    fn from_json(value: &Value) -> Option<Self>;
}

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key)?.as_str()
}

impl Entry for AllowEntry {
    fn to_json(&self) -> Value {
        let mut entry = json!({ "ignoresPlayerLimit": self.ignores_player_limit, "name": self.name });
        if !self.xuid.is_empty() {
            entry["xuid"] = self.xuid.clone().into();
        }
        entry
    }

    fn from_json(value: &Value) -> Option<Self> {
        let ignores_player_limit = value.get("ignoresPlayerLimit").and_then(Value::as_bool).unwrap_or(false);
        Some(Self { name: text(value, "name")?.to_owned(), xuid: text(value, "xuid").unwrap_or_default().to_owned(), ignores_player_limit })
    }
}

impl Entry for Grant {
    fn to_json(&self) -> Value {
        json!({ "permission": self.role.word(), "xuid": self.xuid })
    }

    fn from_json(value: &Value) -> Option<Self> {
        let role = [Role::Visitor, Role::Member, Role::Operator].into_iter().find(|role| text(value, "permission") == Some(role.word()))?;
        Some(Self { xuid: text(value, "xuid")?.to_owned(), role })
    }
}

impl Terms {
    fn write(&self, entry: &mut Map<String, Value>) {
        entry.insert("reason".into(), self.reason.clone().into());
        entry.insert("source".into(), self.source.clone().into());
        entry.insert("created".into(), self.created.into());
        entry.insert("expires".into(), self.expires.map_or(Value::Null, Value::from));
    }

    fn read(value: &Value) -> Option<Self> {
        let expires = value.get("expires").and_then(Value::as_u64);
        Some(Self { reason: text(value, "reason")?.to_owned(), source: text(value, "source")?.to_owned(), created: value.get("created")?.as_u64()?, expires })
    }
}

impl Entry for Ban {
    fn to_json(&self) -> Value {
        let mut entry = Map::new();
        entry.insert("name".into(), self.name.clone().into());
        if !self.xuid.is_empty() {
            entry.insert("xuid".into(), self.xuid.clone().into());
        }
        self.terms.write(&mut entry);
        Value::Object(entry)
    }

    fn from_json(value: &Value) -> Option<Self> {
        Some(Self { name: text(value, "name")?.to_owned(), xuid: text(value, "xuid").unwrap_or_default().to_owned(), terms: Terms::read(value)? })
    }
}

impl Entry for IpBan {
    fn to_json(&self) -> Value {
        let mut entry = Map::new();
        entry.insert("ip".into(), self.ip.to_string().into());
        self.terms.write(&mut entry);
        Value::Object(entry)
    }

    fn from_json(value: &Value) -> Option<Self> {
        Some(Self { ip: text(value, "ip")?.parse().ok()?, terms: Terms::read(value)? })
    }
}
