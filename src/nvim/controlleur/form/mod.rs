use std::sync::LazyLock;

use nvim_oxi::api::{self, types};

use crate::{
    mistral::model::{Form, FormExt, RForm},
    nvim::{
        controlleur::form::tabs::Tab,
        model::{Col, ColRange, Row, RowRange, SharedState, get_cursor, set_cursor},
    },
    utils::set_option,
};

mod tabs;

use tabs::{RTabs, Tabs};

static NS: LazyLock<u32> = LazyLock::new(|| api::create_namespace("mistral_form"));
static HL_SELECTION: LazyLock<&str> = LazyLock::new(|| {
    let hl = "FormSelection";
    // let opts = api::opts::SetHighlightOpts::builder()
    //     // .ctermbg("blue")
    //     .background("blue")
    //     .build();
    // api::set_hl(*NS, hl, &opts);
    api::command(&format!("highlight {hl} ctermfg=blue guifg=blue")).unwrap();
    hl
});
static HL_ERROR: LazyLock<&str> = LazyLock::new(|| {
    let hl = "FormRonError";
    api::command(&format!("highlight {hl} ctermbg=red guibg=red")).unwrap();
    hl
});
// const COL_MAX: usize = crate::nvim::model::Cursor::max_column();
const INDENT: usize = 4;

fn fill_form_last(form: RForm, content: &str, rows: &mut Vec<String>, tabs: &mut Tabs) {
    fill_form_last_multi_or_single(form, content, rows, tabs, false)
}
fn fill_form_last_multi(form: RForm, content: &str, rows: &mut Vec<String>, tabs: &mut Tabs) {
    fill_form_last_multi_or_single(form, content, rows, tabs, true)
}
fn fill_form_last_multi_or_single(form: RForm, content: &str, rows: &mut Vec<String>, tabs: &mut Tabs, multi: bool) {
    let len = content.len();
    let nb_rows = rows.len();
    let (row_index, col_range) = if let Some(row) = rows.last_mut() {
        let comma = if row == "" && nb_rows == 1 { "" } else { "," };
        let c1 = row.len();
        let c2 = c1 + len;
        row.push_str(&format!("{content}{comma} // {form}"));
        let row_index = rows.len() - 1;
        (row_index, c1..=c2)
    } else {
        let c1 = 0;
        let c2 = len;
        rows.push(format!("{content} // {form}"));
        let row_index = 0;
        (row_index, c1..=c2)
    };
    if multi {
        tabs.push_multi(row_index, col_range, form);
    } else {
        tabs.push_single(row_index, col_range, form);
    }
}
fn escape_snake_case(name: &mut String) {
    if name.contains("-") {
        *name = format!("r#{name}");
    }
}

