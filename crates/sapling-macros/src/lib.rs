//! `#[derive(Command)]`, `#[derive(CommandEnum)]` and `#[derive(Form)]` for sapling, documented
//! where they are re-exported (`sapling::command`, `sapling::form`). The generated code names
//! items as `::sapling::...`; `#[command(crate = "path")]` / `#[form(crate = "path")]` changes that.

mod attrs;
mod command;
mod form;

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

fn expand(input: TokenStream, derive: fn(DeriveInput) -> syn::Result<proc_macro2::TokenStream>) -> TokenStream {
    derive(parse_macro_input!(input as DeriveInput)).unwrap_or_else(syn::Error::into_compile_error).into()
}

/// The syntax of a command: a struct's fields are its parameters, in order; an enum's variants are
/// sub-commands, each with its own fields. Field types implement `Parameter` (numbers, `bool`,
/// `String`, `Text`, `Target`, `GameMode`, a `CommandEnum`, or `Option` of one as a trailing
/// optional parameter).
///
/// Container: `#[command(name = "hub", aliases = ["lobby"], description = "...", permission = "operator")]`;
/// the name defaults to the type's in lower case. Variants and fields: `#[command(name = "...")]`.
#[proc_macro_derive(Command, attributes(command))]
pub fn derive_command(input: TokenStream) -> TokenStream {
    expand(input, command::command)
}

/// A command parameter that is one of an enum's unit variants, completed by the client. Words
/// are the variant names in lower case, or `#[command(name = "...")]`.
#[proc_macro_derive(CommandEnum, attributes(command))]
pub fn derive_command_enum(input: TokenStream) -> TokenStream {
    expand(input, command::command_enum)
}

/// A custom form from a struct: one element per field, the value sent holds the defaults and the
/// answer is the struct filled in. `String` is a text box, `bool` a toggle, `f32`/`f64`/`i32`/`u32`
/// a slider (`min`, `max`, optional `step`), `usize` a dropdown over `options = ["a", "b"]`, or a
/// step slider with `step_slider`.
///
/// Container: `#[form(title = "...")]`. Fields: `#[form(label = "...", placeholder = "...",
/// max_len = 16, note = "a line of text above this field")]`.
#[proc_macro_derive(Form, attributes(form))]
pub fn derive_form(input: TokenStream) -> TokenStream {
    expand(input, form::form)
}
