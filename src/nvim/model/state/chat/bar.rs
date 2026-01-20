use std::{
    collections::HashMap,
    sync::{Arc, LazyLock, Mutex},
};

use nvim_oxi::api;

use super::Page;
use crate::{
    mistral::model::{
        message::Role,
        stream::{Status, Usage},
    },
    nvim::model::{self, Locker as _},
    utils::{get_option_win, set_option_win},
};

const OPT: &str = "statusline";

#[derive(Clone, PartialEq, Debug)]
enum PageStatus {
    Header(usize),
    Prompt(usize),
    Message(usize, usize),
}
impl std::fmt::Display for PageStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Header(total_msg_sent) => {
                write!(f, "-/{}", total_msg_sent)
            }
            Self::Prompt(total_msg_sent) => {
                write!(f, "Prompt/{}", total_msg_sent)
            }
            Self::Message(nb_msg, total_msg_sent) => {
                write!(f, "Message {}/{}", nb_msg, total_msg_sent)
            }
        }
    }
}
impl PageStatus {
    fn new(page: &Page, nb_messages: usize) -> Self {
        match page {
            Page::Header => Self::Header(nb_messages),
            Page::Prompt(_) => Self::Prompt(nb_messages),
            Page::Message(index) => Self::Message(index + 1, nb_messages),
        }
    }
}

fn parse_usage(usage: (&Usage, &Role, &Option<u32>)) -> String {
    let (usage, role, max_tokens) = usage;
    match role {
        Role::User => usage.prompt_tokens.to_string(),
        Role::System => usage.prompt_tokens.to_string(),
        Role::Assistant => {
            let tokens_limit = max_tokens
                .map(|n| n.to_string())
                .unwrap_or_else(|| "∞".to_string());
            format!("{}/{tokens_limit}", usage.completion_tokens)
        }
        Role::Tool => usage.prompt_tokens.to_string(),
    }
}
fn parse_role(role: &Role) -> String {
    use super::highlight::*;
    match role {
        Role::User => format!("%#{}# ", *HL_ROLE_USER),
        Role::System => format!("%#{}#󰍹 ", *HL_ROLE_SYSTEM),
        Role::Assistant => format!("%#{}#󰚩 ", *HL_ROLE_ASSISTANT),
        Role::Tool => format!("%#{}# ", *HL_ROLE_TOOL),
    }
}
fn parse_status(status: &Status) -> String {
    use super::highlight::*;
    match status {
        Status::Completed => format!("%#{}#{}", *HL_STATUS_COMPLETED, "✓ Completed"),
        Status::Partial(err) => {
            // let mut err = err.to_string();
            // err.truncate(69);
            format!("%#{}#{}", *HL_STATUS_PARTIAL, format!("⚠ Partial: {}", escape(err)))
        }
        Status::Failed(err, details) => {
            // let mut err = err.to_string();
            // err.truncate(69);
            match details {
                crate::mistral::model::stream::ErrorMessageType::Details(error_message) => {
                    for error in &error_message.detail {
                        crate::notify::error(error);
                    }
                }
                crate::mistral::model::stream::ErrorMessageType::Simple(error) => {
                    crate::notify::error(error);
                }
                crate::mistral::model::stream::ErrorMessageType::Empty => (),
            }
            format!("%#{}#{}", *HL_STATUS_FAILED, format!("✗ Failed: {}", escape(err)))
        }
        Status::Created => format!("%#{}#{}", *HL_STATUS_CREATED, "✚ Created"),
        Status::Initialised => format!("%#{}#{}", *HL_STATUS_INITIALISED, "⚙ Processing"),
    }
}

enum Outdate {
    Full,
    Page,
}

