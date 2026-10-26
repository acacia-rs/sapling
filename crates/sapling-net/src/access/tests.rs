use std::net::IpAddr;
use std::path::PathBuf;

use super::*;

const HOME: IpAddr = IpAddr::V4(std::net::Ipv4Addr::new(203, 0, 113, 7));
const ELSEWHERE: IpAddr = IpAddr::V4(std::net::Ipv4Addr::new(198, 51, 100, 1));

fn folder(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sapling-access-{}-{test}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn terms(created: u64, expires: Option<u64>) -> Terms {
    Terms { reason: "griefing".into(), source: "Alice".into(), created, expires }
}

#[test]
fn bds_files_are_read_and_every_list_is_saved_and_read_back() {
    let dir = folder("files");
    std::fs::create_dir_all(&dir).unwrap();
    // As bedrock_server_how_to.html shows them.
    std::fs::write(dir.join("allowlist.json"), r#"[{"ignoresPlayerLimit": false, "name": "MyPlayer"}, {"ignoresPlayerLimit": true, "name": "AnotherPlayer", "xuid": "274817248"}]"#).unwrap();
    std::fs::write(dir.join("permissions.json"), r#"[{ "permission": "operator", "xuid": "2535413803071674" }]"#).unwrap();
    let lists = AccessLists::open(&dir).unwrap();
    assert_eq!(lists.allowlist(), [AllowEntry::named("MyPlayer"), AllowEntry { name: "AnotherPlayer".into(), xuid: "274817248".into(), ignores_player_limit: true }]);
    assert_eq!((lists.role("2535413803071674"), lists.role("1"), lists.role("")), (Some(Role::Operator), None, None));

    assert!(lists.allow(AllowEntry::named("Bob")) && !lists.allow(AllowEntry::named("bob")));
    lists.set_role("42", Role::Member);
    lists.set_role("", Role::Operator);
    assert!(lists.ban(Ban { name: "Mallory".into(), xuid: "666".into(), terms: terms(10, None) }));
    assert!(lists.ban_ip(IpBan { ip: HOME, terms: terms(10, Some(u64::MAX)) }));
    let again = AccessLists::open(&dir).unwrap();
    assert_eq!(again.allowlist().len(), 3);
    assert_eq!((again.role("42"), again.role("2535413803071674")), (Some(Role::Member), Some(Role::Operator)));
    assert_eq!((again.bans(), again.ip_bans()), (lists.bans(), lists.ip_bans()));

    let written: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(dir.join("allowlist.json")).unwrap()).unwrap();
    assert_eq!(written[2], serde_json::json!({ "ignoresPlayerLimit": false, "name": "Bob" }), "no xuid until one is known, as BDS writes it");
    std::fs::write(dir.join("allowlist.json"), "[]").unwrap();
    lists.reload_allowlist().unwrap();
    assert!(lists.allowlist().is_empty());
    std::fs::write(dir.join("banned-ips.json"), "{}").unwrap();
    assert_eq!(AccessLists::open(&dir).unwrap_err().kind(), std::io::ErrorKind::InvalidData);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_allowlist_admits_by_name_until_it_learns_the_xuid() {
    let lists = AccessLists::in_memory();
    assert_eq!(lists.admit_at("Anyone", "", HOME, 0), (None, false), "off, it lets anyone in");
    lists.set_allowlist_enabled(true);
    lists.allow(AllowEntry { ignores_player_limit: true, ..AllowEntry::named("Steve") });
    assert_eq!(lists.admit_at("Anyone", "", HOME, 0), (Some(Refusal::NotAllowed), false));
    assert_eq!(lists.admit_at("STEVE", "", HOME, 0), (None, true), "offline, by name in any case");
    assert_eq!(lists.admit_at("Steve", "123", HOME, 0), (None, true));
    assert_eq!(lists.allowlist()[0].xuid, "123", "the first online join fills the xuid in");
    assert_eq!(lists.admit_at("Steve2", "123", HOME, 0).0, None, "a new gamertag, the same account");
    assert_eq!(lists.admit_at("Steve", "999", HOME, 0).0, Some(Refusal::NotAllowed), "someone else with the old name");
    assert!(lists.disallow("steve") && !lists.disallow("steve"));
    assert_eq!(lists.admit_at("Steve2", "123", HOME, 0).0, Some(Refusal::NotAllowed));
}

#[test]
fn bans_hold_until_they_expire_or_are_lifted() {
    let lists = AccessLists::in_memory();
    let changed = lists.bans_changed();
    let now = unix_now();
    assert!(lists.ban(Ban { name: "Mallory".into(), xuid: "666".into(), terms: terms(now, Some(now + 90 * 60)) }));
    assert!(!lists.ban(Ban { name: "mallory".into(), xuid: String::new(), terms: terms(now, None) }), "banned already");
    assert_eq!(lists.bans_changed(), changed + 1);
    let Some(Refusal::Banned(screen)) = lists.admit_at("Renamed", "666", ELSEWHERE, now).0 else { panic!("the account is banned under any name") };
    assert_eq!(screen, "You are banned from this server.\nReason: griefing\nYour ban ends in 90 minutes.");
    assert_eq!(lists.admit_at("Mallory", "666", ELSEWHERE, now + 90 * 60).0, None, "expired");
    assert!(lists.ban_screen("Mallory", "", HOME).is_some(), "offline, by name");

    assert!(lists.ban_ip(IpBan { ip: HOME, terms: terms(now, None) }) && !lists.ban_ip(IpBan { ip: HOME, terms: terms(now, None) }));
    assert_eq!(lists.admit_at("Bob", "", HOME, now).0, Some(Refusal::Banned("You are banned from this server.\nReason: griefing".into())));
    assert_eq!(lists.admit_at("Bob", "", ELSEWHERE, now).0, None);
    assert!(lists.pardon_ip(HOME) && !lists.pardon_ip(HOME) && lists.ip_bans().is_empty());
    assert!(lists.pardon("MALLORY") && !lists.pardon("Mallory") && lists.bans().is_empty());

    assert!(lists.ban(Ban { name: "Old".into(), xuid: String::new(), terms: terms(1, Some(2)) }));
    assert!(lists.bans().is_empty() && !lists.pardon("Old"), "an expired ban is no ban");
    assert!(lists.ban(Ban { name: "Old".into(), xuid: String::new(), terms: terms(now, None) }), "and does not stop a new one");
}

#[test]
fn players_in_the_game_are_found_by_name_and_address() {
    let lists = AccessLists::in_memory();
    lists.joined("Alice", HOME);
    lists.joined("Bob", HOME);
    assert_eq!(lists.address_of("alice"), Some(HOME));
    let mut there = lists.players_at(HOME);
    there.sort();
    assert_eq!(there, ["Alice", "Bob"]);
    lists.left("ALICE");
    assert_eq!((lists.address_of("Alice"), lists.players_at(HOME)), (None, vec!["Bob".to_owned()]));
}
