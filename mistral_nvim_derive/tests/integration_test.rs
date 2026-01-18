use std::collections::HashMap;

use mistral_nvim_derive::{FunctionParameters, PropertyType, Tool};
use serde::Deserialize;

#[allow(dead_code)]
#[derive(PropertyType, Deserialize)]
struct MyStruct {
    value: usize,
}

#[allow(dead_code)]
#[derive(PropertyType, Deserialize)]
enum MyEnum {
    Alright(),
}

#[allow(dead_code)]
#[derive(FunctionParameters, Deserialize)]
#[description("Tool qui ne prends pas de paramettes")]
#[param_name("enum_tool")]
#[param_description("Set the new EnumTool value.")]
enum SetEnumTool {
    A,
}

#[allow(dead_code)]
#[derive(Tool, Deserialize)]
#[description("Tool qui ne prends pas de paramettes")]
struct NoArgsTool {}

impl Runnable for NoArgsTool {
    type Ok = ();
    type Err = ();

    fn run(self) -> RunResult<Self::Ok, Self::Err> {
        todo!()
    }
}

#[allow(dead_code)]
#[derive(Tool, Deserialize)]
#[description("A commande that do something.")]
struct CommandTool {}

impl Runnable for CommandTool {
    type Ok = ();
    type Err = ();

    fn run(self) -> RunResult<Self::Ok, Self::Err> {
        todo!()
    }
}

#[allow(dead_code)]
#[derive(FunctionParameters, Deserialize)]
struct ActionArgs {
    #[description("Desc arg1 blabla")]
    arg1: bool,
    #[description("Whatever")]
    arg2: Option<String>,
    #[description("somethhing")]
    arg3: usize,
    #[description("something")]
    arg4: MyStruct,
    #[description("enum")]
    arg5: Option<MyEnum>,
}

pub enum ToolType {
    Function,
}

pub struct Tool {
    pub type_: ToolType,
    pub function: FunctionDefinition,
}

#[derive(Debug, PartialEq)]
pub enum FunctionParametersType {
    Object,
}
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: FunctionParameters,
}

pub struct FunctionParameters {
    pub type_: FunctionParametersType,
    pub required: Vec<String>,
    pub properties: HashMap<String, FunctionParametersProperty>,
}

pub struct FunctionParametersProperty {
    pub type_: PropertyTypeWrapper,
    pub description: String,
}

pub mod extension {
    use serde::{Deserialize, Serialize};

    use super::*;

    /// Received from Mistral and send back in contextual completions
    #[derive(Serialize, Deserialize, Clone)]
    pub struct ToolCall {
        pub function: FunctionCall,
        pub id: Option<String>,
        #[serde(rename = "type")]
        pub type_: ToolType,
        pub index: Option<u32>,
    }
    #[derive(Serialize, Deserialize, Clone, Default)]
    #[serde(rename_all = "kebab-case")]
    pub enum ToolType {
        #[default]
        Function,
    }

    #[derive(Serialize, Deserialize, Clone)]
    pub struct FunctionCall {
        pub name: String,
        pub arguments: String,
    }

    #[derive(Serialize, Deserialize, Clone, Default)]
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

    #[derive(Serialize, Deserialize, Clone, Default)]
    #[serde(rename_all = "kebab-case")]
    pub enum Role {
        User,
        System,
        #[default]
        Assistant,
        Tool,
    }

    #[derive(Serialize)]
    pub enum RunResult<Ok = (), Error = ()>
    where
        Ok: Serialize,
        Error: Serialize,
    {
        Success(Ok),
        Failed(Error),
    }

    pub trait ToolList {
        fn tools() -> Vec<Tool>;
        fn run(tool_call: ToolCall) -> Result<Message, String>;
    }

    /// The easiest way to implement this type is to use the dedicated derive :
    /// ```rust
    /// use mistral_nvim_derive::Tool;
    /// use crate::mistral::model::tools::*;
    ///
    /// #[derive(Tool)]
    /// #[description("Get a usless value somewhere on the local machine.")]
    /// struct Data {
    ///     #[description("A useless value.")]
    ///     value: usize,
    /// }
    /// ```
    pub trait ToolExt: FunctionParametersExt + Runnable {
        fn get_tool() -> Tool;
    }

