use proc_macro2::TokenStream;
use quote::quote_spanned;
use syn::{Attribute, Error, Type, spanned::Spanned};

// Fonction pour récupérer la description depuis les attributs
pub fn get_doc_description(attrs: &[Attribute], spanned: impl Spanned) -> syn::Result<String> {
    let span = spanned.span();
    let Ok(doc) = get_string_from_meta_attrs("doc", attrs, spanned) else {
        return Err(Error::new(
            span.span(),
            format!("Le champs doit contenir une description dans un commentaire de documentation."),
        ));
    };
    let first_line = doc.split('\n').next().unwrap().trim().to_string();
    // first_line.truncate(40);
    Ok(first_line)
}

pub fn get_string_from_meta_attrs(key: &str, attrs: &[Attribute], span: impl Spanned) -> syn::Result<String> {
    for attr in attrs {
        if !attr.path().is_ident(key) {
            continue;
        }
        if let syn::Meta::NameValue(nv) = &attr.meta {
            if let syn::Expr::Lit(expr_lit) = &nv.value {
                if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                    return Ok(lit_str.value());
                }
            }
        }
    }
    Err(Error::new(
        span.span(),
        format!("Le champ doit avoir un attribut `#[{key}(\"...\")]`."),
    ))
}

pub fn assert_trait_form_ext_implemented(type_ident: &Type) -> TokenStream {
    let span = type_ident.span();
    quote_spanned! {
        span =>
        {struct _AssertFormExt where #type_ident: crate::mistral::model::FormExt;}
    }
    // for lifetype use : // const _: #no_conflict_name = #no_conflict_name;
}
