use acacia_proto::{Packet, RawPacket};
use acacia_testserver::{Script, Step};
use sapling_api::command::{Target, Text};
use sapling_api::{Command, GameMode};

use super::*;

#[derive(Command)]
#[command(crate = "sapling_api", aliases = ["lobby", "spawn"], description = "Back to the spawn")]
struct Hub;

#[derive(Command)]
#[command(crate = "sapling_api", name = "gamemode", description = "Sets a game mode", permission = "operator")]
struct SetMode {
    #[command(name = "gameMode")]
    mode: GameMode,
    player: Option<Target>,
}

#[derive(Command)]
#[command(crate = "sapling_api")]
enum Warp {
    List,
    Set { name: String, x: i32, y: f32, public: bool },
    Say { message: Text },
}

macro_rules! silent {
    ($($command:ty),*) => {$(
        impl Command for $command {
            fn run(self, _: &mut ShardCtx<'_>, _: &Sender, _: &mut Output) {}
        }
    )*};
}
silent!(Hub, SetMode, Warp);

fn commands() -> Commands {
    let mut commands = Commands::default();
    commands.register::<Hub>();
    commands.register::<SetMode>();
    commands.register::<Warp>();
    commands
}

fn command<'a>(list: &'a AvailableCommands, name: &str) -> &'a CommandData {
    list.command_data.iter().find(|c| c.name == name).unwrap_or_else(|| panic!("no /{name}"))
}

/// (name, type or enum index, flag bits, optional) of each parameter of an overload.
fn shape(command: &CommandData, overload: usize) -> Vec<(&str, i64, i64, bool)> {
    command.overloads[overload].parameters.iter().map(|p| (p.parameter_name.as_str(), i64::from(p.value_type), p.enum_type.to_raw(), p.optional)).collect()
}

fn words<'a>(list: &'a AvailableCommands, index: i64) -> (&'a str, Vec<&'a str>) {
    let e = &list.enums[index as usize];
    (e.name.as_str(), e.values.iter().map(|v| list.enum_values[*v as usize].as_str()).collect())
}

/// The command list BDS 1.26.52 sent in the recorded join.
fn bds() -> AvailableCommands {
    let recorded = Script::bds_spawn().steps.into_iter().find_map(|step| match step {
        Step::Send { packet, .. } => RawPacket::parse(packet).ok().filter(|p| p.id == AvailableCommands::ID),
        Step::Wait(_) => None,
    });
    recorded.expect("the recording has an AvailableCommands").decode().unwrap()
}

const VALID: i64 = 16;
const ENUM: i64 = 48;

#[test]
fn the_list_decodes_and_is_shaped_like_bds_s() {
    let list = available(&commands(), Permission::Operator);
    let wire = RawPacket::parse(sapling_net::encode(&list)).unwrap();
    assert_eq!(wire.decode::<AvailableCommands>().unwrap(), list);
    assert_eq!(list.values_len as usize, list.enum_values.len());
    let mut unique = list.enum_values.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), list.enum_values.len(), "{:?}", list.enum_values);
    assert!(list.suffixes.is_empty() && list.chained_subcommands.is_empty() && list.dynamic_enums.is_empty() && list.enum_constraints.is_empty());
    assert!(list.command_data.iter().all(|c| c.flags == 0 && c.permission_level == "any" && c.chained_subcommand_offsets.is_empty()));
    assert!(list.command_data.iter().flat_map(|c| &c.overloads).all(|o| !o.chaining));

    let hub = command(&list, "hub");
    assert_eq!((hub.description.as_str(), hub.overloads.len(), hub.overloads[0].parameters.len()), ("Back to the spawn", 1, 0));
    assert_eq!(words(&list, hub.alias.into()), ("hubCommandAliases", vec!["hub", "lobby", "spawn"]));

    let mode = command(&list, "gamemode");
    let [(name, game_modes, flags, false), player] = shape(mode, 0)[..] else { panic!("{:?}", shape(mode, 0)) };
    assert_eq!((name, flags, mode.alias, player), ("gameMode", ENUM, -1, ("player", 8, VALID, true)));
    assert_eq!(words(&list, game_modes), ("GameMode", vec!["survival", "creative", "adventure", "spectator"]));

    let warp = command(&list, "warp");
    let literal = |overload: usize| words(&list, shape(warp, overload)[0].1);
    assert_eq!((literal(0), literal(1), literal(2)), (("warp_list", vec!["list"]), ("warp_set", vec!["set"]), ("warp_say", vec!["say"])));
    let set = shape(warp, 1);
    assert_eq!(set[1..4], [("name", 56, VALID, false), ("x", 1, VALID, false), ("y", 3, VALID, false)]);
    assert_eq!((set[4].0, set[4].2, words(&list, set[4].1)), ("public", ENUM, ("Boolean", vec!["true", "false"])));
    assert_eq!(shape(warp, 2)[1], ("message", 68, VALID, false));
}

