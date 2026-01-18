use std::{
    rc::Rc,
    sync::{Arc, Mutex, RwLock},
};

use nvim_oxi::api;

use crate::{
    mistral::model::{Form, RForm},
    notify::NotifyExt as _,
    nvim::model::{Col, ColRange, Row, RowRange, get_cursor, get_text, set_text},
};

macro_rules! add_assign_isize {
    ($num_usize:expr ;+= $num:ident) => {
        if $num < 0 {
            $num_usize -= $num.abs() as usize
        } else {
            $num_usize += $num as usize
        }
    };
}

// type Row = usize;
// type Col = usize;
// type RowRange = std::ops::Range<Row>;
// type ColRange = std::ops::Range<Col>;

// pub fn get_cursor(win: &api::Window) -> Option<(Row, Col)> {
//     match win.get_cursor() {
//         Ok((row, col)) => Some((row, col)),
//         err => {
//             err.notify_error();
//             None
//         }
//     }
// }

#[derive(Clone)]
pub struct RTabs(Rc<RwLock<Tabs>>);

/// Représente soit une sélection et sa Form associée
///
/// The sele
#[derive(Eq, PartialEq, Debug, Clone)]
pub enum Tab {
    Single {
        row: Row,
        col: ColRange,
        form: RForm,
    },
    Multi {
        row: RowRange,
        col: ColRange,
        form: RForm,
        target: Option<RForm>,
    },
}

/// Liste de tabs
///
/// Chaque tabs est forcément sur une ligne différente (row.start pour le Multi)
/// Les Tab sont triées par numéro de ligne croissantes
/// Deux Tab::Multi ne peuvent pas se chevaucher, mais l'une paut être incluse dans l'autre.
#[derive(Default, Eq, PartialEq, Debug)]
pub struct Tabs(Vec<Tab>);

impl FromIterator<Tab> for Tabs {
    fn from_iter<T: IntoIterator<Item = Tab>>(iter: T) -> Self {
        Tabs(iter.into_iter().collect())
    }
}

#[allow(dead_code)]
impl Tabs {
    pub fn into_rtabs(self) -> RTabs {
        RTabs(Rc::new(RwLock::new(self)))
    }
    pub fn extend_single(&mut self, v: Vec<(impl Into<Row>, impl Into<ColRange>, RForm)>) {
        self.0.extend(v.into_iter().map(|(row, col, form)| {
            // let form = Rc::new(form);
            Tab::Single {
                row: row.into(),
                col: col.into(),
                form,
            }
        }))
    }
    #[cfg(test)]
    pub fn extend_single_test(&mut self, v: Vec<(impl Into<Row>, impl Into<ColRange>, Form)>) {
        self.0.extend(v.into_iter().map(|(row, col, form)| {
            let form = RForm::new(form);
            Tab::Single {
                row: row.into(),
                col: col.into(),
                form,
            }
        }))
    }
    #[cfg(test)]
    pub fn new_single(v: Vec<(impl Into<Row>, impl Into<ColRange>, Form)>) -> Self {
        let mut tabs = Self::default();
        tabs.extend_single_test(v);
        tabs
    }

    /// Add a Single tab
    pub fn push_single(&mut self, row: impl Into<Row>, col: impl Into<ColRange>, form: RForm) {
        self.extend_single(vec![(row, col, form)]);
    }
    #[cfg(test)]
    pub fn push_single_test(&mut self, row: impl Into<Row>, col: impl Into<ColRange>, form: Form) {
        self.extend_single_test(vec![(row, col, form)]);
    }

    /// Add a Multi tab
    pub fn push_multi(&mut self, row: impl Into<RowRange>, col: impl Into<ColRange>, form: RForm) {
        self.0.push(Tab::Multi {
            row: row.into(),
            col: col.into(),
            form,
            target: None,
        })
    }
    pub fn push_target(&mut self, new_target: RForm) {
        if let Some(tab) = self.0.last_mut() {
            match tab {
                Tab::Multi { target, .. } => *target = Some(new_target),
                _ => (),
            }
        }
    }
    #[cfg(test)]
    pub fn push_multi_test(
        &mut self,
        row: impl Into<RowRange>,
        col: impl Into<ColRange>,
        form: &Form,
        target: Option<RForm>,
    ) {
        let form = RForm::new(form.clone());
        self.0.push(Tab::Multi {
            row: row.into(),
            col: col.into(),
            form,
            target,
        })
    }

