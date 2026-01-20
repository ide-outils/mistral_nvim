use std::path::PathBuf;

use mistral_nvim_derive::Form;
use nvim_oxi::api;
use serde::{Deserialize, Serialize};

use crate::{
    mistral,
    notify::{IntoNotification as _, NotifyExt as _},
    nvim::model::{self, Cursor, Locker, Mode, Row, RowRange},
    utils::set_option,
};

pub type MsgIndex = usize;
type Position = usize;

pub mod bar;
mod highlight;
mod parser;

use parser::*;

#[derive(Clone, Serialize, Deserialize, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum Page {
    Prompt(MsgIndex),
    Message(MsgIndex),
    Header,
}

impl Page {
    pub fn from_position(index: Position, messages_len: usize) -> Self {
        match index {
            0 => Page::Header,
            index if index == messages_len => Page::Prompt(index - 1),
            index => Page::Message(index - 1),
        }
    }
}

#[derive(Clone)]
pub struct Chat(std::sync::Arc<std::sync::Mutex<ChatState>>);
impl Locker for Chat {
    type Locked = ChatState;

    fn inner(&self) -> &std::sync::Arc<std::sync::Mutex<Self::Locked>> {
        &self.0
    }
}
impl Chat {
    pub fn from_state(value: ChatState) -> Self {
        Chat(std::sync::Arc::new(std::sync::Mutex::new(value)))
    }
    pub fn from_buffer(state: &super::SharedState, buffer: &api::Buffer) -> Option<Self> {
        let path = buffer.get_name().ok()?;
        Some(Self::clone(state.lock().chats.get_by_path(&path)?))
    }
    pub fn from_current_buffer(state: &super::SharedState) -> Option<Self> {
        let buffer = api::Buffer::current();
        let res = Self::from_buffer(state, &buffer);
        if res.is_none() {
            crate::notify::warn("You're not in a chat's buffer.");
        }
        res
    }
    pub fn from_current_buffer_target_prompt(state: &super::SharedState) -> Option<Self> {
        let res = Self::from_current_buffer(state);
        if let Some(chat) = res.as_ref() {
            let win = api::Window::current();
            if chat.lock().is_in_prompt(&win).unwrap_or(false) {
                crate::notify::warn(
                    "You must be in the current prompt to trigger this operation (at then end of the buffer).",
                );
                return None;
            }
        }
        res
    }
}

pub struct ChatState {
    pub is_running: Option<u32>,
    pub path: PathBuf,
    pub buffer: api::Buffer,
    pub buffer_modifier: Option<super::BufferModifierGroupedUndo>,
    pub metadata: ChatMetadata,
    pub messages: Vec<MessageState>,
    pub positions: MessagesPositions,
}

#[derive(Clone)]
pub struct MessagesPositions(Vec<RowRange>);
impl Default for MessagesPositions {
    fn default() -> Self {
        Self(vec![(Row(0)..Row::MAX).into()])
    }
}
impl MessagesPositions {
    pub fn get_by_row(&self, current_row: Row) -> Option<&RowRange> {
        self.0
            .iter()
            .find(|row_range| row_range.contains(&*current_row))
    }
    pub fn index_by_row(&self, current_row: &Row) -> Option<MsgIndex> {
        let index = self.pos_index_by_row(current_row)?;
        if index == 0 { None } else { Some(index - 1) }
    }
    pub fn pos_index_by_row(&self, current_row: &Row) -> Option<Position> {
        self.0
            .iter()
            .position(|row_range| row_range.contains(current_row))
    }
    pub fn get_range_index_by_row(&self, current_row: Row) -> (&RowRange, Position) {
        self.0
            .iter()
            .enumerate()
            .find(|(_, row_range)| row_range.contains(&*current_row))
            .map(|(p, r)| (r, p))
            .unwrap_or((self.last(), self.nb_msg()))
    }
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &RowRange> {
        self.0.iter()
    }
    #[inline]
    pub fn into_iter(self) -> impl Iterator<Item = RowRange> {
        self.0.into_iter()
    }
    pub fn pages_iterator(&self) -> impl Iterator<Item = Page> {
        use std::iter;
        let last = self.nb_msg() - 1;
        let messages = (0..last).into_iter().map(|index| Page::Message(index));
        iter::once(Page::Header)
            .chain(messages)
            .chain(iter::once(Page::Prompt(last)))
    }
    #[inline]
    pub fn nb_msg(&self) -> usize {
        self.0.len() - 1 // The first range is the Chat's Header
    }
    #[inline]
    pub fn last(&self) -> &RowRange {
        self.0.last().expect("Can't be empty.")
    }
    #[inline]
    pub fn last_start(&self) -> Row {
        self.0
            .last()
            .map(|range| range.start)
            .unwrap_or(Row::MIN)
    }
    #[inline]
    pub fn get_by_msg_index(&mut self, msg_index: MsgIndex) -> Option<&RowRange> {
        self.0.get(msg_index + 1) // First one is the Header not a Message/Prompt
    }
    pub fn pop(&mut self, buffer: &api::Buffer) -> Option<RowRange> {
        if self.0.len() <= 2 {
            None
        } else {
            let poped = self.0.pop();
            bar::StatusLineChatCache::change_positions(&buffer, &self);
            poped
        }
    }
    #[inline]
    pub fn get_range(&self, index: Position) -> Option<&RowRange> {
        self.0.get(index)
    }
}

