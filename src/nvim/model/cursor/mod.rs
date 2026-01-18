mod nvim_range;
mod range_impl;
mod usize_impl;

use nvim_oxi::api::{self, opts::SetMarkOpts};
#[allow(unused_imports)]
pub use nvim_range::{
    Bounded, EndExclusive, Exclusive, FromNvimBound, FromNvimRange, Inclusive, Indexed, NvimBound, NvimRange,
    OneIndexed, StartExclusive, ZeroIndexed,
};
#[allow(unused_imports)]
pub use range_impl::{ColRange, RowRange};
pub use usize_impl::{Col, Row};

use crate::notify::{IntoNotification as _, NotifyExt as _};

impl Row {
    pub fn buf_last_row(buffer: &api::Buffer) -> crate::Result<Self> {
        line_count(buffer).map(|row: Self| row - 1)
    }
}

impl ColRange {
    pub fn from_buffer_row(buffer: &mut api::Buffer, row: Row) -> crate::Result<Self> {
        let lines = get_lines(buffer, row..=row, false)?;
        let lines = lines.collect::<Vec<_>>();
        let Some(line) = lines.get(0) else {
            return Err("No line found for ColRange::from_buffer_row.".into_error());
        };
        let length = line.to_string().len();
        Ok(Self {
            start: Col(0),
            end: Col(length),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Cursor {
    pub row: Row,
    pub col: Col,
}

impl Cursor {
    pub fn zero() -> Self {
        Self {
            row: Row(0),
            col: Col(0),
        }
    }
    pub fn buf_last_row_zero_col(buffer: &api::Buffer) -> crate::Result<Self> {
        Ok(Self {
            row: Row::buf_last_row(buffer)?,
            col: Col(0),
        })
    }
    pub fn from_window(win: &api::Window) -> Option<Self> {
        get_cursor(win).map(|(row, col)| Self { row, col })
    }
    pub fn from_window_current() -> Option<Self> {
        Self::from_window(&api::Window::current())
    }
    pub fn from_mark(buffer: &api::Buffer, mark: char) -> Option<Self> {
        get_mark(buffer, mark).map(|(row, col)| Self { row, col })
    }
    pub fn start_no_column<I: Indexed>(row: impl FromNvimBound<I>) -> Self {
        let row = row.into_from();
        Self { row, col: Col::MIN }
    }
    pub fn end_no_column<I: Indexed>(row: impl FromNvimBound<I>) -> Self {
        let row = row.into_from();
        Self { row, col: Col::MAX }
    }
    pub fn join(start: &Self, end: &Self) -> (RowRange, ColRange) {
        (
            RowRange {
                start: start.row,
                end: end.row,
            },
            ColRange {
                start: start.col,
                end: end.col,
            },
        )
    }
    pub fn join_get_text(start: &Self, end: &Self) -> (RowRange, ColRange) {
        (
            RowRange {
                start: start.row,
                end: end.row,
            },
            ColRange {
                start: start.col,
                end: end.col,
            },
        )
    }
    pub fn join_set_text(start: &Self, end: &Self) -> (RowRange, ColRange) {
        Self::join(start, end)
    }
}

impl From<tree_sitter::Point> for Cursor {
    fn from(tree_sitter::Point { row, column }: tree_sitter::Point) -> Self {
        let row = row.into();
        let col = column.into();
        Self { row, col }
    }
}

fn line_count(buffer: &api::Buffer) -> crate::Result<Row> {
    match buffer.line_count() {
        Ok(row) => Ok(FromNvimBound::<ZeroIndexed>::into_from::<Row>(row)),
        Err(error) => Err(error.into_error()),
    }
}

pub fn get_cursor(win: &api::Window) -> Option<(Row, Col)> {
    match win.get_cursor() {
        Ok((row, col)) => {
            let row = FromNvimBound::<OneIndexed>::into_from::<Row>(row);
            let col = FromNvimBound::<ZeroIndexed>::into_from::<Col>(col);
            Some((row, col))
        }
        err => {
            err.notify_error();
            None
        }
    }
}

pub fn set_cursor<ROW, COL>(win: &mut api::Window, row: ROW, col: COL) -> Option<()>
where
    ROW: Into<Row>,
    COL: Into<Col>,
{
    let row = FromNvimBound::<OneIndexed>::into_nvim(row.into()).value;
    let col = FromNvimBound::<ZeroIndexed>::into_nvim(col.into()).value;
    match win.set_cursor(row, col) {
        Ok(()) => Some(()),
        err => {
            err.notify_error();
            None
        }
    }
}

pub fn set_mark<ROW, COL>(buffer: &mut api::Buffer, mark: char, row: ROW, col: COL, opts: &SetMarkOpts) -> Option<()>
where
    ROW: Into<Row>,
    COL: Into<Col>,
{
    let row = FromNvimBound::<OneIndexed>::into_nvim(row.into()).value;
    let col = FromNvimBound::<ZeroIndexed>::into_nvim(col.into()).value;
    crate::log_libuv!(Debug, "Mark '{mark}' will set to {row} / {col}");
    match buffer.set_mark(mark, row, col, opts) {
        Ok(()) => Some(()),
        err => {
            crate::log_libuv!(Debug, "MARK HAS FAILED !");
            err.notify_error();
            None
        }
    }
}
pub fn get_mark(buffer: &api::Buffer, mark: char) -> Option<(Row, Col)> {
    match buffer.get_mark(mark) {
        Ok((row, col)) => {
            let row = FromNvimBound::<OneIndexed>::into_from::<Row>(row);
            let col = FromNvimBound::<ZeroIndexed>::into_from::<Col>(col);
            Some((row, col))
        }
        err => {
            err.notify_error();
            None
        }
    }
}
pub fn set_text<ROWS, COLS, Lines, Line>(
    buffer: &mut api::Buffer,
    rows: ROWS,
    cols: COLS,
    lines: Lines,
) -> std::result::Result<(), api::Error>
where
    Lines: IntoIterator<Item = Line>,
    Line: Into<nvim_oxi::String>,
    ROWS: Into<RowRange>,
    COLS: Into<ColRange>,
{
    let rows = FromNvimRange::<EndExclusive>::into_nvim(rows.into());
    let cols = FromNvimRange::<EndExclusive>::into_nvim(cols.into());
    buffer.set_text(rows, cols.start(), cols.end(), lines)
}

pub fn get_text<ROWS, COLS>(
    buffer: &mut api::Buffer,
    rows: ROWS,
    cols: COLS,
    opts: &api::opts::GetTextOpts,
) -> crate::Result<impl api::SuperIterator<nvim_oxi::String>>
where
    ROWS: Into<RowRange>,
    COLS: Into<ColRange>,
{
    let rows = FromNvimRange::<EndExclusive>::into_nvim(rows.into());
    let cols = FromNvimRange::<EndExclusive>::into_nvim(cols.into());
    buffer
        .get_text(rows, cols.start(), cols.end(), opts)
        .map_err(|err| format!("Failed to set_text : {err}").into_error())
}

pub fn get_lines<ROWS>(
    buffer: &api::Buffer,
    rows: ROWS,
    strict_indexing: bool,
) -> std::result::Result<impl api::SuperIterator<nvim_oxi::String>, api::Error>
where
    ROWS: Into<RowRange>,
{
    let rows = FromNvimRange::<Inclusive>::into_nvim(rows.into());
    buffer.get_lines(rows, strict_indexing)
}

pub fn set_lines<ROWS, Lines, Line>(
    buffer: &mut api::Buffer,
    rows: ROWS,
    strict_indexing: bool,
    lines: Lines,
) -> std::result::Result<(), api::Error>
where
    Lines: IntoIterator<Item = Line>,
    Line: Into<nvim_oxi::String>,
    ROWS: Into<RowRange>,
{
    let rows = FromNvimRange::<Inclusive>::into_nvim(rows.into());
    buffer.set_lines(rows, strict_indexing, lines)
}
pub fn get_line<ROW>(buffer: &api::Buffer, row: ROW, strict_indexing: bool) -> crate::Result<String>
where
    ROW: Into<Row>,
{
    let row = row.into();
    match get_lines(buffer, row..=row, strict_indexing)?.next() {
        Some(line) => Ok(line.to_string()),
        None => Err("No line found with get_line.".into_error()),
    }
}
pub fn insert_lines<ROW, Lines, Line>(
    buffer: &mut api::Buffer,
    row: ROW,
    lines_raw: Lines,
) -> std::result::Result<(), api::Error>
where
    Lines: IntoIterator<Item = Line>,
    Line: Into<nvim_oxi::String>,
    ROW: Into<Row>,
{
    let row = row.into();
    let rows = row..=row;
    let col = Col(0);
    let cols = col..=col;
    let lines = lines_raw
        .into_iter()
        .map(|s| s.into())
        .chain(std::iter::once(nvim_oxi::String::new()));
    set_text(buffer, rows, cols, lines)
}

pub fn push_lines<Lines, Line>(buffer: &mut api::Buffer, lines: Lines) -> std::result::Result<(), api::Error>
where
    Lines: IntoIterator<Item = Line>,
    Line: Into<nvim_oxi::String>,
{
    let end = Row::MAX;
    let range = end..=end;
    set_lines(buffer, range, false, lines)
}

#[cfg(not(feature = "prod_mode"))]
#[nvim_oxi::test]
#[track_caller]
fn set_text_insert() -> crate::Result<()> {
    use super::state::chat::buffer_content;
    const BUFFER_CONTENT: &'static str = "12345";
    let buffer = &mut api::Buffer::current();
    buffer.set_lines(.., false, BUFFER_CONTENT.split('\n'))?;
    set_text(buffer, 0..=0, 3..=3, ["0"])?;
    assert_eq!(buffer_content(buffer), "123045");
    Ok(())
}

#[cfg(not(feature = "prod_mode"))]
#[nvim_oxi::test]
#[track_caller]
fn set_text_replace() -> crate::Result<()> {
    use super::state::chat::buffer_content;
    const BUFFER_CONTENT: &'static str = "12345";
    let buffer = &mut api::Buffer::current();
    buffer.set_lines(.., false, BUFFER_CONTENT.split('\n'))?;
    set_text(buffer, 0..=0, 1..=3, ["00"])?;
    assert_eq!(buffer_content(buffer), "10045");
    Ok(())
}

#[cfg(not(feature = "prod_mode"))]
#[nvim_oxi::test]
#[track_caller]
fn get_text_insert() -> crate::Result<()> {
    const BUFFER_CONTENT: &'static str = "12345";
    let buffer = &mut api::Buffer::current();
    buffer.set_lines(.., false, BUFFER_CONTENT.split('\n'))?;
    let opts = Default::default();
    let mut content = get_text(buffer, 0..=0, 3..=3, &opts)?;
    assert_eq!(content.next().unwrap(), "");
    Ok(())
}

#[cfg(not(feature = "prod_mode"))]
#[nvim_oxi::test]
#[track_caller]
fn get_text_replace() -> crate::Result<()> {
    const BUFFER_CONTENT: &'static str = "12345";
    let buffer = &mut api::Buffer::current();
    buffer.set_lines(.., false, BUFFER_CONTENT.split('\n'))?;
    let opts = Default::default();
    let mut content = get_text(buffer, 0..=0, 1..=3, &opts)?;
    assert_eq!(content.next().unwrap(), "23");
    Ok(())
}

#[cfg(not(feature = "prod_mode"))]
#[nvim_oxi::test]
#[track_caller]
fn cursor_insersion() -> crate::Result<()> {
    use super::state::chat::{buffer_content, show};
    const BUFFER_CONTENT: &'static str = "L1\nL2\nL3";

    const L4: &'static str = "L4";
    const L2_3: &'static str = "L2_3";

    let buffer = &mut api::Buffer::current();

    // --- Add a line to the end ---
    buffer.set_lines(.., false, BUFFER_CONTENT.split('\n'))?;
    push_lines(buffer, [L4])?;
    assert_eq!(buffer_content(buffer), format!("L1\nL2\nL3\nL4"));

    buffer.set_lines(.., false, BUFFER_CONTENT.split('\n'))?;
    let end = Row::buf_last_row(&buffer)?;
    let range = end..=end;
    set_lines(buffer, range, false, [L4])?;
    assert_eq!(buffer_content(buffer), format!("L1\nL2\n{L4}"));

    buffer.set_lines(.., false, BUFFER_CONTENT.split('\n'))?;
    let end = Row::buf_last_row(&buffer)? + 1;
    let range = end..=end;
    set_lines(buffer, range, false, [L4])?;
    assert_eq!(buffer_content(buffer), format!("L1\nL2\nL3\nL4"));

    buffer.set_lines(.., false, BUFFER_CONTENT.split('\n'))?;
    let end = Row::MAX;
    let range = end..=end;
    set_lines(buffer, range, false, [L4])?;
    assert_eq!(buffer_content(buffer), format!("L1\nL2\nL3\nL4"));

    // --- Insert a line in the middle ---
    buffer.set_lines(.., false, BUFFER_CONTENT.split('\n'))?;
    let row = Row(2);
    insert_lines(buffer, row, [L2_3])?;
    assert_eq!(buffer_content(buffer), format!("L1\nL2\nL2_3\nL3"));

    buffer.set_lines(.., false, BUFFER_CONTENT.split('\n'))?;
    let row = Row(1);
    let range = row..=row;
    set_lines(buffer, range, false, [L2_3])?;
    assert_eq!(buffer_content(buffer), format!("L1\nL2_3\nL3"));

    buffer.set_lines(.., false, BUFFER_CONTENT.split('\n'))?;
    let row = Row(1);
    let range = row..=row;
    let target_lines: Vec<_> = get_lines(buffer, range.clone(), false)?
        .map(|v| v.to_string())
        .collect();
    let line = target_lines[0].clone();
    let lines = [line, L2_3.to_string()];
    set_lines(buffer, range, false, lines)?;
    assert_eq!(buffer_content(buffer), format!("L1\nL2\nL2_3\nL3"));

    buffer.set_lines(.., false, BUFFER_CONTENT.split('\n'))?;
    let row = Row(2);
    let rows = row..=row;
    let col = Col(0);
    let cols = col..=col;
    set_text(buffer, rows, cols, [L2_3, ""])?;
    assert_eq!(buffer_content(buffer), format!("L1\nL2\nL2_3\nL3"));

    buffer.set_lines(.., false, BUFFER_CONTENT.split('\n'))?;
    let row = Row(1);
    let rows = row..=row;
    let target_lines: Vec<_> = get_lines(buffer, rows.clone(), false)?
        .map(|v| v.to_string())
        .collect();
    let line = target_lines[0].clone();
    let col = Col(line.len());
    let cols = col..=col;
    set_text(buffer, rows, cols, ["", L2_3])?;
    assert_eq!(buffer_content(buffer), format!("L1\nL2\nL2_3\nL3"));

    show(buffer);
    Ok(())
}

// #[cfg(not(feature = "prod_mode"))]
// #[nvim_oxi::test]
// #[track_caller]
// fn undo_insertion() -> crate::Result<()> {
//     const BUFFER_CONTENT: &'static str = r###"<CHAT  role="Refactorisation" status="0;0;0" model="Tu es un développeur qui a des outils à ta disposition." id="00000000-0000-0000-0000-000000000000"/>
// <MESSAGE  role="System" model="Tiny Latest" status="Created" usage="0;0;0"/>
// Tu es un développeur qui a des outils à ta disposition.

// <MESSAGE  role="User" model="Tiny Latest" status="Initialised" usage="0;0;0" mode="CodeRefactorisation"/>
// Peux-tu modifier la fonction main dans `tests_files/main.rs` grâce aux outils, pour qu'elle affiche "Salut\n" ?
// <MESSAGE role="Assistant" model="Tiny Latest" status="Initialised" usage="0;0;0" mode="CodeRefactorisation"/>

// <TOOLCALL id="F7EJnRYyb" index="0" name="CodeRetriever">

// ```json
// {"file": "tests_files/main.rs"}
// ```
// </TOOLCALL>
// <MESSAGE role="Tool" model="Tiny Latest" status="Created" usage="0;0;0" mode="CodeRefactorisation" name="CodeRetriever" tool_call_id="F7EJnRYyb"/>
// {"Ok":"fn main() {\n    println!(\"Salut\\n\");\n}\n"}"###;

//     buffer.set_lines(.., false, BUFFER_CONTENT.split('\n'))?;
//     // '','<MESSAGE role="Assistant" model="Tiny Latest" status="Initialised" usage="0;0;0" mode="CodeRefactorisation"/>',''
//     // Code : vim.api.nvim_buf_set_text(1, 15, 54, 17, 0, {'','<MESSAGE role="Assistant" model="Tiny Latest" status="Initialised" usage="0;0;0" mode="CodeRefactorisation"/>',''})

//     Ok(())
// }
