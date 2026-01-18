#![allow(dead_code, unused_imports)]
pub mod ast;
pub mod attr;
pub mod name;

pub mod case;
mod check;
mod ctxt;
mod receiver;
mod respan;
pub mod symbol;

use syn::Type;

pub use self::{ctxt::Ctxt, receiver::replace_receiver};

#[derive(Copy, Clone)]
pub enum Derive {
    Serialize,
    Deserialize,
}

pub fn ungroup(mut ty: &Type) -> &Type {
    while let Type::Group(group) = ty {
        ty = &group.elem;
    }
    ty
}
