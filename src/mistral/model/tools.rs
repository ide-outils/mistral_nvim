use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Received from Mistral and send back in contextual completions
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct ToolCall {
    pub function: FunctionCall,
    pub id: Option<String>,
    // #[serde(rename = "type")]
    // pub type_: String,
    pub index: Option<u32>,
}

// --- ToolCall --- //

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

pub type Tool = schemars::Schema;
// /// Send to Mistral to list the available tools
// #[derive(Serialize, Clone)]
// pub struct Tool {
//     #[serde(rename = "type")]
//     pub type_: ToolType,
//     pub function: FunctionDefinition,
// }

// #[derive(Serialize, Deserialize, Clone, Default)]
// #[serde(rename_all = "kebab-case")]
// pub enum FunctionParametersType {
//     #[default]
//     Object,
// }
// #[derive(Clone)]
// pub struct JsFunction {
//     pub name: String,
//     pub description: String,
//     pub parameters: JsType,
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tool() -> crate::Result<()> {
        let stream = r##"{"id":"0de469cd61ec43918044cdafe2b29b57","object":"chat.completion.chunk","created":1767040920,"model":"mistral-medium-latest","choices":[{"index":0,"delta":{"tool_calls":[{"id":"uKCZB9XEy","function":{"name":"CodeRetriever","arguments":"{\"file\": \"main.rs\"}"},"index":0}]},"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":388,"total_tokens":401,"completion_tokens":13},"p":"abc"}"##;
        let call_json = &stream[171..275];
        assert_eq!(
            call_json,
            r##"[{"id":"uKCZB9XEy","function":{"name":"CodeRetriever","arguments":"{\"file\": \"main.rs\"}"},"index":0}]"##
        );
        let calls: Vec<ToolCall> = serde_json::from_str(call_json)?;
        crate::log_libuv!(Trace, "{:?}", calls);
        let args = &calls[0].function.arguments;
        use crate::nvim::model::tool_mode::code_refactorisation::CodeRetriever;
        let retriever = serde_json::from_str::<CodeRetriever>(args)?;
        assert_eq!(
            retriever,
            CodeRetriever {
                file: "main.rs".to_string()
            }
        );
        Ok(())
    }
}

/// Extends mistral API with our own models
pub mod extension {
    use serde::ser::SerializeStruct as _;

    use super::*;
    use crate::{
        messages::RunToolMessage,
        mistral::model::message::{Message, Role},
        nvim::model::SharedState,
    };

    #[derive(Serialize)]
    pub enum RunResult<Ok = (), Err = ()>
    where
        Ok: Serialize,
        Err: Serialize,
    {
        Ok(Ok),
        Err(Err),
    }

    pub trait ToolListExt {
        fn get_tools() -> Vec<Tool>;
        fn run_tool(state: SharedState, msg: RunToolMessage) -> serde_json::Result<String>;
        fn run(state: SharedState, msg: RunToolMessage) -> crate::mistral::model::Message {
            let tool_call_id = msg.tool.id.clone();
            let name = msg.tool.function.name.clone();
            let content = match Self::run_tool(state, msg) {
                Ok(ok) => ok,
                Err(_json_error) => "Error: Argumentts can't be deserialized. Expected json arguments.".to_string(),
            };
            Message {
                role: Role::Tool,
                content,
                name: Some(name),
                tool_call_id,
                ..Default::default()
            }
        }
    }

    /// The `Tool`'s logic.
    pub trait Runnable: schemars::JsonSchema + for<'de> Deserialize<'de> {
        type Ok: Serialize;
        type Err: Serialize;
        fn run(&mut self, state: SharedState, msg: RunToolMessage) -> std::result::Result<Self::Ok, Self::Err>;
        fn parse(msg: &RunToolMessage) -> serde_json::Result<Self> {
            let args = &msg.tool.function.arguments;
            serde_json::from_str::<Self>(args)
        }
        fn parse_and_run(state: SharedState, msg: RunToolMessage) -> serde_json::Result<String> {
            let run_result = Self::parse(&msg)?.run(state, msg);
            serde_json::to_string(&run_result)
        }
    }

    #[derive(Serialize, Deserialize, Clone, Default)]
    #[serde(rename_all = "kebab-case")]
    pub enum FunctionParametersType {
        #[default]
        Object,
    }

    #[derive(Clone)]
    pub struct JsFunction {
        pub name: String,
        pub description: String,
        pub parameters: schemars::Schema,
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
    pub trait ToolExt: Runnable {
        fn get_tool() -> Tool {
            // let name = Self::get_form().get_name().unwrap_or_default();
            // let description = Self::get_form().get_description().unwrap_or_default();
            let mut parameters = schemars::schema_for!(Self);
            let description = parameters.remove("description");
            let title = parameters.remove("title");
            let _schema = parameters.remove("$schema");
            schemars::json_schema!({
                // "$schema": schema,
                "type": "function",
                "function": {
                    "name": title,
                    "description": description,
                    "parameters": parameters,
                }
            })
        }
    }

