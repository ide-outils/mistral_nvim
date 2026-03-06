use std::{collections::HashMap, sync::LazyLock};

use nvim_oxi::api;

use crate::{
    notify::NotifyExt as _,
    nvim::model::{Col, ColRange, Cursor, Row, RowRange},
};

/// Virtual Text's inline
struct LatexLine {
    cols: ColRange,
    row: Row,
    text: String,
}

/// Virtual Text's block
struct LatexBlock {
    rows: RowRange,
    text: String,
}

enum Latex {
    Line(LatexLine),
    Block(LatexBlock),
}

impl Latex {
    // fn parse_buffer(buffer: &api::Buffer) -> Vec<Self> {
    //     let mut chars = line.chars();
    //     let mut items = Vec::new();
    //     while let Some(c) = chars.next() {
    //         match c {}
    //     }
    //     items
    // }
    fn poc_parse_all(buffer: &api::Buffer) -> Vec<Self> {
        let mut lines = match buffer.get_lines(.., false) {
            Ok(lines) => lines.enumerate(),
            err => {
                err.notify_error();
                return vec![];
            }
        };
        let mut is_in_block = false;
        let mut row_start = Row(0);
        let mut buffer_lines = Vec::new();
        let mut items = Vec::new();
        while let Some((row_index, nvim_line)) = lines.next() {
            let line = nvim_line.to_string();
            match line.as_str() {
                "\\[" if !is_in_block => {
                    is_in_block = true;
                    row_start = Row(row_index + 1);
                }
                "\\]" if is_in_block => {
                    is_in_block = false;
                    let rows = row_start..Row(row_index);
                    let content = std::mem::take(&mut buffer_lines);
                    let lb = LatexBlock::parse(rows.into(), content);
                    items.push(Latex::Block(lb));
                }
                _ if is_in_block => {
                    buffer_lines.push(line);
                }
                // Then look for inline definitions
                _ => {
                    // Take chars two by two
                    let mut chars = line.chars().enumerate();
                    let Some(first) = chars.next() else { return vec![] };
                    let mut prev = first.1;
                    // Then look for opening/closing
                    let mut is_in_line = false;
                    let mut buffer_chars = String::new();
                    let mut col_start = Col(0);
                    while let Some((col_index, c)) = chars.next() {
                        match (prev, c) {
                            ('\\', '(') if !is_in_line => {
                                is_in_line = true;
                                col_start = Col(col_index + 1);
                            }
                            ('\\', ')') if is_in_line => {
                                is_in_line = false;
                                let row = Row(row_index);
                                let cols = col_start..Col(col_index);
                                let content = std::mem::take(&mut buffer_chars);
                                let ll = LatexLine::parse(row, cols.into(), content);
                                items.push(Latex::Line(ll));
                            }
                            (_, c) if is_in_line => buffer_chars.push(c),
                            (_, _) => (),
                        }
                        prev = c;
                    }
                }
            }
        }
        items
    }
    // fn parse_line(row: Row, line: String) -> Vec<Self> {
    //     let mut chars = line.chars();
    //     let Some(mut prev) = chars.next() else { return vec![] };
    //     let mut is_in_line = false;
    //     let mut items = Vec::new();
    //     let mut buffer = String::new();
    //     while let Some(c) = chars.next() {
    //         match (prev, c) {
    //             ('\\', '(') => is_in_line = true,
    //             ('\\', ')') => {
    //                 is_in_line = false;
    //                 let ll = LatexLine::parse(buffer);
    //                 items.push(Latex::Line(ll));
    //             }
    //             (_, c) if is_in_line => buffer.push(c),
    //             (_, _) => (),
    //         }
    //         prev = c;
    //     }
    //     items
    // }
    fn contains_cursor(&self, cursor: &Cursor) -> bool {
        match self {
            Latex::Line(ll) => ll.row == cursor.row && ll.cols.contains(&cursor.col),
            Latex::Block(lb) => lb.rows.contains(&cursor.row),
        }
    }
}

impl LatexLine {
    fn parse(row: Row, cols: ColRange, content: String) -> Self {
        let text = todo!("{content}");
        Self { cols, row, text }
    }
}

impl LatexBlock {
    fn parse(rows: RowRange, content: Vec<String>) -> Self {
        let text = todo!("{content:?}");
        Self { rows, text }
    }
}

struct Token {
    row: Row,
    col: Col,
    text: String,
    unicode: UnicodeResult,
}
type Tokens = Vec<Token>;
fn parse(row: Row, col: Col, content: Vec<String>) -> Vec<Tokens> {
    content
        .into_iter()
        .enumerate()
        .map(|(i, line)| parse_one(row + i, col, line))
        .collect()
}

