use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{Data, DeriveInput, Field, Fields, Type, parse_quote_spanned, spanned::Spanned};

use crate::{serde_derive_internal::internals as ser, utils};

type Assertions = Vec<TokenStream>;

pub fn form_derive(input: DeriveInput) -> syn::Result<TokenStream> {
    let ctxt = ser::Ctxt::new();
    let mut attrs = ser::attr::Container::from_ast(&ctxt, &input.clone());
    ctxt.check()?;
    let data_ident = &input.ident;
    let mut assertions = Vec::new();
    let form = parse_input(&input, &mut assertions, &mut attrs)?;
    // let form = parse_input(&input, &mut assertions)?;
    let expanded = quote! {
        impl crate::mistral::model::FormExt for #data_ident {
            fn get_form() -> crate::mistral::model::RForm {
                #(#assertions)*
                crate::mistral::model::RForm::new(#form)
            }
        }
    };
    Ok(TokenStream::from(expanded))
}

pub fn parse_input(
    input: &DeriveInput,
    assertions: &mut Assertions,
    attrs: &mut ser::attr::Container,
) -> syn::Result<TokenStream> {
    let name = input.ident.to_string();
    let description = utils::get_doc_description(input.attrs.as_slice(), &input).unwrap_or_default();
    match &input.data {
        Data::Struct(data) => parse_data_fields(&name, &description, &data.fields, assertions),
        Data::Union(data) => {
            let span = data.union_token.span();
            Ok(
                quote_spanned! { span => crate::mistral::model::Form::StructTuple(#name.into(), #description.into(), Vec::with_capacity(0)) },
            )
        }
        Data::Enum(data) => {
            let mut default = "".to_string();
            let forms: Vec<_> = data
                .variants
                .iter()
                .map(|variant| {
                    let ctx = ser::Ctxt::new();
                    let serde_variant = ser::attr::Variant::from_ast(&ctx, &variant);
                    ctx.check()?;
                    let variant_span = variant.span();
                    // let v_name = variant.ident.to_string();
                    let rules = attrs.rename_all_rules();
                    let name = &serde_variant.name().deserialize.value;
                    let v_name = rules.deserialize.apply_to_variant(&name);
                    // TODO: impl rename for field and attrs see serde_derive_internal/internals/ast.rs:78
                    let mut it = variant.attrs.iter();
                    if it.any(|attr| attr.path().is_ident("default")) {
                        default = v_name.to_string();
                    }
                    let v_description =
                        utils::get_doc_description(variant.attrs.as_slice(), &variant).unwrap_or_default();
                    let inner_struct = if variant.fields.len() == 0 {
                        quote_spanned! {variant_span => crate::mistral::model::Form::Unit}
                    } else {
                        parse_data_fields(&v_name, &v_description, &variant.fields, assertions)?
                    };
                    Ok(quote_spanned! { variant_span => (#v_name, #v_description, #inner_struct).into() })
                })
                .collect::<syn::Result<_>>()?;

            let enum_span = data.enum_token.span();
            Ok(
                quote_spanned! { enum_span => crate::mistral::model::Form::Enum(#name.into(), #description.into(), #default.into(), vec![#(#forms),*]) },
            )
        }
    }
}

pub fn parse_vec_fields(fields: Vec<&Field>, mut assertions: &mut Assertions) -> syn::Result<Vec<TokenStream>> {
    fields
        .into_iter()
        .map(|field| {
            let ident = field.ident.as_ref();
            let name = ident.map(ToString::to_string).unwrap_or_default();
            let description = utils::get_doc_description(field.attrs.as_slice(), &field).unwrap_or_default();
            let field_type = &field.ty;
            let form = map_rust_type_to_form(field_type, &mut assertions)?;
            Ok(parse_quote_spanned! { field.span() => (#name, #description, #form).into() })
        })
        .collect()
}

pub fn parse_data_fields(
    // input: &DeriveInput,
    name: &String,
    description: &String,
    fields: &Fields,
    mut assertions: &mut Assertions,
) -> syn::Result<TokenStream> {
    // let description = get_doc_description(input.attrs.as_slice(), &input).unwrap_or_default();
    // let name = input.ident.to_string();
    let fields_span = fields.span();
    Ok(match fields {
        Fields::Named(fields) => {
            let fields = fields.named.iter().collect();
            let forms_fields = parse_vec_fields(fields, &mut assertions)?;
            quote_spanned! { fields_span => crate::mistral::model::Form::Struct(#name.into(), #description.into(), vec![#(#forms_fields),*]) }
        }
        Fields::Unnamed(fields) => {
            let fields = fields.unnamed.iter().collect();
            let forms_fields = parse_vec_fields(fields, &mut assertions)?;
            quote_spanned! { fields_span => crate::mistral::model::Form::StructTuple(#name.into(), #description.into(), vec![#(#forms_fields),*]) }
        }
        Fields::Unit => {
            if name == "" {
                quote_spanned! { fields_span => crate::mistral::model::Form::Unit }
            } else {
                quote_spanned! { fields_span => crate::mistral::model::Form::Struct(#name.into(), #description.into(), Vec::with_capacity(0)) }
            }
        }
    })
}

// Fonction pour mapper un type Rust vers un type "string" pour FunctionParameters
fn map_rust_type_to_form(ty: &Type, assertions: &mut Assertions) -> syn::Result<TokenStream> {
    assertions.push(utils::assert_trait_form_ext_implemented(ty));
    // parse_ty_into_call(ty)
    // Ok(quote! { <#ty>::get_form() })
    Ok(quote_spanned! { ty.span() => <#ty>::get_form() })
}
