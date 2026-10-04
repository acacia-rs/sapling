//! What `#[derive(Form)]` builds on: a struct is a [`Custom`] form with one element per field.

use super::{Custom, Element, Form, FormError, Value};

/// A struct that is a custom form: its value holds the defaults, the answer is the struct
/// filled in. Written by `#[derive(Form)]`.
pub trait CustomForm: Sized {
    fn layout(&self) -> Custom;

    /// `values` has passed [`Custom::read`] for [`CustomForm::layout`].
    fn from_values(values: Vec<Value>) -> Option<Self>;
}

impl<T: CustomForm> Form for T {
    type Response = T;

    fn to_json(&self) -> String {
        self.layout().to_json()
    }

    fn read(&self, data: &str) -> Result<T, FormError> {
        T::from_values(self.layout().read(data)?).ok_or(FormError::WrongShape)
    }
}

/// A field's `#[form(...)]` attributes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FieldOptions {
    pub label: &'static str,
    pub placeholder: &'static str,
    pub min: f64,
    pub max: f64,
    pub step: f64,
    pub options: &'static [&'static str],
    pub step_slider: bool,
    pub max_len: usize,
}

impl FieldOptions {
    pub const DEFAULT: Self = Self { label: "", placeholder: "", min: 0.0, max: 0.0, step: 1.0, options: &[], step_slider: false, max_len: Custom::MAX_INPUT };
}

/// A type a form field can have: `String` is a text box, `bool` a toggle, a number a slider and
/// `usize` a dropdown or step slider (the index chosen).
pub trait FormField: Sized {
    /// The element showing `self` as its default.
    fn element(&self, options: &FieldOptions) -> Element;
    fn from_value(value: Value) -> Option<Self>;
}

impl FormField for String {
    fn element(&self, o: &FieldOptions) -> Element {
        Element::Input { label: o.label.into(), placeholder: o.placeholder.into(), default: self.clone(), max_len: o.max_len }
    }

    fn from_value(value: Value) -> Option<Self> {
        if let Value::Text(text) = value { Some(text) } else { None }
    }
}

impl FormField for bool {
    fn element(&self, o: &FieldOptions) -> Element {
        Element::Toggle { label: o.label.into(), default: *self }
    }

    fn from_value(value: Value) -> Option<Self> {
        if let Value::Toggle(on) = value { Some(on) } else { None }
    }
}

impl FormField for usize {
    fn element(&self, o: &FieldOptions) -> Element {
        let (label, choices) = (o.label.to_owned(), o.options.iter().map(|c| (*c).to_owned()).collect());
        if o.step_slider { Element::StepSlider { label, steps: choices, default: *self } } else { Element::Dropdown { label, options: choices, default: *self } }
    }

    fn from_value(value: Value) -> Option<Self> {
        if let Value::Choice(index) = value { Some(index) } else { None }
    }
}

macro_rules! slider {
    ($($number:ty: $from:expr),*) => {$(
        impl FormField for $number {
            fn element(&self, o: &FieldOptions) -> Element {
                Element::Slider { label: o.label.into(), min: o.min, max: o.max, step: o.step, default: f64::from(*self) }
            }

            fn from_value(value: Value) -> Option<Self> {
                if let Value::Number(number) = value { Some($from(number)) } else { None }
            }
        }
    )*};
}
slider!(f64: |n| n, f32: |n| n as f32, i32: |n: f64| n.round() as i32, u32: |n: f64| n.round() as u32);