    pub fn find_tab(&self, form: &RForm) -> Option<&Tab> {
        self.find(form).map(|(_, tab)| tab)
    }
    pub fn find(&self, form: &RForm) -> Option<(usize, &Tab)> {
        self.0
            .iter()
            .enumerate()
            .find(|(_, t)| RForm::ptr_eq(t.form(), form))
    }

    pub fn find_mut(&mut self, form: &RForm) -> Option<(usize, &mut Tab)> {
        self.0
            .iter_mut()
            .enumerate()
            .find(|(_, t)| RForm::ptr_eq(t.form(), form))
    }

    pub fn filter_by_range(&self, row: &RowRange) -> Self {
        self.0
            .iter()
            .filter(|t| row.contains(&t.start_row()))
            .cloned()
            .collect()
    }

    // Remplace la `Tab` situé à l'`index` donné par une liste de `Tabs`.
    //
    // Toutes les autres `Tabs` sont décalés de `nb_rows`.
    pub fn insert(&mut self, target: &RForm, tabs: Tabs, nb_rows: usize) {
        let (index, tab) = self.find(target).unwrap();
        let row_target = tab.start_row();
        let tabs = Tabs(
            tabs.0
                .into_iter()
                .map(|tab| tab.shift(row_target.into()))
                .collect(),
        );

        let len = self.0.len();
        let after: Vec<_> = self
            .0
            .drain(index..)
            .map(|tab| tab.shift(nb_rows as isize))
            .collect();
        let before = self.0.drain(..index);
        let mut new_tabs = Vec::with_capacity(len + tabs.0.len());
        new_tabs.extend(before);
        new_tabs.extend(tabs.0);
        new_tabs.extend(after);
        self.0 = new_tabs;
    }

    /// Search for the next tab based on the current position.
    pub fn next_tab(&self, current_row: Row, current_col: Col) -> Option<Tab> {
        match self
            .0
            .iter()
            .skip_while(|tab| tab.start_row() < current_row)
            .find(|tab| tab.start_row() > current_row || tab.start_col() > current_col)
        {
            Some(tab) => Some(Tab::clone(tab)),
            None => match self.0.get(0) {
                Some(tab) => Some(Tab::clone(tab)),
                None => {
                    crate::notify::warn("Empty Tabs...");
                    None
                }
            },
        }
    }

    /// Search for the next tab based on the current position.
    pub fn previous_tab(&self, current_row: Row, current_col: Col) -> Option<Tab> {
        match self
            .0
            .iter()
            .rev()
            .skip_while(|tab| tab.start_row() > current_row)
            .find(|tab| tab.start_row() < current_row || tab.start_col() < current_col)
        {
            Some(tab) => Some(Tab::clone(tab)),
            None => match self.0.last() {
                Some(tab) => Some(Tab::clone(tab)),
                None => {
                    crate::notify::warn("Empty Tabs...");
                    None
                }
            },
        }
    }

    /// Filter Tab::Multi and keep only the ones that are not contained by another one.
    pub fn multi_tabs_first_level(&self) -> Self {
        let multi_tabs = self.multi_tabs();
        let mut previous_tab: Option<&Tab> = None;
        let mut first_level = Vec::new();
        for tab in multi_tabs.0 {
            if previous_tab.is_none() || !previous_tab.unwrap().contains_tab(&tab) {
                first_level.push(tab);
                previous_tab = Some(first_level.get(0).unwrap());
                continue;
            };
        }
        Self(first_level)
    }

    /// Filter and Clone Tab::Multi from itself
    pub fn multi_tabs(&self) -> Self {
        let it = self.0.iter().cloned();
        Self(it.filter(|t| matches!(t, Tab::Multi { .. })).collect())
    }
    /// Filter and Clone Tab::Single from itself
    pub fn single_tabs(&self) -> Self {
        let it = self.0.iter().cloned();
        Self(
            it.filter(|t| matches!(t, Tab::Single { .. }))
                .collect(),
        )
    }

