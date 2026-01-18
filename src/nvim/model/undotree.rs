use nvim_oxi::{api, conversion::FromObject};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
/// More details in manual :h undotree()
pub struct UndotreeData {
    #[serde(rename = "seq_cur")]
    pub sequence_current: usize,
    #[serde(rename = "seq_last")]
    pub sequence_last: usize,
    #[serde(rename = "save_cur")]
    pub save_current: usize,
}

impl FromObject for UndotreeData {
    fn from_object(object: nvim_oxi::Object) -> Result<Self, nvim_oxi::conversion::Error> {
        Self::deserialize(nvim_oxi::serde::Deserializer::new(object)).map_err(Into::into)
    }
}

impl UndotreeData {
    pub fn from_buffer(buffer: &api::Buffer) -> Result<Self, api::Error> {
        let mut args = nvim_oxi::Array::new();
        let buffer_id = buffer.handle();
        args.push(buffer_id);
        api::call_function("undotree", args)
    }
}