fn fill_form(form: RForm, rows: &mut Vec<String>, tabs: &mut Tabs, indent: usize) {
    let spaces = " ".repeat(INDENT * indent);
    use Form::*;
    match &*RForm::clone(&form) {
        // --- Named types ---
        Struct(name, description, fields) | StructTuple(name, description, fields) => {
            let push = if **description == "" {
                format!("{name} (")
            } else {
                format!("{name} ( // {description}")
            };
            if let Some(row) = rows.last_mut() {
                row.push_str(&push);
            } else {
                rows.push(format!("{spaces}{push}"));
            }
            let shifted_indent = indent + 1;
            let shifted_spaces = " ".repeat(INDENT * shifted_indent);
            for field in fields {
                let (name, description, form) = field.tuple_ref();
                if description != "" {
                    rows.push(format!("{shifted_spaces}// {description}"));
                }
                if name == "" {
                    rows.push(format!("{shifted_spaces}"));
                } else {
                    rows.push(format!("{shifted_spaces}{name}: "));
                }
                fill_form(RForm::clone(form), rows, tabs, shifted_indent);
            }
            if indent == 0 {
                rows.push(format!("{spaces})"));
            } else {
                rows.push(format!("{spaces}),"));
            }
        }
        Enum(_name_enum, _description, default_variant, fields) => {
            if rows.len() == 0 {
                for field in fields {
                    let (name, description, field_form) = field.tuple_ref();
                    let mut name = name.clone();
                    if **default_variant == *name {
                        rows.push(format!("// #[default]"));
                    }
                    escape_snake_case(&mut name);
                    if description != "" {
                        rows.push(format!("// {description}"));
                    }
                    let row = rows.len(); // we want the next item
                    match &**field_form {
                        Form::Unit => {
                            rows.push(format!("{name}"));
                            let col_range = 0..=name.len();
                            tabs.push_multi(row, col_range, RForm::clone(field_form));
                        }
                        _ => {
                            rows.push(format!("{name}("));
                            fill_form(RForm::clone(&field_form), rows, tabs, 1);
                            rows.push(format!(")"));
                            let row_range = row..=rows.len() - 1;
                            let col_range = 0..=1;
                            tabs.push_multi(row_range, col_range, RForm::clone(field_form));
                        }
                    }
                }
            } else {
                let mut default = default_variant.to_string();
                escape_snake_case(&mut default);
                if default != ""
                    && let Some(variant) = fields.iter().find(|v| *v.name == **default_variant)
                {
                    match &*variant.form {
                        Unit => {
                            fill_form_last_multi(form, default.as_str(), rows, tabs);
                            tabs.push_target(RForm::clone(&variant.form));
                            // fill_form_last_multi(RForm::clone(&variant.form), default.as_str(), rows, tabs);
                        }
                        _ => {
                            fill_form(form, rows, tabs, indent + 1);
                            // fill_form(RForm::clone(&variant.form), rows, tabs, indent + 1);
                        }
                    }
                } else {
                    fill_form_last_multi(form, "()", rows, tabs);
                }
            }
        }

        // --- Unnamed types ---
        Tuple(fields) => {
            let fields = fields
                .into_iter()
                .map(|form| ("", "", RForm::clone(&form)).into())
                .collect();
            let fake_form = RForm::new(StructTuple("".into(), "".into(), fields));
            fill_form(fake_form, rows, tabs, indent);
        }
        Map(_boxed) => {
            // let shifted_indent = indent + 1;
            // let shifted_spaces = " ".repeat(INDENT * shifted_indent);

            // let opening = "{ // Map";
            // if let Some(row) = rows.last_mut() {
            //     row.push_str(opening)
            // } else {
            //     rows.push(opening.to_string());
            // }
            // let _ = rows.pop(); // remove added Tab, we only want a multi.
            // rows.push(format!("{shifted_spaces}"));

            // // fill_form_last_multi(Map(boxed), "{}", rows, tabs);
            // // let (key, value) = *boxed;
            // // crate::notify::warn(format!("Not implemented : Map<{key} {value}>"));
        }
        List(_boxed) => {
            // fill_form_last_multi(Map(boxed), "{}", rows, tabs);
        }
        Option(boxed) => {
            let inner = RForm::clone(&*boxed);
            if let Some(row) = rows.last_mut() {
                row.push_str("None, // Option")
            } else {
                rows.push("None".to_string());
                fill_form(inner, rows, tabs, indent + 1);
            }
        }

        // --- Base types ---
        Str => {
            fill_form_last(form, "\"\"", rows, tabs);
        }
        Integer => {
            fill_form_last(form, "0", rows, tabs);
        }
        Float => {
            fill_form_last(form, "0.0", rows, tabs);
        }
        Boolean => {
            fill_form_last(form, "false", rows, tabs);
        }
        Unit => {
            fill_form_last(form, "()", rows, tabs);
        }
    }
}

trait AxelleRed {
    fn taille_sensuelle(self) -> Self;
}

impl AxelleRed for &mut types::WindowConfigBuilder {
    fn taille_sensuelle(self) -> Self {
        let win = api::Window::current();
        let largeur = win.get_width().unwrap_or(69) as f64;
        let longueur = win.get_height().unwrap_or(42) as f64;
        const SENSUALITÉ: f64 = 69. / 42.69;
        const DÉSIR_OU_AMOUR: f64 = SENSUALITÉ * (SENSUALITÉ + SENSUALITÉ);
        self.width((largeur / SENSUALITÉ) as u32)
            .height((longueur / SENSUALITÉ) as u32)
            .col((largeur / DÉSIR_OU_AMOUR) as u32)
            .row((longueur / DÉSIR_OU_AMOUR) as u32)
            .style(api::types::WindowStyle::Minimal)
            .border(api::types::WindowBorder::Rounded)
    }
}