    #[derive(Debug, Serialize, PartialEq, Eq, Clone)]
    pub struct Name(String);
    impl std::fmt::Display for Name {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }
    impl std::ops::Deref for Name {
        type Target = String;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl From<&str> for Name {
        fn from(value: &str) -> Self {
            Self(value.to_string())
        }
    }
    #[derive(Debug, PartialEq, Eq, Clone)]
    pub struct Description(String);
    impl std::fmt::Display for Description {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }
    impl std::ops::Deref for Description {
        type Target = String;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl From<&str> for Description {
        fn from(value: &str) -> Self {
            Self(value.to_string())
        }
    }
    #[derive(Debug, PartialEq, Eq, Clone)]
    pub struct DefaultVariant(String);
    impl std::fmt::Display for DefaultVariant {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }
    impl std::ops::Deref for DefaultVariant {
        type Target = String;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl From<&str> for DefaultVariant {
        fn from(value: &str) -> Self {
            Self(value.to_string())
        }
    }
    #[derive(Debug, PartialEq, Eq, Clone)]
    pub struct FormField {
        pub name: Name,
        pub description: Description,
        pub form: RForm,
    }
    impl Serialize for FormField {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let mut state = serializer.serialize_struct("FormField", 3)?;
            state.serialize_field("name", &self.name.0)?;
            state.serialize_field("form", &*self.form)?;
            if self.description.0 != "" {
                state.serialize_field("description", &self.description.0)?;
            }
            state.end()
        }
    }
    impl FormField {
        pub fn tuple_ref<'d>(&'d self) -> (&'d String, &'d String, &'d RForm) {
            let Self {
                name,
                description,
                form,
            } = self;
            (&**name, &**description, form)
        }
        pub fn tuple(self) -> (String, String, RForm) {
            let Self {
                name,
                description,
                form,
            } = self;
            (name.to_string(), description.to_string(), form)
        }
    }
    impl From<(&str, &str, Form)> for FormField {
        fn from(value: (&str, &str, Form)) -> Self {
            let (name, description, form) = value;
            Self {
                name: name.into(),
                description: description.into(),
                form: RForm::new(form),
            }
        }
    }
    impl From<(&str, &str, RForm)> for FormField {
        fn from(value: (&str, &str, RForm)) -> Self {
            let (name, description, form) = value;
            Self {
                name: name.into(),
                description: description.into(),
                form,
            }
        }
    }

    pub type RForm = std::sync::Arc<Form>;
    #[derive(Debug, PartialEq, Eq, Clone)]
    pub enum Form {
        // --- Named types ---
        Struct(Name, Description, Vec<FormField>),
        Enum(Name, Description, DefaultVariant, Vec<FormField>),
        StructTuple(Name, Description, Vec<FormField>), // FormField with enpty names

        // --- Unnamed types ---
        List(Box<RForm>),
        Option(Box<RForm>),
        Tuple(Vec<RForm>),
        Map(Box<(RForm, RForm)>),

        // --- Base types ---
        Boolean,
        Float,
        Integer,
        Str,
        Unit,
    }
    // impl Form {
    //     fn get_name(&self) -> Option<String> {
    //         match self {
    //             Form::StructTuple(name, _, _) | Form::Struct(name, _, _) | Form::Enum(name, _, _, _) => {
    //                 Some(name.to_string())
    //             }
    //             _ => None,
    //         }
    //     }
    //     fn get_description(&self) -> Option<String> {
    //         match self {
    //             Form::StructTuple(_, desc, _) | Form::Struct(_, desc, _) | Form::Enum(_, desc, _, _) => {
    //                 Some(desc.to_string())
    //             }
    //             _ => None,
    //         }
    //     }
    // }
    impl Serialize for Form {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            match self {
                // Types nommés
                Form::Struct(name, desc, fields) => {
                    let mut state = serializer.serialize_struct("Struct", 3)?;
                    state.serialize_field("type", "struct")?;
                    state.serialize_field("name", name)?;
                    if desc.0 != "" {
                        state.serialize_field("description", &desc.0)?;
                    }
                    state.serialize_field("fields", fields)?;
                    state.end()
                }
                Form::Enum(name, desc, default, fields) => {
                    let mut state = serializer.serialize_struct("Enum", 4)?;
                    state.serialize_field("type", "enum")?;
                    state.serialize_field("name", name)?;
                    if desc.0 != "" {
                        state.serialize_field("description", &desc.0)?;
                    }
                    if default.0 != "" {
                        state.serialize_field("default", &default.0)?;
                    }
                    state.serialize_field("fields", fields)?;
                    state.end()
                }
                Form::StructTuple(name, desc, fields) => {
                    let mut state = serializer.serialize_struct("StructTuple", 3)?;
                    state.serialize_field("type", "struct_tuple")?;
                    state.serialize_field("name", name)?;
                    if desc.0 != "" {
                        state.serialize_field("description", &desc.0)?;
                    }
                    state.serialize_field("fields", fields)?;
                    state.end()
                }

