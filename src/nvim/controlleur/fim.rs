use nvim_oxi::{
    self as oxi,
    api::{self, opts, types},
};

use crate::{
    nvim::{
        self,
        model::{BufferData, Locker as _, Selection, SharedState},
    },
    utils::notify,
};

pub fn new_buffer(state: &SharedState) -> crate::Result<(api::Buffer, usize, BufferData)> {
    let (buffer_data, buffer) = nvim::model::BufferData::from_current_buffer()?;
    let id = state.lock().add_fim(&buffer);
    Ok((buffer, id.clone(), buffer_data))
}

#[macro_export]
macro_rules! n {
    ($message:expr) => {
        |state: SharedState| {
            let (buffer, id, data) = match crate::nvim::controlleur::fim::new_buffer(&state) {
                Ok(r) => r,
                Err(err) => {
                    err.notify();
                    return;
                }
            };
            let envelop = crate::messages::NvimEnveloppe {
                id: crate::messages::IdMessage::FIM(buffer.handle(), id),
                message: $message(crate::messages::Normal { data }),
            };
            state.lock().tx_mistral.send(envelop).unwrap();
        }
    };
}

#[macro_export]
macro_rules! v {
    ($message:expr) => {
        |state: SharedState, selection: crate::nvim::model::Selection| {
            crate::notify::info(format!("Mistral v! : Selection : {selection:?}"));
            let (buffer, id, data) = match crate::nvim::controlleur::fim::new_buffer(&state) {
                Ok(r) => r,
                Err(err) => {
                    err.notify();
                    return;
                }
            };
            let envelop = crate::messages::NvimEnveloppe {
                id: crate::messages::IdMessage::FIM(buffer.handle(), id),
                message: $message(crate::messages::Visual { data, selection }),
            };
            state.lock().tx_mistral.send(envelop).unwrap();
        }
    };
}

pub fn vmap<F>(state: &SharedState, f: F, short_cut: &str, opts: &mut opts::SetKeymapOptsBuilder) -> oxi::Result<()>
where
    F: Fn(SharedState, Selection) + 'static,
{
    let state_clone = SharedState::clone(state);
    api::set_keymap(
        types::Mode::Visual,
        short_cut,
        "",
        &opts
            .callback(move |_| {
                let selection = match Selection::from_mark_visual(&api::get_current_buf()) {
                    Ok(selection) => selection,
                    Err(err) => {
                        notify::error(&err.to_string());
                        return;
                    }
                };
                f(SharedState::clone(&state_clone), selection)
            })
            .build(),
    )?;
    Ok(())
}

pub fn vcmd<F>(state: &SharedState, f: F, command: &str, opts: &mut opts::CreateCommandOptsBuilder) -> oxi::Result<()>
where
    F: Fn(SharedState, Selection) + 'static,
{
    let state_clone = SharedState::clone(state);
    api::create_user_command(
        command,
        move |args: types::CommandArgs| f(SharedState::clone(&state_clone), Selection::from_command_args(&args)),
        &opts.range(types::CommandRange::CurrentLine).build(),
    )?;
    Ok(())
}

pub fn nmap<F>(state: &SharedState, f: F, short_cut: &str, opts: &mut opts::SetKeymapOptsBuilder) -> oxi::Result<()>
where
    F: Fn(SharedState) + 'static,
{
    let state_clone = SharedState::clone(state);
    api::set_keymap(
        types::Mode::Normal,
        short_cut,
        "",
        &opts
            .callback(move |_| f(SharedState::clone(&state_clone)))
            .build(),
    )?;
    Ok(())
}

pub fn ncmd<F>(state: &SharedState, f: F, command: &str, opts: &mut opts::CreateCommandOptsBuilder) -> oxi::Result<()>
where
    F: Fn(SharedState) + 'static,
{
    let state_clone = SharedState::clone(state);
    api::create_user_command(
        command,
        move |_| f(SharedState::clone(&state_clone)),
        &opts.range(types::CommandRange::CurrentLine).build(),
    )?;
    Ok(())
}
