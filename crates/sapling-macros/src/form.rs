use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Type};

use crate::attrs;

#[derive(Default)]
struct Field {
    label: Option<String>,
    placeholder: Option<String>,
    min: Option<f64>,
    max: Option<f64>,
    step: Option<f64>,
    options: Vec<String>,
    step_slider: bool,
    max_len: Option<f64>,
    note: Option<String>,
}

fn field_attrs(field: &syn::Field) -> syn::Result<Field> {
    let mut f = Field::default();
    attrs::each(&field.attrs, "form", |meta| {
        match meta.path.get_ident().map(ToString::to_string).as_deref() {
            Some("label") => f.label = Some(attrs::string(meta)?),
            Some("placeholder") => f.placeholder = Some(attrs::string(meta)?),
            Some("min") => f.min = Some(attrs::number(meta)?),
            Some("max") => f.max = Some(attrs::number(meta)?),
            Some("step") => f.step = Some(attrs::number(meta)?),
            Some("options") => f.options = attrs::strings(meta)?,
            Some("step_slider") => f.step_slider = true,
            Some("max_len") => f.max_len = Some(attrs::number(meta)?),
            Some("note") => f.note = Some(attrs::string(meta)?),
            _ => return Err(attrs::unknown(meta)),
        }
        Ok(())
    })?;
    Ok(f)
}

/// The last word of a field's type, which decides what its element needs.
fn type_name(ty: &Type) -> String {
    match ty {
        Type::Path(path) => path.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default(),
        _ => String::new(),
    }
}

fn check(field: &syn::Field, f: &Field) -> syn::Result<()> {
    let kind = type_name(&field.ty);
    let slider = matches!(kind.as_str(), "f32" | "f64" | "i32" | "u32");
    let problem = if slider && !matches!((f.min, f.max), (Some(min), Some(max)) if min < max) {
        "a slider needs `min` and `max`, with min below max"
    } else if kind == "usize" && f.options.is_empty() {
        "a dropdown or step slider needs `options = [\"...\"]`"
    } else if f.step.is_some_and(|step| step <= 0.0) {
        "`step` must be positive"
    } else {
        return Ok(());
    };
    Err(syn::Error::new_spanned(field, problem))
}

pub fn form(input: DeriveInput) -> syn::Result<TokenStream> {
    let (mut title, mut krate) = (None, None);
    attrs::each(&input.attrs, "form", |meta| {
        match meta.path.get_ident().map(ToString::to_string).as_deref() {
            Some("title") => title = Some(attrs::string(meta)?),
            Some("crate") => krate = Some(attrs::string(meta)?),
            _ => return Err(attrs::unknown(meta)),
        }
        Ok(())
    })?;
    let krate = attrs::krate(krate)?;
    let ident = &input.ident;
    let title = title.ok_or_else(|| syn::Error::new_spanned(ident, "a form needs `#[form(title = \"...\")]`"))?;
    let Data::Struct(syn::DataStruct { fields: Fields::Named(fields), .. }) = &input.data else {
        return Err(syn::Error::new_spanned(ident, "a form is a struct with named fields"));
    };
    let (mut elements, mut readers) = (Vec::new(), Vec::new());
    for field in &fields.named {
        let f = field_attrs(field)?;
        check(field, &f)?;
        let (name, ty) = (field.ident.as_ref().expect("named"), &field.ty);
        if let Some(note) = &f.note {
            elements.push(quote!(#krate::form::Element::Label(#note.to_owned())));
            readers.push(quote!(values.next()?;));
        }
        let label = f.label.unwrap_or_else(|| name.to_string());
        let (placeholder, options, step_slider) = (f.placeholder.unwrap_or_default(), &f.options, f.step_slider);
        let (min, max, step) = (f.min.unwrap_or(0.0), f.max.unwrap_or(0.0), f.step.unwrap_or(1.0));
        let max_len = f.max_len.map_or(quote!(#krate::form::Custom::MAX_INPUT), |n| {
            let n = n as usize;
            quote!(#n)
        });
        elements.push(quote! {
            #krate::form::FormField::element(&self.#name, &#krate::form::FieldOptions {
                label: #label, placeholder: #placeholder, min: #min, max: #max, step: #step,
                options: &[#(#options),*], step_slider: #step_slider, max_len: #max_len,
            })
        });
        readers.push(quote!(let #name = <#ty as #krate::form::FormField>::from_value(values.next()?)?;));
    }
    let names = fields.named.iter().map(|f| &f.ident);
    Ok(quote! {
        impl #krate::form::CustomForm for #ident {
            fn layout(&self) -> #krate::form::Custom {
                #krate::form::Custom { title: #title.to_owned(), elements: vec![#(#elements),*] }
            }

            fn from_values(values: ::std::vec::Vec<#krate::form::Value>) -> ::core::option::Option<Self> {
                let mut values = values.into_iter();
                #(#readers)*
                Some(Self { #(#names),* })
            }
        }
    })
}
