use std::{borrow::Cow, collections::HashSet};

use nvim_oxi::{api, conversion::FromObject};
use serde::Deserialize;

use super::chat;
#[cfg(feature = "prod_mode")]
use crate::utils::notify::NotifyExt;
use crate::{
    notify::{IntoNotification, Notification},
    nvim::model::{self, Col, Cursor, Row, set_text},
    utils::logger::Level,
};

type BMResult<T> = std::result::Result<T, BMError>;

#[expect(dead_code)]
#[derive(Deserialize, Debug)]
/// More details in manual :h undotree()
struct UndotreeData {
    #[serde(rename = "seq_cur")]
    pub sequence_current: usize,
    #[serde(rename = "seq_last")]
    pub sequence_last: usize,
    #[serde(rename = "save_cur")]
    pub save_current: usize,
}

impl FromObject for UndotreeData {
    fn from_object(object: nvim_oxi::Object) -> Result<Self, nvim_oxi::conversion::Error> {
        Self::deserialize(nvim_oxi::serde::Deserializer::new(object)).map_err(Into::into)
    }
}

impl UndotreeData {
    pub fn from_buffer(buffer: &api::Buffer) -> Result<Self, api::Error> {
        let mut args = nvim_oxi::Array::new();
        let buffer_id = buffer.handle();
        args.push(buffer_id);
        api::call_function("undotree", args)
    }
}

#[derive(Debug)]
pub enum BMError {
    AlreadyCreated,
    InsertionInMiddleOfAnother,
    IdNotInitialised,
    RowOutOfBounds,
    ColOutOfBounds,
    ExpectedInMiddleOfInsertion,
    ExistsWithAnotherId(usize),
    WrongReplacementRow(usize, Row, Row),
    Other(Notification),
}
impl From<api::Error> for BMError {
    fn from(value: api::Error) -> Self {
        Self::Other(value.into_error())
    }
}
impl From<Notification> for BMError {
    fn from(value: Notification) -> Self {
        Self::Other(value)
    }
}
impl std::fmt::Display for BMError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let notif: Notification = self.into();
        write!(f, "{}", notif.message)
    }
}
impl From<BMError> for Notification {
    #[track_caller]
    fn from(value: BMError) -> Self {
        (&value).into()
    }
}
impl From<&BMError> for Notification {
    #[track_caller]
    fn from(value: &BMError) -> Self {
        let (message, mut level) = match &value {
            BMError::AlreadyCreated => ("Id already exist.", Level::Warn),
            BMError::InsertionInMiddleOfAnother => (
                "Insert in the middle of a current insertion is not supported yet.",
                Level::Error,
            ),
            BMError::IdNotInitialised => ("Not Initialized Task. Can't find Insertion Id.", Level::Warn),
            BMError::RowOutOfBounds => ("Out of bounds row.", Level::Warn),
            BMError::ColOutOfBounds => ("Out of bounds col.", Level::Warn),
            BMError::ExpectedInMiddleOfInsertion => (
                "A replacement line on the middle of an insertion, should have been preceed by an InsertionLine.",
                Level::Error,
            ),
            BMError::Other(notif) => (notif.message.as_str(), notif.level.clone()),
            _ => ("FOR EASE String MUST BE PARSED AFTER", Level::Off),
        };

        let mut message = message.to_string();
        match &value {
            BMError::ExistsWithAnotherId(id) => {
                message = format!("Modification already exists with another id. Consider using id {id} instead.");
                level = Level::Warn;
            }
            BMError::WrongReplacementRow(id, expected, pass_to_function) => {
                message = format!("Wrong row for {id} : {pass_to_function} != {expected}");
                level = Level::Error;
            }
            _ => (), // FOR EASE &str MUST BE PARSED BEFORE
        }
        Notification {
            level,
            message,
            location: std::panic::Location::caller().to_string(),
        }
    }
}
impl IntoNotification for BMError {}

#[derive(Debug)]
enum Modification {
    InsertionSuccessive(InsertionSuccessive),
    ReplacementLine(ReplacementLine),
}