/// Crée un formulaire interactif dans Neovim
pub fn formulaire<T, F>(state: &SharedState, callback: F)
where
    T: FormExt,
    F: FnMut(T, SharedState) + 'static,
{
    let win_config = types::WindowConfig::builder()
        .relative(types::WindowRelativeTo::Editor)
        .taille_sensuelle()
        .build();
    let buffer = &mut api::create_buf(false, true).unwrap();
    let _win = api::open_win(buffer, true, &win_config).unwrap();
    let form = T::get_form();
    let fake_row = match &*form {
        Form::Struct(_, _, _) | Form::StructTuple(_, _, _) => {
            // let rtabs = setup_buffer_form(buffer, RForm::clone(&form)).into_rtabs();
            // setup(buffer, &rtabs, state, callback);
            false
        }
        _ => {
            true
            // let mut rows = vec!["".into()];
            // let mut tabs = Default::default();
            // fill_form(RForm::clone(&form), &mut rows, &mut tabs, 0);
            // buffer.set_lines(.., false, rows).unwrap();
            // set_option(&buffer, "filetype", "ron");
            // let rtabs = tabs.into_rtabs();
            // setup(buffer, &rtabs, state, callback);
            // form_next_field(&rtabs);
        }
    };
    let rtabs = setup_buffer_form(buffer, RForm::clone(&form), fake_row).into_rtabs();
    setup(buffer, &rtabs, state, callback);
    if fake_row {
        form_next_field(&rtabs);
    }
}
fn setup_buffer_form(buffer: &mut api::Buffer, form: RForm, fake_row: bool) -> Tabs {
    let mut rows = if fake_row { vec!["".into()] } else { Vec::new() };
    let mut tabs = Default::default();
    fill_form(form, &mut rows, &mut tabs, 0);
    buffer.set_lines(.., false, rows).unwrap();
    set_option(&buffer, "filetype", "ron");
    tabs
}

/// Configure les bindings pour Tab/CR/Esc
fn setup<T, F>(buffer: &mut api::Buffer, rtabs: &RTabs, state: &SharedState, mut callback: F)
where
    T: FormExt,
    F: FnMut(T, SharedState) + 'static,
{
    let opts = api::opts::BufAttachOpts::builder()
        .on_lines(move |args: api::opts::OnLinesArgs| {
            #[allow(unused_variables)]
            let (_, mut buffer, tick, start, end, end_updated, prev_len, _, _) = args;
            // Clear Errors' highlight
            buffer.clear_namespace(*NS, args.3..args.5).unwrap();
            crate::Result::Ok(false)
        })
        .build();
    if let Err(err) = buffer.attach(false, &opts) {
        crate::notify::error(err);
    };

    let mut modes = crate::utils::ShortcutBuilder::new(buffer.clone());
    use types::Mode::*;
    crate::set_keymaps! {
        modes (Normal, Insert):
        "<Tab>" => {form_next_field(&rtabs)} <= <rtabs: RTabs>
        "<S-Tab>" => {form_prev_field(&rtabs)} <= <rtabs: RTabs>
    }
    crate::set_keymaps! {
        modes (Normal) :
        "<Esc>" => {form_abort()} <= <>
        "<CR>" => {form_submit(&state, &mut callback)} <= <state: SharedState>
        "n" => {form_next_field(&rtabs)} <= <rtabs: RTabs>
        "N" => {form_prev_field(&rtabs)} <= <rtabs: RTabs>
    }
}

// fn bind_escape(buffer: &mut api::Buffer) {
//     let mut shortcut = ShortcutBuilder::new(buffer);
//     shortcut.normal().insert().visual().visual_select();
//     shortcut.callback(form_abort).set_keymap("<Esc>");
//     // Binding pour <Esc> (annulation)
//     let opts = opts::SetKeymapOpts::builder()
//         .nowait(true)
//         .noremap(true)
//         .silent(true)
//         .callback(form_abort)
//         .build();
//     buffer
//         .set_keymap(types::Mode::Normal, "<Esc>", "", &opts)
//         .unwrap();
// }

// fn bind_escape(buffer: &mut api::Buffer) {
//     // Binding pour <Esc> (annulation)
//     let opts = opts::SetKeymapOpts::builder()
//         .nowait(true)
//         .noremap(true)
//         .silent(true)
//         .callback(form_abort)
//         .build();
//     buffer
//         .set_keymap(types::Mode::Normal, "<Esc>", "", &opts)
//         .unwrap();
// }