#[test]
fn parameter_types_are_the_ones_bds_uses() {
    let (ours, bds) = (available(&commands(), Permission::Operator), bds());
    let ours_of = |name: &str, overload: usize, parameter: usize| shape(command(&ours, name), overload)[parameter];
    let bds_of = |name: &str, parameter: &str| {
        let found = command(&bds, name).overloads.iter().flat_map(|o| &o.parameters).find(|p| p.parameter_name == parameter);
        found.map(|p| (i64::from(p.value_type), p.enum_type.to_raw())).unwrap_or_else(|| panic!("BDS has no /{name} {parameter}"))
    };
    let kind = |(_, value, flags, _): (&str, i64, i64, bool)| (value, flags);
    assert_eq!(kind(ours_of("gamemode", 0, 1)), bds_of("gamemode", "player"));
    assert_eq!(kind(ours_of("warp", 1, 1)), bds_of("scoreboard", "displayName"));
    assert_eq!(kind(ours_of("warp", 1, 2)), bds_of("xp", "amount"));
    assert_eq!(kind(ours_of("warp", 1, 3)), bds_of("aimassist", "x angle"));
    assert_eq!(kind(ours_of("warp", 2, 1)), bds_of("say", "message"));
    // BDS's own game mode parameter is an enum too, with these words among its values.
    let (bds_mode, flags) = bds_of("gamemode", "gameMode");
    assert_eq!((flags, ours_of("gamemode", 0, 0).2), (ENUM, ENUM));
    assert!(["survival", "creative", "adventure", "spectator"].iter().all(|word| words(&bds, bds_mode).1.contains(word)), "{:?}", words(&bds, bds_mode));
    let boolean = bds.enums.iter().position(|e| e.name == "Boolean").expect("BDS has a Boolean enum");
    assert_eq!(words(&bds, boolean as i64).1, words(&ours, ours_of("warp", 1, 4).1).1);
    // BDS sends sub-command words as one-word enums and aliases as an enum led by the name.
    let tell = command(&bds, "tell");
    assert_eq!(words(&bds, tell.alias.into()), ("tellCommandAliases", vec!["tell", "w", "msg"]));
    assert!(bds.command_data.iter().any(|c| c.permission_level == "any") && command(&bds, "gamemode").flags == 0);
}

#[test]
fn a_member_s_list_leaves_out_operator_commands() {
    let names = |permission| available(&commands(), permission).command_data.into_iter().map(|c| c.name).collect::<Vec<_>>();
    assert_eq!(names(Permission::Member), ["hub", "warp"]);
    assert_eq!(names(Permission::Operator), ["hub", "gamemode", "warp"]);
    // Nothing of a hidden command reaches the client, its enums included.
    let member = available(&commands(), Permission::Member);
    assert!(!member.enums.iter().any(|e| e.name == "GameMode") && !member.enum_values.iter().any(|v| v == "creative"));
    let lists = CommandLists::new(&commands());
    assert_eq!((lists.member.clone(), lists.operator.clone()), (sapling_net::encode(&member), sapling_net::encode(&available(&commands(), Permission::Operator))));
}

#[test]
fn command_lines_lose_their_slash_and_control_characters() {
    assert_eq!(clean("/hub").as_deref(), Some("hub"));
    assert_eq!(clean("  / warp set\thome\n").as_deref(), Some("warp sethome"));
    assert_eq!((clean("/"), clean(" \n"), clean("")), (None, None, None));
    assert_eq!(clean(&format!("/say {}", "é".repeat(600))).unwrap().chars().count(), 511);
}