impl Modification {
    fn skip_merge(&self) -> bool {
        match self {
            Modification::InsertionSuccessive(_) => false,
            Modification::ReplacementLine(rep) => rep.in_middle_of_insertion,
        }
    }
    fn insertion_successive_by_id(&mut self, id: usize) -> Option<&mut InsertionSuccessive> {
        match self {
            Modification::InsertionSuccessive(ins) if ins.id == id => Some(ins),
            _ => None,
        }
    }
    fn replacement_line_by_id(&self, id: usize) -> Option<&ReplacementLine> {
        match self {
            Modification::ReplacementLine(rep) if rep.id == id => Some(rep),
            _ => None,
        }
    }
    fn id(&self) -> usize {
        match self {
            Modification::InsertionSuccessive(ins) => ins.id,
            Modification::ReplacementLine(rep) => rep.id,
        }
    }
    fn start_initial(&self) -> Cow<'_, Cursor> {
        match self {
            Modification::InsertionSuccessive(ins) => Cow::Borrowed(&ins.start_initial),
            Modification::ReplacementLine(rep) => Cow::Owned(Cursor {
                row: rep.row_initial,
                col: Col(0),
            }),
        }
    }
    fn start_final(&self) -> Cow<'_, Cursor> {
        match self {
            Modification::InsertionSuccessive(ins) => Cow::Borrowed(&ins.start_final),
            Modification::ReplacementLine(rep) => Cow::Owned(Cursor {
                row: rep.row_final,
                col: Col(0),
            }),
        }
    }
    fn end_initial(&self) -> Cow<'_, Cursor> {
        match self {
            Modification::InsertionSuccessive(ins) => Cow::Borrowed(&ins.start_initial),
            Modification::ReplacementLine(rep) => Cow::Owned(Cursor {
                row: rep.row_initial,
                col: Col(rep.length_initial),
            }),
        }
    }
    fn end_final(&self) -> Cow<'_, Cursor> {
        match self {
            Modification::InsertionSuccessive(ins) => Cow::Borrowed(&ins.end_final),
            Modification::ReplacementLine(rep) => Cow::Owned(Cursor {
                row: rep.row_final,
                col: Col(rep.length_final),
            }),
        }
    }
    fn shift(&mut self, before_modif_final: &Cursor, diff_rows: usize, diff_cols: usize) {
        let before = before_modif_final;
        match self {
            Modification::InsertionSuccessive(ins) => {
                let start_initial = &mut ins.start_initial;
                let start = &mut ins.start_final;
                let end = &mut ins.end_final;
                if start.row == before.row {
                    start.col += diff_cols;
                    start_initial.col += diff_cols;
                }
                start.row += diff_rows;
                start_initial.row += diff_rows;

                if end.row == before.row {
                    end.col += diff_cols;
                }
                end.row += diff_rows;
            }
            Modification::ReplacementLine(rep) => {
                if !rep.in_middle_of_insertion {
                    rep.row_initial += diff_rows;
                    rep.row_final += diff_rows;
                }
            }
        }
    }
}

#[derive(Debug)]
struct InsertionSuccessive {
    pub id: usize,
    // index: usize,
    pub start_initial: Cursor,
    pub start_final: Cursor,
    pub end_final: Cursor,
}

#[derive(Debug)]
struct ReplacementLine {
    pub id: usize,
    // index: usize,
    in_middle_of_insertion: bool,
    row_initial: Row,
    row_final: Row,
    length_initial: usize,
    length_final: usize,
}

pub struct BufferModifierGroupedUndo {
    buffer: api::Buffer,
    undotree_initial: UndotreeData,
    running_insertions_ids: HashSet<usize>,
    running_replacements_ids: HashSet<usize>,
    modifications: Vec<Modification>,
}

