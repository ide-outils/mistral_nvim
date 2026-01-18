use mistral_nvim_derive::Form;
use serde::{Deserialize, Serialize};

use crate::mistral::model::{message::Message, tools::Tool};

/// The list of Mistral Models
#[derive(Form, Serialize, Deserialize, Clone, Default, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum Model {
    MistralLargeLatest,
    #[default]
    MistralMediumLatest,
    MistralTinyLatest,
    MistralNemoLatest,
    CodestralLatest,
    Codestral2405,
    DevstralMediumLatest,
    MagistralMediumLatest,
    PixtralLargeLatest,
    VoxtralMiniLatest,
    MistralOcrLatest,
    Ministral3bLatest,
    Ministral8bLatest,
}
impl std::fmt::Display for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::MistralLargeLatest => "Large Latest",
            Self::MistralMediumLatest => "Medium Latest",
            Self::MistralTinyLatest => "Tiny Latest",
            Self::MistralNemoLatest => "Nemo Latest",
            Self::CodestralLatest => "Codestral Latest",
            Self::Codestral2405 => "Codestra l2405",
            Self::DevstralMediumLatest => "Devstral Medium Latest",
            Self::MagistralMediumLatest => "Magistral Medium Latest",
            Self::PixtralLargeLatest => "Pixtral Large Latest",
            Self::VoxtralMiniLatest => "Voxtral Mini Latest",
            Self::MistralOcrLatest => "Ocr Latest",
            Self::Ministral3bLatest => "Ministral 3b Latest",
            Self::Ministral8bLatest => "Ministral 8b Latest",
        };
        write!(f, "{name}")
    }
}
impl Model {
    pub fn replace_from_str(&mut self, value: &str) {
        match value {
            "Large Latest" => *self = Self::MistralLargeLatest,
            "Medium Latest" => *self = Self::MistralMediumLatest,
            "Tiny Latest" => *self = Self::MistralTinyLatest,
            "Nemo Latest" => *self = Self::MistralNemoLatest,
            "Codestral Latest" => *self = Self::CodestralLatest,
            "Codestra l2405" => *self = Self::Codestral2405,
            "Devstral Medium Latest" => *self = Self::DevstralMediumLatest,
            "Magistral Medium Latest" => *self = Self::MagistralMediumLatest,
            "Pixtral Large Latest" => *self = Self::PixtralLargeLatest,
            "Voxtral Mini Latest" => *self = Self::VoxtralMiniLatest,
            "Ocr Latest" => *self = Self::MistralOcrLatest,
            "Ministral 3b Latest" => *self = Self::Ministral3bLatest,
            "Ministral 8b Latest" => *self = Self::Ministral8bLatest,
            _ => (),
        }
    }
}

impl Model {
    pub fn fim() -> Self {
        Self::CodestralLatest
    }
}

#[derive(Serialize, Default)]
pub struct ChatCompletion {
    pub model: Model,
    pub messages: Vec<Message>,
}

#[derive(Serialize)]
pub struct ChatRequest {
    #[serde(flatten)]
    pub completion: ChatCompletion,
    #[serde(flatten)]
    pub params: CompletionParams,
}

#[derive(Serialize)]
pub struct FimRequest {
    #[serde(flatten)]
    pub completion: FimCompletion,
    #[serde(flatten)]
    pub params: CompletionParams,
}

#[derive(Serialize, Default)]
pub struct FimCompletion {
    #[serde(default = "Model::fim")]
    pub model: Model,
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub suffix: Option<String>,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct CompletionParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_deserializing)] // Not used by ChatState
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
}