    /// The easiest way to implement this type is to use the dedicated derive :
    /// ```rust
    /// use mistral_nvim_derive::FunctionParameters;
    /// use crate::mistral::model::tools::*;
    ///
    /// #[derive(FunctionParameters)]
    /// struct Data {
    ///     #[description("A useless value.")]
    ///     value: usize,
    /// }
    /// ```
    pub trait FunctionParametersExt: PropertyTypeExt {
        fn get_function_parameters() -> FunctionParameters;
    }

    /// The easiest way to implement this type is to use the dedicated derive :
    /// ```rust
    /// use mistral_nvim_derive::PropertyType;
    /// use crate::mistral::model::tools::*;
    ///
    /// #[derive(PropertyType)]
    /// struct Data {
    ///     value: usize,
    /// }
    /// ```
    pub trait PropertyTypeExt: for<'de> Deserialize<'de> {
        fn get_property_type() -> PropertyType;
    }

    /// The `Tool`'s logic.
    pub trait Runnable: FunctionParametersExt {
        type Ok: Serialize;
        type Err: Serialize;
        fn run(self) -> RunResult<Self::Ok, Self::Err>;
        fn parse_and_run(args: &String) -> serde_json::Result<String> {
            let run_result = serde_json::from_str::<Self>(args)?.run();
            serde_json::to_string(&run_result)
        }
    }

    #[derive(Clone)]
    pub struct PropertyTypeWrapper(pub PropertyType);
    impl Serialize for PropertyTypeWrapper {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            match serde_json::to_string(&self.0) {
                Ok(json_str) => serializer.serialize_str(json_str.as_str()),
                _err => serializer.serialize_none(),
            }
        }
    }

    #[derive(Debug, Serialize, PartialEq, Clone)]
    pub enum PropertyType {
        Object(HashMap<String, PropertyType>),
        Enum(HashMap<String, PropertyType>),
        List(Vec<PropertyType>),
        Option(Box<PropertyType>),
        Str,
        Integer,
        Float,
        Boolean,
        Flag,
    }
}

pub use extension::{
    FunctionParametersExt, PropertyType, PropertyTypeExt, PropertyTypeWrapper, RunResult, Runnable, ToolExt,
};

#[test]
fn test_function_parameters_derive() {
    let tool = CommandTool::get_tool();
    assert_eq!(tool.function.name, "CommandTool");
    assert_eq!(tool.function.description, "A commande that do something.");

    let params: FunctionParameters = ActionArgs::get_function_parameters();

    // Vérifie les champs requis
    assert_eq!(params.required, vec!["arg1", "arg3", "arg4"]);

    // Vérifie les paramètres
    let mut expected_parameters = HashMap::new();
    expected_parameters.insert(
        "arg1".to_string(),
        FunctionParametersProperty {
            type_: PropertyTypeWrapper(PropertyType::Boolean),
            description: "Desc arg1 blabla".to_string(),
        },
    );
    expected_parameters.insert(
        "arg2".to_string(),
        FunctionParametersProperty {
            type_: PropertyTypeWrapper(PropertyType::Str),
            description: "Whatever".to_string(),
        },
    );
    expected_parameters.insert(
        "arg3".to_string(),
        FunctionParametersProperty {
            type_: PropertyTypeWrapper(PropertyType::Integer),
            description: "somethhing".to_string(),
        },
    );
    expected_parameters.insert(
        "arg4".to_string(),
        FunctionParametersProperty {
            type_: PropertyTypeWrapper(PropertyType::Object(HashMap::from([(
                "value".to_string(),
                PropertyType::Integer,
            )]))),
            description: "something".to_string(),
        },
    );
    expected_parameters.insert(
        "arg5".to_string(),
        FunctionParametersProperty {
            type_: PropertyTypeWrapper(PropertyType::Enum(HashMap::from([(
                "Alright".to_string(),
                PropertyType::List(Default::default()),
            )]))),
            description: "enum".to_string(),
        },
    );

    assert_eq!(params.properties.len(), expected_parameters.len());
    // assert_eq!(params.properties, expected_parameters);
}