    /// Recherche le Tab selectionné dans la fenêtre donnée.
    fn get_current_tab(&self) -> Option<(api::Window, Tab)> {
        let win = api::Window::current();
        let Some(cursor) = get_cursor(&win) else {
            crate::notify::error("Can't obtain cursor, so take the first tab.");
            return Some((
                win,
                self.0.get(0).cloned().unwrap_or(Tab::Single {
                    row: 0.into(),
                    col: 0.into(),
                    form: RForm::new(Form::Unit),
                }),
            ));
        };
        let tab = match self.0.iter().find(|tab| tab.contains_cursor(cursor)) {
            Some(tab) => tab.clone(),
            None => return None,
        };
        Some((win, tab))
    }
}

impl RTabs {
    pub fn read<'lock>(&'lock self) -> std::sync::RwLockReadGuard<'lock, Tabs> {
        self.0.read().unwrap()
    }
    #[allow(dead_code)]
    pub fn write<'lock>(&'lock self) -> std::sync::RwLockWriteGuard<'lock, Tabs> {
        self.0.write().unwrap()
    }

    // /// Crée une auto command on CursorMoved
    // /// Puis vérifie que le buffer courant est bien le buffer passé à la fonction
    // pub fn on_cursor_move(
    //     &self,
    //     expected_buffer: api::Buffer,
    //     parent_buffer: api::Buffer,
    //     parent_form: RForm,
    //     parent_rtabs: &Self,
    // ) -> Result<u32, api::Error> {
    //     let rtabs = Self::clone(self);
    //     let parent_rtabs = Self::clone(parent_rtabs);
    //     let prev_cursor: Arc<Mutex<(usize, usize)>> = Default::default();
    //     let opts = api::opts::ClearAutocmdsOpts::builder()
    //         .buffer(expected_buffer.clone())
    //         .build();
    //     api::clear_autocmds(&opts).unwrap();
    //     api::create_autocmd(
    //         vec!["CursorMoved"],
    //         &api::opts::CreateAutocmdOpts::builder()
    //             .buffer(expected_buffer.clone())
    //             // .callback(|_args: api::opts::CreateCommandOpts| fun_name(callback))
    //             .callback(move |_| {
    //                 Self::update_popup_parent(
    //                     &rtabs,
    //                     expected_buffer.clone(),
    //                     &mut parent_buffer.clone(),
    //                     &parent_form,
    //                     &parent_rtabs,
    //                     &prev_cursor,
    //                 )
    //             })
    //             .build(),
    //     )
    // }
    pub fn update_popup_parent(
        rtabs: &RTabs,
        expected_buffer: &api::Buffer,
        parent: &super::PopupParent,
        // parent_buffer: &mut api::Buffer,
        // parent_form: &RForm,
        // parent_rtabs: &Self,
        prev_cursor: &Arc<Mutex<(Row, Col)>>,
    ) -> bool {
        let mut buffer = api::get_current_buf();
        if buffer == *expected_buffer {
            let tabs = rtabs.read();
            let multi = tabs.multi_tabs();
            let Some((win, tab)) = multi.get_current_tab() else {
                return false;
            };
            // if the tab does not contains the prev_cursor
            // alors on supprime le Highlight et on surligne à la place les lignes du Tab
            // actuel. Puis on modifie le prev_cursor par l'actuel.
            // Sinon on supprime le highlight et on surligne les lignes du Tab actuel
            // et on modifie le prev_cursor par l'actuel.
            // On ne fait rien si le prev_cursor est le même que l'actuel.
            // Et finalement on modifie le buffer parent.
            let mut prev_cursor = prev_cursor.lock().unwrap();
            if !tab.contains_cursor(*prev_cursor) {
                let opts = Default::default();
                let buf = &mut buffer.clone();
                let lines = match get_text(buf, tab.row_range(), tab.col_range(), &opts) {
                    Ok(lines) => lines,
                    err => {
                        err.notify_error();
                        return false;
                    }
                };
                buffer.clear_namespace(*super::NS, ..).unwrap();
                for row in tab.row_range() {
                    let _ = buffer.add_highlight(*super::NS, *super::HL_SELECTION, *row, ..);
                }
                // let lines = tab.nvim_get_lines(&mut buffer, false).unwrap();
                // let lines = tab.nvim_get_text(&mut buffer, &opt).unwrap();
                let target_form = &parent.form;
                let selected_tabs = tabs.filter_by_range(&tab.row_range());
                assert!(selected_tabs.0.len() > 0);
                assert!(matches!(selected_tabs.0.get(0).unwrap(), Tab::Multi { .. }));
                // let selected_tabs = {
                //     let parent_tabs = parent_rtabs.read();
                //     let Some((_, target)) = parent_tabs.find(target_form) else {
                //         crate::notify::error("Can't select Enum. AutoCommand cancelled.");
                //         return true;
                //     };
                //     let row_range = target.row_range();
                //     let selected_tabs = tabs.filter_by_range(&row_range);
                //     selected_tabs
                // };
                let mut parent_tabs = parent.rtabs.write();
                let Some(target) = parent_tabs.find_tab(target_form) else {
                    crate::notify::warn("Can't select Tab. ");
                    return true;
                };
                let lines: Vec<_> = lines.collect();
                // crate::notify::info(format!("Get Lines from : {tab:?}"));
                // crate::notify::info(format!("Start set text : {lines:?}"));
                // crate::notify::info(format!(" at : {target:?}"));
                let buf = &mut parent.buffer.clone();
                set_text(buf, target.row_range(), target.col_range(), lines).notify_error();
                // target
                //     .nvim_set_text(&mut parent.buffer.clone(), lines.clone())
                //     .notify_error();
                // crate::notify::info(format!("START REPLACE"));
                if !Tabs::replace_tab_multi_range(&mut parent_tabs, target_form, selected_tabs) {
                    crate::notify::error(format!("Failed to replace multi range."));
                };
                // crate::notify::info("End set text");
                *prev_cursor = get_cursor(&win).unwrap();
            }
        }
        false
    }
}

