//! Commands on the wire: the list each client is shown, the lines clients send and the answers.
//! Shapes and type ids are BDS 1.26.52's (`sapling` example `bds_ui_probe`).

use acacia_proto::packets::{
    AvailableCommands, AvailableCommandsCommandDataItem as CommandData, AvailableCommandsCommandDataItemOverloadsItem as OverloadData,
    AvailableCommandsCommandDataItemOverloadsItemParametersItem as ParameterData, AvailableCommandsCommandDataItemOverloadsItemParametersItemEnumType as EnumType,
    AvailableCommandsEnumsItem, CommandOutput, CommandOutputOutputItem, CommandRequest,
};
use acacia_proto::types::{CommandFlags, CommandOrigin};
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_api::command::{CommandSpec, ParamKind};
use sapling_api::{CommandEvent, Commands, Output, Permission, Sender, ShardCtx};

use crate::components::{Outgoing, PacketQueues, PerTick, Stage, in_world, in_world_identity};
use crate::handlers::Ctx;
use crate::plugins::Plugins;

/// A parameter's `value_type` as BDS 1.26.52 numbers them (acacia docs/proto.md has the list and
/// why the schema's own numbers are stale).
const INT: u16 = 1;
const FLOAT: u16 = 3;
const TARGET: u16 = 8;
const STRING: u16 = 56;
const BLOCK_POSITION: u16 = 64;
const POSITION: u16 = 65;
/// The rest of the line.
const MESSAGE: u16 = 68;
const BLOCK_STATES: u16 = 84;
/// Every command is sent at this level: the list is filtered per player on the server instead.
const ANYONE: &str = "any";
/// The chat box takes 512 characters.
const MAX_CHARS: usize = 512;
const MAX_COMMANDS_PER_TICK: usize = 3;
const MAX_REQUEST_ID: usize = 64;

/// A player's command permission and the one its command list was last built for.
#[derive(Component, Default)]
pub(crate) struct Access {
    pub permission: Permission,
    told: Option<Permission>,
}

/// The encoded command list for each permission, rebuilt when a command is added.
#[derive(Resource)]
pub(crate) struct CommandLists {
    member: Bytes,
    operator: Bytes,
}

impl CommandLists {
    pub fn new(commands: &Commands) -> Self {
        Self { member: sapling_net::encode(&available(commands, Permission::Member)), operator: sapling_net::encode(&available(commands, Permission::Operator)) }
    }
}

/// The enum tables of an `AvailableCommands`; a word or an enum used twice is stored once.
#[derive(Default)]
struct Tables {
    values: Vec<String>,
    enums: Vec<AvailableCommandsEnumsItem>,
}

impl Tables {
    fn value(&mut self, word: &str) -> u32 {
        let index = self.values.iter().position(|v| v == word).unwrap_or_else(|| {
            self.values.push(word.to_owned());
            self.values.len() - 1
        });
        index as u32
    }

    fn enumeration(&mut self, name: &str, words: &[String]) -> usize {
        if let Some(index) = self.enums.iter().position(|e| e.name == name) {
            return index;
        }
        let values = words.iter().map(|word| self.value(word)).collect();
        self.enums.push(AvailableCommandsEnumsItem { name: name.to_owned(), values });
        self.enums.len() - 1
    }

    fn parameter(&mut self, name: &str, kind: &ParamKind, optional: bool) -> ParameterData {
        let (value_type, enum_type) = match kind {
            ParamKind::Int => (INT, EnumType::Valid),
            ParamKind::Float => (FLOAT, EnumType::Valid),
            ParamKind::String => (STRING, EnumType::Valid),
            ParamKind::Text => (MESSAGE, EnumType::Valid),
            ParamKind::Target => (TARGET, EnumType::Valid),
            ParamKind::Position => (POSITION, EnumType::Valid),
            ParamKind::BlockPosition => (BLOCK_POSITION, EnumType::Valid),
            ParamKind::BlockStates => (BLOCK_STATES, EnumType::Valid),
            // An enum parameter carries its enum's index where the others carry a type.
            ParamKind::Enum { name, values } => (self.enumeration(name, values) as u16, EnumType::Enum),
        };
        let options = CommandFlags { unused: 0, collapse_enum: 0, has_semantic_constraint: 0, as_chained_command: 0, unknown2: 0 };
        ParameterData { parameter_name: name.to_owned(), value_type, enum_type, optional, options }
    }

    fn command(&mut self, spec: &CommandSpec) -> CommandData {
        // BDS lists the name among its aliases: `tellCommandAliases` is tell, w, msg.
        let alias = if spec.aliases.is_empty() {
            -1
        } else {
            let names: Vec<String> = std::iter::once(&spec.name).chain(&spec.aliases).cloned().collect();
            self.enumeration(&format!("{}CommandAliases", spec.name), &names) as i32
        };
        let overload = |tables: &mut Self, overload: &sapling_api::command::Overload| OverloadData {
            chaining: false,
            parameters: overload.parameters.iter().map(|p| tables.parameter(&p.name, &p.kind, p.optional)).collect(),
        };
        CommandData {
            name: spec.name.clone(),
            description: spec.description.clone(),
            flags: 0,
            permission_level: ANYONE.into(),
            alias,
            chained_subcommand_offsets: Vec::new(),
            overloads: spec.overloads.iter().map(|o| overload(self, o)).collect(),
        }
    }
}

