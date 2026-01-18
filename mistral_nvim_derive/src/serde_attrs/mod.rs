mod ast;
mod attr;
mod case;
mod ctxt;
mod name;
mod symbol;

// Pasted
// pub mod ast;
// pub mod attr;
// pub mod name;

// mod case;
// mod check;
// mod ctxt;
// mod receiver;
// mod respan;
// mod symbol;

pub use ctxt::Ctxt;
use syn::Type;

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
