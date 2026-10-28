//! The plugin UI calls of `sapling_api::UiBackend`: hotbar lines here, the rest in their own
//! modules (labels.rs, hidden.rs, extra_bars.rs, debug_shapes.rs, dialogue.rs).

use acacia_proto::packets::{Text, TextCategory, TextContent, TextContentJukeboxPopup, TextContentPopup, TextContentTip, TextType};
use bevy_ecs::prelude::*;
use sapling_api::{Actor, BossBar, DebugShape, Dialogue, DialogueCallback, EntityId, HudLine, Label, PlayerId, UiBackend};

use crate::components::NetId;
use crate::handlers::Ctx;

/// What tells a client of `line`, as dragonfly's `session/text.go` sends each.
fn hud_line(line: HudLine, text: &str) -> Text {
    let message = text.to_owned();
    let (r#type, category, content) = match line {
        HudLine::Popup => (TextType::Popup, TextCategory::Parameters, TextContent::Popup(TextContentPopup { message, parameters: Vec::new() })),
        HudLine::Tip => (TextType::Tip, TextCategory::MessageOnly, TextContent::Tip(TextContentTip { message })),
        HudLine::JukeboxPopup => (TextType::JukeboxPopup, TextCategory::Parameters, TextContent::JukeboxPopup(TextContentJukeboxPopup { message, parameters: Vec::new() })),
    };
    Text { needs_translation: false, category, r#type, content, xuid: String::new(), platform_chat_id: String::new(), has_filtered_message: false, filtered_message: None }
}

impl Ctx<'_> {
    /// The ECS entity of a player in the world or of an entity of the shard.
    pub(crate) fn actor(&self, actor: Actor) -> Option<Entity> {
        match actor {
            Actor::Player(player) => self.entity(player),
            Actor::Entity(id) => crate::entities::find(self.world, id).or_else(|| crate::labels::floating(self.world, id)),
        }
    }
}

/// The runtime id a client knows `entity` by.
pub(crate) fn net_id(world: &World, entity: Entity) -> Option<u64> {
    world.get::<NetId>(entity).map(|id| id.0)
}

impl UiBackend for Ctx<'_> {
    fn send_hud_line(&mut self, player: PlayerId, line: HudLine, text: &str) {
        if let Some(mut out) = self.out(player) {
            out.push(&hud_line(line, text));
        }
    }

    fn set_label(&mut self, actor: Actor, label: Label, text: Option<&str>) -> bool {
        let Some(entity) = self.actor(actor) else { return false };
        crate::labels::set(self.world, entity, label, text);
        true
    }

    fn set_hidden(&mut self, viewer: PlayerId, actor: Actor, hidden: bool) -> bool {
        let (Some(viewer), Some(target)) = (self.entity(viewer), self.actor(actor)) else { return false };
        crate::hidden::set(self.world, viewer, target, hidden);
        true
    }

    fn add_floating_text(&mut self, at: [f32; 3], text: &str) -> Option<EntityId> {
        let layer = crate::worlds::current(self.world);
        crate::labels::add_floating(self.world, layer, at, text)
    }

    fn set_floating_text(&mut self, id: EntityId, text: &str) -> bool {
        crate::labels::set_floating(self.world, id, text)
    }

    fn set_extra_boss_bar(&mut self, player: PlayerId, key: u32, bar: Option<&BossBar>) {
        if let Some(entity) = self.entity(player) {
            crate::extra_bars::set(self.world, entity, crate::extra_bars::BarKey::Plugin(key), bar);
        }
    }

    fn debug_shape(&mut self, player: PlayerId, id: u64, shape: Option<&DebugShape>) {
        if let Some(mut out) = self.out(player) {
            out.push(&crate::debug_shapes::packet(id, shape));
        }
    }

    fn send_dialogue(&mut self, player: PlayerId, npc: Actor, dialogue: &Dialogue, on_press: DialogueCallback) -> bool {
        let (Some(entity), Some(npc)) = (self.entity(player), self.actor(npc)) else { return false };
        crate::dialogue::open(self.world, entity, npc, dialogue, on_press)
    }

    fn close_dialogue(&mut self, player: PlayerId) {
        if let Some(entity) = self.entity(player) {
            crate::dialogue::close(self.world, entity);
        }
    }
}
