use proc_macro2::TokenStream;
use quote::quote_spanned;
use syn::{Arm, Data, DeriveInput, Expr, Fields, Type, parse_quote_spanned, spanned::Spanned};

#[inline]
fn error<T>(spanable: impl Spanned, message: &str) -> syn::Result<T> {
    Err(syn::Error::new(spanable.span(), message))
}

pub fn tool_list_derive(input: DeriveInput) -> syn::Result<TokenStream> {
    let span = input.span();
    let ident = input.ident;

    pub fn parse_fields(data: Data) -> syn::Result<syn::FieldsUnnamed> {
        let not_struct_err = "ToolList ne peut être dérivé que pour des structs.";
        let not_named_struct_err =
            "ToolList ne peut être dérivé que pour des structs avec des champs non nommés : les tuple struct.";
        // On récupère les champs de la structure
        match data {
            Data::Struct(data) => match data.fields {
                Fields::Unnamed(fields) => Ok(fields),
                fields => error(fields, not_named_struct_err),
            },
            Data::Enum(data) => error(data.enum_token, not_struct_err),
            Data::Union(data) => error(data.union_token, not_struct_err),
        }
    }
    let tools_idents: Vec<syn::Ident> = parse_fields(input.data)?
        .unnamed
        .into_iter()
        .map(|field| match field.ty {
            Type::Path(path) => {
                if path.path.segments.len() != 1 {
                    error(&path, "Only struct's name without generic are supported.")?
                } else {
                    let segs = path.path.segments;
                    Ok(segs.into_iter().next().unwrap().ident)
                }
            }
            ty => error(ty, "Only struct's name without generic are supported.")?,
        })
        .collect::<syn::Result<_>>()?;

    let mut tools_names = std::collections::HashSet::with_capacity(tools_idents.len());
    let mut errors: Option<syn::Error> = None;
    for tool_ident in tools_idents.iter() {
        let name = tool_ident.to_string();
        if tools_names.contains(&name) {
            let new_error = syn::Error::new_spanned(tool_ident, "Duplicated tool. Remove it.");
            match &mut errors {
                Some(prev_errors) => prev_errors.combine(new_error),
                none => *none = Some(new_error),
            }
        }
        tools_names.insert(name);
    }
    if let Some(error) = errors {
        return syn::Result::Err(error);
    }
    let tools_names: Vec<_> = tools_names.into_iter().collect();

    let content_arms: Vec<Arm> = tools_idents
        .iter()
        .map(|ident| {
            let span = ident.span();
            let ident_str = ident.to_string();
            parse_quote_spanned! { span => #ident_str => #ident::parse_and_run(state, msg) }
        })
        .collect();

    let tools_details: Vec<Expr> = tools_idents
        .iter()
        .map(|ident| parse_quote_spanned! { ident.span() => #ident::get_tool() })
        .collect();

    let expanded = quote_spanned! {
    span =>
    impl ToolListExt for #ident{
        fn get_tools() -> Vec<Tool> {
            vec![#(#tools_details),*]
        }
        fn run_tool(state: crate::nvim::model::SharedState, msg: crate::messages::RunToolMessage) -> serde_json::Result<String> {
           match msg.tool.function.name.as_str() {
                #(#content_arms),*,
                wrong_name => {
                    let valid_names = vec![#(#tools_names),*];
                    Ok(crate::mistral::model::Message::tool_name_does_not_exist(wrong_name, valid_names))
                }
            }
        }
    }
    };
    Ok(TokenStream::from(expanded))
}

pub fn tool_derive(input: DeriveInput) -> syn::Result<TokenStream> {
    let span = input.span();
    let struct_ident = &input.ident.clone();
    let expanded = quote_spanned! {
        span =>
        impl ToolExt for #struct_ident {}
    };

    Ok(TokenStream::from(expanded))
}
