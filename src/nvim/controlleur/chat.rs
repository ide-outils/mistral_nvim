use std::sync::{Arc, LazyLock, Mutex};

use nvim_oxi::api::{self, opts::CreateCommandOpts};

use super::form;
use crate::{
    notify::{NotifyExt as _, NotifyExtV2},
    nvim::model::{self, Chat, ChatForm, Locker as _, RowRange, SharedState, state::chat::bar},
};

const CHAT_FILES: [&'static str; 1] = ["*.chat"];
static GROUP: LazyLock<u32> = LazyLock::new(|| api::create_augroup("MistralChat", &Default::default()).unwrap_or(0));

type ModifiedRows = Arc<Mutex<Vec<RowRange>>>;

pub fn setup_keymaps(state: &SharedState, buffer: &api::Buffer) -> crate::Result<()> {
    let mut modes = crate::utils::ShortcutBuilder::new(buffer.clone());
    use api::types::Mode::*;
    crate::set_keymaps! {
        modes (Normal) :
        "<Leader>cr" => {change_role(&state)} <= <state: SharedState>
        "<Leader>ct" => {change_mode(&state)} <= <state: SharedState>
        "<Leader>cm" => {change_model(&state)} <= <state: SharedState>
        "<Left>" => {prev_message(&state)} <= <state: SharedState>
        "<Right>" => {next_message(&state)} <= <state: SharedState>
        "<CR><CR>" => {send_prompt(&state)} <= <state: SharedState>
        "<Leader>cp" => {add_prompt(&state).notify()} <= <state: SharedState>
    }
    Ok(())
}

pub fn setup_commands(s: &SharedState) -> crate::Result<()> {
    use api::create_user_command as cmd;

    let d = "Chat commands";
    let state = SharedState::clone(&s);
    let opts = CreateCommandOpts::builder().desc(d).build();
    cmd("MistralNewChat", move |_| new_chat(&state), &opts)?;
    // let state = SharedState::clone(&s);
    // cmd("MistralTool", move |_| launch_tool(&state), &opts)?;
    // let state = SharedState::clone(&s);
    // cmd("MistralListChat", move |args| model::State::load_chat(&state), &opts)?;
    // let state = SharedState::clone(&s);
    // let one = CommandNArgs::One;
    // let opts_arg = CreateCommandOpts::builder().nargs(one).build();
    // cmd("MistralLoadChat", move |args| load_chat(&state, args), &opts_arg)?;

    let opts = CreateCommandOpts::builder().build();
    let state = SharedState::clone(&s);
    cmd("MistralChatSendPrompt", move |_| send_prompt(&state), &opts)?;
    let state = SharedState::clone(&s);
    cmd("MistralChatNextMessage", move |_| next_message(&state), &opts)?;
    let state = SharedState::clone(&s);
    cmd("MistralChatPrevMessage", move |_| prev_message(&state), &opts)?;
    let state = SharedState::clone(&s);
    cmd("MistralChatChangeRole", move |_| change_role(&state), &opts)?;
    let state = SharedState::clone(&s);
    cmd("MistralChatChangeMode", move |_| change_mode(&state), &opts)?;
    let state = SharedState::clone(&s);
    cmd("MistralChatChangeModel", move |_| change_model(&state), &opts)?;
    let state = SharedState::clone(&s);
    cmd("MistralChatShowMessage", move |_| show_current_message(&state), &opts)?;
    let state = SharedState::clone(&s);
    cmd("MistralChatUpdateBuffer", move |_| update_buffer(&state), &opts)?;
    let state = SharedState::clone(&s);
    cmd("MistralChatNewPrompt", move |_| add_prompt(&state).notify(), &opts)?;

    let state = SharedState::clone(&s);
    let opts = api::opts::CreateAutocmdOpts::builder()
        .group(*GROUP)
        .desc("Load Chat's buffers.")
        .patterns(CHAT_FILES)
        // .nested(true)
        .callback(move |args: api::types::AutocmdCallbackArgs| -> bool {
            load_chat(&state, args.buffer);
            false
        })
        .build();
    api::create_autocmd(["BufRead", "VimEnter"], &opts).unwrap();

    let modified_rows: ModifiedRows = Default::default();
    let modified_rows_cloned = Arc::clone(&modified_rows);
    let opts = api::opts::CreateAutocmdOpts::builder()
        .group(*GROUP)
        .desc("Notify the changements in the buffer.")
        .patterns(CHAT_FILES)
        .callback(move |args: api::types::AutocmdCallbackArgs| -> bool {
            text_changed(&modified_rows_cloned, &args.buffer);
            false
        })
        .build();
    api::create_autocmd(["TextChanged", "TextChangedI", "TextChangedP", "TextChangedT"], &opts).unwrap();

    let state = SharedState::clone(&s);
    let opts = api::opts::CreateAutocmdOpts::builder()
        .group(*GROUP)
        .desc("Notify the changements in the buffer.")
        .patterns(CHAT_FILES)
        .callback(move |args: api::types::AutocmdCallbackArgs| -> bool {
            update_buffer_with_modified_rows(&state, &modified_rows, &args.buffer);
            false
        })
        .build();
    api::create_autocmd(["BufWritePost"], &opts).unwrap();

    let opts = api::opts::CreateAutocmdOpts::builder()
        .group(*GROUP)
        .desc("Update Chat's status line.")
        .patterns(CHAT_FILES)
        .callback(|mut args: api::types::AutocmdCallbackArgs| -> bool {
            bar::StatusLineChatCache::update_current_window(&mut args.buffer);
            false
        })
        .build();
    api::create_autocmd(["CursorMoved", "CursorHold", "CursorMovedC", "CursorMovedI"], &opts).unwrap();

    Ok(())
}

fn text_changed(modified_rows: &ModifiedRows, buffer: &api::Buffer) {
    let Some(start_cursor) = model::Cursor::from_mark(buffer, '[') else {
        return;
    };
    let Some(end_cursor) = model::Cursor::from_mark(buffer, ']') else {
        return;
    };
    match modified_rows.lock() {
        Ok(mut lock) => lock.push((start_cursor.row..=end_cursor.row).into()),
        Err(error) => crate::notify::error(format!("Can't modify TextChanged : {error}")),
    }
}
fn update_buffer_with_modified_rows(state: &SharedState, modified_rows: &ModifiedRows, buffer: &api::Buffer) {
    let ranges: Vec<_> = match modified_rows.lock() {
        Ok(mut lock) => lock.drain(..).collect(),
        Err(error) => {
            crate::notify::error(format!("Can't modify TextChanged : {error}"));
            return;
        }
    };
    if !ranges.is_empty() {
        if let Some(chat) = Chat::from_buffer(&state, buffer) {
            let mut chat = chat.lock();
            // We can unwrap, we are sure to have at least one item.
            let range = Option::unwrap(ranges.into_iter().reduce(|range, next_range| {
                let start = std::cmp::min(range.start, next_range.start);
                let end = std::cmp::max(range.end, next_range.end);
                (start..=end).into()
            }));
            // Extend range to neighbours (otherwise we might delete some positions/messages)
            let pos_start = chat.positions.get_range_index_by_row(range.start).1;
            let pos_end = chat.positions.get_range_index_by_row(range.end).1;
            let range_start = chat
                .positions
                .get_range(pos_start.saturating_sub(1))
                .cloned()
                .unwrap_or(range.clone());
            let range_end = chat
                .positions
                .get_range(pos_end.saturating_add(1))
                .cloned()
                .unwrap_or(range.clone());
            let range = (range_start.start..=range_end.end).into();
            chat.update_buffer(range).notify_error();
            drop(chat);
            bar::StatusLineChatCache::update_current_window(&mut buffer.clone());
        }
    }
}

fn new_chat(state: &SharedState) {
    form::formulaire(&state, |chat_form: ChatForm, state: SharedState| {
        let buffer = api::Buffer::current();
        let filename = buffer.get_name();
        if filename.is_err() || Result::unwrap(filename).to_string_lossy() == "" {
            crate::notify::warn("Can't create chat : buffer must point to a path (existing or not).");
            return;
        }
        let chat = match crate::nvim::model::ChatState::new(chat_form, &state) {
            Ok(chat) => chat,
            Err(err) => {
                err.notify();
                return;
            }
        };
        let chat = crate::nvim::model::Chat::from_state(chat);
        Chat::clone(&chat).configure_statusline(&buffer, api::Window::current());
        state.lock().chats.insert(chat);
    })
}
pub fn load_chat(state: &SharedState, buffer: api::Buffer) {
    // let buffer = api::Buffer::current();
    let filename = buffer.get_name();
    if filename.is_err() || Result::unwrap(filename).to_string_lossy() == "" {
        return crate::notify::warn("Can't load chat : buffer must point to a path (existing or not).");
    }
    let chat = match crate::nvim::model::ChatState::load(&state, &buffer) {
        Ok(chat) => chat,
        Err(err) => {
            err.notify();
            return;
        }
    };
    let buffer = chat.buffer.clone();
    let chat = crate::nvim::model::Chat::from_state(chat);
    Chat::clone(&chat).configure_statusline(&buffer, api::Window::current());
    state.lock().chats.insert(chat);
}

// fn launch_tool(state: &SharedState) {
//     form::formulaire(&state, |mode: model::Mode, state: SharedState| {
//         let mut target_win = api::Window::current();
//         let target_buffer = api::Buffer::current();
//         let chat_form = match mode {
//             model::Mode::None => return,
//             model::Mode::CodeRefactorisation => ChatForm {
//                 name: "Outil Refactorisation".to_string(),
//                 description: "Développeur qui a pour mission d'écrire la documentation.".to_string(),
//                 model: crate::mistral::model::completion::Model::MistralLargeLatest,
//                 mode: model::Mode::CodeRefactorisation,
//             },
//         };
//         api::command("tabnew").unwrap();
//         let buffer = api::Buffer::current();
//         let window = api::Window::current();
//         let chat = crate::nvim::model::ChatState::new(chat_form, buffer, window, &state);
//         let id = chat.id.clone();
//         let chat = crate::nvim::model::Chat::from_state(chat);
//         let chat_clone = Chat::clone(&chat);
//         {
//             state.lock().chats.insert(id, chat);
//         }
//         api::set_current_win(&target_win).notify_error();
//         target_win.set_buf(&target_buffer).notify_error();
//         form::formulaire(&state, move |prompt: String, state: SharedState| {
//             crate::notify::debug("Before chat");
//             let mut chat = chat_clone.lock();
//             crate::notify::debug("After chat");
//             let prompt: Vec<String> = prompt.split("\n").map(ToString::to_string).collect();
//             // if let Some(prompt_buffer) = chat.buffers.get_mut(&Page::Prompt) {
//             //     prompt_buffer
//             //         .set_lines(.., false, prompt.clone())
//             //         .notify_error();
//             // }
//             chat.prompt = prompt;
//             crate::notify::debug("Before prompt");
//             chat.send_prompt(&state);
//             crate::notify::debug("After prompt");
//             api::set_current_win(&target_win).notify_error();
//             target_win.set_buf(&target_buffer).notify_error();
//         })
//     })
// }

fn send_prompt(state: &SharedState) {
    if let Some(chat) = Chat::from_current_buffer_target_prompt(&state) {
        chat.lock().send_prompt(&state).notify_warn()
    }
}

fn next_message(state: &SharedState) {
    if let Some(chat) = Chat::from_current_buffer(&state) {
        chat.lock().next_message();
    }
}
fn prev_message(state: &SharedState) {
    if let Some(chat) = Chat::from_current_buffer(&state) {
        chat.lock().prev_message();
    }
}

fn change_model(state: &SharedState) {
    if let Some(chat) = Chat::from_current_buffer(&state) {
        form::formulaire(&state, move |model, _state: SharedState| {
            chat.lock()
                .mut_message_under_cursor(|message| message.model = model)
                .notify();
        })
    }
}
fn change_mode(state: &SharedState) {
    if let Some(chat) = Chat::from_current_buffer(&state) {
        form::formulaire(&state, move |mode, _state: SharedState| {
            chat.lock()
                .mut_message_under_cursor(|message| message.mode = mode)
                .notify();
        })
    }
}
fn change_role(state: &SharedState) {
    if let Some(chat) = Chat::from_current_buffer(&state) {
        form::formulaire(&state, move |role, _state: SharedState| {
            chat.lock()
                .mut_message_under_cursor(|message| message.message.role = role)
                .notify();
        })
    }
}
fn show_current_message(state: &SharedState) {
    if let Some(chat) = Chat::from_current_buffer(&state) {
        let chat = chat.lock();
        if let Some(message) = chat.get_current_message(&api::Window::current()) {
            crate::notify::info(format!("{:#?}", message));
        }
    }
}
fn update_buffer(state: &SharedState) {
    if let Some(chat) = Chat::from_current_buffer(&state) {
        let mut chat = chat.lock();
        chat.update_buffer(model::RowRange::FULL)
            .notify_error()
    }
}
fn add_prompt(state: &SharedState) -> crate::Result<()> {
    if let Some(chat) = Chat::from_current_buffer(&state) {
        let mut chat = chat.lock();
        if chat.is_running.is_some() {
            crate::notify::warn("Chat is running.");
            return Ok(());
        }
        chat.push_new_message(None)?;
        chat.mut_message_by_index_isize(-1, |msg| {
            msg.message.role = crate::mistral::model::Role::User;
        })?;
    }
    Ok(())
}
