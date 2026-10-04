use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Data, DeriveInput, Fields, Path};

use crate::attrs;

#[derive(Default)]
struct Container {
    name: Option<String>,
    aliases: Vec<String>,
    description: String,
    permission: Option<String>,
    krate: Option<String>,
}

fn container(input: &DeriveInput) -> syn::Result<Container> {
    let mut c = Container::default();
    attrs::each(&input.attrs, "command", |meta| {
        match meta.path.get_ident().map(ToString::to_string).as_deref() {
            Some("name") => c.name = Some(attrs::string(meta)?),
            Some("aliases") => c.aliases = attrs::strings(meta)?,
            Some("description") => c.description = attrs::string(meta)?,
            Some("permission") => c.permission = Some(attrs::string(meta)?),
            Some("crate") => c.krate = Some(attrs::string(meta)?),
            _ => return Err(attrs::unknown(meta)),
        }
        Ok(())
    })?;
    Ok(c)
}

/// The `#[command(name = "...")]` of a variant or field, or `default` in lower case.
fn name_of(attributes: &[Attribute], default: &syn::Ident) -> syn::Result<String> {
    let mut name = None;
    attrs::each(attributes, "command", |meta| {
        if !meta.path.is_ident("name") {
            return Err(attrs::unknown(meta));
        }
        name = Some(attrs::string(meta)?);
        Ok(())
    })?;
    Ok(name.unwrap_or_else(|| default.to_string().to_lowercase()))
}

/// For one struct or variant: its parameter specs and the expression that parses its fields.
fn fields(krate: &Path, fields: &Fields, constructor: TokenStream) -> syn::Result<(Vec<TokenStream>, TokenStream)> {
    let named = match fields {
        Fields::Named(named) => &named.named,
        Fields::Unit => return Ok((Vec::new(), constructor)),
        Fields::Unnamed(unnamed) => return Err(syn::Error::new_spanned(unnamed, "command parameters need names: use named fields")),
    };
    let (mut specs, mut parsers) = (Vec::new(), Vec::new());
    for field in named {
        let (ident, ty) = (field.ident.as_ref().expect("named"), &field.ty);
        let name = name_of(&field.attrs, ident)?;
        specs.push(quote!(#krate::command::ParameterSpec::of::<#ty>(#name)));
        parsers.push(quote!(#ident: <#ty as #krate::command::Parameter>::parse(args)?));
    }
    Ok((specs, quote!(#constructor { #(#parsers),* })))
}

pub fn command(input: DeriveInput) -> syn::Result<TokenStream> {
    let c = container(&input)?;
    let krate = attrs::krate(c.krate)?;
    let ident = &input.ident;
    let name = c.name.unwrap_or_else(|| ident.to_string().to_lowercase());
    let (aliases, description) = (&c.aliases, &c.description);
    let permission = match c.permission.as_deref() {
        None | Some("member") => quote!(Member),
        Some("operator") => quote!(Operator),
        Some(other) => return Err(syn::Error::new_spanned(ident, format!("unknown permission `{other}`: expected \"member\" or \"operator\""))),
    };
    let (overloads, parse) = match &input.data {
        Data::Struct(data) => {
            let (specs, parse) = fields(&krate, &data.fields, quote!(Self))?;
            (vec![quote!(vec![#(#specs),*])], quote!(Ok(#parse)))
        }
        Data::Enum(data) => {
            let (mut overloads, mut arms) = (Vec::new(), Vec::new());
            for variant in &data.variants {
                let (word, variant_ident) = (name_of(&variant.attrs, &variant.ident)?, &variant.ident);
                let (specs, parse) = fields(&krate, &variant.fields, quote!(Self::#variant_ident))?;
                overloads.push(quote!(vec![#krate::command::ParameterSpec::literal(#name, #word) #(, #specs)*]));
                arms.push(quote!(#word => Ok(#parse)));
            }
            let parse = quote! {
                let mark = args.mark();
                match args.word()?.to_ascii_lowercase().as_str() {
                    #(#arms,)*
                    _ => Err(args.error_at(mark)),
                }
            };
            (overloads, parse)
        }
        Data::Union(_) => return Err(syn::Error::new_spanned(ident, "a command is a struct or an enum of sub-commands")),
    };
    Ok(quote! {
        impl #krate::command::CommandSyntax for #ident {
            fn spec() -> #krate::command::CommandSpec {
                #krate::command::CommandSpec {
                    name: #name.to_owned(),
                    aliases: vec![#(#aliases.to_owned()),*],
                    description: #description.to_owned(),
                    permission: #krate::Permission::#permission,
                    overloads: vec![#(#krate::command::Overload { parameters: #overloads }),*],
                }
            }

            #[allow(unused_variables)]
            fn parse(args: &mut #krate::command::Args<'_>) -> ::core::result::Result<Self, #krate::command::ParseError> {
                #parse
            }
        }
    })
}

pub fn command_enum(input: DeriveInput) -> syn::Result<TokenStream> {
    let c = container(&input)?;
    let krate = attrs::krate(c.krate)?;
    let ident = &input.ident;
    let name = c.name.unwrap_or_else(|| ident.to_string());
    let Data::Enum(data) = &input.data else { return Err(syn::Error::new_spanned(ident, "CommandEnum is for enums")) };
    let (mut words, mut variants) = (Vec::new(), Vec::new());
    for variant in &data.variants {
        if !matches!(variant.fields, Fields::Unit) {
            return Err(syn::Error::new_spanned(variant, "a CommandEnum variant is a single word and holds nothing"));
        }
        words.push(name_of(&variant.attrs, &variant.ident)?);
        variants.push(&variant.ident);
    }
    Ok(quote! {
        impl #krate::command::Parameter for #ident {
            fn kind() -> #krate::command::ParamKind {
                #krate::command::ParamKind::Enum { name: #name.to_owned(), values: vec![#(#words.to_owned()),*] }
            }

            fn parse(args: &mut #krate::command::Args<'_>) -> ::core::result::Result<Self, #krate::command::ParseError> {
                let mark = args.mark();
                match args.word()?.to_ascii_lowercase().as_str() {
                    #(#words => Ok(Self::#variants),)*
                    _ => Err(args.error_at(mark)),
                }
            }
        }
    })
}