impl BufferModifierGroupedUndo {
    pub fn new(buffer: &api::Buffer) -> crate::Result<Self> {
        let undotree_initial = UndotreeData::from_buffer(&buffer)?;
        Ok(Self {
            buffer: buffer.clone(),
            undotree_initial,
            running_insertions_ids: Default::default(),
            running_replacements_ids: Default::default(),
            modifications: Default::default(),
        })
    }
    fn parse_max_cursor(&self, cursor: &mut Cursor) -> BMResult<()> {
        let buffer = &self.buffer;
        if cursor.row == model::Row::MAX {
            let row = model::Row::buf_last_row(buffer)?;
            cursor.row = row;
            let rows = row..=row;
            let Some(line) = model::get_lines(buffer, rows, true)?.next() else {
                unreachable!("It will always returns a row even in an empty buffer.");
            };
            cursor.col = model::Col(line.len());
        } else if cursor.col == model::Col::MAX {
            let row = cursor.row;
            let rows = row..=row;
            let Some(line) = model::get_lines(buffer, rows, false)?.next() else {
                return Err(BMError::RowOutOfBounds);
            };
            cursor.col = model::Col(line.len());
        }
        Ok(())
    }
    #[track_caller]
    pub fn start_insertion_successive(&mut self, id: usize, mut cursor: model::Cursor) -> BMResult<()> {
        self.parse_max_cursor(&mut cursor)?;
        if self.running_insertions_ids.contains(&id) {
            return Err(BMError::AlreadyCreated);
        }
        let mut index = 0;
        for modif in self.modifications.iter() {
            match modif {
                Modification::InsertionSuccessive(ins) => {
                    if ins.id == id {
                        self.running_insertions_ids.insert(id);
                        return Ok(());
                    }
                }
                _ => (),
            }
            index += 1;
            // Look for where to insert to make easier the shift, by keeping insertions sorted
            let start = modif.start_final();
            // TODO: it should be possible to add an InsertionSuccessive in the middle of another
            if *start < cursor {
                // prev_modif = Some(modif);
                continue;
            }
            if cursor < *start {
                index -= 1;
                break;
            }
            // Can't start a new insertion on the middle of another one
            return Err(BMError::InsertionInMiddleOfAnother);
        }
        self.running_insertions_ids.insert(id);
        self.modifications.insert(
            index,
            Modification::InsertionSuccessive(InsertionSuccessive {
                id,
                // index,
                start_initial: cursor.clone(),
                start_final: cursor.clone(),
                end_final: cursor,
            }),
        );
        Ok(())
    }
    #[track_caller]
    pub fn start_replacement_line(&mut self, id: usize, row_initial: Row, length_initial: usize) -> BMResult<()> {
        let rep = self
            .modifications
            .iter()
            .map(|modif| modif.replacement_line_by_id(id))
            .find(|rep| rep.is_some());
        if let Some(Some(_)) = rep {
            self.running_replacements_ids.insert(id);
            return Ok(());
        }
        let mut index = 0;
        let mut in_middle_of_insertion = false;
        // Look for where to insert to make easier the shift, by keeping insertions sorted
        for modif in self.modifications.iter() {
            index += 1;
            let start = modif.start_final();
            let end = modif.end_final();
            // Replacements are allowed in the middle so we check for end.row.
            if end.row < row_initial {
                continue;
            }
            if row_initial < start.row {
                // Then we are not on the middle.
                index -= 1;
                break;
            }
            match modif {
                Modification::InsertionSuccessive(_) => {
                    in_middle_of_insertion = true;
                    // index -= 1;
                    break;
                }
                Modification::ReplacementLine(_) => {
                    let modif_id = modif.id();
                    if modif_id == id {
                        self.running_replacements_ids.insert(id);
                        return Ok(());
                    }
                    return Err(BMError::ExistsWithAnotherId(modif_id));
                }
            }
        }
        self.running_replacements_ids.insert(id);
        self.modifications.insert(
            index,
            Modification::ReplacementLine(ReplacementLine {
                id,
                // index,
                in_middle_of_insertion,
                row_final: row_initial.clone(),
                row_initial,
                length_initial,
                length_final: length_initial,
            }),
        );
        Ok(())
    }
    #[track_caller]
    pub fn insert(&mut self, id: usize, lines: Vec<String>) -> BMResult<()> {
        let nb_rows = lines.len().saturating_sub(1);
        let nb_columns = lines.last().map(|line| line.len()).unwrap_or(0);
        let Some(model::Cursor { row, col }) = self.insertion_successive(id, nb_rows, nb_columns) else {
            return Err(BMError::IdNotInitialised);
        };
        let buffer = &mut self.buffer;
        if let Err(err) = set_text(buffer, row..=row, col..=col, lines.clone()) {
            return Err(BMError::Other(
                format!("Error with set_text({row}..{row}, {col}, {col}, {lines:?}) : {}", err).into_error(),
            ));
        }
        Ok(())
    }
    pub fn replace_line(&mut self, id: usize, row: Row, line: &str) -> BMResult<(Row, usize)> {
        let mut prev_ins: Option<&mut InsertionSuccessive> = None;
        for modif in self.modifications.iter_mut() {
            if modif.id() == id {
                return match modif {
                    Modification::ReplacementLine(rep) => {
                        if row != rep.row_final {
                            return Err(BMError::WrongReplacementRow(id, rep.row_final, row));
                        }
                        let diff_nb_cols = line.len() as isize - rep.length_final as isize;
                        rep.length_final = line.len();
                        let range = rep.row_final..=rep.row_final;
                        if let Err(err) = model::cursor::set_lines(&mut self.buffer, range, true, [line]) {
                            return Err(BMError::Other(
                                format!("Error with set_lines({}, [<one_line>]) : {}", rep.row_final, err).into_error(),
                            ));
                        }
                        if rep.in_middle_of_insertion {
                            // then we need to shift cols on the same row.
                            match prev_ins {
                                Some(prev_ins) => {
                                    let end = &mut prev_ins.end_final;
                                    if rep.row_final == end.row {
                                        end.col = Col(end.col.saturating_add_signed(diff_nb_cols));
                                    }
                                }
                                None => return Err(BMError::ExpectedInMiddleOfInsertion),
                            }
                        }
                        Ok((rep.row_final, rep.length_final))
                    }
                    Modification::InsertionSuccessive(ins) => {
                        prev_ins = Some(ins);
                        continue;
                    }
                };
            }
            prev_ins = match modif {
                Modification::InsertionSuccessive(ins) => Some(ins),
                _ => prev_ins,
            };
        }
        Err(BMError::IdNotInitialised)
    }
    fn insertion_successive(&mut self, id: usize, nb_rows: usize, nb_column: usize) -> Option<model::Cursor> {
        let mut it_cursors = self.modifications.iter_mut();
        let mut target = None;
        while let Some(modif) = it_cursors.next() {
            if let Some(ins) = modif.insertion_successive_by_id(id) {
                target = Some(&mut ins.end_final);
                break;
            }
        }
        let modified = target?;
        let before = modified.clone();
        if nb_rows == 0 {
            modified.col += nb_column;
        } else {
            modified.row += nb_rows;
            *modified.col = nb_column;
        }
        let diff_cols = modified.col - before.col;
        while let Some(modif) = it_cursors.next() {
            modif.shift(&before, nb_rows, diff_cols.0);
        }
        Some(before)
    }
    pub fn ids_finished(&mut self, ids: impl IntoIterator<Item = usize>) -> bool {
        ids.into_iter()
            .map(|id| self.id_finished(&id))
            .reduce(|_, result| result)
            .unwrap_or(false)
        // for id in ids {
        //     self.running_insertions_ids.remove(&id);
        // }
        // self.running_insertions_ids.is_empty()
        // Ok(if self.running_insertions_ids.is_empty() {
        //     self.merge_undoes()?;
        //     true
        // } else {
        //     false
        // })
    }
    pub fn id_finished(&mut self, id: &usize) -> bool {
        self.running_insertions_ids.remove(id);
        self.running_replacements_ids.remove(id);
        let all_finished = self.running_insertions_ids.is_empty() && self.running_replacements_ids.is_empty();
        all_finished
    }

