//! Soft enums on the wire (sapling-api `command/soft_enum.rs`): the command list carries each one's
//! first words in `dynamic_enums`; a change goes to every player in the world as UpdateSoftEnum, and
//! a player sent the list later gets the changed enums' words after it.

use std::collections::HashMap;

use acacia_proto::packets::{UpdateSoftEnum, UpdateSoftEnumActionType};
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_api::command::{ParamKind, SoftEnumChange};

use crate::plugins::CommandRegistry;

/// The words of each soft enum changed since the shard started.
#[derive(Resource, Default)]
pub(crate) struct SoftEnums(HashMap<String, Vec<String>>);

/// The words `name` has now: its parameters' first words until it is changed.
pub(crate) fn values(world: &World, name: &str) -> Vec<String> {
    if let Some(values) = world.get_resource::<SoftEnums>().and_then(|enums| enums.0.get(name)) {
        return values.clone();
    }
    let commands = world.resource::<CommandRegistry>().0.clone();
    let mut first = Vec::new();
    for parameter in commands.visible(sapling_api::Permission::Operator).flat_map(|spec| &spec.overloads).flat_map(|overload| &overload.parameters) {
        if let ParamKind::SoftEnum { name: of, values } = &parameter.kind
            && of == name
        {
            SoftEnumChange::Add(values.clone()).apply(&mut first);
        }
    }
    first
}

/// Changes `name`'s words and tells every player in the world.
pub(crate) fn change(world: &mut World, name: &str, change: SoftEnumChange) {
    let mut now = values(world, name);
    change.apply(&mut now);
    world.get_resource_or_insert_with(SoftEnums::default).0.insert(name.to_owned(), now);
    let (action_type, options) = match change {
        SoftEnumChange::Add(words) => (UpdateSoftEnumActionType::Add, words),
        SoftEnumChange::Remove(words) => (UpdateSoftEnumActionType::Remove, words),
        SoftEnumChange::Set(words) => (UpdateSoftEnumActionType::Update, words),
    };
    crate::chat::send_to_all(world, &sapling_net::encode(&UpdateSoftEnum { enum_type: name.to_owned(), options, action_type }));
}

/// What a player sent the command list now also needs: each changed enum's words.
pub(crate) fn catch_up(enums: &SoftEnums) -> Vec<Bytes> {
    let update = |(name, values): (&String, &Vec<String>)| {
        sapling_net::encode(&UpdateSoftEnum { enum_type: name.clone(), options: values.clone(), action_type: UpdateSoftEnumActionType::Update })
    };
    enums.0.iter().map(update).collect()
}