#[derive(Default)]
pub struct ChatMetadata {
    pub name: String,
    pub description: String,
    pub usage: mistral::model::stream::Usage,
}

#[derive(Default, Clone, Debug)]
pub struct MessageState {
    pub message: mistral::model::Message,
    pub mode: model::Mode,
    pub model: mistral::model::completion::Model,
    pub usage: mistral::model::stream::Usage,
    pub params: mistral::model::completion::CompletionParams,
    pub status: mistral::model::stream::Status,
    pub tool_calls_positions: Option<Vec<RowRange>>,
}

/// This form serves to setup a Chat
#[derive(Form, Deserialize, Debug)]
pub struct ChatForm {
    /// A short description of the chat.
    pub name: String,
    /// What the goal of this chat, used to create an initial system's message.
    pub description: String,
    /// The Model to use in this Chat. You'll be able to change it later.
    pub model: mistral::model::completion::Model,
    /// The tools activated. You'll be able to change it later.
    pub mode: Mode,
}

impl ChatState {
    pub fn new(form: ChatForm, state: &super::SharedState) -> crate::Result<Self> {
        let buffer = &mut api::Buffer::current();
        let ChatForm {
            name,
            description,
            model,
            mode,
        } = form;
        let desc = description.clone();
        let mut chat_state = Self {
            is_running: None,
            path: buffer
                .get_name()
                .expect("Buffer should already have a path."),
            buffer: buffer.clone(),
            buffer_modifier: None,
            metadata: ChatMetadata {
                name,
                description,
                ..ChatMetadata::default()
            },
            messages: Vec::default(),
            positions: MessagesPositions::default(),
        };
        chat_state.write_config_line();
        if !desc.is_empty() {
            let mut system = MessageState::default();
            system.model = model.clone();
            system.mode = mode.clone();
            system.message.role = mistral::model::Role::System;
            system.message.content = desc;
            chat_state.push_message(system, None)?;
        }
        {
            let mut prompt = MessageState::default();
            prompt.model = model.clone();
            prompt.mode = mode.clone();
            prompt.message.role = mistral::model::Role::User;
            prompt.message.content = Default::default();
            chat_state.push_message(prompt, None)?;
        }
        chat_state.init_buffer(state)?;
        Ok(chat_state)
    }
    pub fn load(state: &super::SharedState, buffer: &api::Buffer) -> crate::Result<Self> {
        let messages = vec![];
        let mut chat_state = Self {
            is_running: None,
            path: buffer
                .get_name()
                .expect("Buffer should already have a path."),
            buffer: buffer.clone(),
            buffer_modifier: None,
            metadata: Default::default(),
            messages,
            positions: MessagesPositions::default(),
        };
        chat_state.init_buffer(state)?;
        Ok(chat_state)
    }
    #[track_caller]
    pub fn buffer_modifier_get_or_create<'bm>(
        &'bm mut self,
    ) -> crate::Result<&'bm mut super::BufferModifierGroupedUndo> {
        match &mut self.buffer_modifier {
            Some(_) => (),
            none => {
                let bm = super::BufferModifierGroupedUndo::new(&self.buffer)?;
                *none = Some(bm);
            }
        }
        Ok(self.buffer_modifier.as_mut().unwrap())
    }
    #[track_caller]
    pub fn start_insertion_successive(&mut self, id: usize, cursor: model::Cursor) -> crate::Result<()> {
        let bm = self.buffer_modifier_get_or_create()?;
        Ok(bm.start_insertion_successive(id, cursor)?)
    }
    // #[track_caller]
    // pub fn start_replace_line(
    //     &mut self,
    //     id: MsgIndex,
    //     row_final: model::Row,
    //     length_initial: usize,
    // ) -> crate::Result<()> {
    //     let bm = self.buffer_modifier_get_or_create()?;
    //     Ok(bm.start_replacement_line(id, , 0)?)
    // }
    fn exec2(
        &mut self,
        command: String,
        opts: api::opts::ExecOpts,
        timeout: Option<(u64, u32)>,
    ) -> crate::Result<Option<String>> {
        // Don't use a rendez-vous, we are here in the same thread, it would block.
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        self.buffer.call(move |_| {
            tx.send(api::exec2(&command, &opts))
                .expect("Call buffer : sync channel unexpectedly closed.")
        })?;
        let (secs, nanos) = timeout.unwrap_or((1, 0));
        match rx.recv_timeout(std::time::Duration::new(secs, nanos)) {
            Ok(undo_res) => match undo_res {
                Ok(nvim_string) => Ok(nvim_string.map(|s| s.to_string())),
                Err(err) => Err(err.into_error()),
            },
            Err(_) => Err("Command has timed out.".into_error()),
        }
    }
    fn undojoin(&mut self) -> crate::Result<()> {
        if let Some(prev_tick) = self.is_running.as_mut() {
            let tick = self.buffer.get_changedtick()?;
            if tick != *prev_tick {
                *prev_tick = tick;
                self.exec2("undojoin".to_string(), Default::default(), None)?;
            }
        }
        Ok(())
    }
    pub fn replace_line(&mut self, row: model::Row, line: String, id: Option<usize>) -> crate::Result<()> {
        self.undojoin()?;
        if let Some(id) = id
            && let Some(bm) = self.buffer_modifier.as_mut()
        {
            bm.start_replacement_line(id, row, line.len())?;
            bm.replace_line(id, row, &line)?;
        } else {
            let buffer = &mut self.buffer;
            model::cursor::set_lines(buffer, row..=row, true, [line])?;
        }
        Ok(())
    }
    pub fn insert(&mut self, lines: Vec<String>, id: Option<usize>) -> crate::Result<()> {
        self.undojoin()?;
        if let Some(id) = id
            && let Some(bm) = self.buffer_modifier.as_mut()
        {
            bm.insert(id, lines)?;
            let range = self
                .positions
                .get_by_msg_index(id)
                .cloned()
                .unwrap_or(RowRange::FULL);
            self.update_buffer(range)
        } else {
            let buffer = &mut self.buffer;
            model::cursor::push_lines(buffer, lines)?;
            self.update_buffer(self.positions.last().clone())
        }
    }
    pub fn buffer_modifier_ids_finished(&mut self, ids: Vec<usize>) {
        match &mut self.buffer_modifier {
            None => self.is_running = None,
            some => {
                let bm = some.as_mut().unwrap();
                if bm.ids_finished(ids) {
                    self.is_running = None;
                    *some = None;
                }
            }
        }
    }
    pub fn push_message(&mut self, message: MessageState, id: Option<usize>) -> crate::Result<()> {
        let lines = build_tag_message_lines(message);
        self.insert(lines, id)?;
        bar::StatusLineChatCache::outdate_all_pages(&self.buffer, self.messages.len());
        Ok(())
    }
    pub fn push_new_message(&mut self, id: Option<usize>) -> crate::Result<()> {
        let Some(last) = self.messages.last() else {
            return Err("No message stored in this Chat.".into_error());
        };
        let mut next_msg = MessageState::default();
        next_msg.mode = last.mode.clone();
        next_msg.model = last.model.clone();
        next_msg.params = last.params.clone();
        self.push_message(next_msg, id)
    }
    pub fn push_tool_call(&mut self, tool_call: &mistral::model::ToolCall, id: Option<usize>) -> crate::Result<()> {
        let lines = build_tag_tool_call_lines(tool_call);
        self.insert(lines, id)
    }
    fn write_config_line(&mut self) {
        let ChatMetadata {
            name,
            description,
            usage,
        } = &self.metadata;
        let mut args = String::new();
        args.push_str(&format!(r#" name="{name}""#));
        args.push_str(&format!(r#" usage="{usage}""#));
        args.push_str(&format!(r#" description="{description}""#));
        let lines = format!(r#"<{TAG_CHAT}{args}/>"#);
        // Erase whole buffer (this function is used only during chat's creation).
        model::cursor::set_lines(&mut self.buffer, RowRange::FULL, false, [lines]).notify_error();
    }
    fn update_message_tag_line(&mut self, message_index: MsgIndex) -> crate::Result<()> {
        if let Some(message) = self.messages.get_mut(message_index) {
            let buf = &mut self.buffer.clone();
            let Some(pos) = self.positions.get_by_msg_index(message_index) else {
                // TODO: could unreachable, messages and positions must be sync.
                return Err("Can't get position, but message exists for the same index.".into_error());
            };
            let row = pos.start;
            let line = model::cursor::get_line(buf, row, false)?;
            let mut line = line.to_string();
            let mut updates = Vec::new();
            let mut nb_cols_diff = 0isize;
            parse_tag_line(&line, |key, current_val, cols| {
                if let Some(new_value) = message_getter(key, message) {
                    if new_value != current_val {
                        nb_cols_diff += new_value.len() as isize - current_val.len() as isize;
                        updates.push((cols, new_value))
                    }
                }
            });
            for (cols, value) in updates.into_iter().rev() {
                let range = model::FromNvimRange::<model::EndExclusive, model::ZeroIndexed>::into_nvim(cols);
                line.replace_range(range, &value);
            }
            self.replace_line(row, line, Some(message_index))
                .unwrap();
            bar::StatusLineChatCache::outdate_page(&self.buffer, message_index + 1);
            Ok(())
        } else {
            Ok(())
        }
    }
    fn update_config_tag_line(&mut self) -> crate::Result<()> {
        let buf = &mut self.buffer.clone();
        let row = Row(0);
        let line = model::cursor::get_line(buf, row, false)?;
        let mut updates = Vec::new();
        parse_tag_line(&line.to_string(), |key, current_val, cols| {
            if let Some(new_value) = config_getter(key, self) {
                if new_value != current_val {
                    updates.push((cols, new_value))
                }
            }
        });
        for (cols, value) in updates.into_iter().rev() {
            model::set_text(buf, row..=row, cols, [value]).notify_error();
        }
        Ok(())
    }

    fn init_buffer(&mut self, state: &super::SharedState) -> crate::Result<()> {
        let buffer = &mut self.buffer.clone();
        self.update_buffer(RowRange::FULL)?;
        set_option(&buffer, "filetype", "markdown");
        crate::nvim::controlleur::chat::setup_keymaps(state, buffer).notify_error();
        Ok(())
    }
    pub fn update_buffer(&mut self, rows_range: RowRange) -> crate::Result<()> {
        if rows_range.start > rows_range.end {
            return Err("Can't update. RowRange is reversed.".into_warn());
        }
        let mut index_row = rows_range.start.clone();
        let buffer = &self.buffer.clone();
        let Ok(lines) = model::get_lines(buffer, rows_range.clone(), false) else {
            return Err("Can't read buffer content.".into_error());
        };
        let mut lines = lines.map(|v| v.to_string());
        let args = GeneratorArgs::new(&self.positions, rows_range, buffer)?;
        if args.from_begin {
            let Some(first_line) = lines.next() else {
                return Err(format!("Empty buffer : A Chat file must start by the {TAG_CHAT} tag.").into_warn());
            };
            if is_self_tag_line(&first_line, TAG_CHAT) {
                parse_tag_line(&first_line, |key, val, _cols| config_setter(key, val, self));
            } else {
                return Err(format!("A Chat file must start by the {TAG_CHAT} tag.").into_warn());
            }
            index_row += 1;
        }
        let mut messages_generator = MsgGen::new(&args, self.messages.len());
        let mut positions_generator = PosGen::new(&args, self.positions.0.len());
        loop {
            let line = lines.next().map(|l| (index_row, l));
            messages_generator.next_line_option(&line)?;
            positions_generator.next_line_option(&line)?;
            if line.is_none() {
                break;
            }
            index_row += 1;
        }
        messages_generator.replace(&mut self.messages, 1)?;
        positions_generator.replace(&mut self.positions.0, 0)?;
        bar::StatusLineChatCache::change_positions(&buffer, &self.positions);
        Ok(())
    }
    pub fn get_cursor(&self, window: &api::Window) -> Option<(Row, model::Col)> {
        if window.get_buf().ok()? == self.buffer {
            model::get_cursor(window)
        } else {
            return None;
        }
    }
    pub fn get_range_index_by_row(&self, window: &api::Window) -> (&RowRange, Position) {
        let row = self
            .get_cursor(window)
            .map(|c| c.0)
            .unwrap_or(Row::MAX);
        self.positions.get_range_index_by_row(row)
    }
    pub fn get_position_index(&self, window: &api::Window) -> Option<Position> {
        let (row, _) = self.get_cursor(window)?;
        self.positions.pos_index_by_row(&row)
    }
    pub fn get_position_index_cursor(&self, window: &api::Window) -> Option<(Position, Cursor)> {
        let (row, col) = self.get_cursor(window)?;
        Some((self.positions.pos_index_by_row(&row)?, Cursor { row, col }))
    }
    /// Get the current message.
    ///
    /// Panic:
    /// You must ensure that the current buffer refers to the chat. You can use
    /// ```rust
    /// Chat::from_current_buffer()
    /// ```
    pub fn get_current_message(&self, win: &api::Window) -> Option<&MessageState> {
        match self.get_position_index(win)? {
            0 => None, // Header
            position => self.messages.get(position - 1),
        }
    }
    pub fn is_in_prompt(&self, win: &api::Window) -> Option<bool> {
        Some(self.get_position_index(win)? == self.positions.nb_msg() - 1)
    }

    pub fn update_prompt(&mut self) -> crate::Result<()> {
        if let Some(last_msg) = self.messages.last_mut() {
            if !matches!(last_msg.status, mistral::model::stream::Status::Created) {
                return Err("Prompt already sent.".into_warn());
            } else if !matches!(last_msg.message.role, mistral::model::Role::User) {
                if matches!(last_msg.message.role, mistral::model::Role::Tool) {
                    return Ok(());
                }
                return Err("Must be a prompt to be sent.".into_warn());
            }
        } else {
            return Err("No prompt found (no message).".into_error());
        }
        self.update_buffer(self.positions.last().clone())
    }

    pub fn send_prompt(&mut self, state: &super::SharedState) -> crate::Result<()> {
        self.update_prompt()?;
        let envelop = self.build_request_envelop()?;
        state.lock().tx_mistral.send(envelop).unwrap();
        Ok(())
    }
    pub fn build_request_envelop(&mut self) -> crate::Result<crate::messages::NvimEnveloppe> {
        use crate::mistral::model::completion::{ChatCompletion, ChatRequest};
        let messages = self
            .messages
            .iter()
            .map(|m| m.message.clone())
            .collect();
        let Some(last) = self.messages.last() else {
            return Err("No message stored in this Chat.".into_error());
        };
        let last_message_index = self.messages.len() - 1;
        let model = last.model.clone();
        let mut params = last.params.clone();
        if !matches!(last.mode, Mode::None) {
            params.tools = Some(last.mode.current_tools());
        }
        let request = ChatRequest {
            completion: ChatCompletion { model, messages },
            params,
        };

        let envelop = crate::messages::NvimEnveloppe {
            id: crate::messages::IdMessage::Chat(self.buffer.handle(), last_message_index),
            message: crate::messages::NvimMessage::Chat(request),
        };
        Ok(envelop)
    }
    fn goto_message(&mut self, inc_position: isize, win: &mut api::Window) {
        if inc_position == 0 {
            return;
        };
        let max_index = self.positions.nb_msg();
        // let Some((current_index, cursor)) = self.get_position_index_cursor(win) else {
        let Some(current_index) = self.get_position_index(win) else {
            return;
        };
        let target_index = std::cmp::min(current_index.saturating_add_signed(inc_position), max_index);
        // cmd::min ensure safe unwrap.
        let target_line = Option::unwrap(
            self.positions
                .get_range(target_index)
                .map(|range| range.start),
        );
        // The Nvim API, does not force the jumplist, we must pass through normal mode.
        // model::cursor::set_mark(&mut self.buffer, '\'', cursor.row, cursor.col, &Default::default());
        api::exec2("normal! m'", &Default::default()).notify_warn();
        model::set_cursor(win, target_line, 0);
    }
    pub fn next_message(&mut self) {
        let win = &mut api::Window::current();
        self.goto_message(1, win);
    }
    pub fn prev_message(&mut self) {
        let win = &mut api::Window::current();
        self.goto_message(-1, win);
    }

    pub fn mut_chat<Callback>(&mut self, modifier: Callback) -> crate::Result<()>
    where
        Callback: FnOnce(&mut ChatState),
    {
        let prev_len = self.messages.len();
        modifier(self);
        self.update_config_tag_line()?;
        let new_len = self.messages.len();
        if new_len != prev_len {
            bar::StatusLineChatCache::outdate_all_pages(&self.buffer, new_len);
        }
        bar::StatusLineChatCache::outdate_page(&self.buffer, 0);
        Ok(())
    }

    pub fn mut_message_by_index_isize<Callback>(&mut self, index: isize, modifier: Callback) -> crate::Result<()>
    where
        Callback: FnOnce(&mut MessageState),
    {
        let len = self.messages.len().clone();
        let abs = index.abs() as usize;
        let msg_index = if index.is_negative() {
            len.saturating_sub(abs)
        } else {
            // std::cmp::min(abs, len - 1)
            if abs >= len {
                return Err("Index does not exists.".into_error());
            }
            abs
        };
        if let Some(mut message) = self.messages.get_mut(msg_index) {
            modifier(&mut message);
        }
        self.update_message_tag_line(msg_index)?;
        Ok(())
    }

    pub fn mut_message_by_index<Callback>(&mut self, index: usize, modifier: Callback) -> crate::Result<()>
    where
        Callback: FnOnce(&mut MessageState),
    {
        self.mut_message_by_index_isize(index as isize, modifier)
    }

    pub fn mut_last_message<Callback>(&mut self, modifier: Callback) -> crate::Result<()>
    where
        Callback: FnOnce(&mut MessageState),
    {
        self.mut_message_by_index_isize(-1, modifier)
    }

    pub fn mut_message_under_cursor<Callback>(&mut self, modifier: Callback) -> crate::Result<()>
    where
        Callback: FnOnce(&mut MessageState),
    {
        let win = api::Window::current();
        let Some(position) = self.get_position_index(&win) else {
            return Ok(());
        };
        let message_index = match position {
            0 => 0, // Header, so lets modify first message.
            position => position - 1,
        };
        self.mut_message_by_index(message_index, modifier)
    }

    // pub fn mut_prompt_message<Callback>(&mut self, modifier: Callback)
    // where
    //     Callback: FnOnce(&mut MessageState),
    // {
    //     if let Some(msg) = self.messages.last() {
    //         if matches!(msg.status, mistral::model::stream::Status::Initialised) {
    //             crate::notify::warn("Can't modify the prompt while is is sending.");
    //             return;
    //         }
    //     }
    //     self.mut_message_by_index(-1, modifier)
    // }

    fn rerun_tools(&mut self, state: &super::SharedState, message_index: MsgIndex, target: Option<Row>) {
        let Some(message) = self.messages.get_mut(message_index) else {
            crate::log_libuv!(Error, "Run tool : MessageIndex does not exist.");
            return;
        };
        let (Some(tools), Some(positions)) = (
            message.message.tool_calls.as_ref(),
            message.tool_calls_positions.clone(),
        ) else {
            crate::notify::info("No tool_calls to run.");
            return;
        };
        let mut mode = message.mode.clone();
        let tools = tools.clone();
        let following_messages = &mut self.messages[message_index..];
        'next_tool_call: for (tool, position) in tools.into_iter().zip(positions) {
            if let Some(target_row) = target
                && !position.contains(&target_row)
            {
                continue;
            }
            let Some(id) = tool.id.clone() else {
                crate::notify::warn("Can't run tool without an id.");
                continue;
            };
            let run_tool_message = crate::messages::RunToolMessage {
                buffer: self.buffer.clone(),
                tool,
            };
            let response = mode.run_tool(model::SharedState::clone(state), run_tool_message);
            for msg_response in following_messages.iter_mut() {
                if let Some(resp_id) = msg_response.message.tool_call_id.as_ref()
                    && *resp_id == id
                {
                    msg_response.message.content = response.content;
                    if target.is_some() {
                        break 'next_tool_call;
                    } else {
                        continue 'next_tool_call;
                    }
                }
            }
            crate::notify::warn("Tool Call's Message Response not found.");
        }
    }
    pub fn rerun_tools_under_cursor(&mut self, state: &super::SharedState) {
        let Some(Cursor { row, .. }) = Cursor::from_window_current() else {
            return;
        };
        let Some(msg_index) = self.positions.index_by_row(&row) else {
            crate::notify::info("Cursor not under a Message.");
            return;
        };
        self.rerun_tools(state, msg_index, None);
    }
    pub fn rerun_tool_under_cursor(&mut self, state: &super::SharedState) {
        let Some(Cursor { row, .. }) = Cursor::from_window_current() else {
            return;
        };
        let Some(msg_index) = self.positions.index_by_row(&row) else {
            crate::notify::info("Cursor not under a Message.");
            return;
        };
        self.rerun_tools(state, msg_index, Some(row));
    }
}

#[cfg(not(feature = "prod_mode"))]
#[track_caller]
pub fn show(buffer: &mut api::Buffer) {
    let lines = buffer_content(buffer);
    // crate::log_libuv!(Trace, "Content : START\n{}\nEND\n", lines);
    println!("Content : START\n{}\nEND\n", lines);
}

#[cfg(feature = "prod_mode")]
#[track_caller]
pub fn show(buffer: &mut api::Buffer) {
    let lines = buffer_content(buffer);
    crate::log_libuv!(Trace, "Content : START\n{}\nEND\n", lines);
}
// #[cfg(not(feature = "prod_mode"))]
pub fn buffer_content(buffer: &mut api::Buffer) -> String {
    let lines: Vec<String> = buffer
        .get_lines(.., false)
        .unwrap()
        .map(|ns| ns.to_string())
        .collect();
    lines.join("\n")
}

#[cfg(not(feature = "prod_mode"))]
#[track_caller]
pub fn assert_content(buffer: &mut api::Buffer, expected: impl ToString) {
    let content = buffer_content(buffer);
    println!("Content : START\n{}\nEND\n", content);
    let expected = expected.to_string();
    let content_split_count = content.chars().filter(|c| *c == '\n').count();
    let expected_split_count = expected.chars().filter(|c| *c == '\n').count();
    let content_split = content.split("\n");
    let expected_split = expected.split("\n");
    for (i, (line_modif, line_expected)) in content_split.zip(expected_split).enumerate() {
        assert_eq!(line_modif, line_expected, "\n\nDoes not match line n°{i}.\n");
    }
    assert_eq!(
        content_split_count, expected_split_count,
        "More/less lines than expected."
    );
}

#[cfg(not(feature = "prod_mode"))]
#[nvim_oxi::test]
#[track_caller]
fn chat_update_buffer() -> crate::Result<()> {
    const BUFFER_CONTENT: &'static str = r###"<CHAT  role="Refactorisation" status="0;0;0" model="Tu es un développeur qui a des outils à ta disposition." id="00000000-0000-0000-0000-000000000000"/>
<MESSAGE  role="System" model="Medium Latest" status="Created" usage="0;0;0"/>
Tu es un développeur qui a des outils à ta disposition.
<MESSAGE  role="User" model="Medium Latest" status="Created" usage="0;0;0" mode="CodeRefactorisation"/>
Peux-tu modifier la fonction main gràce aux outils, pour qu'elle affiche plop ?"###;

    const M0: &'static str = "Tu es un développeur qui a des outils à ta disposition.";
    const M1: &'static str = "Peux-tu modifier la fonction main gràce aux outils, pour qu'elle affiche plop ?";

    let buffer = &mut api::Buffer::current();
    buffer.set_lines(.., false, BUFFER_CONTENT.split('\n'))?;
    let mut chat = ChatState {
        is_running: None,
        path: Default::default(),
        buffer: buffer.clone(),
        buffer_modifier: None,
        metadata: ChatMetadata::default(),
        messages: Vec::default(),
        positions: MessagesPositions::default(),
    };
    chat.update_buffer(RowRange::FULL)?;
    show(buffer);

    let ms0 = &chat.messages[0];
    let m0 = &ms0.message;
    let ms1 = &chat.messages[1];
    let m1 = &ms1.message;
    let pos = &chat.positions.0;
    assert_eq!(m0.content, M0);
    assert_eq!(m1.content, M1);
    assert_eq!(chat.messages.len(), 2);
    assert_eq!(pos.len(), 3);
    assert_eq!(pos[0], (0..1).into());
    assert_eq!(pos[1], (1..3).into());
    assert_eq!(pos[2], (3..Row::MAX.0).into());
    assert_eq!(m0.role.to_string(), "System");
    assert_eq!(m1.role.to_string(), "User");
    assert_eq!(ms0.mode.to_string(), "None");
    assert_eq!(ms1.mode.to_string(), "CodeRefactorisation");
    assert_eq!(buffer_content(buffer), BUFFER_CONTENT);

    chat.mut_last_message(|msg| msg.status = mistral::model::stream::Status::Initialised)?;
    chat.mut_last_message(|msg| msg.status = mistral::model::stream::Status::Created)?;
    chat.update_buffer(RowRange::FULL)?;
    assert_eq!(buffer_content(buffer), BUFFER_CONTENT);
    let mut win = api::Window::current();
    win.set_cursor(4, 0)?;
    chat.update_prompt()?;
    let envelop = chat.build_request_envelop()?;
    const M2: &'static str = "";
    show(buffer);
    let (m0, m1) = match &envelop.message {
        crate::messages::NvimMessage::Chat(chat_request) => {
            let m = &chat_request.completion.messages;
            (&m[0], &m[1])
        }
        _ => return Err("Expect a Chat Request.".into_error()),
    };
    assert_eq!(m0.content, M0);
    assert_eq!(m1.content, M1);
    // Simulate  initialisation
    chat.push_new_message(None)?;

    let ms0 = &chat.messages[0];
    let m0 = &ms0.message;
    let ms1 = &chat.messages[1];
    let m1 = &ms1.message;
    let ms2 = &chat.messages[2];
    let m2 = &ms2.message;
    let pos = &chat.positions.0;
    assert_eq!(chat.messages.len(), 3);
    assert_eq!(pos.len(), 4);
    assert_eq!(m1.role.to_string(), "User");
    assert_eq!(m2.role.to_string(), "Assistant");
    assert_eq!(pos[0], (0..1).into());
    assert_eq!(pos[1], (1..3).into());
    assert_eq!(pos[2], (3..7).into());
    assert_eq!(pos[3], (7..Row::MAX.0).into());
    assert_eq!(m0.content, M0);
    assert_eq!(m1.content, M1);
    assert_eq!(m2.content, M2);
    show(buffer);
    assert_eq!(Row::buf_last_row(buffer).unwrap(), Row(8));
    chat.update_buffer(RowRange::FULL)?;
    let ms0 = &chat.messages[0];
    let m0 = &ms0.message;
    let ms1 = &chat.messages[1];
    let m1 = &ms1.message;
    let ms2 = &chat.messages[2];
    let m2 = &ms2.message;
    assert_eq!(m0.content, M0);
    assert_eq!(m1.content, M1);
    assert_eq!(m2.content, M2);

    // Test tool_call insertion
    let js = "{\"id\":\"1QBcNzvu0\",\"function\":{\"name\":\"CodeRetriever\",\"arguments\":\"{\\\"file\\\": \\\"src/main.rs\\\"}\"},\"index\":0}";
    let tool_call: mistral::model::ToolCall = serde_json::from_str(js)?;
    assert_eq!(tool_call.function.arguments, "{\"file\": \"src/main.rs\"}");
    chat.push_tool_call(&tool_call, None)?;
    let lines = buffer.get_lines(.., false)?.map(|v| v.to_string());
    // let expected =
    //     r#"<TOOLCALL id="1QBcNzvu0" index="0" name="CodeRetriever" arguments="{\"file\": \"src/main.rs\"}"/>"#;
    let expected = [
        "",
        "",
        r#"<TOOLCALL id="1QBcNzvu0" index="0" name="CodeRetriever">"#,
        "",
        "```json",
        r#"{"file": "src/main.rs"}"#,
        "```",
        "</TOOLCALL>",
    ];
    assert_eq!(lines.skip(8).take(8).collect::<Vec<_>>(), expected);
    show(buffer);
    chat.update_buffer(RowRange::FULL)?;
    assert_eq!(chat.messages.len(), 3);
    let ms2 = &chat.messages[2];
    let m2 = &ms2.message;
    assert!(m2.tool_calls.is_some());
    let tcs = m2.tool_calls.as_ref().unwrap();
    assert_eq!(tcs.len(), 1);
    assert_eq!(tcs[0].id, Some("1QBcNzvu0".to_string()));
    assert_eq!(tcs[0].index, Some(0));
    assert_eq!(tcs[0].function.name, "CodeRetriever");
    assert_eq!(tcs[0].function.arguments, "{\"file\": \"src/main.rs\"}");

    if let Some(msg_target) = chat.messages.get(1) {
        use crate::mistral::model::Role;
        if matches!(msg_target.message.role, Role::User) {
            chat.push_new_message(None)?;
            chat.mut_message_by_index_isize(-1, |msg| {
                msg.message.role = Role::User;
            })?;
        }
    }
    assert_eq!(chat.positions.0.len(), 5);
    assert_eq!(chat.messages.len(), 4);
    Ok(())
}
