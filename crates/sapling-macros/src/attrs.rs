//! Reading `#[command(...)]` / `#[form(...)]` values.

use syn::meta::ParseNestedMeta;
use syn::{Attribute, Expr, ExprLit, Lit, LitStr, Path, UnOp};

/// Calls `each` for every `key` / `key = value` inside the `#[name(...)]` attributes among `attrs`.
pub fn each(attrs: &[Attribute], name: &str, mut each: impl FnMut(&ParseNestedMeta<'_>) -> syn::Result<()>) -> syn::Result<()> {
    for attr in attrs.iter().filter(|a| a.path().is_ident(name)) {
        attr.parse_nested_meta(|meta| each(&meta))?;
    }
    Ok(())
}

pub fn string(meta: &ParseNestedMeta<'_>) -> syn::Result<String> {
    Ok(meta.value()?.parse::<LitStr>()?.value())
}

/// `key = ["a", "b"]`
pub fn strings(meta: &ParseNestedMeta<'_>) -> syn::Result<Vec<String>> {
    let array: syn::ExprArray = meta.value()?.parse()?;
    let text = |expr: &Expr| match expr {
        Expr::Lit(ExprLit { lit: Lit::Str(s), .. }) => Ok(s.value()),
        other => Err(syn::Error::new_spanned(other, "expected a string")),
    };
    array.elems.iter().map(text).collect()
}

/// `key = 3`, `key = 0.5` or `key = -2`
pub fn number(meta: &ParseNestedMeta<'_>) -> syn::Result<f64> {
    let expr: Expr = meta.value()?.parse()?;
    let (negative, literal) = match &expr {
        Expr::Unary(unary) if matches!(unary.op, UnOp::Neg(_)) => (true, &*unary.expr),
        other => (false, other),
    };
    let value = match literal {
        Expr::Lit(ExprLit { lit: Lit::Int(i), .. }) => i.base10_parse::<f64>()?,
        Expr::Lit(ExprLit { lit: Lit::Float(f), .. }) => f.base10_parse::<f64>()?,
        other => return Err(syn::Error::new_spanned(other, "expected a number")),
    };
    Ok(if negative { -value } else { value })
}

/// The `crate = "path"` override, or `::sapling`.
pub fn krate(path: Option<String>) -> syn::Result<Path> {
    syn::parse_str(path.as_deref().unwrap_or("::sapling"))
}

pub fn unknown(meta: &ParseNestedMeta<'_>) -> syn::Error {
    meta.error("unknown attribute")
}