    #[expect(dead_code)]
    #[track_caller]
    fn merge_undoes(&mut self) -> crate::Result<()> {
        if self.modifications.is_empty() {
            return Ok(());
        }
        crate::log_libuv!(Off, "{:#?}", self.modifications);
        let buffer = &mut self.buffer.clone();
        let mut cached: Vec<(Modification, Vec<_>)> = Vec::with_capacity(self.modifications.len());
        let modifications = self.modifications.drain(..);
        for modif in modifications {
            if modif.skip_merge() {
                continue;
            }
            let start = modif.start_final();
            let end = modif.end_final();
            let (rows, cols) = Cursor::join_get_text(&start, &end);
            let opts = Default::default();
            let lines = model::get_text(buffer, rows, cols, &opts)?.collect();
            cached.push((modif, lines));
        }

        crate::log_libuv!(Trace, "BEFORE UNDO\n");
        chat::show(buffer);

        // Don't use a rendez-vous, we are here in the same thread, it would block.
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        // Remove all previous changes and discard its history.
        let undo_cmd = format!("undo! {}", self.undotree_initial.sequence_current);
        let undo_cmd_clone = undo_cmd.clone();
        self.buffer.call(move |_| {
            if let Err(_err) = api::exec2(&undo_cmd, &Default::default()) {
                tx.send(Err("Can't undo tree.".into_error())).unwrap();
            } else {
                tx.send(Ok(())).unwrap();
            }
        })?;
        // TODO: improve timeout's duration
        match rx.recv_timeout(std::time::Duration::from_secs(1)) {
            Ok(undo_res) => match undo_res {
                Ok(_) => (),
                Err(err) => {
                    crate::notify::error(format!(
                        "Error with exec2(undo!) : {err}\n<cmd> {undo_cmd_clone}\nInitial {:?}\nAcutal {:?}",
                        self.undotree_initial,
                        UndotreeData::from_buffer(buffer)
                    ));
                    return Err(err);
                }
            },
            Err(_) => return Err("Call `undo` in clean_up_undo_tree has timed out.".into_error()),
        }
        crate::log_libuv!(Trace, "AFTER UNDO\n");
        chat::show(buffer);

        // for (modif, lines) in cached.into_iter().rev() {
        for (modif, lines) in cached {
            let start = modif.start_initial();
            let end = modif.end_initial();
            let (rows, cols) = Cursor::join_set_text(&start, &end);
            #[cfg(not(feature = "prod_mode"))]
            let msg = format!(
                "PUT TARGET BACK :\n {lines:?}\n{rows:?}\n{cols:?}\n\ntargets rows :\n{:?}\n\n",
                model::get_lines(buffer, rows.clone(), false)
                    .unwrap()
                    .map(|s| {
                        let s = s.to_string();
                        (s.len(), s)
                    })
                    .collect::<Vec<_>>()
            );
            #[cfg(not(feature = "prod_mode"))]
            set_text(buffer, rows, cols, lines).expect(&msg);
            #[cfg(feature = "prod_mode")]
            set_text(buffer, rows, cols, lines).notify_error();
        }
        Ok(())
    }
}

