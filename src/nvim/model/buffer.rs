use std::path::PathBuf;

use nvim_oxi::api::{self, opts::OptionOpts};

use crate::nvim::model::{Cursor, NvimBound, OneIndexed};

#[derive(Debug)]
pub struct Selection {
    pub start: Cursor,
    pub end: Cursor,
}

// #[derive(Debug, Clone)]
// pub struct Cursor {
//     pub row: usize,
//     pub column: usize,
// }

pub struct BufferData {
    pub filepath: Option<PathBuf>,
    pub filetype: String,
    pub content: Vec<String>,
    pub cursor: Cursor,
    pub modified: bool,
    pub readonly: bool,
}

impl Selection {
    /// Get the mark if the mode is visual.
    pub fn from_mark_visual(buffer: &api::Buffer) -> crate::Result<Self> {
        let mode = api::get_mode()?;
        if !mode.mode.is_visual() {
            Err("Not in visual Mode.".into())
        } else {
            let keys = api::replace_termcodes("<ESC>", true, true, true);
            api::feedkeys(&keys, c"x", true);
            Self::from_mark_last_visual(buffer)
        }
    }
    /// Whatever the mode, it will get the visual marks.
    pub fn from_mark_last_visual(buffer: &api::Buffer) -> crate::Result<Self> {
        Self::from_mark(buffer, '<', '>')
    }
    pub fn from_mark(buffer: &api::Buffer, m_start: char, m_end: char) -> crate::Result<Self> {
        let mut start = Cursor::from_mark(buffer, m_start).ok_or("Can't init cursor.")?;
        let mut end = Cursor::from_mark(buffer, m_end).ok_or("Can't init cursor.")?;
        start.row -= 1;
        end.row -= 1;
        Ok(Self { start, end })
    }
    pub fn from_command_args(args: &api::types::CommandArgs) -> Self {
        Self {
            start: Cursor::start_no_column(NvimBound::<OneIndexed>::new(args.line1)),
            end: Cursor::end_no_column(NvimBound::<OneIndexed>::new(args.line2)),
        }
    }
}

// impl From<(usize, usize)> for Cursor {
//     fn from(value: (usize, usize)) -> Self {
//         Self {
//             row: value.0,
//             column: value.1,
//         }
//     }
// }

// impl Cursor {
//     pub fn start_no_column(row: usize) -> Self {
//         Self { row, column: 0 }
//     }
//     pub fn end_no_column(row: usize) -> Self {
//         Self {
//             row,
//             column: Self::max_column(),
//         }
//     }
//     pub const fn max_column() -> usize {
//         2usize.pow(31) - 1
//     }
// }

// impl From<tree_sitter::Point> for Cursor {
//     fn from(tree_sitter::Point { row, column }: tree_sitter::Point) -> Self {
//         Self { row, column }
//     }
// }

// impl std::cmp::PartialEq for Cursor {
//     fn eq(&self, other: &Self) -> bool {
//         self.row == other.row && self.column == other.column
//     }
// }
// impl std::cmp::Eq for Cursor {}
// impl std::cmp::PartialOrd for Cursor {
//     fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
//         Some(self.cmp(other))
//     }
// }
// impl std::cmp::Ord for Cursor {
//     fn cmp(&self, other: &Self) -> std::cmp::Ordering {
//         match self.row.cmp(&other.row) {
//             std::cmp::Ordering::Equal => self.column.cmp(&other.column),
//             inf_or_sup => inf_or_sup,
//         }
//         // end.row < cursor.row || (end.row == cursor.row && end.column < cursor.column)
//     }
// }

impl BufferData {
    pub fn from_win_buffer(win: &api::Window, buffer: api::Buffer) -> crate::Result<(Self, api::Buffer)> {
        // TODO
        // let Ok((row, column)) = crate::nvim::controlleur::form::row_col::get_cursor(api::Window::current()) else {
        let Some(cursor) = Cursor::from_window(win) else {
            let msg = "Can't get window's position.";
            crate::notify::error(msg);
            return Err(msg.into());
        };
        let opt_buffer = OptionOpts::builder().buffer(buffer.clone()).build();
        let content = buffer
            .get_lines(0.., false)?
            .map(|v| v.to_string())
            .collect();
        Ok((
            Self {
                filepath: buffer.get_name().ok(),
                filetype: api::get_option_value("filetype", &opt_buffer).unwrap_or_default(),
                cursor,
                modified: api::get_option_value("modified", &opt_buffer).unwrap_or_default(),
                readonly: api::get_option_value("readonly", &opt_buffer).unwrap_or_default(),
                content,
            },
            buffer,
        ))
    }
    pub fn from_current_buffer() -> crate::Result<(Self, api::Buffer)> {
        let win = api::Window::current();
        let buffer = api::Buffer::current();
        Self::from_win_buffer(&win, buffer)
    }
}
