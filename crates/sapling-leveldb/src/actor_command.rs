//! What a stored command block minecart keeps of its command, as BDS's
//! (research/captures/bds-e3cart-actor.txt): a command block's fields beside the cart's.

use acacia_proto::nbt::{self, List, LittleEndian, Value};

use crate::Error;

const STRING: u8 = 8;
/// BDS's `Version` of a cart's command fields.
const VERSION: i32 = 52;

/// A command block minecart's command and its last run.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CartCommand {
    pub command: String,
    /// The tick of the last run; 0 before any.
    pub last_execution: i64,
    pub success_count: i32,
    /// The last run's last line and its parameters, kept with output tracked.
    pub last_output: String,
    pub parameters: Vec<String>,
}

/// The command a stored actor keeps; `None` for one without `Command`.
pub fn actor_command(compound: &[u8]) -> Option<CartCommand> {
    let Value::Compound(fields) = nbt::read::<LittleEndian>(&mut &compound[..]).ok()?.value else { return None };
    let get = |key: &str| fields.iter().find(|(name, _)| name == key).map(|(_, value)| value);
    let text = |key: &str| match get(key) {
        Some(Value::String(text)) => text.to_string(),
        _ => String::new(),
    };
    let Some(Value::String(command)) = get("Command") else { return None };
    let parameters = match get("LastOutputParams") {
        Some(Value::List(list)) => list.items.iter().filter_map(|item| if let Value::String(text) = item { Some(text.to_string()) } else { None }).collect(),
        _ => Vec::new(),
    };
    Some(CartCommand {
        command: command.to_string(),
        last_execution: if let Some(Value::Long(tick)) = get("LastExecution") { *tick } else { 0 },
        success_count: if let Some(Value::Int(count)) = get("SuccessCount") { *count } else { 0 },
        last_output: text("LastOutput"),
        parameters,
    })
}

/// A stored actor keeping `command` as BDS's cart does, output tracked.
pub fn commanded_actor(compound: &[u8], command: &CartCommand) -> Result<Vec<u8>, Error> {
    let text = |text: &str| Value::String(text.into());
    let parameters = List { tag: if command.parameters.is_empty() { 0 } else { STRING }, items: command.parameters.iter().map(|text| Value::String(text.as_str().into())).collect() };
    crate::actors::with_fields(compound, [
        ("Command", text(&command.command)),
        ("CurrentTickCount", Value::Int(0)),
        ("ExecuteOnFirstTick", Value::Byte(0)),
        ("LastExecution", Value::Long(command.last_execution)),
        ("LastOutput", text(&command.last_output)),
        ("LastOutputParams", Value::List(parameters)),
        ("SuccessCount", Value::Int(command.success_count)),
        ("TickDelay", Value::Int(0)),
        ("Ticking", Value::Byte(0)),
        ("TrackOutput", Value::Byte(1)),
        ("Version", Value::Int(VERSION)),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cart_s_command_is_kept_and_read_back() {
        let actor = crate::new_actor(-5, "minecraft:command_block_minecart", [0.5, 0.0, 0.5], [0.0; 3]);
        assert_eq!(actor_command(&actor), None);
        let command = CartCommand { command: "say cart".into(), last_execution: 687, success_count: 1, last_output: String::new(), parameters: Vec::new() };
        let kept = commanded_actor(&actor, &command).unwrap();
        assert_eq!(actor_command(&kept), Some(command));
    }
}
