use crate::TypeTagger;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Range(pub tree_sitter::Range);
impl Range {
    pub const MAX: Self = Self(tree_sitter::Range {
        start_byte: usize::MAX,
        end_byte: usize::MAX,
        start_point: tree_sitter::Point {
            row: usize::MAX,
            column: 0,
        },
        end_point: tree_sitter::Point {
            row: usize::MAX,
            column: 0,
        },
    });

    pub fn before_last_character(mut self) -> Self {
        self.0.end_byte -= 1;
        self.0.start_byte = self.end_byte;
        // self.0.end_point -= 1;
        self.0.start_point = self.end_point;
        self
    }
}
impl std::ops::BitOr for Range {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(tree_sitter::Range {
            start_byte: std::cmp::min(self.start_byte, rhs.start_byte),
            end_byte: std::cmp::max(self.end_byte, rhs.end_byte),
            start_point: std::cmp::min(self.start_point, rhs.start_point),
            end_point: std::cmp::max(self.end_point, rhs.end_point),
        })
    }
}
impl std::ops::Deref for Range {
    type Target = tree_sitter::Range;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl From<tree_sitter::Range> for Range {
    fn from(value: tree_sitter::Range) -> Self {
        Self(value)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct GroupTag<TT>
where
    TT: TypeTagger,
{
    pub tag: Vec<String>,
    pub range: Range,
    pub tagger: TT,
}

impl<TT: TypeTagger> std::fmt::Debug for GroupTag<TT> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("GroupTag").field(&self.tag()).finish()
    }
}
impl<TT: TypeTagger> std::ops::Add<&str> for GroupTag<TT> {
    type Output = Self;

    fn add(mut self, rhs: &str) -> Self::Output {
        if rhs != "" {
            self.tag.push(rhs.to_string());
        }
        self
    }
}
impl<TT: TypeTagger> std::ops::AddAssign<&str> for GroupTag<TT> {
    fn add_assign(&mut self, rhs: &str) {
        self.tag.push(rhs.to_string())
    }
}
impl<TT: TypeTagger> PartialEq<&str> for GroupTag<TT> {
    fn eq(&self, other: &&str) -> bool {
        self.tag() == *other
    }
}

impl<TT: TypeTagger> GroupTag<TT> {
    fn tag(&self) -> String {
        self.tag.join("::")
    }
}
