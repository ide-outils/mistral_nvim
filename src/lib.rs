#![feature(stmt_expr_attributes)]
#![feature(normalize_lexically)]
use nvim_oxi::{self as oxi, libuv::AsyncHandle};
use tokio::sync::mpsc;

pub mod messages;
pub mod mistral;
pub mod nvim;
pub mod utils;

use nvim::model::SharedState;
pub use utils::notify::{self, Result};

#[oxi::plugin]
pub fn mistral_nvim() -> oxi::Result<()> {
    let (nvim_tx, mut nvim_rx) = mpsc::unbounded_channel();
    let (mistral_tx, mistral_rx) = mpsc::unbounded_channel();

    #[cfg(feature = "prod_mode")]
    {
        struct LogUv;
        impl code_modifier::Logger for LogUv {
            fn log(&self, content: &str) {
                log_libuv!(Trace, "{}", content);
            }
        }
        code_modifier::set_logger(LogUv)
    }

    let state = nvim::model::State::new(mpsc::UnboundedSender::clone(&mistral_tx));

    let state_clone = SharedState::clone(&state);
    let async_handle = AsyncHandle::new(move || {
        loop {
            let s = SharedState::clone(&state_clone);
            match nvim_rx.try_recv() {
                Ok(enveloppe) => {
                    // FIXME: I don't think this schedule is a good idea, AsyncHandle normally do the same job.
                    oxi::schedule(move |_| {
                        let messages::MistralEnveloppe { id, message } = enveloppe;
                        match id {
                            messages::IdMessage::FIM(buf_handle, id) => {
                                if let Err(err) = nvim::vue::fim::handle_nvim_message(buf_handle, id, message, &s) {
                                    notify::error(format!("FIM : {}", err));
                                }
                            }
                            messages::IdMessage::Chat(buf_handle, msg_index) => {
                                if let Err(err) =
                                    nvim::vue::chat::handle_nvim_message(buf_handle, msg_index, message, &s)
                                {
                                    notify::error(format!("Chat : {}", err));
                                }
                            }
                        }
                    })
                }
                Err(mpsc::error::TryRecvError::Empty) => break,
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    notify::error("Channel disconnected!");
                    break;
                }
            }
        }
    })?;
    std::thread::spawn(move || mistral::mistral_loop(mistral_rx, mpsc::UnboundedSender::clone(&nvim_tx), async_handle));
    if let Err(err) = nvim::controlleur::setup(&state) {
        notify::error(&format!("Mistral FAILED to setup. {err}"));
    };

    Ok(())
}
