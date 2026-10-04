use serde_json::json;

use super::{Form, FormError, index, parse};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Button {
    pub text: String,
    /// A texture path of the client's resource packs (`textures/items/apple`) or an `http(s)` URL.
    pub image: Option<String>,
}

/// A list of buttons under a text. The answer is the index of the button pressed.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Menu {
    pub title: String,
    pub body: String,
    pub buttons: Vec<Button>,
}

impl Menu {
    pub fn new(title: impl Into<String>) -> Self {
        Self { title: title.into(), ..Self::default() }
    }

    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = body.into();
        self
    }

    pub fn button(mut self, text: impl Into<String>) -> Self {
        self.buttons.push(Button { text: text.into(), image: None });
        self
    }

    pub fn button_with_image(mut self, text: impl Into<String>, image: impl Into<String>) -> Self {
        self.buttons.push(Button { text: text.into(), image: Some(image.into()) });
        self
    }
}

fn button_json(button: &Button) -> serde_json::Value {
    match &button.image {
        Some(image) => {
            let kind = if image.starts_with("http://") || image.starts_with("https://") { "url" } else { "path" };
            json!({"text": button.text, "image": {"type": kind, "data": image}})
        }
        None => json!({"text": button.text}),
    }
}

impl Form for Menu {
    type Response = usize;

    fn to_json(&self) -> String {
        let buttons: Vec<_> = self.buttons.iter().map(button_json).collect();
        json!({"type": "form", "title": self.title, "content": self.body, "buttons": buttons}).to_string()
    }

    fn read(&self, data: &str) -> Result<usize, FormError> {
        index(&parse(data)?, self.buttons.len(), 0)
    }
}

/// A question with two buttons. The answer is whether the first one was pressed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Modal {
    pub title: String,
    pub body: String,
    pub yes: String,
    pub no: String,
}

impl Modal {
    /// Buttons labelled with the client's own "Yes" and "No".
    pub fn new(title: impl Into<String>, body: impl Into<String>) -> Self {
        Self { title: title.into(), body: body.into(), yes: "gui.yes".into(), no: "gui.no".into() }
    }

    pub fn buttons(mut self, yes: impl Into<String>, no: impl Into<String>) -> Self {
        (self.yes, self.no) = (yes.into(), no.into());
        self
    }
}

impl Form for Modal {
    type Response = bool;

    fn to_json(&self) -> String {
        json!({"type": "modal", "title": self.title, "content": self.body, "button1": self.yes, "button2": self.no}).to_string()
    }

    fn read(&self, data: &str) -> Result<bool, FormError> {
        parse(data)?.as_bool().ok_or(FormError::WrongShape)
    }
}
