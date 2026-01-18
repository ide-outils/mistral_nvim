use serde::{Deserialize, Serialize};

use super::{
    message::{Message, Role},
    tools::ToolCall,
};
use crate::{
    messages::{IdMessage, MistralMessage},
    mistral::controlleur::fim::SenderHandle,
};

#[derive(Deserialize)]
pub struct StreamEvent {
    pub choices: Vec<StreamChoice>,
    #[allow(dead_code)]
    pub object: String,
    #[allow(dead_code)]
    pub created: u32,
    #[allow(dead_code)]
    pub model: super::completion::Model,
    pub usage: Option<Usage>,
    // No idea what it is, it seems to appear during tool call.
    // Example : r##"...,\"p\":\"abcdefghijklmnopqrstuvwxyz0\"}"##
    pub p: Option<String>,
}

#[derive(Deserialize)]
pub struct Delta {
    content: Option<String>,
    role: Option<Role>,
    tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

impl From<String> for Usage {
    fn from(value: String) -> Self {
        let mut split = value.split(';');
        fn parse<'it>(next: Option<&str>) -> u32 {
            next.map(|s| s.parse().unwrap_or_default())
                .unwrap_or_default()
        }
        Self {
            prompt_tokens: parse(split.next()),
            completion_tokens: parse(split.next()),
            total_tokens: parse(split.next()),
        }
    }
}

impl std::fmt::Display for Usage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{};{};{}",
            self.prompt_tokens, self.completion_tokens, self.total_tokens
        )
    }
}

impl std::ops::AddAssign for Usage {
    fn add_assign(&mut self, rhs: Self) {
        *self += &rhs;
    }
}

impl std::ops::AddAssign<&Self> for Usage {
    fn add_assign(&mut self, rhs: &Self) {
        self.prompt_tokens += rhs.prompt_tokens;
        self.completion_tokens += rhs.completion_tokens;
        self.total_tokens += rhs.total_tokens;
    }
}

#[derive(Deserialize)]
pub struct StreamChoice {
    #[allow(dead_code)]
    pub index: u32,
    pub delta: Delta,
    #[allow(dead_code)]
    pub finish_reason: Option<String>,
}

#[derive(Debug)]
pub struct StreamResponse {
    pub message: Message,
    pub status: Status,
    pub usage: Usage,
}

impl StreamResponse {
    pub fn new() -> Self {
        StreamResponse {
            message: Message::default(),
            status: Status::Completed,
            usage: Usage::default(),
        }
    }

    pub async fn add_delta(&mut self, other: Delta, sendle: SenderHandle, id: IdMessage) -> Result<(), std::io::Error> {
        // logs!("SEND_DATA");
        let Delta {
            role,
            content,
            tool_calls,
        } = other;
        if let Some(role) = role {
            self.message.role = role.clone();
            sendle.send(id, MistralMessage::UpdateRole(role));
        }
        if let Some(content) = content {
            self.message.content += &content;
            let chunk = content.split('\n').map(ToString::to_string).collect();
            sendle.send(id, MistralMessage::UpdateContent(chunk));
        }
        if let Some(tool_calls) = tool_calls {
            sendle.send(id, MistralMessage::RunTool(tool_calls.clone()));
            match &mut self.message.tool_calls {
                Some(msg_tool_calls) => {
                    msg_tool_calls.extend(tool_calls);
                }
                none_ptr => {
                    *none_ptr = Some(tool_calls);
                }
            }
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub enum Status {
    Completed,
    Partial(String),
    Failed(String, ErrorMessageType),
    //
    #[default]
    Created,
    Initialised,
}
impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Status::*;
        let content = match self {
            Completed => "Completed",
            Partial(err) => &format!("Partial : {err}"),
            Failed(err, errors) => &format!("Failed : {err} {errors}"),
            Created => "Created",
            Initialised => "Initialised",
        };
        write!(f, "{content}")
    }
}
impl Status {
    pub fn replace_from_str(&mut self, value: &str) {
        match value {
            "Completed" => *self = Self::Completed,
            "Created" => *self = Self::Created,
            "Initialised" => *self = Self::Initialised,
            _ => {
                if value.starts_with("Partial : ") {
                    *self = Self::Partial(value.chars().skip(10).collect())
                } else if value.starts_with("Failed : ") {
                    *self = Self::Partial(value.chars().skip(9).collect())
                }
            }
        }
    }
}

#[derive(Serialize)]
pub struct StreamParam {
    pub stream: bool,
}

#[derive(Deserialize)]
pub struct StreamError {
    pub object: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub message: ErrorMessageType,
    pub param: Option<String>,
    pub code: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(untagged)]
pub enum ErrorMessageType {
    Details(ErrorMessage),
    Simple(String),
    #[default]
    Empty,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct ErrorMessage {
    pub detail: Vec<ErrorDetail>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ErrorDetail {
    #[serde(rename = "type")]
    pub type_: String,
    pub loc: Vec<String>,
    pub msg: String,
    // pub input: WhatHaveBeenSent,
}

impl std::fmt::Display for ErrorMessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorMessageType::Details(errors) => {
                write!(f, "{errors}")
            }
            ErrorMessageType::Simple(error) => {
                write!(f, "{error}")
            }
            ErrorMessageType::Empty => {
                write!(f, "")
            }
        }
    }
}

impl std::fmt::Display for ErrorMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "[")?;
        for detail in &self.detail {
            writeln!(f, "    {detail},")?;
        }
        write!(f, "]")
    }
}

impl std::fmt::Display for ErrorDetail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { type_, loc, msg } = self;
        write!(f, "{loc:?} {type_} : « {msg} »")
    }
}
