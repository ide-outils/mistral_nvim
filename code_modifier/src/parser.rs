use tree_sitter::{Language, Parser, Query, QueryCursor, QueryMatches, StreamingIterator as _, Tree};

use crate::{GroupTag, LanguageExt, Range, TypeTagger};

pub struct CodeQuery {
    pub query: Query,
    pub cursor: QueryCursor,
}

pub struct CodeParser<'code, TT>
where
    TT: TypeTagger,
{
    pub code: &'code [u8],
    pub parser: Parser,
    pub tree: Tree,
    // pub language: Language,
    _type_tagger: std::marker::PhantomData<TT>,
}

pub type RangeInjection = Range;
pub type RangeTarget = Range;
// pub type ReversedInjectionsRanges = Vec<(RangeTarget, RangeInjection)>;
pub struct ReversedInjectionsRanges(Vec<(RangeTarget, RangeInjection)>);

impl ReversedInjectionsRanges {
    pub fn apply_injections<'code_i>(self, code: &mut Vec<u8>, injection: &'code_i [u8]) {
        fn modify_slice(code: &mut Vec<u8>, start: usize, end: usize, replacement: &[u8]) {
            if start == usize::MAX {
                code.extend_from_slice(replacement);
            } else if end == usize::MAX {
                // assert!(start <= code.len(), "Plage invalide");
                if start > code.len() {
                    crate::log!("Plage invalide");
                };
                code.splice(start.., replacement.iter().cloned());
            } else {
                // assert!(end <= code.len() && start <= end, "Plage invalide");
                let len = code.len();
                if start > end || start > len || end > len {
                    crate::log!("Plage invalide")
                }
                code.splice(start..end, replacement.iter().cloned());
            }
        }
        for (range, range_inject) in self.0 {
            let injection_code = &injection[range_inject.start_byte..range_inject.end_byte];
            crate::log!(
                "\n\nAjoute le code suivant : :\n{}",
                str::from_utf8(injection_code).unwrap()
            );
            modify_slice(code, range.start_byte, range.end_byte, injection_code);
        }
    }
}

impl CodeQuery {
    pub(crate) fn new_valid_query(lang: &Language, query: std::borrow::Cow<'_, str>) -> Self {
        CodeQuery {
            query: Query::new(lang, &query).unwrap(),
            cursor: QueryCursor::new(),
        }
    }
}

impl<'code, TT: TypeTagger> CodeParser<'code, TT> {
    pub fn new<'new_code, NewTT: TypeTagger>(code: &'new_code [u8]) -> Option<CodeParser<'new_code, NewTT>> {
        let mut parser = Parser::new();
        parser.set_language(&TT::Lang::language()).unwrap();
        let tree = parser.parse(code, None)?;
        Some(CodeParser::<NewTT> {
            code,
            parser,
            tree,
            _type_tagger: std::marker::PhantomData,
        })
    }
    pub fn code_utf8(&self) -> std::result::Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(self.code)
    }
    pub fn language(&self) -> Language {
        TT::Lang::language()
    }
    pub fn matches<'query>(
        &'query mut self,
        query: &'query mut CodeQuery,
    ) -> QueryMatches<'query, 'query, &'code [u8], &'code [u8]> {
        query
            .cursor
            .matches(&query.query, self.tree.root_node(), self.code)
    }

    pub fn list_tags(&mut self, tagger: &TT) -> Option<Vec<GroupTag<TT>>> {
        let code = self.code;
        let query = &mut tagger.get_query();
        let mut matches = self.matches(query);
        let mut tags = Vec::new();
        while let Some(m) = matches.next() {
            if let Some(valid_tag) = tagger.tag_from_match(m, code) {
                tags.push(valid_tag);
            }
        }
        Some(tags)
    }
    pub fn list_leafs_tags(&mut self) -> Vec<GroupTag<TT>> {
        TT::LEAFS
            .iter()
            .filter_map(|tagger| self.list_tags(tagger))
            .flatten()
            .collect()
    }
    pub fn list_branches_tags(&mut self) -> Vec<GroupTag<TT>> {
        TT::BRANCHES
            .iter()
            .filter_map(|tagger| self.list_tags(tagger))
            .flatten()
            .collect()
    }
    pub fn list_all_tags(&mut self) -> Vec<GroupTag<TT>> {
        TT::LEAFS
            .iter()
            .chain(TT::BRANCHES)
            .filter_map(|tagger| self.list_tags(tagger))
            .flatten()
            .collect()
    }

    pub fn find_tag(&mut self, target: &GroupTag<TT>) -> Option<GroupTag<TT>> {
        let tagger = &target.tagger;
        let code = self.code;
        let query = &mut tagger.get_query();
        let mut matches = self.matches(query);
        while let Some(m) = matches.next() {
            if let Some(group_tag) = tagger.tag_from_match(m, code) {
                if group_tag.tag == target.tag {
                    return Some(group_tag);
                }
            }
        }
        None
    }
    pub fn range_where_to_inject(&mut self, inject_tag: &GroupTag<TT>) -> Range {
        if let Some(leaf) = self.find_tag(&inject_tag) {
            return TT::parse_range_leaf(leaf);
        }
        // Then look for sub tags.
        for branch in self.list_branches_tags() {
            if inject_tag.tag.starts_with(branch.tag.as_slice()) {
                // Inject into the branch.
                TT::parse_range_branch(branch);
            }
        }
        // Nothing found then inject to then end of the code.
        TT::parse_range_no_match(&inject_tag)
    }

    #[cfg(test)]
    pub fn test_inject<'code_i>(&mut self, injection: &'code_i [u8]) -> Option<Vec<u8>> {
        let mut modification: Vec<u8> = self.code.to_vec();
        let injections = self.inject(injection)?;
        injections.apply_injections(&mut modification, injection);
        Some(modification)
    }
    pub fn inject<'code_i>(&mut self, inject: &'code_i [u8]) -> Option<ReversedInjectionsRanges> {
        let mut inject_parser = Self::new::<TT>(inject)?;
        let mut modifications: Vec<_> = inject_parser
            .list_leafs_tags()
            .into_iter()
            .map(|tag| (self.range_where_to_inject(&tag), tag.range))
            // .map(|tag| (self.(&tag), tag.range))
            .collect();
        //
        modifications.sort_by(|a, b| b.0.start_byte.cmp(&a.0.start_byte));
        Some(ReversedInjectionsRanges(modifications))
    }
}