fn parse_one(row: Row, col: Col, content: String) -> Tokens {
    let mut tokens = Vec::with_capacity(content.len());
    let mut is_in_token = false;
    let mut is_in_function = false;
    let mut text = String::new();
    let mut token_str = String::new();
    let mut argument = String::new();
    let mut args = Vec::new();
    let mut start_col = 0;
    for (col_index, c) in content.chars().enumerate() {
        match c {
            '\\' if !is_in_token => {
                is_in_token = true;
                start_col = col_index; // not + 1, we consider \\
                text.push(c);
            }
            '{' if is_in_token => {
                is_in_token = false;
                is_in_function = true;
                text.push(c);
            }
            '}' if is_in_function => {
                is_in_function = false;
                is_in_token = false;
                text.push(c);
                args.push(std::mem::take(&mut argument));
                let args = std::mem::take(&mut args);
                let token_str = std::mem::take(&mut token_str);
                let unicode = token_to_unicode(&token_str, args);
                let text = std::mem::take(&mut text);
                let col = col + start_col;
                #[rustfmt::skip]
                tokens.push(Token { row, col, text, unicode });
            }
            ' ' if is_in_token => {
                is_in_function = false;
                is_in_token = false;
                args.push(std::mem::take(&mut argument));
                let args = std::mem::take(&mut args);
                let token_str = std::mem::take(&mut token_str);
                let unicode = token_to_unicode(&token_str, args);
                let text = std::mem::take(&mut text);
                let col = col + start_col;
                #[rustfmt::skip]
                tokens.push(Token { row, col, text, unicode });
            }
            '0'..='9' | 'A'..='Z' | 'a'..='z' if is_in_token || is_in_function => {
                if is_in_token {
                    token_str.push(c);
                }
                if is_in_function {
                    argument.push(c);
                }
                text.push(c);
            }
            c if is_in_token || is_in_function => {
                text.push(c);
            }
            _ => (),
        }
    }

    tokens
}

type TokenChar = HashMap<&'static str, char>;
static TOKENS_MATHBB: LazyLock<TokenChar> = LazyLock::new(|| {
    // For \mathbb{} commands
    #[rustfmt::skip]
    HashMap::from([
        ("N", 'ℕ'), ("Z", 'ℤ'), ("Q", 'ℚ'), ("R", 'ℝ'), ("C", 'ℂ'),
        ("A", '𝔸'), ("B", '𝔹'), ("D", '𝔻'), ("E", '𝔼'), ("F", '𝔽'),
        ("G", '𝔾'), ("H", 'ℍ'), ("I", '𝕀'), ("J", '𝕁'), ("K", '𝕂'),
        ("L", '𝕃'), ("M", '𝕄'), ("O", '𝕆'), ("P", 'ℙ'), ("S", '𝕊'),
        ("T", '𝕋'), ("U", '𝕌'), ("V", '𝕍'), ("W", '𝕎'), ("X", '𝕏'), ("Y", '𝕐')
   ])
});
static TOKENS_MATHBF: LazyLock<TokenChar> = LazyLock::new(|| {
    // For \mathbf{} commands
    #[rustfmt::skip]
    HashMap::from([
        ("A", '𝐀'), ("B", '𝐁'), ("C", '𝐂'), ("D", '𝐃'), ("E", '𝐄'), ("F", '𝐅'), ("G", '𝐆'), ("H", '𝐇'), ("I", '𝐈'), ("J", '𝐉'),
        ("K", '𝐊'), ("L", '𝐋'), ("M", '𝐌'), ("N", '𝐍'), ("O", '𝐎'), ("P", '𝐏'), ("Q", '𝐐'), ("R", '𝐑'), ("S", '𝐒'), ("T", '𝐓'),
        ("U", '𝐔'), ("V", '𝐕'), ("W", '𝐖'), ("X", '𝐗'), ("Y", '𝐘'), ("Z", '𝐙'),
        ("a", '𝐚'), ("b", '𝐛'), ("c", '𝐜'), ("d", '𝐝'), ("e", '𝐞'), ("f", '𝐟'), ("g", '𝐠'), ("h", '𝐡'), ("i", '𝐢'), ("j", '𝐣'),
        ("k", '𝐤'), ("l", '𝐥'), ("m", '𝐦'), ("n", '𝐧'), ("o", '𝐨'), ("p", '𝐩'), ("q", '𝐪'), ("r", '𝐫'), ("s", '𝐬'), ("t", '𝐭'),
        ("u", '𝐮'), ("v", '𝐯'), ("w", '𝐰'), ("x", '𝐱'), ("y", '𝐲'), ("z", '𝐳')
   ])
});

