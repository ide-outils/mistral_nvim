use super::*;

const CODE_FN: &'static [u8; 444] = br###"
fn fonction(code: &str) -> Result<Vec<String>, String> {
    fn inner(code: &str) -> Result<Vec<String>, String> {todo!()}
}
trait Trait {
    fn fonction(code: &str) -> Result<Vec<String>, String> {todo!()}
}
impl Implementation {
    fn fonction(code: &str) -> Result<Vec<String>, String> {todo!()}
}
mod mymod {
    impl quelque::part::Trait for SomeStruct {
        fn fonction(code: &str) -> Result<Vec<String>, String> {todo!()}
    }
}
"###;

#[test]
fn list_fn() -> Result<()> {
    let code = CODE_FN;
    assert_no_error_in_code(code)?;
    let mut parser = LANG::new_parser(code).unwrap();
    let tags = parser.list_tags(&TAGGER::FunctionItem).unwrap();
    crate::log!("{tags:?}");
    assert_eq!(
        tags,
        vec![
            "fonction",
            // "Trait::fonction",
            "impl<Implementation>::fonction",
            "mymod::impl<quelque-part-Trait>for<SomeStruct>::fonction"
        ]
    );
    parser.find_tag(&tags[1]).unwrap();
    Ok(())
}

const CODE_TYPE: &'static [u8; 581] = br###"
type Type = usize;
fn fonction(code: &str) -> Result<Vec<String>, String> {
    type Type = usize;
}
trait Trait {
    type Type = usize;
    fn fonction(code: &str) -> Result<Vec<String>, String> {todo!()}
}
impl Implementation {
    type Type = usize;
    type AssociatedType: Trait;
    fn fonction(code: &str) -> Result<Vec<String>, String> {todo!()}
}
mod mymod {
    type Type = usize;
    impl quelque::part::Trait for SomeStruct {
        type Type = usize;
        fn fonction(code: &str) -> Result<Vec<String>, String> {
            type Type = usize;
        }
    }
}
"###;

#[test]
fn list_type() -> Result<()> {
    let code = CODE_TYPE;
    assert_no_error_in_code(code)?;
    let mut parser = LANG::new_parser(code).unwrap();
    let tags = parser.list_tags(&TAGGER::TypeItem).unwrap();
    crate::log!("{tags:?}");
    assert_eq!(
        tags,
        vec![
            "Type",
            // "fonction::Type",
            // "Trait::Type",
            "impl<Implementation>::Type",
            "mymod::Type",
            "mymod::impl<quelque-part-Trait>for<SomeStruct>::Type",
            // "mymod::impl<quelque-part-Trait>for<SomeStruct>::fonction::Type",
        ]
    );
    parser.find_tag(&tags[1]).unwrap();
    Ok(())
}
const CODE_MOD: &'static [u8; 69] = br###"
mod un;
mod deux;
mod trois;
mod zero;
mod inner {
    mod inner;
}
"###;
#[test]
fn list_mod_succesive() -> Result<()> {
    let code = CODE_MOD;
    assert_no_error_in_code(code)?;
    let mut parser = LANG::new_parser(code).unwrap();
    let tags = parser.list_tags(&TAGGER::ModItemLeaf).unwrap();
    // let tags = list_tags(code, QUERY_MOD_IMPORT_SUCCESSIVE)?;
    crate::log!("{tags:?}");
    assert_eq!(tags.len(), 2);
    assert_eq!(tags, vec!["<mod>", "inner::<mod>"]);
    parser.find_tag(&tags[1]).unwrap();
    Ok(())
}
const CODE_CONST: &'static [u8; 131] = br###"
const MY_CONST: usize = 0;
mod mymod {
    const INNER_CONST: usize = 0;
}
trait Trait {
    const ASSOCIATED_CONST: usize = 0;
}
"###;
#[test]
fn list_const() -> Result<()> {
    let code = CODE_CONST;
    assert_no_error_in_code(code)?;
    let mut parser = LANG::new_parser(code).unwrap();
    let tags = parser.list_tags(&TAGGER::ConstItem).unwrap();
    // let tags = list_tags(code, QUERY_CONSTANT)?;
    crate::log!("{tags:?}");
    assert_eq!(
        tags,
        vec![
            "MY_CONST",
            "mymod::INNER_CONST",
            // "Trait::ASSOCIATED_CONST",
        ]
    );
    parser.find_tag(&tags[1]).unwrap();
    Ok(())
}
const CODE_STATIC: &'static [u8; 137] = br###"
static MY_STATIC: usize = 0;
mod mymod {
    static INNER_STATIC: usize = 0;
}
trait Trait {
    static ASSOCIATED_STATIC: usize = 0;
}
"###;
#[test]
fn list_static() -> Result<()> {
    let code = CODE_STATIC;
    assert_no_error_in_code(code)?;
    let mut parser = LANG::new_parser(code).unwrap();
    let tags = parser.list_tags(&TAGGER::StaticItem).unwrap();
    crate::log!("{tags:?}");
    assert_eq!(
        tags,
        vec![
            "MY_STATIC",
            "mymod::INNER_STATIC",
            // "Trait::ASSOCIATED_STATIC",
        ]
    );
    parser.find_tag(&tags[1]).unwrap();
    crate::log!("{tags:?}");
    Ok(())
}
const CODE_ENUM: &'static [u8; 111] = br###"
enum Enum { A, B }
mod mymod {
    enum InnerEnum { X, Y }
}
fn fonction() {
    enum FonctionEnum { P, Q }
}
"###;

