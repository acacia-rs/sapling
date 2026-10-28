//! BDS's `server.properties`: `key=value` lines, `#` comments. The keys sapling has a setting for
//! are read into [`ServerProperties`] and applied by [`ServerBuilder::properties`]; the rest are
//! listed in `ignored` and logged when applied.

use std::io;
use std::net::IpAddr;
use std::path::{Path, PathBuf};

use sapling_api::{Difficulty, GameMode};
use sapling_net::Compression;

#[cfg(doc)]
use super::ServerBuilder;

/// The settings a `server.properties` holds that sapling has a match for. An absent or empty key
/// is `None` and leaves the builder's setting alone.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ServerProperties {
    /// `server-name`: the server list's name.
    pub server_name: Option<String>,
    pub gamemode: Option<GameMode>,
    pub difficulty: Option<Difficulty>,
    pub max_players: Option<usize>,
    pub online_mode: Option<bool>,
    /// `allow-list` (or the old `white-list`): needs the access lists ([`ServerBuilder::access`]).
    pub allow_list: Option<bool>,
    /// `server-ip`: the address bound; BDS reads it only for NetherNet.
    pub server_ip: Option<IpAddr>,
    pub server_port: Option<u16>,
    /// `server-portv6`: the IPv6 socket's port ([`ServerBuilder::bind_v6`]).
    pub server_portv6: Option<u16>,
    /// `enable-query` (Java's key; BDS has none): answer the GameSpy4 query on the game ports.
    pub enable_query: Option<bool>,
    /// `view-distance`, in chunks: the most a client may ask for.
    pub view_distance: Option<i32>,
    /// `tick-distance`, in chunks: how far about a player the world is simulated.
    pub tick_distance: Option<i32>,
    /// `player-idle-timeout`, in minutes; 0 is never.
    pub player_idle_timeout: Option<u32>,
    /// `level-name`: the world's name in the server list and on the client. The folder of that
    /// name is the binary's to open (`WorldFolder`).
    pub level_name: Option<String>,
    /// `level-seed`, when a number: the binary's to generate with (`Generated`). BDS also takes
    /// text, which it hashes; sapling does not.
    pub level_seed: Option<i64>,
    pub compression_threshold: Option<u16>,
    pub compression_algorithm: Option<Compression>,
    /// `player-position-acceptance-threshold` ([`sapling_shard::MovementCheck::threshold`]).
    pub position_threshold: Option<f32>,
    /// `player-rewind-min-correction-delay-ticks` ([`sapling_shard::MovementCheck::correction_delay`]).
    pub correction_delay: Option<u64>,
    /// Keys sapling has no setting for, in the file's order.
    pub ignored: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("reading {}: {source}", .path.display())]
    Read { path: PathBuf, source: io::Error },
    #[error("line {line}: {key}={value:?} is not {expected}")]
    Value { line: usize, key: String, value: String, expected: &'static str },
}

fn game_mode(value: &str) -> Option<GameMode> {
    match value.to_ascii_lowercase().as_str() {
        "survival" | "0" => Some(GameMode::Survival),
        "creative" | "1" => Some(GameMode::Creative),
        "adventure" | "2" => Some(GameMode::Adventure),
        _ => None,
    }
}

fn difficulty(value: &str) -> Option<Difficulty> {
    match value.to_ascii_lowercase().as_str() {
        "peaceful" | "0" => Some(Difficulty::Peaceful),
        "easy" | "1" => Some(Difficulty::Easy),
        "normal" | "2" => Some(Difficulty::Normal),
        "hard" | "3" => Some(Difficulty::Hard),
        _ => None,
    }
}

