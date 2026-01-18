use nvim_oxi::api;

use crate::{
    messages::MistralMessage,
    notify::{IntoNotification as _, NotifyExt as _},
    nvim::model::{self, Locker as _},
    utils::notify,
};

#[track_caller]
fn stop<'lock>(buffer: &api::Buffer, id: usize, state: std::sync::MutexGuard<'lock, model::State>) {
    state
        .tx_mistral
        .send(crate::messages::NvimEnveloppe {
            id: crate::messages::IdMessage::FIM(buffer.handle(), id),
            message: crate::messages::NvimMessage::Abort,
        })
        .notify_error();
}

pub fn handle_nvim_message(
    buf_handle: i32,
    id: usize,
    message: MistralMessage,
    state: &model::SharedState,
) -> crate::Result<()> {
    let mut buffer: api::Buffer = buf_handle.into();
    let buffer = &mut buffer;
    let mut s = state.lock();
    match message {
        MistralMessage::InitializeTask(cursor) => {
            s.start_insertion_successive(buffer, id, cursor)?;
        }
        MistralMessage::UpdateRole(_) => {}
        MistralMessage::UpdateContent(chunk) => {
            let buffer_modifier = s.get_mut_buffer_modifier(buffer)?;
            if let Err(err) = buffer_modifier.insert(id, chunk) {
                err.into_error().notify();
                stop(buffer, id, s);
            }
        }
        MistralMessage::FinalizeTask(_stream_result) => {
            crate::log_libuv!(Debug, "Cleaned up FIM");
            s.buffer_modifier_id_finished(buffer, &id)?;
            s.remove_fim(buffer);
            stop(buffer, id, s);
            crate::log_libuv!(Debug, "FIM Done.");
        }
        MistralMessage::RunTool(tool) => {
            crate::log_libuv!(Warn, "FIM should not RunTool: {tool:?}");
        }
        MistralMessage::Notify { message, level } => {
            use notify::NotifyLevel::*;
            match level {
                Trace => notify::trace(&message),
                Debug => notify::debug(&message),
                Info => notify::info(&message),
                Warn => notify::warn(&message),
                Error => notify::error(&message),
                Off => notify::off(&message),
            }
        }
    }
    Ok(())
}