#[cfg(not(feature = "prod_mode"))]
#[nvim_oxi::test]
#[track_caller]
fn test_buffer_modifier() -> crate::Result<()> {
    use chat::{assert_content, show};
    let buffer = &mut api::Buffer::current();
    let mut bm = BufferModifierGroupedUndo::new(buffer)?;

    const BUFFER_CONTENT: &'static str = r###"
INSERT_BEFORE
INSERT_AFTER
REPLACE_LINE
INSERT_BEFORE_AFTER
"###;
    // Initial and final rows values by id
    const I_ROW: [usize; 5] = [1, 2, 4, 4, 3];
    buffer.set_lines(.., false, BUFFER_CONTENT.split('\n'))?;
    // Force undotree generation
    api::exec2("undo", &Default::default())?;
    api::exec2("redo", &Default::default())?;
    #[rustfmt::skip]
    {
    bm.start_insertion_successive(0, Cursor {row: Row(I_ROW[0]), col: Col(0)})?;
    bm.start_insertion_successive(1, Cursor {row: Row(I_ROW[1]), col: Col(12)})?;
    bm.start_insertion_successive(2, Cursor {row: Row(I_ROW[2]), col: Col(0)})?;
    bm.start_insertion_successive(3, Cursor {row: Row(I_ROW[3]), col: Col(19)})?;
    bm.start_replacement_line(4, Row(I_ROW[4]), 12)?;

    bm.insert(0, ["0", ""].into_iter().map(ToString::to_string).collect())?;
    bm.insert(1, ["", "1"].into_iter().map(ToString::to_string).collect())?;
    bm.insert(2, ["2", ""].into_iter().map(ToString::to_string).collect())?;
    bm.insert(3, ["", "3"].into_iter().map(ToString::to_string).collect())?;
    }
    bm.replace_line(4, Row(5), "REPLACEddd_LINE")?;
    // model::set_text(buffer, row..=row, 7..=7, ["ddd"])?;
    bm.replace_line(4, Row(5), "REPLACEd_LINE")?;
    // model::set_text(buffer, row..=row, 7..=10, ["d"])?;

    const EXPECTED: &'static str = r###"
0
INSERT_BEFORE
INSERT_AFTER
1
REPLACEd_LINE
2
INSERT_BEFORE_AFTER
3
"###;
    show(buffer);
    assert_content(buffer, EXPECTED);

    bm.ids_finished([0, 1, 2, 3, 4]);
    assert_content(buffer, EXPECTED);

    Ok(())
}

#[cfg(not(feature = "prod_mode"))]
#[nvim_oxi::test]
#[track_caller]
fn test_buffer_modifier_inbetween() -> crate::Result<()> {
    use chat::{assert_content, show};
    let buffer = &mut api::Buffer::current();
    let mut bm = BufferModifierGroupedUndo::new(buffer)?;

    const BUFFER_CONTENT: &'static str = r###"
INSERT_BEFORE
INSERT_AFTER
REPLACE_LINE
INSERT_BEFORE_AFTER
"###;
    // Initial and final rows values by id
    const I_ROW: [usize; 5] = [1, 2, 4, 4, 3];
    // Start Final if inserted before
    const S_ROW: [usize; 5] = [1, 3, 6, 7, 5];
    buffer.set_lines(.., false, BUFFER_CONTENT.split('\n'))?;
    // Force undotree generation
    api::exec2("undo", &Default::default())?;
    api::exec2("redo", &Default::default())?;
    #[rustfmt::skip]
    {
    bm.start_insertion_successive(0, Cursor {row: Row(I_ROW[0]), col: Col(0)})?;
    bm.insert(0, ["0", ""].into_iter().map(ToString::to_string).collect())?;
    bm.start_insertion_successive(1, Cursor {row: Row(S_ROW[1]), col: Col(12)})?;
    bm.insert(1, ["", "1"].into_iter().map(ToString::to_string).collect())?;
    bm.start_insertion_successive(2, Cursor {row: Row(S_ROW[2]), col: Col(0)})?;
    bm.start_insertion_successive(3, Cursor {row: Row(S_ROW[2]), col: Col(19)})?;
    bm.start_replacement_line(4, Row(S_ROW[4]), 12)?;

    bm.insert(2, ["2", ""].into_iter().map(ToString::to_string).collect())?;
    bm.insert(3, ["", "3"].into_iter().map(ToString::to_string).collect())?;
    }
    bm.replace_line(4, Row(5), "REPLACEddd_LINE")?;
    bm.replace_line(4, Row(5), "REPLACEd_LINE")?;
    // let row = bm.replace_line(4, 3)?;
    // model::set_text(buffer, row..=row, 7..=7, ["ddd"])?;
    // bm.replace_line(4, -2)?;
    // model::set_text(buffer, row..=row, 7..=10, ["d"])?;

    const EXPECTED: &'static str = r###"
0
INSERT_BEFORE
INSERT_AFTER
1
REPLACEd_LINE
2
INSERT_BEFORE_AFTER
3
"###;
    show(buffer);
    assert_content(buffer, EXPECTED);

    bm.ids_finished([0, 1, 2, 3, 4]);
    assert_content(buffer, EXPECTED);

    Ok(())
}