struct StatusLine {
    pre_computed: String,
    computed_without_page: String,
    computed: String,
    outdated: Option<Outdate>,
}
impl StatusLine {
    fn new(page: &Page) -> Self {
        Self {
            pre_computed: Self::pre_computed(page),
            computed_without_page: String::new(),
            computed: String::new(),
            outdated: Some(Outdate::Full),
        }
    }
    fn pre_computed(page: &Page) -> String {
        use super::highlight::*;
        match page {
            Page::Header => {
                format!(
                    "%#{}#%{{NAME}} | \
                     %#{}# %{{PAGE}} | \
                     %#{}# %{{H_USAGE_PROMPT}} | \
                     %#{}# %{{H_USAGE_COMPLETION}}",
                    *HL_NAME, *HL_PAGE, *HL_USAGE, *HL_USAGE,
                )
            }
            _ => {
                format!(
                    "%#{}#%{{NAME}} %{{ROLE}} | \
                     %#{}# %{{PAGE}} | \
                     %#{}# %{{MODEL}} | \
                     %#{}# %{{USAGE}} | \
                     %#{}#%{{MODE}} \
                     %{{STATUS}}",
                    *HL_NAME, *HL_PAGE, *HL_MODEL, *HL_USAGE, *HL_MODE,
                )
            }
        }
    }
    fn update(&mut self, chat: &super::Chat, page: &Page) {
        match chat.0.try_lock() {
            Ok(lock) => match self.outdated {
                None => (),
                Some(Outdate::Full) => self.update_full(lock, page),
                Some(Outdate::Page) => self.update_page(page, lock.messages.len()),
            },
            // TODO: happens way too often
            // Err(error) => crate::log_libuv!(Warn, "Can't update bar, cause chat can't be locked. ({error})"),
            Err(_error) => (),
        }
    }
    fn update_full(&mut self, chat: std::sync::MutexGuard<'_, super::ChatState>, page: &Page) {
        let len_messages = chat.messages.len();
        match page {
            Page::Message(msg_index) | Page::Prompt(msg_index) => self.update_message(chat, *msg_index),
            Page::Header => self.update_header(chat),
        }
        self.update_page(page, len_messages);
    }
    fn update_message(&mut self, chat: std::sync::MutexGuard<'_, super::ChatState>, msg_index: usize) {
        let Some(msg) = &chat.messages.get(msg_index) else {
            return;
        };
        let md = &chat.metadata;
        let usage: (&Usage, &Role, &Option<u32>) = (&msg.usage, &msg.message.role, &msg.params.max_tokens);
        self.computed_without_page = self
            .pre_computed
            .replace("%{NAME}", &escape(&md.name))
            .replace("%{MODEL}", &msg.model.to_string())
            .replace("%{MODE}", &msg.mode.to_string())
            .replace("%{STATUS}", &parse_status(&msg.status))
            .replace("%{ROLE}", &parse_role(&usage.1))
            .replace("%{USAGE}", &parse_usage(usage));
    }
    fn update_header(&mut self, chat: std::sync::MutexGuard<'_, super::ChatState>) {
        let len_messages = chat.messages.len();
        let md = &chat.metadata;
        let usage = &md.usage;
        self.computed_without_page = self
            .pre_computed
            .replace("%{NAME}", &escape(&md.name))
            .replace("%{H_USAGE_PROMPT}", &usage.prompt_tokens.to_string())
            .replace("%{H_USAGE_COMPLETION}", &usage.completion_tokens.to_string());
        self.update_page(&Page::Header, len_messages);
    }
    fn update_page(&mut self, page: &Page, len_messages: usize) {
        let page_status = PageStatus::new(page, len_messages);
        self.computed = self
            .computed_without_page
            .replace("%{PAGE}", &page_status.to_string());
    }
}
fn escape(s: &str) -> String {
    s.replace('%', "%%").replace('|', "%%|")
}

type StatusLineCache = HashMap<api::Buffer, StatusLineChatCache>;
static CACHE: LazyLock<Arc<Mutex<StatusLineCache>>> = LazyLock::new(|| Default::default());

pub struct StatusLineChatCache {
    // Outside this struct
    chat: super::Chat, // Récupérable par le buffer
    // Real content of the struct
    windows_prev_range: HashMap<api::Window, crate::nvim::model::RowRange>,
    positions: super::MessagesPositions,
    statuslines: Vec<StatusLine>,
}
impl StatusLineChatCache {
    fn new(chat: super::Chat, window: api::Window) -> Self {
        let (positions, current_range, current_index) = {
            let lock = chat.lock();
            let (range, index) = lock.get_range_index_by_row(&window);
            (lock.positions.clone(), range.clone(), index)
        };
        let nb_msg = positions.nb_msg();
        let pos_indexes = 0..=nb_msg;
        let statuslines: Vec<_> = pos_indexes
            .map(|position| StatusLine::new(&Page::from_position(position, nb_msg)))
            .collect();
        let mut windows_prev_range = HashMap::default();
        if let Some(line) = statuslines.get(current_index) {
            set_option_win(&window, OPT, line.computed.clone());
            windows_prev_range = HashMap::from([(window, current_range)]);
        }
        Self {
            chat,
            windows_prev_range,
            positions,
            statuslines,
        }
    }

