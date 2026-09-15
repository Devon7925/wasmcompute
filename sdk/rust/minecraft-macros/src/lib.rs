use proc_macro::TokenStream;
use quote::{format_ident, quote};
use std::collections::BTreeSet;
use syn::{
    parse::{Parse, ParseStream},
    Expr, Ident, LitStr, Token, Type,
};

struct Arg {
    name: Ident,
    ty: Type,
    value: Expr,
}
struct Command {
    text: LitStr,
    args: Vec<Arg>,
}
impl Parse for Command {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let text = input.parse()?;
        let mut args = Vec::new();
        while !input.is_empty() {
            input.parse::<Token![,]>()?;
            if input.is_empty() {
                break;
            }
            let name = input.parse()?;
            let ty = if input.peek(Token![:]) {
                input.parse::<Token![:]>()?;
                input.parse()?
            } else {
                syn::parse_quote!(i32)
            };
            input.parse::<Token![=]>()?;
            args.push(Arg {
                name,
                ty,
                value: input.parse()?,
            });
        }
        Ok(Self { text, args })
    }
}
fn expand(input: TokenStream, prefix: &str, capture: &str) -> TokenStream {
    let c = syn::parse_macro_input!(input as Command);
    match make(c, prefix, capture) {
        Ok(t) => t.into(),
        Err(e) => e.into_compile_error().into(),
    }
}
fn make(c: Command, prefix: &str, capture: &str) -> syn::Result<proc_macro2::TokenStream> {
    let mut command = format!("{prefix}{}", c.text.value());
    if command.starts_with('/') {
        command.remove(0);
    }
    if command.is_empty() || command.contains(['\n', '\r', '\0']) {
        return Err(syn::Error::new_spanned(
            &c.text,
            "command must be one nonempty line",
        ));
    }
    let mut names = BTreeSet::new();
    let mut types = Vec::new();
    let mut values = Vec::new();
    let mut params = Vec::new();
    // Parse placeholders once: binding names must not collide with generated aN names.
    let mut substitutions = std::collections::BTreeMap::new();
    for (i, a) in c.args.iter().enumerate() {
        let name = a.name.to_string();
        if !names.insert(name.clone()) {
            return Err(syn::Error::new_spanned(
                &a.name,
                "duplicate command binding",
            ));
        }
        if !command.contains(&format!("$({name})")) {
            return Err(syn::Error::new_spanned(&a.name, "unused command binding"));
        }
        match &a.ty {
            Type::Path(p)
                if p.path.is_ident("i32") || p.path.is_ident("f32") || p.path.is_ident("f64") =>
            {
                ()
            }
            _ => {
                return Err(syn::Error::new_spanned(
                    &a.ty,
                    "bindings support i32 (default), f32 or f64",
                ))
            }
        }
        substitutions.insert(name, format!("$(a{i})"));
        types.push(&a.ty);
        values.push(&a.value);
        params.push(format_ident!("a{i}"));
    }
    let mut normalized = String::new();
    let mut remaining = command.as_str();
    while let Some(at) = remaining.find("$(") {
        normalized.push_str(&remaining[..at]);
        let tail = &remaining[at + 2..];
        let end = tail
            .find(')')
            .ok_or_else(|| syn::Error::new_spanned(&c.text, "unclosed command placeholder"))?;
        normalized.push_str(substitutions.get(&tail[..end]).ok_or_else(|| {
            syn::Error::new_spanned(&c.text, format!("missing binding for {}", &tail[..end]))
        })?);
        remaining = &tail[end + 1..];
    }
    normalized.push_str(remaining);
    let metadata = serde_json::json!({"command":normalized,"capture":capture}).to_string();
    let import = "wc1_".to_owned()
        + &metadata
            .bytes()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
    let ret = if capture == "outcome" {
        quote!(i64)
    } else {
        quote!(i32)
    };
    // Keep the Rust foreign item name unique too. Recent toolchains may merge
    // same-named foreign items across macro scopes before applying link_name.
    let foreign = syn::Ident::new(&import, proc_macro2::Span::call_site());
    let call = quote! { unsafe { #foreign(#(#values),*) } };
    let result = match capture {
        "success" => quote! { (#call) != 0 },
        "outcome" => quote! { ::minecraft::CommandOutcome::from_raw(#call) },
        _ => call,
    };
    Ok(quote! {{
        #[link(wasm_import_module="minecraft")]
        extern "C" { #[link_name=#import] fn #foreign(#(#params: #types),*) -> #ret; }
        #result
    }})
}
#[proc_macro]
pub fn command(input: TokenStream) -> TokenStream {
    expand(input, "", "result")
}
#[proc_macro]
pub fn command_success(input: TokenStream) -> TokenStream {
    expand(input, "", "success")
}
#[proc_macro]
pub fn command_outcome(input: TokenStream) -> TokenStream {
    expand(input, "", "outcome")
}
macro_rules! aliases { ($($name:ident),*) => { $(#[proc_macro] pub fn $name(input:TokenStream)->TokenStream { expand(input,concat!(stringify!($name)," "),"result") })* }; }
aliases!(
    fill,
    execute,
    scoreboard,
    data,
    schedule,
    summon,
    tellraw,
    setblock,
    kill,
    teleport,
    tp,
    function,
    advancement,
    attribute,
    effect,
    give,
    item,
    particle,
    playsound,
    tag,
    time,
    title,
    weather
);

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn placeholders_and_raw_json() {
        let c = syn::parse_str::<Command>(
            r#""data modify storage a:b x set value {v:$(a1),n:$(n)}", a1=2, n=3"#,
        )
        .unwrap();
        assert!(make(c, "", "result").is_ok());
    }
    #[test]
    fn every_capture_is_a_valid_rust_expression() {
        for capture in ["result", "success", "outcome"] {
            let tokens = make(syn::parse_str(r#""say $(x)", x=3"#).unwrap(), "", capture).unwrap();
            syn::parse2::<syn::Expr>(tokens).unwrap();
        }
    }
    #[test]
    fn distinct_commands_have_distinct_foreign_item_names() {
        let mut names = BTreeSet::new();
        for source in [
            r#""fill 0 0 0 1 1 1 minecraft:air strict""#,
            r#""fill 0 0 0 1 1 1 minecraft:stone strict""#,
            r#""fill 0 0 0 1 1 1 minecraft:dirt strict""#,
        ] {
            let tokens = make(syn::parse_str(source).unwrap(), "", "result").unwrap();
            let block = syn::parse2::<syn::ExprBlock>(tokens).unwrap();
            let syn::Stmt::Item(syn::Item::ForeignMod(module)) = &block.block.stmts[0] else {
                panic!("foreign import missing")
            };
            let syn::ForeignItem::Fn(function) = &module.items[0] else {
                panic!("foreign function missing")
            };
            let name = function.sig.ident.to_string();
            assert!(name.starts_with("wc1_"));
            assert!(
                names.insert(name),
                "foreign names must not alias across macro scopes"
            );
        }
    }
    #[test]
    fn rejects_missing_and_duplicate() {
        for src in [
            r#""say $(x)""#,
            r#""say $(x)", x=1,x=2"#,
            r#""say yes", x=1"#,
        ] {
            assert!(make(syn::parse_str(src).unwrap(), "", "result").is_err());
        }
    }
}
