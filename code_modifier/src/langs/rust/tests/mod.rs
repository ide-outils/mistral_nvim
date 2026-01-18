mod injections;
mod queries;

use super::*;
use crate::Result;

type LANG = Rust;
type TAGGER = <Rust as LanguageExt>::Tagger;

fn count_errors(code: &[u8]) -> Result<usize> {
    let mut parser = LANG::new_parser(code).unwrap();
    crate::log!("{}", parser.tree.root_node().to_sexp());
    let mut query = TAGGER::query_error();
    use tree_sitter::StreamingIterator as _;
    Ok(parser.matches(&mut query).count())
}
fn assert_no_error_in_code(code: &[u8]) -> Result<()> {
    assert_eq!(count_errors(code)?, 0, "CODE contains errors !!!");
    Ok(())
}
#[test]
fn error() -> Result<()> {
    assert_eq!(count_errors(b"fn f;")?, 1);
    assert_eq!(count_errors(b"mod plop::truc;")?, 1);
    Ok(())
}
