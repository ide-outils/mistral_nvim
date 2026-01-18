#[cfg(test)]
mod tests;

use std::sync::LazyLock;

use tree_sitter::QueryMatch;

use crate::{GroupTag, LanguageExt, Range, TypeTagger};

fn block_comment_attribute_query(subquery: &str) -> String {
    format!(r#"([(block_comment(doc_comment)) (line_comment(doc_comment)) (attribute_item)]* . {subquery})"#)
}
pub const QUERY_FONCTION_BLOCK: LazyLock<String> =
    LazyLock::new(|| block_comment_attribute_query(r#"(function_item name: (identifier) @function)"#));
pub const QUERY_STRUCTURE_BLOCK: LazyLock<String> =
    LazyLock::new(|| block_comment_attribute_query(r#"(struct_item name: (type_identifier) @structure)"#));
pub const QUERY_ENUMERATION_BLOCK: LazyLock<String> =
    LazyLock::new(|| block_comment_attribute_query(r#"(enum_item name: (type_identifier) @enumeration)"#));
pub const QUERY_TRAIT_BLOCK: LazyLock<String> =
    LazyLock::new(|| block_comment_attribute_query(r#"(trait_item name: (type_identifier) @trait)"#));

pub const QUERY_TYPE: &'static str = r#"(type_item name: (type_identifier) @type)"#;
pub const QUERY_STATIC: &'static str = r#"(static_item name: (identifier) @static)"#;
pub const QUERY_CONSTANT: &'static str = r#"(const_item name: (identifier) @constante)"#;

pub const QUERY_MOD_IMPORT_SUCCESSIVE: &'static str = r#"((mod_item !body name: (_)) @module_import)+"#;
pub const QUERY_USE_SUCCESSIVE: &'static str = r#"((use_declaration argument: (_) @use))+"#;

// pub const QUERY_DECLARATION_LIST: &'static str = r#"(_ name: (_) @branchs body: (declaration_list))"#;
pub const QUERY_DECLARATION_LIST: &'static str = r#"(_ [name: (_) type: (_)] @branchs body: (declaration_list))"#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tagger {
    // Branchs
    // ModItemBlock,
    DeclarationList,
    // Leafs
    ConstItem,
    EnumItem,
    FunctionItem,
    ModItemLeaf,
    StaticItem,
    StructItem,
    TraitItem,
    TypeItem,
    UseDeclaration,
}
use Tagger::*;

pub struct Rust;

impl LanguageExt for Rust {
    const LANG: tree_sitter_language::LanguageFn = tree_sitter_rust::LANGUAGE;
    type Tagger = Tagger;
}

impl TypeTagger for Tagger {
    type Lang = Rust;
    // Granular queries
    const BRANCHES: &[Tagger] = &[DeclarationList];
    const LEAFS: &[Tagger] = &[
        ConstItem,
        EnumItem,
        FunctionItem,
        ModItemLeaf,
        StaticItem,
        StructItem,
        TraitItem,
        TypeItem,
        UseDeclaration,
    ];

    fn get_query_str(&self) -> std::borrow::Cow<'_, str> {
        match self {
            // Branchs
            StructItem => QUERY_STRUCTURE_BLOCK.clone().into(),
            EnumItem => QUERY_ENUMERATION_BLOCK.clone().into(),
            DeclarationList => QUERY_DECLARATION_LIST.into(),
            // Leafs
            ConstItem => QUERY_CONSTANT.into(),
            FunctionItem => QUERY_FONCTION_BLOCK.clone().into(),
            StaticItem => QUERY_STATIC.into(),
            TraitItem => QUERY_TRAIT_BLOCK.clone().into(),
            TypeItem => QUERY_TYPE.into(),
            ModItemLeaf => QUERY_MOD_IMPORT_SUCCESSIVE.into(),
            UseDeclaration => QUERY_USE_SUCCESSIVE.into(),
        }
    }

    fn parse_range_leaf(leaf: GroupTag<Self>) -> Range {
        match leaf.tagger {
            ModItemLeaf => leaf.range.before_last_character(),
            UseDeclaration => leaf.range.before_last_character(),
            _ => leaf.range,
        }
    }

    fn tag_from_match(&self, m: &QueryMatch, code: &[u8]) -> Option<GroupTag<Self>> {
        let target_node = m.captures.get(0)?.node;
        let Some(parent) = target_node.parent() else {
            // Must not happen, all queries must have been setup and tested correctly
            // And anyway we have (source_file) above
            unreachable!();
        };
        let target_name = match self {
            UseDeclaration => "<use>",
            ModItemLeaf => "<mod>",
            _ => target_node.utf8_text(code).ok()?,
        };
        // We are already sure there's at least one capture's parent. So unwrap.
        let range = Option::unwrap(
            m.captures
                .iter()
                .filter_map(|c| Range(c.node.parent()?.range()).into())
                .reduce(|range, next_range| range | next_range),
        );

        // Parse ancestors to generate a tag's parts for this query
        let mut tag_parts: Vec<std::borrow::Cow<_>> = vec![target_name.into()];
        let mut current_node = if matches!(self, DeclarationList) {
            tag_parts.pop();
            target_node
        } else {
            parent
        };
        while let Some(parent) = current_node.parent() {
            let kind = parent.kind();
            if kind == "mod_item" {
                if let Some(name_node) = parent.child_by_field_name("name") {
                    let part = name_node.utf8_text(code).unwrap().into();
                    tag_parts.push(part);
                }
            } else if kind == "trait_item" {
                return None;
            } else if kind == "function_item" {
                return None;
            } else if kind == "impl_item" {
                let mut impl_parts = vec!["impl".to_string()];
                if let Some(trait_node) = parent.child_by_field_name("trait") {
                    let part = trait_node.utf8_text(code).unwrap();
                    let part = part.replace(" ", "").replace("::", "-");
                    impl_parts.push(format!("<{part}>for"));
                }
                if let Some(type_node) = parent.child_by_field_name("type") {
                    let part = type_node.utf8_text(code).unwrap();
                    let part = part.replace(" ", "").replace("::", "-");
                    impl_parts.push(format!("<{part}>"));
                }
                let part = impl_parts.join("");
                tag_parts.push(part.into());
            }
            current_node = parent;
        }

        let tag = GroupTag {
            range,
            tagger: self.clone(),
            tag: tag_parts
                .into_iter()
                .rev() // Parts are reversed cause we iterated from node to root.
                .map(|c| c.to_string())
                .collect(),
        };
        Some(tag)
    }
}