#[allow(dead_code)]
impl Tab {
    // fn nvim_set_text<Lines, Line>(&self, buffer: &mut api::Buffer, lines: Lines) -> std::result::Result<(),
    // api::Error> where
    //     Lines: IntoIterator<Item = Line>,
    //     Line: Into<nvim_oxi::String>,
    // {
    //     set_text(buffer, self.row_range(), self.col_range(), lines)
    //     // let col = self.col_range().into_nvim();
    //     // buffer.set_text(self.row_range().exclusive(), *col.start, *col.end, lines)
    // }

    // fn nvim_get_text(
    //     &self,
    //     buffer: &mut api::Buffer,
    //     opts: &api::opts::GetTextOpts,
    // ) -> std::result::Result<impl api::SuperIterator<nvim_oxi::String>, api::Error> {
    //     get_text(buffer, self.row_range(), self.col_range(), opts)
    //     // let col = self.col_range().exclusive();
    //     // buffer.get_text(self.row_range().exclusive(), *col.start, *col.end, opts)
    // }

    // fn nvim_get_lines(
    //     &self,
    //     buffer: &mut api::Buffer,
    //     strict_indexing: bool,
    // ) -> std::result::Result<impl api::SuperIterator<nvim_oxi::String>, api::Error> {
    //     get_lines(buffer, self.row_range(), strict_indexing)
    //     // buffer.get_lines(self.row_range().inclusive(), strict_indexing)
    // }

    // Check if the given Tab is contained by the Tab
    fn contains_tab(&self, tab: &Tab) -> bool {
        if RForm::ptr_eq(tab.form(), self.form()) {
            return false;
        }
        self.contains_cursor((tab.start_row(), tab.start_col())) && self.contains_cursor((tab.end_row(), tab.end_col()))
    }
    // Check if the cursor is contained by the Tab
    fn contains_cursor(&self, (row, _col): (Row, Col)) -> bool {
        match self {
            Tab::Single { row: r, .. } => *r == row,
            Tab::Multi { row: r, .. } => r.contains(&row)
            // Tab::Multi { row: r, .. } => r.start() <= row && row >= r.end,
            // Tab::Single { row: r, col: c, .. } => *r == row && c.contains(&col),
            // Tab::Multi { row: r, col: c, .. } => {
            //     if r.contains(&row) {
            //         if row == r.start {
            //             c.start < col
            //         } else if row == r.end {
            //             col < c.end
            //         } else {
            //             true
            //         }
            //     } else {
            //         false
            //     }
            // }
        }
    }

    // Return the start Row
    pub fn start_row(&self) -> Row {
        match self {
            Tab::Single { row, .. } => *row,
            Tab::Multi { row, .. } => row.start,
        }
    }

    // Return the start Col
    pub fn start_col(&self) -> Col {
        match self {
            Tab::Single { col, .. } => col.start,
            Tab::Multi { col, .. } => col.start,
        }
    }

