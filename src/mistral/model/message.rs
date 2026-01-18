use mistral_nvim_derive::Form;
use serde::{Deserialize, Serialize};

use super::ToolCall;

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct Message {
    pub role: Role,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub prefix: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize, Form, Clone, Debug, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Role {
    User,
    System,
    #[default]
    Assistant,
    Tool,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::User => "User",
            Self::System => "System",
            Self::Assistant => "Assistant",
            Self::Tool => "Tool",
        };
        write!(f, "{name}")
    }
}

impl Role {
    pub fn replace_from_str(&mut self, value: &str) {
        // crate::notify::error(&format!("Replace Role : {}", value));
        match value {
            "User" => *self = Self::User,
            "System" => *self = Self::System,
            "Assistant" => *self = Self::Assistant,
            "Tool" => *self = Self::Tool,
            _ => (),
        }
    }
}

impl Message {
    pub fn tool_name_does_not_exist(wrong_name: &str, existing_names: Vec<&str>) -> String {
        format!("Failed : tool '{wrong_name}' does not exist. Existing names are : {existing_names:?}")
    }
}
