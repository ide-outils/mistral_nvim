use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

mod form;
mod function_parameters;
mod serde_derive_internal;
mod utils;

pub(crate) use serde_derive_internal::internals;

#[proc_macro_derive(ToolList, attributes(doc, param_doc))]
pub fn tool_list_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    function_parameters::tool_list_derive(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_derive(Tool, attributes(doc, param_doc, name, param_name, description, param_description))]
pub fn tool_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    function_parameters::tool_derive(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_derive(Form, attributes(doc, param_doc, serde, param_default))]
pub fn form_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    form::form_derive(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