#[cfg(not(feature = "prod_mode"))]
#[nvim_oxi::test]
#[track_caller]
fn test_buffer_modifier_followed() -> crate::Result<()> {
    use chat::{assert_content, show};
    let buffer = &mut api::Buffer::current();
    let mut bm = BufferModifierGroupedUndo::new(buffer)?;

    const BUFFER_CONTENT: &'static str = r###"
INSERT_BEFORE
INSERT_AFTER
INSERT_BEFORE_AFTER
"###;
    // Initial and final rows values by id
    const I_ROW: [usize; 5] = [1, 2, 4, 4, 3];
    // Start Final if inserted before
    const S_ROW: [usize; 5] = [1, 3, 6, 7, 5];
    buffer.set_lines(.., false, BUFFER_CONTENT.split('\n'))?;
    // Force undotree generation
    api::exec2("undo", &Default::default())?;
    api::exec2("redo", &Default::default())?;
    #[rustfmt::skip]
    {
    bm.start_insertion_successive(0, Cursor {row: Row(I_ROW[0]), col: Col(0)})?;
    bm.insert(0, ["0", ""].into_iter().map(ToString::to_string).collect())?;
    bm.start_insertion_successive(1, Cursor {row: Row(S_ROW[1]), col: Col(12)})?;
    bm.insert(1, ["", "1"].into_iter().map(ToString::to_string).collect())?;
    bm.start_insertion_successive(4, Cursor {row: Row(S_ROW[4] - 1), col: Col::MAX})?;
    bm.insert(4, ["", "REPLACEd_LINE"].into_iter().map(ToString::to_string).collect())?;
    bm.start_insertion_successive(2, Cursor {row: Row(S_ROW[2]), col: Col(0)})?;
    bm.start_insertion_successive(3, Cursor {row: Row(S_ROW[2]), col: Col(19)})?;
    // bm.start_replacement_line(4, Row(3 + 2), 12)?;

    bm.insert(2, ["2", ""].into_iter().map(ToString::to_string).collect())?;
    bm.insert(3, ["", "3"].into_iter().map(ToString::to_string).collect())?;
    }

    const EXPECTED: &'static str = r###"
0
INSERT_BEFORE
INSERT_AFTER
1
REPLACEd_LINE
2
INSERT_BEFORE_AFTER
3
"###;
    show(buffer);
    assert_content(buffer, EXPECTED);

    bm.ids_finished([0, 1, 2, 3, 4]);
    assert_content(buffer, EXPECTED);

    Ok(())
}

#[cfg(not(feature = "prod_mode"))]
#[nvim_oxi::test]
#[track_caller]
fn test_buffer_modifier_replace_in_middle_of_insertion() -> crate::Result<()> {
    use chat::{assert_content, show};
    let buffer = &mut api::Buffer::current();
    let mut bm = BufferModifierGroupedUndo::new(buffer)?;

    const BUFFER_CONTENT: &'static str = r###"
INSERT_BEFORE
INSERT_AFTER
INSERT_BEFORE_AFTER
"###;
    // Initial and final rows values by id
    const I_ROW: [usize; 5] = [1, 2, 4, 4, 3];
    // Start Final if inserted before
    const S_ROW: [usize; 5] = [1, 3, 6, 7, 5];
    buffer.set_lines(.., false, BUFFER_CONTENT.split('\n'))?;
    // Force undotree generation
    api::exec2("undo", &Default::default())?;
    api::exec2("redo", &Default::default())?;
    #[rustfmt::skip]
    {
    bm.start_insertion_successive(0, Cursor {row: Row(I_ROW[0]), col: Col(0)})?;
    bm.insert(0, ["0", ""].into_iter().map(ToString::to_string).collect())?;
    bm.start_insertion_successive(1, Cursor {row: Row(S_ROW[1]), col: Col(12)})?;
    bm.insert(1, ["", "1", "REPLACE_LINE", "2"].into_iter().map(ToString::to_string).collect())?;
    bm.start_insertion_successive(3, Cursor {row: Row(S_ROW[3]), col: Col(19)})?;
    bm.start_replacement_line(4, Row(S_ROW[4]), 12)?;
        assert!(matches!(bm.modifications[1], Modification::InsertionSuccessive(_)), "Expected InsertionSuccessive\n{:?}", bm.modifications);
        assert!(matches!(bm.modifications[2], Modification::ReplacementLine(_)), "Expected ReplacementLine\n{:?}", bm.modifications);

    bm.insert(3, ["", "3"].into_iter().map(ToString::to_string).collect())?;
    }
    bm.replace_line(4, Row(5), "REPLACEddd_LINE")?;
    bm.replace_line(4, Row(5), "REPLACEd_LINE")?;
    // let row = bm.replace_line(4, 3)?;
    // model::set_text(buffer, row..=row, 7..=7, ["ddd"])?;
    // bm.replace_line(4, -2)?;
    // model::set_text(buffer, row..=row, 7..=10, ["d"])?;

    const EXPECTED: &'static str = r###"
0
INSERT_BEFORE
INSERT_AFTER
1
REPLACEd_LINE
2
INSERT_BEFORE_AFTER
3
"###;
    show(buffer);
    assert_content(buffer, EXPECTED);

    bm.ids_finished([0, 1, 2, 3, 4]);
    assert_content(buffer, EXPECTED);

    Ok(())
}