static TOKENS: LazyLock<TokenChar> = LazyLock::new(|| {
    #[rustfmt::skip]
    HashMap::from([
    // Greek letters (lowercase)
    ("alpha", 'α'), ("beta", 'β'), ("gamma", 'γ'), ("delta", 'δ'), ("epsilon", 'ε'), ("varepsilon", 'ε'),
    ("zeta", 'ζ'), ("eta", 'η'), ("theta", 'θ'), ("vartheta", 'ϑ'), ("iota", 'ι'), ("kappa", 'κ'),
    ("lambda", 'λ'), ("mu", 'μ'), ("nu", 'ν'), ("xi", 'ξ'), ("omicron", 'ο'),
    ("pi", 'π'), ("varpi", 'ϖ'), ("rho", 'ρ'), ("varrho", 'ϱ'), ("sigma", 'σ'), ("varsigma", 'ς'),
    ("tau", 'τ'), ("upsilon", 'υ'), ("phi", 'φ'), ("varphi", 'ϕ'), ("chi", 'χ'), ("psi", 'ψ'), ("omega", 'ω'),

    // Greek letters (uppercase)
    ("Alpha", 'Α'), ("Beta", 'Β'), ("Gamma", 'Γ'), ("Delta", 'Δ'), ("Epsilon", 'Ε'),
    ("Zeta", 'Ζ'), ("Eta", 'Η'), ("Theta", 'Θ'), ("Iota", 'Ι'), ("Kappa", 'Κ'),
    ("Lambda", 'Λ'), ("Mu", 'Μ'), ("Nu", 'Ν'), ("Xi", 'Ξ'), ("Omicron", 'Ο'),
    ("Pi", 'Π'), ("Rho", 'Ρ'), ("Sigma", 'Σ'), ("Tau", 'Τ'), ("Upsilon", 'Υ'),
    ("Phi", 'Φ'), ("Chi", 'Χ'), ("Psi", 'Ψ'), ("Omega", 'Ω'),

    // Mathematical operators
    ("infty", '∞'), ("sum", '∑'), ("prod", '∏'), ("coprod", '∐'),
    ("int", '∫'), ("iint", '∬'), ("iiint", '∭'), ("oint", '∮'),
    ("partial", '∂'), ("nabla", '∇'), ("sqrt", '√'),
    ("pm", '±'), ("mp", '∓'), ("times", '×'), ("div", '÷'), ("cdot", '⋅'),
    ("ast", '∗'), ("star", '⋆'), ("circ", '∘'), ("bullet", '∙'),

    // Relations
    ("leq", '≤'), ("geq", '≥'), ("neq", '≠'), ("approx", '≈'), ("simeq", '≃'),
    ("equiv", '≡'), ("sim", '∼'), ("propto", '∝'), ("parallel", '∥'), ("perp", '⊥'),
    ("ll", '≪'), ("gg", '≫'), ("asymp", '≍'), ("bowtie", '⋈'),

    // Set theory
    ("in", '∈'), ("notin", '∉'), ("ni", '∋'), ("subset", '⊂'), ("supset", '⊃'),
    ("subseteq", '⊆'), ("supseteq", '⊇'), ("nsubset", '⊄'), ("nsupset", '⊅'),
    ("cap", '∩'), ("cup", '∪'), ("uplus", '⊎'), ("sqcap", '⊓'), ("sqcup", '⊔'),
    ("vee", '∨'), ("wedge", '∧'), ("oplus", '⊕'), ("ominus", '⊖'), ("otimes", '⊗'),
    ("emptyset", '∅'), ("varnothing", '∅'),

    // Logic
    ("forall", '∀'), ("exists", '∃'), ("nexists", '∄'), ("neg", '¬'), ("lnot", '¬'),
    ("land", '∧'), ("lor", '∨'), ("implies", '⟹'), ("iff", '⟺'),
    ("therefore", '∴'), ("because", '∵'),

    // Arrows
    ("leftarrow", '←'), ("rightarrow", '→'), ("leftrightarrow", '↔'), ("mapsto", '↦'),
    ("Leftarrow", '⇐'), ("Rightarrow", '⇒'), ("Leftrightarrow", '⇔'),
    ("uparrow", '↑'), ("downarrow", '↓'), ("updownarrow", '↕'),
    ("nwarrow", '↖'), ("nearrow", '↗'), ("searrow", '↘'), ("swarrow", '↙'),
    ("to", '→'),  // Common arrow used in limits and mappings

    // Geometry
    ("angle", '∠'), ("measuredangle", '∡'), ("sphericalangle", '∢'),
    ("triangle", '△'), ("square", '□'), ("diamond", '◊'),

    // Miscellaneous
    ("hbar", 'ℏ'), ("ell", 'ℓ'), ("wp", '℘'), ("Re", 'ℜ'), ("Im", 'ℑ'),
    ("aleph", 'ℵ'), ("beth", 'ℶ'), ("gimel", 'ℷ'), ("daleth", 'ℸ'),

    // Fractions
    ("frac12", '½'), ("frac13", '⅓'), ("frac23", '⅔'), ("frac14", '¼'), ("frac34", '¾'),
    ("frac15", '⅕'), ("frac25", '⅖'), ("frac35", '⅗'), ("frac45", '⅘'),
    ("frac16", '⅙'), ("frac56", '⅚'), ("frac18", '⅛'), ("frac38", '⅜'), ("frac58", '⅝'), ("frac78", '⅞'),

    // Spaces
    ("quad", '\u{2001}') // em Quad (quadratone) ' ' 
   ])
});

