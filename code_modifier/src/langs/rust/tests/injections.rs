use std::{
    io::Write as _,
    process::{Command, Stdio},
};

use super::*;

type Code = [u8];
fn rustfmt(code: &[u8]) -> Vec<u8> {
    let mut child = Command::new("rustfmt")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to execute rustfmt");
    let mut stdin = child
        .stdin
        .take()
        .expect("Failed to open stdin for rustfmt.");
    stdin.write_all(code).unwrap();
    stdin.write_all(b"\n\n").unwrap();
    child
        .wait_with_output()
        .expect("Failed to read rustfmt output")
        .stdout
}
fn inject(initial: &Code, injected: &Code, expected: &Code) -> Result<()> {
    assert_no_error_in_code(initial)?;
    assert_no_error_in_code(injected)?;
    assert_no_error_in_code(expected)?;
    let mut parser = LANG::new_parser(initial).unwrap();
    let modif = parser.test_inject(injected).unwrap();
    let modif = rustfmt(modif.as_slice());
    crate::log!("\n\nNEW :\n{}", str::from_utf8(modif.as_slice()).unwrap());
    let modif = str::from_utf8(modif.as_slice()).unwrap();
    let initial = str::from_utf8(initial).unwrap();
    let expected = str::from_utf8(expected).unwrap();
    assert_ne!(modif, initial, "\n\nHas not changed at all.\n");
    for (line_modif, line_expected) in modif.split("\n").zip(expected.split("\n")) {
        assert_eq!(line_modif, line_expected, "\n\nDoes not match final.\n");
    }
    Ok(())
}

const CODE_FN_INITIAL: &'static [u8; 202] = br###"
fn fonction(code: &str) -> Result<Vec<String>, String> {
    fn inner(code: &str) -> Result<Vec<String>, String> {todo!()}
    inner()
}
fn other(code: &str) -> Result<Vec<String>, String> { todo!() }
"###;
const CODE_FN_INJECTION: &'static [u8; 238] = br###"
fn fonction(code: &str) -> Result<Vec<String>, String> {
    fn inner(code: &str) -> Result<Vec<String>, String> {
        Ok(vec![code.to_string()])
    }
    inner().map(|items| items.map(|c| (c + "\n// ...").to_string()).collect())
}
"###;
const CODE_FN_FINAL: &'static [u8; 302] = br###"
fn fonction(code: &str) -> Result<Vec<String>, String> {
    fn inner(code: &str) -> Result<Vec<String>, String> {
        Ok(vec![code.to_string()])
    }
    inner().map(|items| items.map(|c| (c + "\n// ...").to_string()).collect())
}
fn other(code: &str) -> Result<Vec<String>, String> { todo!() }
"###;

#[test]
fn inject_fn() -> Result<()> {
    let initial = CODE_FN_INITIAL;
    let injected = CODE_FN_INJECTION;
    let expected = CODE_FN_FINAL;
    inject(initial, injected, expected)
}

const CODE_FN_INITIAL_MULTI: &'static [u8; 269] = br###"
fn fonction(code: &str) -> Result<Vec<String>, String> {
    fn inner(code: &str) -> Result<Vec<String>, String> {todo!()}
    inner()
}
fn whatever(code: &str) -> Result<Vec<String>, String> { todo!() }
fn other(code: &str) -> Result<Vec<String>, String> { todo!() }
"###;
const CODE_FN_INJECTION_MULTI: &'static [u8; 285] = br###"
fn fonction(code: &str) -> Result<Vec<String>, String> {
    fn inner(code: &str) -> Result<Vec<String>, String> {
        Ok(vec![code.to_string()])
    }
    inner().map(|items| items.map(|c| (c + "\n// ...").to_string()).collect())
}
fn other(code: &str) -> Vec<String> { vec![] }
"###;
const CODE_FN_FINAL_MULTI: &'static [u8; 352] = br###"
fn fonction(code: &str) -> Result<Vec<String>, String> {
    fn inner(code: &str) -> Result<Vec<String>, String> {
        Ok(vec![code.to_string()])
    }
    inner().map(|items| items.map(|c| (c + "\n// ...").to_string()).collect())
}
fn whatever(code: &str) -> Result<Vec<String>, String> { todo!() }
fn other(code: &str) -> Vec<String> { vec![] }
"###;

#[test]
fn inject_fn_multiple() -> Result<()> {
    let initial = CODE_FN_INITIAL_MULTI;
    let injected = CODE_FN_INJECTION_MULTI;
    let expected = CODE_FN_FINAL_MULTI;
    inject(initial, injected, expected)
}

const CODE_FN_INITIAL_NEW: &'static [u8; 202] = br###"
fn fonction(code: &str) -> Result<Vec<String>, String> {
    fn inner(code: &str) -> Result<Vec<String>, String> {todo!()}
    inner()
}
fn other(code: &str) -> Result<Vec<String>, String> { todo!() }
"###;
const CODE_FN_INJECTION_NEW: &'static [u8; 69] = br###"

