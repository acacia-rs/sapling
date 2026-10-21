//! Forms: windows a server opens on a player's screen. [`Menu`] is a list of buttons, [`Modal`] a
//! question with two, [`Custom`] a sheet of inputs; `#[derive(Form)]` turns a struct into a custom
//! form whose answer is the struct filled in. Sent with [`crate::ShardCtx::send_form`].
//!
//! Answers come from the client and are checked against the form that was sent: an index past the
//! buttons, a slider value outside its bounds or a slot of the wrong type is a [`FormError`], and
//! the callback is told the form was closed.

mod custom;
mod field;
mod simple;

pub use custom::{Custom, Element, Value};
pub use field::{CustomForm, FieldOptions, FormField};
pub use simple::{Button, Menu, Modal};

/// A window to show a player and how to read their answer.
pub trait Form {
    type Response;

    /// The form as the client is sent it.
    fn to_json(&self) -> String;

    /// The answer in `data`, the client's JSON, if it fits this form.
    fn read(&self, data: &str) -> Result<Self::Response, FormError>;
}

/// How a player dealt with a form.
#[derive(Debug, Clone, PartialEq)]
pub enum FormResponse<T> {
    Submitted(T),
    /// Closed without an answer, or answered with something the form does not allow.
    Closed,
}

impl<T> FormResponse<T> {
    /// What `data`, a client's answer to `form`, amounts to. No data and JSON `null` are a close.
    pub fn of<F: Form<Response = T>>(form: &F, data: Option<&str>) -> Self {
        match data.filter(|d| d.trim() != "null").map(|d| form.read(d)) {
            Some(Ok(answer)) => Self::Submitted(answer),
            _ => Self::Closed,
        }
    }

    pub fn submitted(self) -> Option<T> {
        match self {
            Self::Submitted(answer) => Some(answer),
            Self::Closed => None,
        }
    }
}

/// Why an answer does not fit its form. `slot` counts the form's elements from 0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FormError {
    NotJson,
    /// The answer is valid JSON of the wrong shape (a string for a menu, an object for a custom form).
    WrongShape,
    WrongLength { expected: usize, got: usize },
    WrongType { slot: usize },
    OutOfRange { slot: usize },
    TooLong { slot: usize },
}

impl std::fmt::Display for FormError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotJson => write!(f, "the answer is not JSON"),
            Self::WrongShape => write!(f, "the answer has the wrong shape for this form"),
            Self::WrongLength { expected, got } => write!(f, "{got} values for {expected} elements"),
            Self::WrongType { slot } => write!(f, "element {slot} got a value of the wrong type"),
            Self::OutOfRange { slot } => write!(f, "element {slot} got a value outside its range"),
            Self::TooLong { slot } => write!(f, "element {slot} got more text than it takes"),
        }
    }
}

impl std::error::Error for FormError {}

/// The client's answer as JSON; it ends the text with a newline.
fn parse(data: &str) -> Result<serde_json::Value, FormError> {
    serde_json::from_str(data.trim()).map_err(|_| FormError::NotJson)
}

/// `value` as an index below `len`.
fn index(value: &serde_json::Value, len: usize, slot: usize) -> Result<usize, FormError> {
    let index = value.as_u64().ok_or(FormError::WrongType { slot })?;
    usize::try_from(index).ok().filter(|i| *i < len).ok_or(FormError::OutOfRange { slot })
}
