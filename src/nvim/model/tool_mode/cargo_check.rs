#![allow(dead_code, unused_variables)]
use std::io::Write as _;

use mistral_nvim_derive::{Tool, ToolList};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{mistral::model::tools::*, nvim::model::SharedState};

const CHECK_DIR: &'static str = "/home/yoann/tmp/rust_mistral_check/";
const CHECK_FILE: &'static str = "/home/yoann/tmp/rust_mistral_check/src/main.rs";

/// Rnvoyer du code qui sera compilé avec la lib `io_uring`. Le code sera placé dans un fichier `main.rs`.
/// Puis les check/tests de cargo seront lancés.
#[derive(Serialize, Deserialize, Tool, JsonSchema)]
pub struct CargoCheck {
    /// Code qui doit être vérifié. (lance `cargo check`)
    code: String,
    /// Si true, alors lance aussi `cargo test`
    should_test: bool,
}

impl Runnable for CargoCheck {
    type Ok = String;
    type Err = crate::notify::Notification;
    fn run(&mut self, state: SharedState, msg: crate::messages::RunToolMessage) -> Result<Self::Ok, Self::Err> {
        let mut file = std::fs::File::options()
            .read(true)
            .write(true)
            .truncate(true)
            .open(&CHECK_FILE)?;
        file.write_all(self.code.as_bytes())?;
        let check = if self.should_test {
            let test_handle = std::process::Command::new("cargo")
                .arg("test")
                .spawn()?;
            test_handle.wait_with_output()?
        } else {
            let check_handle = std::process::Command::new("cargo")
                .arg("check")
                .spawn()?;
            check_handle.wait_with_output()?
        };
        let check = String::from_utf8_lossy(check.stdout.as_slice()).to_string();
        Ok(check)
    }
}

#[derive(ToolList)]
pub struct Cargo(CargoCheck);
