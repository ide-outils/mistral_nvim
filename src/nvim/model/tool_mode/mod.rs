pub mod code_refactorisation;

use code_refactorisation::CodeRefactorisation;
use mistral_nvim_derive::{Form, Tool};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{mistral, mistral::model::tools::*, nvim::model::SharedState};

/// Change the mode to activate dedicated tools.
#[derive(Tool, Form, JsonSchema, Serialize, Deserialize, Default, Clone, Debug)]
#[param_name("set_mode")]
#[param_description("Select a mode to extend the tools available.")]
pub enum Mode {
    /// No mode
    #[default]
    r#None,
    /// Tools to refactor the code (move function, etc).
    CodeRefactorisation,
}
impl Mode {
    pub fn replace_from_str(&mut self, value: &str) {
        match value {
            "None" => *self = Self::None,
            "CodeRefactorisation" => *self = Self::CodeRefactorisation,
            _ => (),
        }
    }
}
impl Runnable for Mode {
    type Ok = Self;
    type Err = String;

    fn run(&mut self, _state: SharedState, _msg: crate::messages::RunToolMessage) -> Result<Self::Ok, Self::Err> {
        Ok(self.clone())
        // let FunctionCall { name, arguments } = msg.tool.function;
        // if name != "set_mode" {
        //     return RunResult::Err(format!("Function '{name}' does not exist."));
        // } else {
        //     *self = match arguments.as_str() {
        //         "None" => Self::None,
        //         "CodeRefactorisation" => Self::CodeRefactorisation,
        //         _ => return RunResult::Err(format!("Arguments value '{arguments}' does not exist, it is an enum.")),
        //     };
        // };
        // RunResult::Ok(())
    }
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Mode::None => "None",
            Mode::CodeRefactorisation => "CodeRefactorisation",
        };
        write!(f, "{name}")
    }
}

impl Mode {
    pub fn run_tool(&mut self, state: SharedState, msg: crate::messages::RunToolMessage) -> mistral::model::Message {
        match self {
            Self::None => self.set_mode(state, msg),
            Self::CodeRefactorisation => CodeRefactorisation::run(state, msg),
        }
    }
    pub fn set_mode(&mut self, _state: SharedState, msg: crate::messages::RunToolMessage) -> mistral::model::Message {
        match Self::parse(&msg) {
            Ok(mode_to_set) => {
                *self = mode_to_set;
                msg.create_mistral_message("Mode changed.")
            }
            Err(json_parse_error) => msg.create_mistral_message(json_parse_error),
        }
    }
    pub fn current_tools(&self) -> Vec<Tool> {
        match self {
            Self::None => vec![Self::get_tool()],
            Self::CodeRefactorisation => CodeRefactorisation::get_tools(),
            // ...
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn get_form() {
        let _ = std::collections::HashMap::<String, String>::get_form();
        let _ = <Vec<String>>::get_form();
    }

    const SER_MODE: &'static str = r###"[
  {
    "type": "function",
    "function": {
      "description": "Change the mode to activate dedicated tools.",
      "name": "Mode",
      "parameters": {
        "oneOf": [
          {
            "description": "No mode",
            "type": "string",
            "const": "None"
          },
          {
            "description": "Tools to refactor the code (move function, etc).",
            "type": "string",
            "const": "CodeRefactorisation"
          }
        ]
      }
    }
  }
]"###;

    const SER_CODE_REFACTO: &'static str = r###"[
  {
    "type": "function",
    "function": {
      "description": "Permet de modifier de le code d'un fichier de manière granulaire.\nPar exemple, si une fonction existe déjà elle sera remplacée si non, elle sera ajoutée.\nUn formatteur puis un vérificateur seront lancés après ajout.\nLes fichiers/dossiers qui n'existent pas seront créé.",
      "name": "CodeModifier",
      "parameters": {
        "type": "object",
        "properties": {
          "file": {
            "description": "Chemin relatif à la du racine projet.",
            "type": "string"
          },
          "code": {
            "description": "Code à ajouter.",
            "type": "string"
          }
        },
        "required": [
          "file",
          "code"
        ]
      }
    }
  },
  {
    "type": "function",
    "function": {
      "description": "Permet de récupérer le contenu des fichiers d'un projet.",
      "name": "CodeRetriever",
      "parameters": {
        "type": "object",
        "properties": {
          "file": {
            "description": "Chemin relatif à la du racine projet.",
            "type": "string"
          }
        },
        "required": [
          "file"
        ]
      }
    }
  },
  {
    "type": "function",
    "function": {
      "description": "Récupère le arbre des fichiers d'un dossier.\nÉquivalent à la commande bash `tree`.",
      "name": "CodeTree",
      "parameters": {
        "type": "object",
        "properties": {
          "directory": {
            "description": "Chemin relatif à la du racine projet. Permettant de cibler un dossier précis.",
            "type": [
              "string",
              "null"
            ]
          }
        }
      }
    }
  }
]"###;

    fn assert_lines(output: String, expected: &str) {
        let s_out = output.split("\n");
        let s_exp = expected.split("\n");
        for (i, (out, exp)) in s_out.zip(s_exp).enumerate() {
            assert_eq!(out, exp, "On line {i}");
        }
    }

    #[test]
    fn schema_set_tool() -> crate::Result<()> {
        let tool = Mode::None.current_tools();
        let request = serde_json::to_string_pretty(&tool)?;
        crate::log_libuv!(Trace, "{request}");
        // assert_lines(request, SER_MODE);
        assert_lines(request, SER_MODE);
        Ok(())
    }

    #[test]
    fn schema_code_refactorisation() -> crate::Result<()> {
        let tool = Mode::CodeRefactorisation.current_tools();
        let request = serde_json::to_string_pretty(&tool)?;
        crate::log_libuv!(Trace, "{request}");
        // assert_lines(request, SER_CODE_REFACTO);
        assert_lines(request, SER_CODE_REFACTO);
        Ok(())
    }
}