#[cfg(not(feature = "prod_mode"))]
#[nvim_oxi::test]
#[track_caller]
fn test_buffer_modifier_replace_on_start_of_insertion() -> crate::Result<()> {
    use chat::{assert_content, show};
    let buffer = &mut api::Buffer::current();
    let mut bm = BufferModifierGroupedUndo::new(buffer)?;

    const BUFFER_CONTENT: &'static str = r###"
INSERT_BEFORE
INSERT_AFTER
INSERT_BEFORE_AFTER
"###;
    // Initial and final rows values by id
    const I_ROW: [usize; 5] = [1, 2, 4, 4, 3];
    // Start Final if inserted before
    const S_ROW: [usize; 5] = [1, 3, 6, 7, 5];
    // Start Final if inserted before and missing REPLACE line
    const M_ROW: [usize; 5] = [1, 3, 5, 7, 5];
    buffer.set_lines(.., false, BUFFER_CONTENT.split('\n'))?;
    // Force undotree generation
    api::exec2("undo", &Default::default())?;
    api::exec2("redo", &Default::default())?;
    #[rustfmt::skip]
    {
    bm.start_insertion_successive(0, Cursor {row: Row(I_ROW[0]), col: Col(0)})?;
    bm.insert(0, ["0", ""].into_iter().map(ToString::to_string).collect())?;
    bm.start_insertion_successive(1, Cursor {row: Row(S_ROW[1]), col: Col(12)})?;
    bm.insert(1, ["", "1"].into_iter().map(ToString::to_string).collect())?;
    bm.start_insertion_successive(2, Cursor {row: Row(M_ROW[2]), col: Col(0)})?;
    bm.insert(2, ["REPLACE_LINE", "2", ""].into_iter().map(ToString::to_string).collect())?;
    bm.start_insertion_successive(3, Cursor {row: Row(S_ROW[3]), col: Col::MAX})?;
    bm.start_replacement_line(4, Row(S_ROW[4]), 12)?;
        assert!(matches!(bm.modifications[2], Modification::InsertionSuccessive(_)), "Expected InsertionSuccessive\n{:?}", bm.modifications);
        assert!(matches!(bm.modifications[3], Modification::ReplacementLine(_)), "Expected ReplacementLine\n{:?}", bm.modifications);

    bm.insert(3, ["", "3"].into_iter().map(ToString::to_string).collect())?;
    }
    bm.replace_line(4, Row(5), "REPLACEddd_LINE")?;
    bm.replace_line(4, Row(5), "REPLACEd_LINE")?;
    // let row = bm.replace_line(4, 3)?;
    // model::set_text(buffer, row..=row, 7..=7, ["ddd"])?;
    // bm.replace_line(4, -2)?;
    // model::set_text(buffer, row..=row, 7..=10, ["d"])?;

    const EXPECTED: &'static str = r###"
0
INSERT_BEFORE
INSERT_AFTER
1
REPLACEd_LINE
2
INSERT_BEFORE_AFTER
3
"###;
    show(buffer);
    assert_content(buffer, EXPECTED);

    bm.ids_finished([0, 1, 2, 3, 4]);
    assert_content(buffer, EXPECTED);

    Ok(())
}