                // Types non-nommés
                Form::List(form) => {
                    let mut state = serializer.serialize_struct("List", 1)?;
                    state.serialize_field("type", "list")?;
                    state.serialize_field("element", &***form)?;
                    state.end()
                }
                Form::Option(form) => {
                    let mut state = serializer.serialize_struct("Option", 1)?;
                    state.serialize_field("type", "option")?;
                    state.serialize_field("element", &***form)?;
                    state.end()
                }
                Form::Tuple(forms) => {
                    let mut state = serializer.serialize_struct("Tuple", 1)?;
                    state.serialize_field("type", "tuple")?;
                    state.serialize_field("elements", &forms.iter().map(|f| &**f).collect::<Vec<_>>())?;
                    state.end()
                }
                Form::Map(map) => {
                    let (key, value) = &**map;
                    let mut state = serializer.serialize_struct("Map", 1)?;
                    state.serialize_field("type", "map")?;
                    state.serialize_field("key", &**key)?;
                    state.serialize_field("value", &**value)?;
                    state.end()
                }

                // Types de base
                Form::Boolean => serializer.serialize_str("boolean"),
                Form::Float => serializer.serialize_str("float"),
                Form::Integer => serializer.serialize_str("integer"),
                Form::Str => serializer.serialize_str("string"),
                Form::Unit => serializer.serialize_str("unit"),
            }
        }
    }
    impl std::fmt::Display for Form {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            use Form::*;
            let variant = match self {
                Struct(_, _, _) => "Struct",
                Enum(_, _, _, _) => "Enum",
                StructTuple(_, _, _) => "StructTuple",
                List(_) => "List",
                Option(_) => "Option",
                Tuple(_) => "Tuple",
                Map(_) => "Map",
                Boolean => "Boolean",
                Float => "Float",
                Integer => "Integer",
                Str => "Str",
                Unit => "Unit",
            };
            write!(f, "{}", variant)
        }
    }

    macro_rules! impl_form {
        ($form:path => $($target:ty),*) => {
            $(
            impl FormExt for $target {
                fn get_form() -> RForm {
                    RForm::new($form)
                }
            }
            )*
        }
    }
    macro_rules! impl_form_boxed {
        ($form:path => $($target:ident<$($constraint:ident),*>),*) => {
            $(
            impl<$($constraint: FormExt),*> FormExt for $target<$($constraint),*>
            {
                fn get_form() -> RForm {
                    RForm::new($form(Box::new(<($($constraint),*,)>::get_form())))
                }
            }
            )*
        }
    }
    macro_rules! impl_form_tuple {
        ($($constraint:ident),*) => {
            impl<$($constraint: FormExt),*> FormExt for ($($constraint),*,)
            {
                fn get_form() -> RForm {
                    RForm::new(Form::Tuple(vec![$($constraint::get_form()),*,]))
                }
            }
        }
    }

    impl_form!(Form::Boolean => bool);
    impl_form!(Form::Float => f32, f64);
    impl_form!(Form::Integer => usize, u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);
    impl_form!(Form::Str => String, std::path::PathBuf);
    impl_form!(Form::Unit => ());

    impl_form_boxed!(Form::Option => Option<T>);
    impl_form_boxed!(Form::List => Vec<T>);

    impl_form_tuple!(A);
    impl_form_tuple!(A, B);
    impl_form_tuple!(A, B, C);
    impl_form_tuple!(A, B, C, D);
    impl_form_tuple!(A, B, C, D, E);
    impl_form_tuple!(A, B, C, D, E, F);
    impl_form_tuple!(A, B, C, D, E, F, G);
    impl_form_tuple!(A, B, C, D, E, F, G, H);
    impl_form_tuple!(A, B, C, D, E, F, G, H, I);
    impl_form_tuple!(A, B, C, D, E, F, G, H, I, J);
    impl_form_tuple!(A, B, C, D, E, F, G, H, I, J, K);
    impl_form_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);

    impl<K, V> FormExt for HashMap<K, V>
    where
        K: FormExt + Eq + std::hash::Hash,
        V: FormExt,
    {
        fn get_form() -> RForm {
            RForm::new(Form::Map(Box::new((K::get_form(), V::get_form()))))
        }
    }

    /// Helper too create a Form from a structure or an enumerate.
    ///
    /// The easiest way to implement this type is to use the dedicated derive :
    /// ```rust
    /// use mistral_nvim_derive::Form;
    /// use crate::mistral::model::tools::*;
    ///
    /// #[derive(Form)]
    /// struct Data {
    ///     value: usize,
    /// }
    /// ```
    pub trait FormExt: for<'de> Deserialize<'de> {
        fn get_form() -> RForm;
    }
}

pub use extension::{Description, Form, FormExt, Name, RForm, RunResult, Runnable, ToolExt, ToolListExt};