type ExpectedLen = usize;

#[derive(Debug, Clone, PartialEq)]
enum UnicodeError {
    TokenNotFound,
    FunctionTokenNotFound(&'static str),
    ArgsLen(ExpectedLen, usize),
}
const MATHBB: &'static str = "mathbb";
const MATHBF: &'static str = "mathbf";
type UnicodeResult = Result<char, UnicodeError>;
fn token_to_unicode(token: &str, args: Vec<String>) -> UnicodeResult {
    use UnicodeError::*;
    match token {
        MATHBB => {
            let len = args.len();
            if len != 1 {
                Err(ArgsLen(1, len))
            } else {
                let arg = args[0].as_str();
                TOKENS_MATHBB
                    .get(arg)
                    .cloned()
                    .ok_or(FunctionTokenNotFound(MATHBB))
            }
        }
        MATHBF => {
            let len = args.len();
            if len != 1 {
                Err(ArgsLen(1, len))
            } else {
                let arg = args[0].as_str();
                TOKENS_MATHBF
                    .get(arg)
                    .cloned()
                    .ok_or(FunctionTokenNotFound(MATHBF))
            }
        }
        _ => TOKENS.get(token).cloned().ok_or(TokenNotFound),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn latex_empty() {
        let line = "Nothing to change.".to_string();
        let tokens = parse_one(Row(0), Col(0), line);
        assert_eq!(tokens.len(), 0);
    }
    #[test]
    fn latex_token_classic() {
        let line = "Thing \\quad to change.".to_string();
        let tokens = parse_one(Row(0), Col(0), line);
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].unicode, Ok('\u{2001}'));
    }
    #[test]
    fn latex_function_mathbb() {
        let line = "Thing \\mathbb{Z} to change.".to_string();
        let tokens = parse_one(Row(0), Col(0), line);
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].unicode, Ok('ℤ'));
    }
    #[test]
    fn latex_function_mathbf() {
        let line = "Thing \\mathbf{A} to change.".to_string();
        let tokens = parse_one(Row(0), Col(0), line);
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].unicode, Ok('𝐀'));
    }
    #[test]
    fn latex_three_tokens() {
        let line = "Thing \\quad \\mathbb{Z} \\mathbf{A} to change.".to_string();
        let tokens = parse_one(Row(7), Col(2), line);
        assert_eq!(tokens.len(), 3);
        let t0 = &tokens[0];
        let t1 = &tokens[1];
        let t2 = &tokens[2];
        // row
        assert_eq!(t0.row.0, 7);
        assert_eq!(t1.row.0, 7);
        assert_eq!(t2.row.0, 7);
        // col
        assert_eq!(t0.col.0, 8);
        assert_eq!(t1.col.0, 14);
        assert_eq!(t2.col.0, 25);
        // text
        assert_eq!(t0.text, "\\quad");
        assert_eq!(t1.text, "\\mathbb{Z}");
        assert_eq!(t2.text, "\\mathbf{A}");
        // unicodes
        assert_eq!(t0.unicode, Ok('\u{2001}'));
        assert_eq!(t1.unicode, Ok('ℤ'));
        assert_eq!(t2.unicode, Ok('𝐀'));
    }
}