#[cfg(not(feature = "prod_mode"))]
#[nvim_oxi::test]
#[track_caller]
fn test_buffer_modifier_replace_on_end_of_insertion() -> crate::Result<()> {
    use chat::{assert_content, show};
    let buffer = &mut api::Buffer::current();
    let mut bm = BufferModifierGroupedUndo::new(buffer)?;

    const BUFFER_CONTENT: &'static str = r###"
INSERT_BEFORE
INSERT_AFTER
INSERT_BEFORE_AFTER
"###;
    // Initial and final rows values by id
    const I_ROW: [usize; 5] = [1, 2, 4, 4, 3];
    // Start Final if inserted before
    const S_ROW: [usize; 5] = [1, 3, 6, 7, 5];
    buffer.set_lines(.., false, BUFFER_CONTENT.split('\n'))?;
    // Force undotree generation
    api::exec2("undo", &Default::default())?;
    api::exec2("redo", &Default::default())?;
    #[rustfmt::skip]
    {
    bm.start_insertion_successive(0, Cursor {row: Row(I_ROW[0]), col: Col(0)})?;
    bm.insert(0, ["0", ""].into_iter().map(ToString::to_string).collect())?;
    bm.start_insertion_successive(1, Cursor {row: Row(S_ROW[1]), col: Col(12)})?;
    bm.insert(1, ["", "1", "REPLACE_LINE"].into_iter().map(ToString::to_string).collect())?;
    bm.start_insertion_successive(3, Cursor {row: Row(S_ROW[2]), col: Col(19)})?;
    bm.start_replacement_line(4, Row(S_ROW[4]), 12)?;
        assert!(matches!(bm.modifications[1], Modification::InsertionSuccessive(_)), "Expected InsertionSuccessive\n{:?}", bm.modifications);
        assert!(matches!(bm.modifications[2], Modification::ReplacementLine(_)), "Expected ReplacementLine\n{:?}", bm.modifications);

    bm.insert(3, ["", "3"].into_iter().map(ToString::to_string).collect())?;
    bm.replace_line(4, Row(5), "REPLACEddd_LINE")?;
    bm.replace_line(4, Row(5), "REPLACEd_LINE")?;
    // let row = bm.replace_line(4, 3)?;
    // model::set_text(buffer, row..=row, 7..=7, ["ddd"])?;
    // bm.replace_line(4, -2)?;
    // model::set_text(buffer, row..=row, 7..=10, ["d"])?;
    bm.insert(1, ["", "2"].into_iter().map(ToString::to_string).collect())?;
    }

    const EXPECTED: &'static str = r###"
0
INSERT_BEFORE
INSERT_AFTER
1
REPLACEd_LINE
2
INSERT_BEFORE_AFTER
3
"###;
    show(buffer);
    assert_content(buffer, EXPECTED);

    bm.ids_finished([0, 1, 2, 3, 4]);
    assert_content(buffer, EXPECTED);

    Ok(())
}

#[cfg(not(feature = "prod_mode"))]
#[nvim_oxi::test]
#[track_caller]
fn test_buffer_modifier_errors() -> crate::Result<()> {
    let buffer = &mut api::Buffer::current();
    let mut bm = BufferModifierGroupedUndo::new(buffer)?;

    const BUFFER_CONTENT: &'static str = r###"
INSERT_BEFORE
INSERT_AFTER
REPLACE_LINE
INSERT_BEFORE_AFTER
"###;
    // Initial and final rows values by id
    const I_ROW: [usize; 5] = [1, 2, 4, 4, 3];
    buffer.set_lines(.., false, BUFFER_CONTENT.split('\n'))?;
    // Force undotree generation
    api::exec2("undo", &Default::default())?;
    api::exec2("redo", &Default::default())?;
    #[rustfmt::skip]
    {
    bm.start_insertion_successive(0, Cursor {row: Row(I_ROW[0]), col: Col(0)})?;
    bm.insert(0, ["0", ""].into_iter().map(ToString::to_string).collect())?;
    let res = bm.start_insertion_successive(1, Cursor {row: Row(I_ROW[0]), col: Col(0)});
        assert!(matches!(res, Err(BMError::InsertionInMiddleOfAnother)));

    let res = bm.start_insertion_successive(0, Cursor {row: Row(I_ROW[0]), col: Col(0)});
        assert!(matches!(res, Err(BMError::AlreadyCreated)));
    bm.start_replacement_line(0, Row(0), 0)?;
    let res = bm.start_insertion_successive(0, Cursor {row: Row(I_ROW[0]), col: Col(0)});
        assert!(matches!(res, Err(BMError::AlreadyCreated)));
        bm.modifications.clear();
        bm.running_replacements_ids.clear();
        bm.running_insertions_ids.clear();

    bm.start_insertion_successive(0, Cursor {row: Row(I_ROW[0]), col: Col(0)})?;
    let res = bm.insert(12, vec![]);
        assert!(matches!(res, Err(BMError::IdNotInitialised)));
    let res = bm.replace_line(12, Row(5), "");
        assert!(matches!(res, Err(BMError::IdNotInitialised)));

    let res = bm.start_insertion_successive(1, Cursor {row: Row(12), col: Col::MAX});
        assert!(matches!(res, Err(BMError::RowOutOfBounds)));

    bm.start_replacement_line(1, Row(I_ROW[0]), 12)?;
        assert!(matches!(bm.modifications[0], Modification::InsertionSuccessive(_)), "Expected InsertionSuccessive\n{:?}", bm.modifications);
        assert!(matches!(bm.modifications[1], Modification::ReplacementLine(_)), "Expected ReplacementLine\n{:?}", bm.modifications);
    bm.modifications.remove(0);
    let res = bm.replace_line(1, Row(1), "");
        assert!(matches!(res, Err(BMError::ExpectedInMiddleOfInsertion)), "{res:?} :\n{:?}", bm.modifications);

    let res = bm.replace_line(1, Row(5), "");
        assert!(matches!(res, Err(BMError::WrongReplacementRow(1, Row(1), Row(5)))), "{res:?}");

    let res = bm.start_replacement_line(2, Row(I_ROW[0]), 12);
        assert!(matches!(res, Err(BMError::ExistsWithAnotherId(1))));
    }
    Ok(())
}