    // Return the end Row
    pub fn end_row(&self) -> Row {
        match self {
            Tab::Single { row, .. } => *row,
            Tab::Multi { row, .. } => row.end,
        }
    }

    // Return the end Col
    pub fn end_col(&self) -> Col {
        match self {
            Tab::Single { col, .. } => col.end,
            Tab::Multi { col, .. } => col.end,
        }
    }
    // Change the end Col
    pub fn set_end_col(&mut self, other: Col) -> Col {
        match self {
            Tab::Single { col, .. } => std::mem::replace(&mut col.end, other),
            Tab::Multi { col, .. } => std::mem::replace(&mut col.end, other),
        }
    }

    // Return the Form
    pub fn form(&self) -> &RForm {
        match self {
            Tab::Single { form, .. } => form,
            Tab::Multi { form, .. } => form,
        }
    }
    // Change the form
    pub fn set_form(&mut self, other: &RForm) -> RForm {
        let other = RForm::clone(other);
        match self {
            Tab::Single { form, .. } => std::mem::replace(form, other),
            Tab::Multi { form, .. } => std::mem::replace(form, other),
        }
    }

    // Return the RowRange
    pub fn row_range(&self) -> RowRange {
        match self {
            Tab::Single { row, .. } => (**row).into(),
            Tab::Multi { row, .. } => row.clone(),
        }
    }
    // Change the RowRange
    pub fn set_row_range(&mut self, other: RowRange) -> RowRange {
        match self {
            Tab::Single { row, .. } => {
                let start = std::mem::replace(row, other.start);
                (*start).into()
            }
            Tab::Multi { row, .. } => std::mem::replace(row, other),
        }
    }

    // Return the ColRange
    pub fn col_range(&self) -> ColRange {
        match self {
            Tab::Single { col, .. } => col.clone(),
            Tab::Multi { col, .. } => col.clone(),
        }
    }
    // Change the ColRange
    pub fn set_col_range(&mut self, other: ColRange) -> ColRange {
        match self {
            Tab::Single { col, .. } => std::mem::replace(col, other),
            Tab::Multi { col, .. } => std::mem::replace(col, other),
        }
    }

    // Return the ColRange
    pub fn len_rows(&self) -> usize {
        match self {
            Tab::Single { .. } => 1,
            Tab::Multi { row, .. } => row.len_abs(),
        }
    }
    pub fn set_target_from_multi_tab(&mut self, tab: &Tab) {
        match (self, tab) {
            (Tab::Multi { target, .. }, Tab::Multi { form, .. }) => *target = Some(RForm::clone(form)),
            _ => (),
        }
    }
    pub fn set_target(&mut self, new_target: RForm) {
        match self {
            Tab::Multi { target, .. } => *target = Some(new_target),
            _ => (),
        }
    }