#[test]
fn list_enum() -> Result<()> {
    let code = CODE_ENUM;
    assert_no_error_in_code(code)?;
    let mut parser = LANG::new_parser(code).unwrap();
    let tags = parser.list_tags(&TAGGER::EnumItem).unwrap();
    crate::log!("{tags:?}");
    assert_eq!(tags, vec!["Enum", "mymod::InnerEnum"]);
    parser.find_tag(&tags[1]).unwrap();
    Ok(())
}
const CODE_STRUCT: &'static [u8; 131] = br###"
struct MyStruct;
mod mymod {
    struct InnerStruct;
}
trait Trait {
    struct AssociatedStruct {
        field: String,
    }
}
"###;
#[test]
fn list_struct() -> Result<()> {
    let code = CODE_STRUCT;
    assert_no_error_in_code(code)?;
    let mut parser = LANG::new_parser(code).unwrap();
    let tags = parser.list_tags(&TAGGER::StructItem).unwrap();
    crate::log!("{tags:?}");
    assert_eq!(
        tags,
        vec![
            "MyStruct",
            "mymod::InnerStruct",
            // "Trait::AssociatedStruct",
        ]
    );
    parser.find_tag(&tags[1]).unwrap();

    Ok(())
}
const CODE_USE: &'static [u8; 128] = br###"
use std::collections::HashMap;
use std::io;
use std::path::Path;
use mymod::path::Path;
mod tests {
    use std::path::Path;
}
"###;
#[test]
fn list_use() -> Result<()> {
    let code = CODE_USE;
    assert_no_error_in_code(code)?;
    let mut parser = LANG::new_parser(code).unwrap();
    let tags = parser.list_tags(&TAGGER::UseDeclaration).unwrap();
    crate::log!("{tags:?}");
    assert_eq!(tags, vec!["<use>", "tests::<use>"]);
    parser.find_tag(&tags[1]).unwrap();
    let range = tags[0].range;
    let start = range.start_byte;
    let end = range.end_byte;
    let first_block = str::from_utf8(&code[start..end]).unwrap();
    crate::log!("\n```rust\n{}\n```", first_block);
    assert_eq!(start, 1, "\n\nSTART RANGE issue\n");
    assert_eq!(end, 88, "\n\nEND RANGE issue\n");
    let expected = r#"use std::collections::HashMap;
use std::io;
use std::path::Path;
use mymod::path::Path;"#;
    assert_eq!(first_block, expected);

    Ok(())
}
const CODE_TAG_IMPL: &'static [u8; 195] = br###"
struct Truc<T>(T);
struct Plop<T, U>(T, U);
trait A {};
trait B {};
impl A for Truc<T> {type Num = usize;}
impl B for Truc<T> {type Num = usize;}
impl<T: A, U: B> Plop<T, U> {type Num = usize;}
"###;
#[test]
fn tags() -> Result<()> {
    let code = CODE_TAG_IMPL;
    assert_no_error_in_code(code)?;
    let mut parser = LANG::new_parser(code).unwrap();
    let tags = parser.list_tags(&TAGGER::TypeItem).unwrap();
    crate::log!("{tags:?}");
    assert_eq!(
        tags,
        vec![
            "impl<A>for<Truc<T>>::Num",
            "impl<B>for<Truc<T>>::Num",
            "impl<Plop<T,U>>::Num"
        ]
    );
    parser.find_tag(&tags[1]).unwrap();
    Ok(())
}
const CODE_DECLARATION: &'static [u8; 136] = br###"
mod a {}
impl A for Truc<T> {type Num = usize;}
impl B for Truc<T> {type Num = usize;}
impl<T: A, U: B> Plop<T, U> {type Num = usize;}
"###;
#[test]
fn list_declaration() -> Result<()> {
    let code = CODE_DECLARATION;
    assert_no_error_in_code(code)?;
    let mut parser = LANG::new_parser(code).unwrap();
    let tags = parser.list_tags(&TAGGER::DeclarationList).unwrap();
    crate::log!("{tags:?}");
    assert_eq!(
        tags,
        vec!["a", "impl<A>for<Truc<T>>", "impl<B>for<Truc<T>>", "impl<Plop<T,U>>"]
    );
    parser.find_tag(&tags[1]).unwrap();
    Ok(())
}
