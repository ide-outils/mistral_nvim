use nvim_oxi::api;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::{
    mistral,
    nvim::{
        self,
        model::{Cursor, state::chat::MsgIndex},
    },
    utils::notify::NotifyLevel,
};

pub type NvimSender = UnboundedSender<NvimEnveloppe>;
pub type NvimReceiver = UnboundedReceiver<NvimEnveloppe>;

pub type MistralSender = UnboundedSender<MistralEnveloppe>;
pub type MistralReceiver = UnboundedReceiver<MistralEnveloppe>;

pub type BufferHandle = i32;

#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum IdMessage {
    FIM(BufferHandle, usize),
    Chat(BufferHandle, MsgIndex),
}

pub struct MistralEnveloppe {
    pub id: IdMessage,
    pub message: MistralMessage,
}
pub enum MistralMessage {
    InitializeTask(nvim::model::Cursor),
    // InitializeTask { id: Uuid, cursor: nvim::model::Cursor },
    UpdateContent(Vec<String>),
    UpdateRole(mistral::model::Role),
    // UpdateContent { id: Uuid, chunk: Vec<String> },
    RunTool(Vec<mistral::model::ToolCall>),
    FinalizeTask(mistral::model::stream::StreamResponse),
    Notify { message: String, level: NotifyLevel },
}
pub struct RunToolMessage {
    pub buffer: api::Buffer,
    pub tool: mistral::model::ToolCall,
}
impl RunToolMessage {
    pub fn create_mistral_message(&self, content: impl ToString) -> mistral::model::Message {
        let tool_call_id = self.tool.id.clone();
        let name = self.tool.function.name.clone();
        mistral::model::Message {
            role: mistral::model::Role::Tool,
            content: content.to_string(),
            name: Some(name),
            tool_call_id,
            ..Default::default()
        }
    }
}

impl MistralEnveloppe {
    pub fn notify(id: IdMessage, level: NotifyLevel, message: impl ToString) -> Self {
        Self {
            id,
            message: MistralMessage::Notify {
                message: message.to_string(),
                level,
            },
        }
    }
    pub fn notify_error(id: IdMessage, message: impl ToString) -> Self {
        Self::notify(id, NotifyLevel::Error, message)
    }
    pub fn notify_warn(id: IdMessage, message: impl ToString) -> Self {
        Self::notify(id, NotifyLevel::Warn, message)
    }
}

pub struct NvimEnveloppe {
    pub id: IdMessage,
    pub message: NvimMessage,
}

pub enum NvimMessage {
    Abort,
    FimCursorLine(Normal),
    FimFunction(Normal),
    // FimStatement(Normal),
    // FimStructure(Normal),
    FimVisual(Visual),
    Chat(mistral::model::completion::ChatRequest),
}

pub struct Normal {
    pub data: nvim::model::BufferData,
}

pub struct Visual {
    pub data: nvim::model::BufferData,
    pub selection: nvim::model::Selection,
}

impl Visual {
    pub fn get_selected_content(&self) -> (String, nvim::model::Cursor) {
        let Visual {
            data: nvim::model::BufferData { content, .. },
            selection: nvim::model::Selection { start, end },
            ..
        } = self;
        // let nvim::model::Selection { start, end } = self.selection;
        if content.is_empty() {
            return (String::new(), nvim::model::Cursor::zero());
        }
        // FIXME: Ensure selection is not OutOfBounds
        let mut lines = content[*start.row..*end.row].to_vec();
        if lines.len() == 0 {
            return (String::new(), nvim::model::Cursor::zero());
        }
        let last_index = lines.len() - 1;
        // Now we work with chars, so we can't use String::len().
        // Do last_index first in case only one line is selected.
        let last_line = lines[last_index].to_string();
        let len_last_col = (*end.col + 1).min(last_line.chars().count());
        lines[last_index] = last_line.chars().take(len_last_col).collect();

        let first_line = lines[0].to_string();
        lines[0] = first_line
            .chars()
            .skip((*start.col).min(first_line.chars().count()))
            .collect();
        let cursor = Cursor {
            row: end.row,
            col: len_last_col.into(),
        };
        (lines.join("\n"), cursor)
    }
}