/// The command list of a player with `permission`: syntax and completions for what it may run.
pub(crate) fn available(commands: &Commands, permission: Permission) -> AvailableCommands {
    let mut tables = Tables::default();
    let command_data = commands.visible(permission).map(|spec| tables.command(spec)).collect();
    AvailableCommands {
        values_len: tables.values.len() as u32,
        enum_values: tables.values,
        chained_subcommand_values: Vec::new(),
        suffixes: Vec::new(),
        enums: tables.enums,
        chained_subcommands: Vec::new(),
        command_data,
        dynamic_enums: Vec::new(),
        enum_constraints: Vec::new(),
    }
}

/// Sends the command list to players who entered the world or whose permission changed.
pub(crate) fn announce(lists: Res<CommandLists>, mut players: Query<(&Stage, &mut Access, &mut Outgoing)>) {
    for (stage, mut access, mut out) in &mut players {
        if *stage == Stage::InWorld && access.told != Some(access.permission) {
            access.told = Some(access.permission);
            out.0.push(if access.permission == Permission::Operator { lists.operator.clone() } else { lists.member.clone() });
        }
    }
}

/// A command line without control characters, its `/` and excess length; `None` if nothing is left.
fn clean(command: &str) -> Option<String> {
    let line: String = command.chars().filter(|c| !c.is_control()).take(MAX_CHARS).collect();
    let line = line.trim();
    let line = line.strip_prefix('/').unwrap_or(line).trim_start();
    (!line.is_empty()).then(|| line.to_owned())
}

/// The answer as BDS addresses one: the request's origin echoed, entity id 0.
fn output_packet(origin: &CommandOrigin, out: &Output) -> CommandOutput {
    let request_id = if origin.request_id.len() <= MAX_REQUEST_ID { origin.request_id.clone() } else { String::new() };
    let line = |m: &sapling_api::command::OutputMessage| CommandOutputOutputItem { message_id: m.text.clone(), success: m.success, parameters: m.parameters.clone() };
    CommandOutput {
        origin: CommandOrigin { r#type: "player".into(), uuid: origin.uuid, request_id, player_entity_id: 0 },
        output_type: "alloutput".into(),
        success_count: u32::from(out.succeeded()),
        output: out.messages().iter().map(line).collect(),
        has_data: false,
        data: None,
    }
}

/// Runs this tick's command lines: handlers first (`on_command`), then the registry; the sender
/// gets what the command printed.
pub(crate) fn dispatch(world: &mut World, plugins: &mut Plugins) {
    let requests = world.resource_mut::<PacketQueues>().take::<CommandRequest>();
    let mut limit = PerTick::new(MAX_COMMANDS_PER_TICK);
    for (entity, request) in requests {
        let Some(player) = in_world_identity(world, entity) else { continue };
        let Some(line) = clean(&request.command).filter(|_| limit.allow(entity)) else { continue };
        crate::worlds::focus_on(world, entity);
        let event = plugins.offer(world, CommandEvent::new(line), |handler, ctx, event| handler.on_command(ctx, &player, event));
        if event.is_cancelled() {
            continue;
        }
        // An earlier handler may have kicked the sender or changed what it may run.
        let Some(permission) = world.get::<Access>(entity).map(|a| a.permission) else { continue };
        let name = player.name.clone();
        let mut backend = Ctx { world };
        let out = plugins.commands.run(&mut ShardCtx::new(&mut backend), &Sender::Player(player), permission, event.line());
        if let Some(mut outgoing) = world.get_mut::<Outgoing>(entity).filter(|_| !out.messages().is_empty()) {
            outgoing.push(&output_packet(&request.origin, &out));
        }
        echo(world, entity, &name, &out);
    }
}

/// Shows the other operators on the shard what an operator's command did, as BDS does: a grey
/// italic `[name: line]`.
fn echo(world: &mut World, sender: Entity, name: &str, out: &Output) {
    let lines: Vec<Bytes> = out
        .echoed()
        .map(|m| {
            let part = |text: &str| serde_json::json!({ "text": text });
            crate::chat::json_packet(vec![part("§7§o["), serde_json::json!({ "translate": name }), part(": "), crate::chat::translation(&m.text, &m.parameters), part("]")])
        })
        .collect();
    if lines.is_empty() {
        return;
    }
    for operator in in_world(world) {
        if operator == sender || world.get::<Access>(operator).is_none_or(|access| access.permission != Permission::Operator) {
            continue;
        }
        if let Some(mut outgoing) = world.get_mut::<Outgoing>(operator) {
            outgoing.0.extend(lines.iter().cloned());
        }
    }
}

#[cfg(test)]
mod tests;