    // Décale toutes les lignes de `nb_rows`.
    pub fn shift(mut self, nb_rows: isize) -> Self {
        match &mut self {
            Tab::Single { row, .. } => {
                add_assign_isize!(**row ;+= nb_rows);
                // if isize < 0 {
                //     *row -= nb_rows.abs() as usize
                // } else {
                //     *row += nb_rows as usize
                // }
            }
            Tab::Multi { row, .. } => {
                add_assign_isize!(*row.start ;+= nb_rows);
                add_assign_isize!(*row.end ;+= nb_rows);
            }
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mistral::model::tools::extension::FormField;

    #[test]
    fn test_insert() {
        let mut tabs = Tabs::new_single(vec![
            (0, 0..1, Form::Str),
            (1, 0..1, Form::Str),
            (2, 0..1, Form::Str),
            (3, 0..1, Form::Str),
        ]);
        let target = tabs.0.get(2).unwrap().form().clone();
        let new_tabs = Tabs::new_single(vec![(0, 0..1, Form::Str), (1, 0..1, Form::Str)]);
        tabs.insert(&target, new_tabs, 2);
        assert_eq!(
            tabs,
            Tabs::new_single(vec![
                (0, 0..1, Form::Str),
                (1, 0..1, Form::Str),
                // insersion
                (2, 0..1, Form::Str),
                (3, 0..1, Form::Str),
                //
                (4, 0..1, Form::Str),
                (5, 0..1, Form::Str),
            ])
        );
    }

    #[test]
    fn test_replace_tab_multi_range_no_change() {
        let field = FormField {
            name: "".into(),
            description: "".into(),
            form: RForm::new(Form::Str),
        };
        let multi = Form::Enum("".into(), "".into(), "".into(), vec![field.clone(), field.clone()]);

        let mut tabs = Tabs::default();
        tabs.push_single_test(0, 0..1, Form::Str);
        tabs.push_multi_test(1..4, 4..0, &multi, None);
        tabs.push_single_test(2, 2..2, Form::Str);
        tabs.push_single_test(3, 3..3, Form::Str);
        tabs.push_single_test(5, 5..5, Form::Str);

        let target = tabs.0.get(1).unwrap().form().clone();
        let mut new_tabs = Tabs::default();
        new_tabs.push_multi_test(0..3, 4..0, &multi, None);
        new_tabs.extend_single_test(vec![(1, 0..1, Form::Str), (2, 0..1, Form::Str)]);
        assert!(Tabs::replace_tab_multi_range(&mut tabs, &target, new_tabs));

        let mut expected = Tabs::default();
        expected.push_single_test(0, 0..1, Form::Str);
        expected.push_multi_test(1..4, 4..5, &multi, Some(target));
        expected.push_single_test(2, 0..1, Form::Str);
        expected.push_single_test(3, 0..1, Form::Str);
        expected.push_single_test(5, 5..5, Form::Str);

        assert_eq!(tabs.0.len(), expected.0.len());
        for (t, e) in tabs.0.iter().zip(expected.0.iter()) {
            crate::log_libuv!(Trace, "{:?}", t);
            assert_eq!(*t, *e);
        }
        assert_eq!(tabs, expected);
    }

    #[test]
    fn test_replace_tab_multi_range() {
        let field = FormField {
            name: "".into(),
            description: "".into(),
            form: RForm::new(Form::Str),
        };
        let multi = Form::Enum("".into(), "".into(), "".into(), vec![field.clone(), field.clone()]);

        let mut tabs = Tabs::default();
        tabs.push_single_test(0, 8..9, Form::Str);
        tabs.push_multi_test(1..1, 4..5, &multi, None);
        tabs.push_single_test(2, 5..6, Form::Str);

        let target = tabs.0.get(1).unwrap().form().clone();
        let mut new_tabs = Tabs::default();
        new_tabs.push_multi_test(0..3, 0..1, &multi, None);
        new_tabs.extend_single_test(vec![(1, 4..4, Form::Integer), (2, 4..6, Form::Float)]);

        assert!(Tabs::replace_tab_multi_range(&mut tabs, &target, new_tabs));

        let mut expected = Tabs::default();
        expected.push_single_test(0, 8..9, Form::Str);
        expected.push_multi_test(1..4, 4..5, &multi, Some(target));
        expected.push_single_test(2, 4..4, Form::Integer);
        expected.push_single_test(3, 4..6, Form::Float);
        expected.push_single_test(3, 5..6, Form::Str);

        assert_eq!(tabs.0.len(), expected.0.len());
        for (t, e) in tabs.0.iter().zip(expected.0.iter()) {
            crate::log_libuv!(Trace, "{:?}", t);
            assert_eq!(*t, *e);
        }
        assert_eq!(tabs, expected);
    }

    #[test]
    fn test_replace_tab_multi_range_not_first() {
        let field = FormField {
            name: "".into(),
            description: "".into(),
            form: RForm::new(Form::Str),
        };
        let multi = Form::Enum("".into(), "".into(), "".into(), vec![field.clone(), field.clone()]);

        let mut tabs = Tabs::default();
        tabs.push_single_test(0, 8..9, Form::Str);
        tabs.push_multi_test(1..1, 4..5, &multi, None);
        tabs.push_single_test(2, 5..6, Form::Str);

        let target = tabs.0.get(1).unwrap().form().clone();
        let mut new_tabs = Tabs::default();
        new_tabs.push_multi_test(2..5, 0..1, &multi, None);
        new_tabs.extend_single_test(vec![(3, 4..4, Form::Integer), (4, 4..6, Form::Float)]);

        assert!(Tabs::replace_tab_multi_range(&mut tabs, &target, new_tabs));

        let mut expected = Tabs::default();
        expected.push_single_test(0, 8..9, Form::Str);
        expected.push_multi_test(1..4, 4..5, &multi, Some(target));
        expected.push_single_test(2, 4..4, Form::Integer);
        expected.push_single_test(3, 4..6, Form::Float);
        expected.push_single_test(3, 5..6, Form::Str);

        assert_eq!(tabs.0.len(), expected.0.len());
        for (t, e) in tabs.0.iter().zip(expected.0.iter()) {
            crate::log_libuv!(Trace, "{:?}", t);
            assert_eq!(*t, *e);
        }
        assert_eq!(tabs, expected);
    }

    #[test]
    fn test_replace_tab_multi_range_empty() {
        let field = FormField {
            name: "".into(),
            description: "".into(),
            form: RForm::new(Form::Str),
        };
        let multi = Form::Enum("".into(), "".into(), "".into(), vec![field.clone(), field.clone()]);

        let mut tabs = Tabs::default();
        tabs.push_multi_test(1..1, 4..5, &multi, None);

        let target = tabs.0.get(0).unwrap().form().clone();
        let mut new_tabs = Tabs::default();
        new_tabs.push_multi_test(2..5, 0..1, &multi, None);
        new_tabs.extend_single_test(vec![(3, 4..4, Form::Integer), (4, 4..6, Form::Float)]);

        assert!(Tabs::replace_tab_multi_range(&mut tabs, &target, new_tabs));

        let mut expected = Tabs::default();
        expected.push_multi_test(1..4, 4..5, &multi, Some(target));
        expected.push_single_test(2, 4..4, Form::Integer);
        expected.push_single_test(3, 4..6, Form::Float);

        assert_eq!(tabs.0.len(), expected.0.len());
        for (t, e) in tabs.0.iter().zip(expected.0.iter()) {
            crate::log_libuv!(Trace, "{:?}", t);
            assert_eq!(*t, *e);
        }
        assert_eq!(tabs, expected);
    }
}

impl Tabs {
    /// Cherche la tab multi correspondant à la `RForm` ciblée,
    /// Puis retire tous les tabs inclus dans la range via Tab::contains,
    /// Et enfin insert les nouvelles tabs en ajustant les range de la Tab ciblée
    pub fn replace_tab_multi_range(parents: &mut Tabs, target: &RForm, children: Tabs) -> bool {
        // logs!("plop");
        // --- Get target Multi from parents ---
        let (mut index, parent_target_multi) = parents.find_mut(target).unwrap();
        if !matches!(parent_target_multi, Tab::Multi { .. }) {
            crate::notify::warn(format!("FAILED TO get parent target"));
            return false;
        }
        index += 1; // To not delete parent_target_multi
        let tab_multi_clone = parent_target_multi.clone();

        // --- Prepare children ---
        if children.0.len() == 0 {
            crate::notify::warn(format!("children empty can't get multi target"));
            return false;
        };
        let shift_children_start = -(*children.0.get(0).unwrap().start_row() as isize);
        let shift_parent_target_start = *parent_target_multi.start_row() as isize;
        // logs!("Shift children : {shift_children_start} + {shift_parent_target_start}");
        let mut children = Tabs(
            children
                .0
                .into_iter()
                .map(|tab| tab.shift(shift_children_start + shift_parent_target_start))
                .collect(),
        );

        // --- Extract Multi from children ---
        let child_multi = children.0.remove(0);
        if !matches!(child_multi, Tab::Multi { .. }) {
            crate::notify::warn(format!("Can't remove first child to get multi target"));
            return false;
        }
        let children_inner = children;

        // --- Change Target range using the new Multi child ---
        let shift_parent_after = child_multi.len_rows() as isize - parent_target_multi.len_rows() as isize;
        parent_target_multi.set_row_range(child_multi.row_range());
        parent_target_multi.set_end_col(parent_target_multi.start_col() + *child_multi.end_col());
        parent_target_multi.set_target_from_multi_tab(&child_multi);

        // --- Shift parents after target ---
        let mut parents_without_previous: Vec<_> = parents
            .0
            .drain(..)
            .filter(|t| !tab_multi_clone.contains_tab(t))
            .collect();
        let len = parents_without_previous.len();
        let after: Vec<_> = parents_without_previous
            .drain(index..)
            .map(|tab| tab.shift(shift_parent_after))
            .collect();
        let before = parents_without_previous.drain(..index);

        // --- Gather all parts ---
        let mut new_tabs = Vec::with_capacity(len + children_inner.0.len());
        new_tabs.extend(before);
        new_tabs.extend(children_inner.0);
        new_tabs.extend(after);
        parents.0 = new_tabs;
        true
    }
}
