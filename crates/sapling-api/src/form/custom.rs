use serde_json::json;

use super::{Form, FormError, index, parse};

/// One row of a [`Custom`] form.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Element {
    Label(String),
    /// A text box. An answer longer than `max_len` characters is refused.
    Input { label: String, placeholder: String, default: String, max_len: usize },
    Toggle { label: String, default: bool },
    Slider { label: String, min: f64, max: f64, step: f64, default: f64 },
    Dropdown { label: String, options: Vec<String>, default: usize },
    StepSlider { label: String, steps: Vec<String>, default: usize },
}

/// What a player left in one [`Element`].
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// A label holds nothing.
    None,
    Text(String),
    Toggle(bool),
    Number(f64),
    /// The index of the dropdown option or slider step chosen.
    Choice(usize),
}

/// A sheet of inputs with a submit button. The answer is one [`Value`] per element.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Custom {
    pub title: String,
    pub elements: Vec<Element>,
}

impl Custom {
    /// Characters a text box takes unless its element says otherwise.
    pub const MAX_INPUT: usize = 256;

    pub fn new(title: impl Into<String>) -> Self {
        Self { title: title.into(), elements: Vec::new() }
    }

    pub fn element(mut self, element: Element) -> Self {
        self.elements.push(element);
        self
    }

    pub fn label(self, text: impl Into<String>) -> Self {
        self.element(Element::Label(text.into()))
    }

    pub fn input(self, label: impl Into<String>, placeholder: impl Into<String>, default: impl Into<String>) -> Self {
        self.element(Element::Input { label: label.into(), placeholder: placeholder.into(), default: default.into(), max_len: Self::MAX_INPUT })
    }

    pub fn toggle(self, label: impl Into<String>, default: bool) -> Self {
        self.element(Element::Toggle { label: label.into(), default })
    }

    pub fn slider(self, label: impl Into<String>, range: std::ops::RangeInclusive<f64>, step: f64, default: f64) -> Self {
        self.element(Element::Slider { label: label.into(), min: *range.start(), max: *range.end(), step, default })
    }

    pub fn dropdown<S: Into<String>>(self, label: impl Into<String>, options: impl IntoIterator<Item = S>, default: usize) -> Self {
        self.element(Element::Dropdown { label: label.into(), options: options.into_iter().map(Into::into).collect(), default })
    }

    pub fn step_slider<S: Into<String>>(self, label: impl Into<String>, steps: impl IntoIterator<Item = S>, default: usize) -> Self {
        self.element(Element::StepSlider { label: label.into(), steps: steps.into_iter().map(Into::into).collect(), default })
    }
}

fn element_json(element: &Element) -> serde_json::Value {
    match element {
        Element::Label(text) => json!({"type": "label", "text": text}),
        Element::Input { label, placeholder, default, .. } => json!({"type": "input", "text": label, "placeholder": placeholder, "default": default}),
        Element::Toggle { label, default } => json!({"type": "toggle", "text": label, "default": default}),
        Element::Slider { label, min, max, step, default } => json!({"type": "slider", "text": label, "min": min, "max": max, "step": step, "default": default}),
        Element::Dropdown { label, options, default } => json!({"type": "dropdown", "text": label, "options": options, "default": default}),
        Element::StepSlider { label, steps, default } => json!({"type": "step_slider", "text": label, "steps": steps, "default": default}),
    }
}

/// The client rounds a slider to its step in single precision; bounds get this much slack.
const SLIDER_SLACK: f64 = 1e-3;

fn value(element: &Element, answer: &serde_json::Value, slot: usize) -> Result<Value, FormError> {
    let wrong = FormError::WrongType { slot };
    Ok(match element {
        Element::Label(_) => answer.is_null().then_some(Value::None).ok_or(wrong)?,
        Element::Input { max_len, .. } => {
            let text = answer.as_str().ok_or(wrong)?;
            if text.chars().count() > *max_len {
                return Err(FormError::TooLong { slot });
            }
            Value::Text(text.to_owned())
        }
        Element::Toggle { .. } => Value::Toggle(answer.as_bool().ok_or(wrong)?),
        Element::Slider { min, max, .. } => {
            let number = answer.as_f64().filter(|n| n.is_finite()).ok_or(wrong)?;
            if number < min - SLIDER_SLACK || number > max + SLIDER_SLACK {
                return Err(FormError::OutOfRange { slot });
            }
            Value::Number(number.clamp(*min, *max))
        }
        Element::Dropdown { options: choices, .. } | Element::StepSlider { steps: choices, .. } => Value::Choice(index(answer, choices.len(), slot)?),
    })
}

impl Form for Custom {
    type Response = Vec<Value>;

    fn to_json(&self) -> String {
        let content: Vec<_> = self.elements.iter().map(element_json).collect();
        json!({"type": "custom_form", "title": self.title, "content": content}).to_string()
    }

    fn read(&self, data: &str) -> Result<Vec<Value>, FormError> {
        let parsed = parse(data)?;
        let answers = parsed.as_array().ok_or(FormError::WrongShape)?;
        if answers.len() != self.elements.len() {
            return Err(FormError::WrongLength { expected: self.elements.len(), got: answers.len() });
        }
        self.elements.iter().zip(answers).enumerate().map(|(slot, (element, answer))| value(element, answer, slot)).collect()
    }
}
