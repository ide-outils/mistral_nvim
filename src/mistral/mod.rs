use std::sync::Arc;

use crate::{
    messages::{IdMessage, MistralEnveloppe, MistralSender, NvimEnveloppe, NvimMessage, NvimReceiver},
    mistral::controlleur::fim::SharedContext,
};

pub mod client;
pub mod controlleur;
pub mod model;

use controlleur::fim;

#[tokio::main]
pub async fn mistral_loop(mut rx: NvimReceiver, tx_nvim: MistralSender, nvim_handle: nvim_oxi::libuv::AsyncHandle) {
    // Setup logger. IMPORTANT: No libuv logs should happen before this call.
    crate::log_tokio!(Trace, "");
    let tx_nvim_clone = tx_nvim.clone();
    let context = Arc::new(controlleur::fim::Context::new(tx_nvim, nvim_handle));
    while let Some(NvimEnveloppe { id, message }) = rx.recv().await {
        let tx_nvim_clone = tx_nvim_clone.clone();
        let ctx = Arc::clone(&context);
        tokio::spawn(async move {
            if let Err(err) = handle_message(id, message, Arc::clone(&ctx)).await {
                let _ = tx_nvim_clone.send(MistralEnveloppe::notify_error(id, err));
            }
        });
    }
}

pub async fn handle_message(id: IdMessage, message: NvimMessage, ctx: SharedContext) -> crate::Result<()> {
    match message {
        NvimMessage::Abort => fim::abort_task(id, ctx).await,
        // FIM
        NvimMessage::FimCursorLine(normal) => fim::cursor(id, normal, ctx).await,
        NvimMessage::FimFunction(normal) => fim::function(id, normal, ctx).await,
        // NvimMessage::FimStatement(normal) => todo!(),
        NvimMessage::FimVisual(visual) => fim::visual(id, visual, ctx).await,
        NvimMessage::Chat(request) => fim::chat_completion(id, request, ctx).await,
    }
}
