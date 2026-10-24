//! Who may join and who is an operator: the allowlist and `permissions.json` as BDS keeps them, and
//! bans by name or XUID and by IP. BDS has no bans; these are Java's and PocketMine's, kept in
//! `banned-players.json` and `banned-ips.json` in sapling's own format (times in Unix seconds).
//! The listener checks each login against them before the player reaches the game
//! ([`crate::NetConfig::access`]) and drops players a new ban names; the game edits them.

mod entries;
mod store;
#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::fmt;
use std::io;
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

pub use entries::{AllowEntry, Ban, Grant, IpBan, Role, Terms};

use acacia_proto::types::DisconnectFailReason;

use entries::{Entry, same_player};
use store::{ALLOWLIST, BANNED_IPS, BANNED_PLAYERS, PERMISSIONS};

/// The lists, shared by the listener and every shard: clones are one set of lists. Each change is
/// saved at once when the lists were opened from a folder.
#[derive(Clone)]
pub struct AccessLists {
    shared: Arc<Shared>,
}

struct Shared {
    dir: Option<PathBuf>,
    lists: Mutex<Lists>,
    /// Counts ban changes, so the listener knows when to look for banned players in the game.
    bans_changed: AtomicU64,
}

#[derive(Default)]
struct Lists {
    allowlist_on: bool,
    allowlist: Vec<AllowEntry>,
    grants: Vec<Grant>,
    bans: Vec<Ban>,
    ip_bans: Vec<IpBan>,
    /// Players in the game by lower-case name: their name and the address they came from.
    online: HashMap<String, (String, IpAddr)>,
}

/// Why a login is turned away: the disconnect screen of a ban, or not being on the allowlist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Refusal {
    Banned(String),
    NotAllowed,
}

impl Refusal {
    /// What the turned-away client is told.
    pub(crate) fn screen(self) -> (DisconnectFailReason, String) {
        match self {
            Self::Banned(screen) => (DisconnectFailReason::Kicked, screen),
            Self::NotAllowed => (DisconnectFailReason::NotAllowed, "disconnectionScreen.notAllowed".to_owned()),
        }
    }
}

impl fmt::Debug for AccessLists {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AccessLists").field("dir", &self.shared.dir).finish_non_exhaustive()
    }
}

impl AccessLists {
    /// Empty lists kept only while the server runs.
    pub fn in_memory() -> Self {
        Self::with(None, Lists::default())
    }

    /// The lists saved in `dir` (missing files are empty lists), saved back there on each change.
    pub fn open(dir: impl Into<PathBuf>) -> io::Result<Self> {
        let dir = dir.into();
        let lists = Lists {
            allowlist: store::read(&dir, ALLOWLIST)?,
            grants: store::read(&dir, PERMISSIONS)?,
            bans: store::read(&dir, BANNED_PLAYERS)?,
            ip_bans: store::read(&dir, BANNED_IPS)?,
            ..Lists::default()
        };
        Ok(Self::with(Some(dir), lists))
    }

    fn with(dir: Option<PathBuf>, lists: Lists) -> Self {
        Self { shared: Arc::new(Shared { dir, lists: Mutex::new(lists), bans_changed: AtomicU64::new(0) }) }
    }

