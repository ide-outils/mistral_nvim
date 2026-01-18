pub mod group;
pub mod langs;
mod parser;

use std::sync::{LazyLock, RwLock};

pub use group::{GroupTag, Range};
pub use parser::{CodeParser, CodeQuery};
use tree_sitter::{Language, QueryMatch};

pub type Result<T> = std::result::Result<T, String>;

pub const QUERY_ERROR: &'static str = r#"[(ERROR) (MISSING)] @error"#;

pub static LOGGER: LazyLock<RwLock<Box<dyn Logger>>> = LazyLock::new(|| RwLock::new(Box::new(PrintLogger)));

pub trait Logger: Send + Sync {
    fn log(&self, content: &str);
}

pub struct PrintLogger;

impl Logger for PrintLogger {
    fn log(&self, content: &str) {
        println!("{}", content);
    }
}

pub fn set_logger<L: Logger + 'static>(logger: L) {
    let mut writer = LOGGER.write().unwrap();
    *writer = Box::new(logger);
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {{
        {
            let message = format!("{}", format_args!($($arg)*));
            let reader = crate::LOGGER.read().unwrap();
            reader.log(&message);
        }
    }};
}

pub trait LanguageExt {
    type Tagger: TypeTagger;
    const LANG: tree_sitter_language::LanguageFn;
    fn language() -> Language {
        Self::LANG.into()
    }
    fn new_parser<'code>(code: &'code [u8]) -> Option<CodeParser<'code, Self::Tagger>> {
        CodeParser::<Self::Tagger>::new(code)
    }
}

pub trait TypeTagger: std::fmt::Debug + Sized + 'static {
    type Lang: LanguageExt;

    const QUERY_ERROR: &'static str = QUERY_ERROR;
    const BRANCHES: &[Self];
    const LEAFS: &[Self];

    fn get_query_str(&self) -> std::borrow::Cow<'_, str>;
    fn get_query(&self) -> CodeQuery {
        CodeQuery::new_valid_query(&Self::Lang::language(), self.get_query_str())
    }
    fn query_error() -> CodeQuery {
        CodeQuery::new_valid_query(&Self::Lang::language(), Self::QUERY_ERROR.into())
    }
    fn tag_from_match(&self, m: &QueryMatch, code: &[u8]) -> Option<GroupTag<Self>>;
    fn parse_range_leaf(leaf: GroupTag<Self>) -> Range {
        leaf.range
    }
    fn parse_range_branch(branch: GroupTag<Self>) -> Range {
        branch.range.before_last_character()
    }
    // fn parse_range_sub_tag(parser: CodeParser<Self>, _inject: GroupTag<Self>, branch: GroupTag<Self>) -> Range {
    //     let Range(tree_sitter::Range {
    //         start_byte,
    //         end_byte,
    //         start_point,
    //         end_point,
    //     }) = branch.range;
    //     Range(tree_sitter::Range {
    //         start_byte: end_byte,
    //         end_byte,
    //         start_point: end_point,
    //         end_point,
    //     })
    // }
    fn parse_range_no_match(_inject: &GroupTag<Self>) -> Range {
        Range::MAX
    }
}

pub fn get_parser<'code, TT>(lang: Language, code: &'code [u8]) -> Option<CodeParser<'code, TT>>
where
    TT: TypeTagger,
{
    use langs::*;
    if lang == rust::Rust::language() {
        CodeParser::<rust::Tagger>::new(code)
    } else {
        return None;
    }
}