    fn update_window(&mut self, window: &api::Window) {
        let Some((cursor_row, _)) = model::get_cursor(window) else {
            return;
        };
        let (new_range, position) = self.positions.get_range_index_by_row(cursor_row);
        let mut not_in_range = true;
        use std::collections::hash_map::Entry;
        match self.windows_prev_range.entry(window.clone()) {
            Entry::Occupied(mut entry) => {
                let prev_range = entry.get_mut();
                not_in_range = !prev_range.contains(&cursor_row);
                *prev_range = new_range.clone();
            }
            Entry::Vacant(vacant) => {
                vacant.insert(new_range.clone());
            }
        }
        let statusline = match self.statuslines.get_mut(position) {
            Some(statusline) => statusline,
            None => {
                self.fill_missing_statuslines(position);
                self.statuslines
                    .get_mut(position)
                    .expect("Statusline added.")
            }
        };
        let is_outdated = statusline.outdated.is_some();
        if not_in_range || is_outdated {
            if is_outdated {
                statusline.update(&self.chat, &Page::from_position(position, self.positions.nb_msg()));
                set_option_win(window, OPT, statusline.computed.clone());
            } else {
                let new_line = statusline.computed.clone();
                if get_option_win::<String>(window, OPT).unwrap_or_default() != new_line {
                    set_option_win(window, OPT, new_line);
                }
            }
        }
    }
    pub fn change_positions(buffer: &api::Buffer, new_positions: &super::MessagesPositions) {
        if let Some(cache) = CACHE.lock().unwrap().get_mut(buffer) {
            cache.windows_prev_range.clear();
            cache.positions = new_positions.clone();
        }
    }
    fn fill_missing_statuslines(&mut self, new_last: super::Position) {
        let last = self.statuslines.len() - 1;
        if new_last > last {
            let page = &Page::from_position(last, new_last);
            self.statuslines[last].update_page(page, new_last);
            for pos in last + 1..=new_last {
                let page = &Page::from_position(pos, new_last);
                self.statuslines.push(StatusLine::new(page));
            }
        }
    }

    #[track_caller]
    pub fn update_current_window(buffer: &api::Buffer) {
        let window = api::Window::current();
        if let Ok(buf_win) = window.get_buf() {
            if buf_win == *buffer {
                if let Ok(mut lock) = CACHE.try_lock() {
                    if let Some(cache) = lock.get_mut(buffer) {
                        cache.update_window(&window);
                    }
                } else {
                    crate::log_libuv!(Warn, "STATUS CACHE NOT UPDATED");
                }
            }
        }
    }
    pub fn outdate_messages(buffer: &api::Buffer, positions: std::ops::RangeInclusive<super::Position>) {
        let start = positions.end().clone();
        let end = positions.end().clone();
        if start > end {
            crate::log_libuv!(Error, "Outdate Messages : range (start: {start} > end: {end}).");
            return;
        }
        if let Some(cache) = CACHE.lock().unwrap().get_mut(buffer) {
            cache.fill_missing_statuslines(end);
            for statusline in &mut cache.statuslines[positions] {
                statusline.outdated = Some(Outdate::Full);
            }
        }
    }

    #[track_caller]
    pub fn outdate_page(buffer: &api::Buffer, position: super::Position) {
        if let Some(cache) = CACHE.lock().unwrap().get_mut(buffer) {
            cache.fill_missing_statuslines(position);
            if let Some(statusline) = cache.statuslines.get_mut(position) {
                statusline.outdated = Some(Outdate::Full);
            }
        }
        Self::update_current_window(buffer);
    }
    #[track_caller]
    pub fn outdate_all_pages(buffer: &api::Buffer, nb_messages: usize) {
        if let Some(cache) = CACHE.lock().unwrap().get_mut(buffer) {
            cache.fill_missing_statuslines(nb_messages);
            for statusline in cache.statuslines.iter_mut() {
                statusline.outdated = Some(Outdate::Page);
            }
        }
        Self::update_current_window(buffer);
    }
}

impl super::Chat {
    pub fn configure_statusline(self, buffer: &api::Buffer, window: api::Window) {
        let mut global_cache = CACHE.lock().unwrap();
        if let Some(cache) = global_cache.get_mut(buffer) {
            cache.update_window(&window);
            return;
        }
        global_cache.insert(buffer.clone(), StatusLineChatCache::new(self, window));
    }
}