/// Configure les bindings pour Tab/CR/Esc
// fn setup_popup<T, F>(buffer: &mut api::Buffer, r: &RTabs, s: &SharedState, mut callback: F)
// where
//     T: FormExt,
//     F: FnMut(T, SharedState) + 'static,
// {
//     let opts = api::opts::BufAttachOpts::builder()
//         .on_changedtick(move |args: api::opts::OnLinesArgs| {
//             #[allow(unused_variables)]
//             let (_, mut buffer, tick, start, end, end_updated, prev_len, _, _) = args;
//             // Clear Errors' highlight
//             buffer.clear_namespace(ns, args.3..args.5).unwrap();
//             crate::Result::Ok(false)
//         })
//         .build();

//     api::create_autocmd(
//         vec!["CursorMoved"],
//         &api::opts::CreateAutocmdOpts::builder()
//             .buffer(buffer.clone())
//             // .callback(|_args: api::opts::CreateCommandOpts| fun_name(callback))
//             .callback(move |_| {
//                 Self::update_popup_parent(
//                     &rtabs,
//                     expected_buffer.clone(),
//                     ns,
//                     &mut parent_buffer.clone(),
//                     &parent_form,
//                     &parent_rtabs,
//                     &prev_cursor,
//                 )
//             })
//             .build(),
//     );

//     let mut modes = ShortcutBuilder::new(buffer);
//     use types::Mode::*;
//     // --- ALl ---
//     set_keymaps! {
//         modes (Normal, Insert, Visual, VisualSelect):
//         "<Tab>" => form_next_field(&rtabs, ns, &state) <= <rtabs: RTabs, state: SharedState>
//         "<S-Tab>" => form_prev_field(&rtabs, ns, &state) <= <rtabs: RTabs, state: SharedState>
//     }
//     set_keymaps! {
//         modes (Normal) :
//         "ESC" => form_abort(()) <= <>
//         "<CR>" => form_submit(&state, ns, &mut callback) <= <state: SharedState>
//         "n" => form_next_field(&rtabs, ns, &state) <= <rtabs: RTabs, state: SharedState>
//         "N" => form_prev_field(&rtabs, ns, &state) <= <rtabs: RTabs, state: SharedState>
//     }
// }

#[derive(Clone)]
struct PopupParent {
    buffer: api::Buffer,
    form: RForm,
    rtabs: RTabs,
}
#[derive(Clone)]
struct Popup {
    buffer: api::Buffer,
    parent: PopupParent,
    prev_cursor: std::sync::Arc<std::sync::Mutex<(Row, Col)>>,
    rtabs: RTabs,
}

impl Popup {
    fn new(row: RowRange, col: ColRange, parent: PopupParent) -> Self {
        // Création du buffer pour la pop-up
        let mut buffer = api::create_buf(false, true).unwrap().clone();
        let prev_cursor: std::sync::Arc<std::sync::Mutex<(Row, Col)>> = Default::default();
        // Configuration de la fenêtre pop-up
        let win_config = types::WindowConfig::builder()
            .relative(types::WindowRelativeTo::Window(api::Window::current()))
            .taille_sensuelle()
            .row(*row.start as f64 + 1.)
            .col(*col.start as f64)
            .build();
        let win = &mut api::open_win(&buffer, true, &win_config).unwrap();
        let rtabs = setup_buffer_form(&mut buffer.clone(), RForm::clone(&parent.form), false).into_rtabs();
        {
            match parent.rtabs.read().find_tab(&parent.form) {
                Some(Tab::Multi {
                    target: Some(target), ..
                }) => {
                    if let Some(tab) = rtabs.read().multi_tabs().find_tab(&target) {
                        set_cursor(win, tab.start_row(), tab.start_col());
                        for row in tab.row_range() {
                            let _ = buffer.add_highlight(*NS, *HL_SELECTION, *row, ..);
                        }
                    } else {
                        crate::notify::error("Not found");
                    }
                }
                _ => (),
            }
        }
        Self {
            buffer,
            parent,
            prev_cursor,
            rtabs,
        }
    }
    fn enum_setup_keymap(&self) {
        let mut modes = crate::utils::ShortcutBuilder::new(self.buffer.clone());
        let popup = self;
        use types::Mode::*;
        crate::set_keymaps! {
            modes (Normal) :
            "<Esc>" => {form_abort()} <= <>
            "<CR>" => {form_abort()} <= <>
            "n" => {popup.next_variant()} <= <popup: Self>
            "<Tab>" => {popup.next_variant()} <= <popup: Self>
            "N" => {popup.previous_variant()} <= <popup: Self>
            "<S-Tab>" => {popup.previous_variant()} <= <popup: Self>
        }
        let buffer = self.buffer.clone();
        let popup = popup.clone();
        let opts = api::opts::CreateAutocmdOpts::builder()
            .buffer(buffer)
            // .callback(|_args: api::opts::CreateCommandOpts| fun_name(callback))
            .callback(move |_| popup.select_variant(TabSelector::None))
            .build();
        let _ = api::create_autocmd(vec!["CursorMoved"], &opts);
    }
    fn next_variant(&self) {
        self.select_variant(TabSelector::Next);
    }
    fn previous_variant(&self) {
        self.select_variant(TabSelector::Previous);
    }
    fn select_variant(&self, selector: TabSelector) -> bool {
        let Self {
            parent,
            buffer,
            prev_cursor,
            rtabs,
        } = self;
        let mut win = api::Window::current();
        let Some((row, col)) = get_cursor(&win) else {
            return false;
        };
        let tabs = rtabs.read();
        if let Some(tab) = selector.select(&tabs.multi_tabs_first_level(), row, col) {
            if set_cursor(&mut win, tab.start_row(), tab.start_col()).is_none() {
                return false;
            };
        };
        RTabs::update_popup_parent(&rtabs, buffer, parent, &prev_cursor)
    }
}