    fn lists(&self) -> MutexGuard<'_, Lists> {
        self.shared.lists.lock().expect("access lists lock")
    }

    fn save<E: Entry>(&self, file: &str, entries: &[E]) {
        if let Some(dir) = &self.shared.dir
            && let Err(error) = store::write(dir, file, entries)
        {
            tracing::warn!(%error, file, "access list not saved");
        }
    }

    pub fn allowlist_enabled(&self) -> bool {
        self.lists().allowlist_on
    }

    /// Turns the allowlist check on or off. Not saved: BDS's `allowlist on` leaves
    /// server.properties as it was.
    pub fn set_allowlist_enabled(&self, on: bool) {
        self.lists().allowlist_on = on;
    }

    pub fn allowlist(&self) -> Vec<AllowEntry> {
        self.lists().allowlist.clone()
    }

    /// Puts a player on the allowlist; false if they are on it already.
    pub fn allow(&self, entry: AllowEntry) -> bool {
        let mut lists = self.lists();
        if lists.allowlist.iter().any(|on| same_player(&on.name, &on.xuid, &entry.name, &entry.xuid)) {
            return false;
        }
        lists.allowlist.push(entry);
        self.save(ALLOWLIST, &lists.allowlist);
        true
    }

    /// Takes the player of that name off the allowlist; false if they were not on it.
    pub fn disallow(&self, name: &str) -> bool {
        let mut lists = self.lists();
        let before = lists.allowlist.len();
        lists.allowlist.retain(|on| !on.name.eq_ignore_ascii_case(name));
        let removed = lists.allowlist.len() != before;
        if removed {
            self.save(ALLOWLIST, &lists.allowlist);
        }
        removed
    }

    /// Reads `allowlist.json` again, after someone edited it.
    pub fn reload_allowlist(&self) -> io::Result<()> {
        if let Some(dir) = &self.shared.dir {
            self.lists().allowlist = store::read(dir, ALLOWLIST)?;
        }
        Ok(())
    }

    /// The role `permissions.json` gives this XUID.
    pub fn role(&self, xuid: &str) -> Option<Role> {
        self.lists().grants.iter().find(|grant| !xuid.is_empty() && grant.xuid == xuid).map(|grant| grant.role)
    }

    /// Keeps a role for this XUID. Nothing is kept for a player without one (offline): BDS keeps
    /// `/op` only for players Xbox Live vouched for.
    pub fn set_role(&self, xuid: &str, role: Role) {
        if xuid.is_empty() {
            return;
        }
        let mut lists = self.lists();
        lists.grants.retain(|grant| grant.xuid != xuid);
        lists.grants.push(Grant { xuid: xuid.to_owned(), role });
        self.save(PERMISSIONS, &lists.grants);
    }

    /// Bans a player; false if a ban of theirs is in force already. Players it names are dropped.
    pub fn ban(&self, ban: Ban) -> bool {
        let mut lists = self.lists();
        let now = unix_now();
        lists.bans.retain(|old| old.terms.in_force(now));
        if lists.bans.iter().any(|old| same_player(&old.name, &old.xuid, &ban.name, &ban.xuid)) {
            return false;
        }
        lists.bans.push(ban);
        self.save(BANNED_PLAYERS, &lists.bans);
        self.shared.bans_changed.fetch_add(1, Ordering::Relaxed);
        true
    }

    /// Lifts the bans of that name; false if there were none.
    pub fn pardon(&self, name: &str) -> bool {
        let mut lists = self.lists();
        let now = unix_now();
        let before = lists.bans.iter().filter(|ban| ban.terms.in_force(now)).count();
        lists.bans.retain(|ban| ban.terms.in_force(now) && !ban.name.eq_ignore_ascii_case(name));
        self.save(BANNED_PLAYERS, &lists.bans);
        lists.bans.len() != before
    }

    /// The bans in force.
    pub fn bans(&self) -> Vec<Ban> {
        let now = unix_now();
        self.lists().bans.iter().filter(|ban| ban.terms.in_force(now)).cloned().collect()
    }

    /// Bans an address; false if it is banned already. Players from it are dropped.
    pub fn ban_ip(&self, ban: IpBan) -> bool {
        let mut lists = self.lists();
        let now = unix_now();
        lists.ip_bans.retain(|old| old.terms.in_force(now));
        if lists.ip_bans.iter().any(|old| old.ip == ban.ip) {
            return false;
        }
        lists.ip_bans.push(ban);
        self.save(BANNED_IPS, &lists.ip_bans);
        self.shared.bans_changed.fetch_add(1, Ordering::Relaxed);
        true
    }

    pub fn pardon_ip(&self, ip: IpAddr) -> bool {
        let mut lists = self.lists();
        let now = unix_now();
        let before = lists.ip_bans.iter().filter(|ban| ban.terms.in_force(now)).count();
        lists.ip_bans.retain(|ban| ban.terms.in_force(now) && ban.ip != ip);
        self.save(BANNED_IPS, &lists.ip_bans);
        lists.ip_bans.len() != before
    }

    pub fn ip_bans(&self) -> Vec<IpBan> {
        let now = unix_now();
        self.lists().ip_bans.iter().filter(|ban| ban.terms.in_force(now)).cloned().collect()
    }

    /// Where the player of that name in the game connected from.
    pub fn address_of(&self, name: &str) -> Option<IpAddr> {
        self.lists().online.get(&name.to_ascii_lowercase()).map(|(_, ip)| *ip)
    }

    /// The players in the game who connected from `ip`.
    pub fn players_at(&self, ip: IpAddr) -> Vec<String> {
        self.lists().online.values().filter(|(_, at)| *at == ip).map(|(name, _)| name.clone()).collect()
    }

    pub(crate) fn bans_changed(&self) -> u64 {
        self.shared.bans_changed.load(Ordering::Relaxed)
    }

    /// Whether a login may join, and whether it may join a full server (`ignoresPlayerLimit`). An
    /// allowlist entry without an XUID takes the player's, as BDS resolves it on first join.
    pub(crate) fn admit(&self, name: &str, xuid: &str, ip: IpAddr) -> (Option<Refusal>, bool) {
        self.admit_at(name, xuid, ip, unix_now())
    }

    fn admit_at(&self, name: &str, xuid: &str, ip: IpAddr, now: u64) -> (Option<Refusal>, bool) {
        let mut lists = self.lists();
        if let Some(screen) = lists.ban_screen(name, xuid, ip, now) {
            return (Some(Refusal::Banned(screen)), false);
        }
        let on = lists.allowlist_on;
        let Some(entry) = lists.allowlist.iter_mut().find(|entry| same_player(&entry.name, &entry.xuid, name, xuid)) else {
            return (on.then_some(Refusal::NotAllowed), false);
        };
        let ignores_limit = entry.ignores_player_limit;
        if entry.xuid.is_empty() && !xuid.is_empty() {
            entry.xuid = xuid.to_owned();
            self.save(ALLOWLIST, &lists.allowlist);
        }
        (None, ignores_limit)
    }

    /// The disconnect screen of a ban in force on this player or address.
    pub(crate) fn ban_screen(&self, name: &str, xuid: &str, ip: IpAddr) -> Option<String> {
        self.lists().ban_screen(name, xuid, ip, unix_now())
    }

    pub(crate) fn joined(&self, name: &str, ip: IpAddr) {
        self.lists().online.insert(name.to_ascii_lowercase(), (name.to_owned(), ip));
    }

    pub(crate) fn left(&self, name: &str) {
        self.lists().online.remove(&name.to_ascii_lowercase());
    }
}

impl Lists {
    fn ban_screen(&self, name: &str, xuid: &str, ip: IpAddr, now: u64) -> Option<String> {
        let by_name = self.bans.iter().find(|ban| ban.terms.in_force(now) && same_player(&ban.name, &ban.xuid, name, xuid)).map(|ban| &ban.terms);
        let terms = by_name.or_else(|| self.ip_bans.iter().find(|ban| ban.terms.in_force(now) && ban.ip == ip).map(|ban| &ban.terms))?;
        Some(terms.screen(now))
    }
}

pub(crate) fn unix_now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_secs())
}