fn boolean(value: &str) -> Option<bool> {
    match value.to_ascii_lowercase().as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn compression(value: &str) -> Option<Compression> {
    match value.to_ascii_lowercase().as_str() {
        "zlib" => Some(Compression::Zlib),
        "snappy" => Some(Compression::Snappy),
        _ => None,
    }
}

impl ServerProperties {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path).map_err(|source| ConfigError::Read { path: path.to_owned(), source })?;
        Self::parse(&text)
    }

    pub fn parse(text: &str) -> Result<Self, ConfigError> {
        let mut props = Self::default();
        for (index, line) in text.lines().enumerate() {
            let line = line.trim();
            let Some((key, value)) = line.split_once('=').filter(|_| !line.starts_with('#')) else { continue };
            let (key, value) = (key.trim(), value.trim());
            if !value.is_empty() {
                props.set(key, value).map_err(|expected| ConfigError::Value { line: index + 1, key: key.to_owned(), value: value.to_owned(), expected })?;
            }
        }
        Ok(props)
    }

    /// Reads one key; the error names what the value should have been.
    fn set(&mut self, key: &str, value: &str) -> Result<(), &'static str> {
        fn read<T>(parsed: Option<T>, expected: &'static str) -> Result<Option<T>, &'static str> {
            parsed.map(Some).ok_or(expected)
        }
        match key {
            "server-name" => self.server_name = Some(value.to_owned()),
            "gamemode" => self.gamemode = read(game_mode(value), "survival, creative or adventure")?,
            "difficulty" => self.difficulty = read(difficulty(value), "peaceful, easy, normal or hard")?,
            "max-players" => self.max_players = read(value.parse().ok().filter(|max| *max > 0), "a positive whole number")?,
            "online-mode" => self.online_mode = read(boolean(value), "true or false")?,
            "allow-list" | "white-list" => self.allow_list = read(boolean(value), "true or false")?,
            "server-ip" => self.server_ip = read(value.parse().ok(), "an IP address")?,
            "server-port" => self.server_port = read(value.parse().ok().filter(|port| *port > 0), "a port from 1 to 65535")?,
            "server-portv6" => self.server_portv6 = read(value.parse().ok().filter(|port| *port > 0), "a port from 1 to 65535")?,
            "enable-query" => self.enable_query = read(boolean(value), "true or false")?,
            "view-distance" => self.view_distance = read(value.parse().ok().filter(|chunks| *chunks >= 5), "a whole number of 5 or more")?,
            "tick-distance" => self.tick_distance = read(value.parse().ok().filter(|chunks| (4..=12).contains(chunks)), "a whole number from 4 to 12")?,
            "player-idle-timeout" => self.player_idle_timeout = read(value.parse().ok(), "a whole number of minutes")?,
            "level-name" => self.level_name = Some(value.to_owned()),
            "level-seed" => match value.parse() {
                Ok(seed) => self.level_seed = Some(seed),
                Err(_) => self.ignored.push(format!("{key} (text seeds are not hashed)")),
            },
            "compression-threshold" => self.compression_threshold = read(value.parse().ok(), "a whole number from 0 to 65535")?,
            "compression-algorithm" => self.compression_algorithm = read(compression(value), "zlib or snappy")?,
            "player-position-acceptance-threshold" => self.position_threshold = read(value.parse().ok().filter(|t: &f32| *t >= 0.0), "a number of 0 or more")?,
            "player-rewind-min-correction-delay-ticks" => self.correction_delay = read(value.parse().ok(), "a whole number of ticks")?,
            _ => self.ignored.push(key.to_owned()),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BDS: &str = "server-name=Dedicated Server\n# Used as the server name\ngamemode=creative\nforce-gamemode=false\ndifficulty=easy\n\
        max-players=10\nonline-mode=false\nallow-list=true\nserver-port=19182\nserver-portv6=19183\nview-distance=32\ntick-distance=4\n\
        player-idle-timeout=30\nlevel-name=blocks\nlevel-seed=\ncompression-threshold=256\ncompression-algorithm=snappy\n\
        player-position-acceptance-threshold=0.5\n  # comment = with an equals sign\nnot a setting\n";

    #[test]
    fn a_bds_file_is_read_and_the_keys_without_a_setting_are_listed() {
        let props = ServerProperties::parse(BDS).unwrap();
        assert_eq!((props.server_name.as_deref(), props.gamemode, props.difficulty), (Some("Dedicated Server"), Some(GameMode::Creative), Some(Difficulty::Easy)));
        assert_eq!((props.max_players, props.online_mode, props.allow_list, props.server_port), (Some(10), Some(false), Some(true), Some(19182)));
        assert_eq!((props.view_distance, props.player_idle_timeout, props.level_name.as_deref(), props.level_seed), (Some(32), Some(30), Some("blocks"), None));
        assert_eq!((props.compression_threshold, props.compression_algorithm, props.position_threshold), (Some(256), Some(Compression::Snappy), Some(0.5)));
        // server-portv6 is read now (research/captures/bds-query.txt).
        assert_eq!((props.ignored, props.tick_distance, props.server_portv6), (vec!["force-gamemode".to_owned()], Some(4), Some(19183)));
    }

    #[test]
    fn bad_values_name_their_line_and_numbers_stand_for_modes() {
        let error = ServerProperties::parse("gamemode=1\ndifficulty=3\nview-distance=4\n").unwrap_err();
        assert!(matches!(&error, ConfigError::Value { line: 3, key, .. } if key == "view-distance"), "{error}");
        let props = ServerProperties::parse("gamemode=1\ndifficulty=3\nlevel-seed=-42\nlevel-seed=abc\nonline-mode=TRUE").unwrap();
        assert_eq!((props.gamemode, props.difficulty, props.level_seed, props.online_mode), (Some(GameMode::Creative), Some(Difficulty::Hard), Some(-42), Some(true)));
        assert_eq!(props.ignored, ["level-seed (text seeds are not hashed)"]);
        assert!(ServerProperties::parse("compression-algorithm=zstd").is_err() && ServerProperties::parse("server-port=0").is_err());
        assert!(matches!(ServerProperties::load("no/such/server.properties"), Err(ConfigError::Read { .. })));
    }
}