enum TabSelector {
    Next,
    Previous,
    None,
}
impl TabSelector {
    fn select(&self, tabs: &Tabs, row: Row, col: Col) -> Option<Tab> {
        use TabSelector::*;
        match self {
            Next => tabs.next_tab(row, col),
            Previous => tabs.previous_tab(row, col),
            None => Option::None,
        }
    }
}
fn form_prev_field(rtabs: &RTabs) {
    form_field_selector(rtabs, TabSelector::Previous);
}
fn form_next_field(rtabs: &RTabs) {
    form_field_selector(rtabs, TabSelector::Next);
}

fn form_field_selector(parent_rtabs: &RTabs, selector: TabSelector) {
    let parent_buffer = api::Buffer::current();
    let mut parent_win = api::Window::current();
    let Some((row, col)) = get_cursor(&parent_win) else {
        return;
    };

    let (row, col, parent_form) = {
        let tabs = parent_rtabs.read();
        let Some(parent_tab) = selector.select(&*tabs, row, col) else {
            return;
        };
        let row = parent_tab.row_range();
        let col = parent_tab.col_range();
        let form = RForm::clone(parent_tab.form());
        (row, col, form)
    };
    if set_cursor(&mut parent_win, row.start, col.start).is_none() {
        crate::notify::error("Can't set cursor.");
        return;
    }
    let parent = PopupParent {
        buffer: parent_buffer,
        form: parent_form,
        rtabs: RTabs::clone(parent_rtabs),
    };
    use Form::*;
    match &*parent.form {
        Enum(_name, _description, _default_variant, _fields) => {
            Popup::new(row, col, parent).enum_setup_keymap();
        }
        // IDEA: Sélectionner en visual les autres (ajouter visual dans les keymaps et utiliser
        // `parent.form` pour obtenir le Tab sélectionné. Pour sélectionner utiliser :
        // api::exec2("normal! v", &opts::ExecOpts::builder().build()).unwrap();
        // puis set_cursor
        _ => (),
    }
}