fn whatever(code: &str) -> Result<Vec<String>, String> { todo!() }
"###;
const CODE_FN_FINAL_NEW: &'static [u8; 269] = br###"
fn fonction(code: &str) -> Result<Vec<String>, String> {
    fn inner(code: &str) -> Result<Vec<String>, String> {todo!()}
    inner()
}
fn other(code: &str) -> Result<Vec<String>, String> { todo!() }
fn whatever(code: &str) -> Result<Vec<String>, String> { todo!() }
"###;

#[test]
fn inject_fn_new() -> Result<()> {
    let initial = CODE_FN_INITIAL_NEW;
    let injected = CODE_FN_INJECTION_NEW;
    let expected = CODE_FN_FINAL_NEW;
    inject(initial, injected, expected)
}

const CODE_TRAIT_INITIAL: &'static [u8; 577] = br###"
pub trait TypeTagger: Sized + 'static {
    type Lang: LanguageExt;

    const BRANCHES: &[Self];
    const LEAFS: &[Self];
    const QUERY_ERROR: &'static str = QUERY_ERROR;

    fn get_query_str(&self) -> std::borrow::Cow<'_, str>;
    fn get_query(&self) -> CodeQuery {
        CodeQuery::new_valid_query(&Self::Lang::language(), self.get_query_str())
    }
    fn query_error() -> CodeQuery {
        CodeQuery::new_valid_query(&Self::Lang::language(), Self::QUERY_ERROR.into())
    }
    fn tag_from_match(&self, m: &QueryMatch, code: &[u8]) -> Option<GroupTag<Self>>;
}
"###;
const CODE_TRAIT_INJECTION: &'static [u8; 329] = br###"
pub trait TypeTagger: Sized + 'static {
    type Other: LanguageExt;

    const QUERY_ERROR: &'static str = QUERY_ERROR;

    fn query_error() -> CodeQuery {
        CodeQuery::new_valid_query(&Self::Lang::language(), Self::QUERY_ERROR.into())
    }
    fn other(&self, m: &QueryMatch, code: &[u8]) -> Option<GroupTag<Self>>;
}
"###;
const CODE_TRAIT_FINAL: &'static [u8; 329] = CODE_TRAIT_INJECTION;

#[test]
fn inject_trait() -> Result<()> {
    let initial = CODE_TRAIT_INITIAL;
    let injected = CODE_TRAIT_INJECTION;
    let expected = CODE_TRAIT_FINAL;
    inject(initial, injected, expected)
}

const CODE_MOD_BLOCK_INITIAL: &'static [u8; 84] = br###"
mod a {
    mod b {
        mod c {
            fn fonction() {}
        }
    }
}
"###;
const CODE_MOD_BLOCK_INJECTION: &'static [u8; 114] = br###"
mod a {
    mod b {
        mod c {
            fn fonction() { todo!() }
        }
    }
    fn fonction() {}
}
"###;
const CODE_MOD_BLOCK_FINAL: &'static [u8; 114] = br###"
mod a {
    mod b {
        mod c {
            fn fonction() { todo!() }
        }
    }
    fn fonction() {}
}
"###;

#[test]
fn inject_mod_block() -> Result<()> {
    let initial = CODE_MOD_BLOCK_INITIAL;
    let injected = CODE_MOD_BLOCK_INJECTION;
    let expected = CODE_MOD_BLOCK_FINAL;
    inject(initial, injected, expected)
}

const CODE_MOD_LEAF_INITIAL: &'static [u8; 47] = br###"
mod a;
mod b;
mod c;
mod block {
    mod a;
}
"###;
const CODE_MOD_LEAF_INJECTION: &'static [u8; 29] = br###"
mod d;
mod a {
    mod c;
}
"###;
const CODE_MOD_LEAF_FINAL: &'static [u8; 65] = br###"
mod a;
mod b;
mod c;
mod d;
mod block {
    mod a;
    mod c;
}
"###;

#[test]
fn inject_mod_leaf() -> Result<()> {
    let initial = CODE_MOD_LEAF_INITIAL;
    let injected = CODE_MOD_LEAF_INJECTION;
    let expected = CODE_MOD_LEAF_FINAL;
    inject(initial, injected, expected)
}

const CODE_USE_INITIAL: &'static [u8; 47] = br###"
use a;
use b;
use c;
mod block {
    use a;
}
"###;
const CODE_USE_INJECTION: &'static [u8; 29] = br###"
use d;
mod a {
    use c;
}
"###;
const CODE_USE_FINAL: &'static [u8; 65] = br###"
use a;
use b;
use c;
use d;
mod block {
    use a;
    use c;
}
"###;

#[test]
fn inject_use() -> Result<()> {
    let initial = CODE_USE_INITIAL;
    let injected = CODE_USE_INJECTION;
    let expected = CODE_USE_FINAL;
    inject(initial, injected, expected)
}