fn form_submit<T, F>(state: &SharedState, callback: &mut F)
where
    T: FormExt,
    F: FnMut(T, SharedState) + 'static,
{
    let mut buffer = api::Buffer::current();
    let lines: Vec<String> = buffer
        .get_lines(.., false)
        .unwrap()
        .map(|v| v.to_string())
        .collect();
    let data_json = lines.join("\n");
    let obj: T = match ron::from_str(&data_json) {
        Ok(obj) => obj,
        Err(err) => {
            use ron::error::{Position, Span, SpannedError};
            crate::notify::warn(err.clone());
            let SpannedError {
                // code,
                span:
                    Span {
                        start:
                            Position {
                                line: start_line,
                                col: start_col,
                            },
                        end:
                            Position {
                                line: end_line,
                                col: end_col,
                            },
                    },
                ..
            } = err;
            let start_col = start_col - 1;
            let mut end_col = end_col - 1;
            let start_line = start_line - 1;
            let end_line = end_line - 1;

            let hl = *HL_ERROR;
            if start_line == end_line {
                if start_col == end_col {
                    end_col += 1;
                }
                buffer
                    .add_highlight(*NS, hl, start_line, start_col..end_col)
                    .unwrap();
            } else {
                buffer
                    .add_highlight(*NS, hl, start_line, start_col..)
                    .unwrap();
                let mut line = start_line + 1;
                while line < end_line {
                    buffer.add_highlight(*NS, hl, line, ..).unwrap();
                    line += 1
                }
                buffer
                    .add_highlight(*NS, hl, end_line, ..end_col)
                    .unwrap();
            }
            return;
        }
    };
    let _ = api::Window::current().close(true);
    callback(obj, SharedState::clone(state));
}

fn form_abort() {
    api::Window::current().close(true).unwrap();
}

#[cfg(test)]
mod tests {
    use mistral_nvim_derive::Form;
    use serde::Deserialize;

    use super::*;

    #[allow(dead_code)]
    #[derive(Form, Deserialize)]
    pub struct StructTuple(
        /// Field name
        String,
        /// Field model (enum)
        Enum,
    );

    /// Some struct
    #[allow(dead_code)]
    #[derive(Form, Deserialize)]
    pub struct Struct {
        /// Field name
        name: String,
        /// Field model (enum)
        model: Enum,
    }

    /// Some enum
    #[derive(Form, Deserialize)]
    pub enum Enum {
        /// A
        A,
        /// B
        B,
    }

    /// Some struct imbriquée
    #[allow(dead_code)]
    #[derive(Form, Deserialize)]
    pub struct StructImbriquee {
        /// Field name
        name: String,
        /// Field struct imbriquee
        s: Struct,
    }

    #[test]
    fn filled_form_struct() {
        use crate::mistral::model::FormExt;
        crate::log_libuv!(Trace, "Get Form");
        let form = Struct::get_form();
        crate::log_libuv!(Trace, "Get Form");
        let mut rows = Vec::new();
        let mut tabs = Default::default();
        fill_form(form, &mut rows, &mut tabs, 0);
        let content = rows.join("\n");
        assert_eq!(
            content,
            r##"Struct ( // Some struct
    // Field name
    name: "", // Str
    // Field model (enum)
    model: (), // Enum
)"##
        );
        let mut expected = Tabs::default();
        expected.push_single(2, 10..=12, String::get_form());
        expected.push_multi(4, 11..=13, Enum::get_form());
        assert_eq!(tabs, expected);
    }

    #[test]
    fn filled_form_struct_tuple() {
        crate::log_libuv!(Trace, "Get Form");
        let form = StructTuple::get_form();
        crate::log_libuv!(Trace, "Get Form");
        let mut rows = Vec::new();
        let mut tabs = Default::default();
        fill_form(form, &mut rows, &mut tabs, 0);
        let content = rows.join("\n");
        assert_eq!(
            content,
            r##"StructTuple (
    // Field name
    "", // Str
    // Field model (enum)
    (), // Enum
)"##
        );
        let mut expected = Tabs::default();
        expected.push_single(2, 4..=6, String::get_form());
        expected.push_multi(4, 4..=6, Enum::get_form());
        assert_eq!(tabs, expected);
    }

    #[test]
    fn filled_form_imbriquee() {
        crate::log_libuv!(Trace, "Get Form");
        let form = StructImbriquee::get_form();
        crate::log_libuv!(Trace, "Get Form");
        let mut rows = Vec::new();
        let mut tabs = Default::default();
        fill_form(form, &mut rows, &mut tabs, 0);
        let content = rows.join("\n");
        assert_eq!(
            content,
            r##"StructImbriquee ( // Some struct imbriquée
    // Field name
    name: "", // Str
    // Field struct imbriquee
    s: Struct ( // Some struct
        // Field name
        name: "", // Str
        // Field model (enum)
        model: (), // Enum
    ),
)"##
        );
        let mut expected = Tabs::default();
        expected.push_single(2, 10..=12, String::get_form());
        expected.push_single(6, 14..=16, String::get_form());
        expected.push_multi(8, 15..=17, Enum::get_form());
        assert_eq!(tabs, expected);
    }
}
