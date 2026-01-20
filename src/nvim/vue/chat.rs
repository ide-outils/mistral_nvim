use nvim_oxi::api;

use crate::{
    messages::MistralMessage,
    mistral::model::stream::Status,
    notify::{IntoNotification as _, NotifyExt as _},
    nvim::model::{self, Cursor, Locker as _, state::chat},
    utils::notify,
};

#[track_caller]
fn stop<'lock>(buffer: &api::Buffer, message_index: chat::MsgIndex, state: std::sync::MutexGuard<'lock, model::State>) {
    state
        .tx_mistral
        .send(crate::messages::NvimEnveloppe {
            id: crate::messages::IdMessage::Chat(buffer.handle(), message_index),
            message: crate::messages::NvimMessage::Abort,
        })
        .notify_error();
}

pub fn handle_nvim_message(
    buf_handle: i32,
    message_index: chat::MsgIndex,
    message: MistralMessage,
    state: &model::SharedState,
) -> crate::Result<()> {
    let assistant_index = message_index + 1;
    let mut buffer: api::Buffer = buf_handle.into();
    let buffer = &mut buffer;
    // let insert_id = (id, message_index);
    let Some(chat) = state.lock().chats.get_by_buffer(buffer).cloned() else {
        stop(buffer, message_index, state.lock());
        return Err("Does not exist.".into());
    };
    match message {
        MistralMessage::InitializeTask(_cursor) => {
            let mut chat = chat.lock();
            // let mut buffer = chat.buffer.clone();
            // let last_index = chat.messages.len() - 1;
            let Some(position) = chat.positions.get_by_msg_index(message_index) else {
                stop(buffer, message_index, state.lock());
                return Err("Target Message does not exist.".into_error());
            };
            // let cursor_tag_line = Cursor {
            //     row: position.start,
            //     col: model::Col::MIN,
            // };
            let mut row = position.end.clone();
            let real_row = if row == model::Row::MAX {
                model::Row::buf_last_row(buffer)?
            } else {
                row
            };
            let mut append_empty_line = false;
            if row == model::Row::MAX {
                let line = model::cursor::get_line(buffer, real_row, false)?;
                if line == "" {
                    row = real_row - 1;
                    append_empty_line = false;
                } else {
                    append_empty_line = true;
                }
            }
            let cursor_end = Cursor {
                row,
                col: model::Col::MAX,
            };
            chat.start_insertion_successive(assistant_index, cursor_end)?;
            if append_empty_line {
                model::cursor::push_lines(buffer, [""])?;
            }
            chat.push_new_message(Some(assistant_index))?;
            chat.is_running = Some(0);
            let set_status = |msg: &mut chat::MessageState| msg.status = Status::Initialised;
            chat.mut_message_by_index(message_index, set_status)?;
            chat.mut_message_by_index(assistant_index, set_status)?;
        }
        MistralMessage::UpdateRole(role) => {
            chat.lock()
                .mut_message_by_index(assistant_index, |msg| msg.message.role = role)?;
        }
        MistralMessage::UpdateContent(chunk) => {
            // crate::log_libuv!(Off, "[index {message_index}] {chunk:?}");
            if let Err(err) = chat.lock().insert(chunk, Some(assistant_index)) {
                err.notify();
                stop(buffer, message_index, state.lock());
            }
        }
        MistralMessage::RunTool(tool_calls) => {
            // crate::log_libuv!(Off, "[index {message_index}] {tool_calls:?}");
            let mut chat = chat.lock();
            let range_to_update = chat.positions.last().clone();
            let Some(target_message) = chat.messages.last_mut() else {
                return Err("No more messages in chat : Can't run tool.".into_error());
            };
            let mode = target_message.mode.clone();
            let params = target_message.params.clone();
            let model = target_message.model.clone();
            // let assistant_index = message_index + 1;
            // chat.mut_message_by_index(assistant_index as isize, |message| {
            //     match &mut message.message.tool_calls {
            //         Some(tc) => tc.extend(tool_calls.clone()),
            //         none => *none = Some(tool_calls.clone()),
            //     }
            // });

            for tool in &tool_calls {
                chat.push_tool_call(&tool, Some(assistant_index))?;
            }
            let mut messages_tool = Vec::with_capacity(tool_calls.len());
            for tool in tool_calls {
                let tool_id =
                    crate::utils::tool_id::tool_id_to_usize(tool.id.as_ref().map(|s| s.as_str()).unwrap_or(""));
                crate::log_libuv!(Trace, "Tool {tool_id}");
                let mut mode = mode.clone();
                let params = params.clone();
                let model = model.clone();
                let run_tool_message = crate::messages::RunToolMessage {
                    buffer: buffer.clone(),
                    tool,
                };
                let message = mode.run_tool(model::SharedState::clone(state), run_tool_message);
                let message_state = chat::MessageState {
                    message,
                    mode,
                    model,
                    usage: Default::default(),
                    params,
                    status: Default::default(),
                };
                messages_tool.push(message_state);
            }
            for message_state in messages_tool {
                chat.push_message(message_state, Some(assistant_index))?;
            }
            let mut envelop = chat.build_request_envelop()?;
            let next_id = chat.messages.len() - 1;
            envelop.id = crate::messages::IdMessage::Chat(buffer.handle(), next_id);
            // chat.buffer_modifier_ids_finished(vec![assistant_index]);
            state.lock().tx_mistral.send(envelop).unwrap();
        }
        MistralMessage::FinalizeTask(stream_result) => {
            let mut chat = chat.lock();
            // s.buffer_modifier_id_finished(buffer, &assistant_index)?;
            // let Some(position) = chat.positions.get_by_msg_index(assistant_index) else {
            //     stop(buffer, message_index, state.lock());
            //     return Err("Target Message does not exist.".into_error());
            // };
            // let row_tag_line = position.start;
            // let cols = model::ColRange::from_buffer_row(buffer, row_tag_line)?;
            // let _ = s.start_replace_line(buffer, assistant_index, row_tag_line, *cols.end);
            let crate::mistral::model::stream::StreamResponse { message, status, usage } = &stream_result;
            crate::log_libuv!(Trace, "Response : {message:?}");
            match status {
                Status::Failed(_, _) => {
                    chat.mut_message_by_index(message_index, |msg| msg.status = status.clone())?;
                    chat.mut_message_by_index(assistant_index, |msg| msg.status = status.clone())?;
                }
                _ => {
                    chat.mut_message_by_index(message_index, |msg| {
                        msg.usage = usage.clone();
                        msg.status = status.clone();
                    })?;
                    chat.mut_message_by_index(assistant_index, |msg| {
                        msg.message = message.clone();
                        msg.usage = usage.clone();
                        msg.status = status.clone();
                    })?;
                }
            }
            chat.buffer_modifier_ids_finished(vec![message_index, assistant_index]);
            // if chat.(buffer, &message_index)?
            //     || s.buffer_modifier_id_finished(buffer, &assistant_index)?
            // {
            //     chat.is_running = None;
            // }
            stop(buffer, message_index, state.lock());
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

#[cfg(not(feature = "prod_mode"))]
#[nvim_oxi::test]
#[track_caller]
fn test_chat_requests() -> crate::Result<()> {
    use tokio::sync::mpsc;

    use crate::{
        messages::{IdMessage, NvimEnveloppe, NvimMessage},
        mistral::model::{
            Message, Role, ToolCall,
            stream::{StreamResponse, Usage},
            tools::FunctionCall,
        },
        nvim::model::State,
    };

    const BUFFER_CONTENT: &'static str = r##"<CHAT  role="Refactorisation" status="0;0;0" model="Tu es un développeur qui a des outils à ta disposition." id="00000000-0000-0000-0000-000000000000"/>
<MESSAGE  role="System" model="Tiny Latest" status="Created" usage="0;0;0"/>
Tu es un développeur qui a des outils à ta disposition.

<MESSAGE  role="User" model="Tiny Latest" status="Created" usage="0;0;0" mode="CodeRefactorisation"/>
Peux-tu modifier la fonction main dans `tests_files/main.rs` grâce aux outils, pour qu'elle affiche "Salut\n" ?"##;

    const CHUNKS_1: [[&'static str; 1]; 1] = [[""]];
    let tool_calls_1: [ToolCall; 1] = [ToolCall {
        function: FunctionCall {
            name: "CodeRetriever".to_string(),
            arguments: "{\"file\": \"tests_files/main.rs\"}".to_string(),
        },
        id: Some("F7EJnRYyb".to_string()),
        index: Some(0),
    }];
    const RESPONSE_2_CONTENT: &'static str = r##"{"Ok":"fn main() {\n    println!(\"Salut\\n\");\n}\n"}"##;

    const CHUNKS_3: [[&'static str; 1]; 23] = [
        [""],
        ["Je"],
        [" suis"],
        [" dés"],
        ["olé"],
        [","],
        [" mais"],
        [" la"],
        [" fonction"],
        [" main"],
        [" existe"],
        [" déjà"],
        [" et"],
        [" contient"],
        [" déjà"],
        [" le"],
        [" code"],
        [" que"],
        [" vous"],
        [" voulez"],
        [" ajouter"],
        ["."],
        [""],
    ];
    const _CONTENT_3: &'static str =
        "Je suis désolé, mais la fonction main existe déjà et contient déjà le code que vous voulez ajouter.";

    fn init(buffer: &api::Buffer, message_index: chat::MsgIndex, state: &model::SharedState) -> crate::Result<()> {
        let message: MistralMessage = MistralMessage::InitializeTask(Cursor::zero());
        handle_nvim_message(buffer.handle(), message_index, message, state)
    }
    fn chunks(
        buffer: &api::Buffer,
        message_index: chat::MsgIndex,
        chunks: &[[&str; 1]],
        state: &model::SharedState,
    ) -> crate::Result<()> {
        for chunk in chunks {
            let message: MistralMessage = MistralMessage::UpdateContent(chunk.iter().map(|s| s.to_string()).collect());
            handle_nvim_message(buffer.handle(), message_index, message, state)?;
        }
        Ok(())
    }
    fn tools(
        buffer: &api::Buffer,
        message_index: chat::MsgIndex,
        tools: &[ToolCall],
        state: &model::SharedState,
    ) -> crate::Result<()> {
        let message: MistralMessage = MistralMessage::RunTool(tools.iter().map(|tc| tc.clone()).collect());
        handle_nvim_message(buffer.handle(), message_index, message, state)?;
        Ok(())
    }
    fn finalise(
        buffer: &api::Buffer,
        message_index: chat::MsgIndex,
        message: Message,
        state: &model::SharedState,
    ) -> crate::Result<()> {
        let message: MistralMessage = MistralMessage::FinalizeTask(StreamResponse {
            message,
            status: Status::Completed,
            usage: Usage::default(),
        });
        handle_nvim_message(buffer.handle(), message_index, message, state)
    }
    fn extract_envelop(nvim_envelop: NvimEnveloppe) -> crate::Result<(i32, usize, NvimMessage)> {
        let NvimEnveloppe {
            id: IdMessage::Chat(sent_handle, sent_index),
            message: sent_message,
        } = nvim_envelop
        else {
            return Err("Expected a Chat Id.".into_error());
        };
        Ok((sent_handle, sent_index, sent_message))
    }

    let (mistral_tx, mut mistral_rx) = mpsc::unbounded_channel();
    let state = &State::new(mistral_tx);
    let buffer = &mut api::Buffer::current();
    buffer.set_lines(.., false, BUFFER_CONTENT.split('\n'))?;
    // In tests, we must force the activation of the undotree
    api::exec2("undo", &Default::default())?;
    api::exec2("redo", &Default::default())?;
    crate::nvim::controlleur::chat::load_chat(state, buffer.clone());

    let message_index = 1;
    init(buffer, message_index, state)?;
    chunks(buffer, message_index, &CHUNKS_1, state)?;
    chat::show(buffer);

    tools(buffer, message_index, &tool_calls_1, state)?;
    let message_index_tool_call = message_index + 2;
    let nvim_envelop = mistral_rx.blocking_recv().unwrap();
    let (sent_handle, sent_index, sent_message) = extract_envelop(nvim_envelop)?;
    assert_eq!(sent_handle, buffer.handle());
    assert_eq!(sent_index, message_index_tool_call);
    let NvimMessage::Chat(sent_request) = sent_message else {
        return Err("Expected a Chat Message.".into_error());
    };
    let sent_messages = sent_request.completion.messages;
    assert_eq!(sent_messages.len(), message_index_tool_call + 1);
    assert!(sent_messages[2].tool_calls.is_some());
    assert_eq!(sent_messages[2].tool_calls.as_ref().unwrap().len(), 1);
    assert!(matches!(sent_messages[3].role, Role::Tool), "Expected Role::Tool;");
    assert!(sent_request.params.tools.is_some());
    assert_eq!(sent_request.params.tools.unwrap().len(), 3);
    assert_eq!(sent_messages[3].content, RESPONSE_2_CONTENT);

    const BUFFER_CONTENT_1: &'static str = r##"<CHAT  role="Refactorisation" status="0;0;0" model="Tu es un développeur qui a des outils à ta disposition." id="00000000-0000-0000-0000-000000000000"/>
<MESSAGE  role="System" model="Tiny Latest" status="Created" usage="0;0;0"/>
Tu es un développeur qui a des outils à ta disposition.

<MESSAGE  role="User" model="Tiny Latest" status="Initialised" usage="0;0;0" mode="CodeRefactorisation"/>
Peux-tu modifier la fonction main dans `tests_files/main.rs` grâce aux outils, pour qu'elle affiche "Salut\n" ?

<MESSAGE role="Assistant" model="Tiny Latest" status="Initialised" usage="0;0;0" mode="CodeRefactorisation"/>

<TOOLCALL id="F7EJnRYyb" index="0" name="CodeRetriever">

```json
{"file": "tests_files/main.rs"}
```
</TOOLCALL>

<MESSAGE role="Tool" model="Tiny Latest" status="Created" usage="0;0;0" mode="CodeRefactorisation" name="CodeRetriever" tool_call_id="F7EJnRYyb"/>
{"Ok":"fn main() {\n    println!(\"Salut\\n\");\n}\n"}
"##;
    chat::assert_content(buffer, BUFFER_CONTENT_1);

    init(buffer, message_index_tool_call, state)?;

    const BUFFER_CONTENT_2: &'static str = r##"<CHAT  role="Refactorisation" status="0;0;0" model="Tu es un développeur qui a des outils à ta disposition." id="00000000-0000-0000-0000-000000000000"/>
<MESSAGE  role="System" model="Tiny Latest" status="Created" usage="0;0;0"/>
Tu es un développeur qui a des outils à ta disposition.

<MESSAGE  role="User" model="Tiny Latest" status="Initialised" usage="0;0;0" mode="CodeRefactorisation"/>
Peux-tu modifier la fonction main dans `tests_files/main.rs` grâce aux outils, pour qu'elle affiche "Salut\n" ?

<MESSAGE role="Assistant" model="Tiny Latest" status="Initialised" usage="0;0;0" mode="CodeRefactorisation"/>

<TOOLCALL id="F7EJnRYyb" index="0" name="CodeRetriever">

```json
{"file": "tests_files/main.rs"}
```
</TOOLCALL>

<MESSAGE role="Tool" model="Tiny Latest" status="Initialised" usage="0;0;0" mode="CodeRefactorisation" name="CodeRetriever" tool_call_id="F7EJnRYyb"/>
{"Ok":"fn main() {\n    println!(\"Salut\\n\");\n}\n"}

<MESSAGE role="Assistant" model="Tiny Latest" status="Initialised" usage="0;0;0" mode="CodeRefactorisation"/>

"##;
    chat::assert_content(buffer, BUFFER_CONTENT_2);

    let full_response = Message {
        role: Role::Assistant,
        content: "".to_string(),
        prefix: None,
        tool_calls: Some(tool_calls_1.into_iter().collect()),
        tool_call_id: None,
        name: None,
    };
    finalise(buffer, message_index, full_response, state)?;
    let nvim_envelop = mistral_rx.blocking_recv().unwrap();
    let (sent_handle, sent_index, sent_message) = extract_envelop(nvim_envelop)?;
    assert_eq!(sent_handle, buffer.handle());
    assert_eq!(sent_index, message_index);
    let is_abort = matches!(sent_message, NvimMessage::Abort);
    assert!(is_abort, "Expect finalise to sent Abort.");

    const BUFFER_CONTENT_3: &'static str = r##"<CHAT  role="Refactorisation" status="0;0;0" model="Tu es un développeur qui a des outils à ta disposition." id="00000000-0000-0000-0000-000000000000"/>
<MESSAGE  role="System" model="Tiny Latest" status="Created" usage="0;0;0"/>
Tu es un développeur qui a des outils à ta disposition.

<MESSAGE  role="User" model="Tiny Latest" status="Completed" usage="0;0;0" mode="CodeRefactorisation"/>
Peux-tu modifier la fonction main dans `tests_files/main.rs` grâce aux outils, pour qu'elle affiche "Salut\n" ?

<MESSAGE role="Assistant" model="Tiny Latest" status="Completed" usage="0;0;0" mode="CodeRefactorisation"/>

<TOOLCALL id="F7EJnRYyb" index="0" name="CodeRetriever">

```json
{"file": "tests_files/main.rs"}
```
</TOOLCALL>

<MESSAGE role="Tool" model="Tiny Latest" status="Initialised" usage="0;0;0" mode="CodeRefactorisation" name="CodeRetriever" tool_call_id="F7EJnRYyb"/>
{"Ok":"fn main() {\n    println!(\"Salut\\n\");\n}\n"}

<MESSAGE role="Assistant" model="Tiny Latest" status="Initialised" usage="0;0;0" mode="CodeRefactorisation"/>

"##;
    chat::assert_content(buffer, BUFFER_CONTENT_3);

    chunks(buffer, message_index_tool_call, &CHUNKS_3, state)?;

    const BUFFER_CONTENT_4: &'static str = r##"<CHAT  role="Refactorisation" status="0;0;0" model="Tu es un développeur qui a des outils à ta disposition." id="00000000-0000-0000-0000-000000000000"/>
<MESSAGE  role="System" model="Tiny Latest" status="Created" usage="0;0;0"/>
Tu es un développeur qui a des outils à ta disposition.

<MESSAGE  role="User" model="Tiny Latest" status="Completed" usage="0;0;0" mode="CodeRefactorisation"/>
Peux-tu modifier la fonction main dans `tests_files/main.rs` grâce aux outils, pour qu'elle affiche "Salut\n" ?

<MESSAGE role="Assistant" model="Tiny Latest" status="Completed" usage="0;0;0" mode="CodeRefactorisation"/>

<TOOLCALL id="F7EJnRYyb" index="0" name="CodeRetriever">

```json
{"file": "tests_files/main.rs"}
```
</TOOLCALL>

<MESSAGE role="Tool" model="Tiny Latest" status="Initialised" usage="0;0;0" mode="CodeRefactorisation" name="CodeRetriever" tool_call_id="F7EJnRYyb"/>
{"Ok":"fn main() {\n    println!(\"Salut\\n\");\n}\n"}

<MESSAGE role="Assistant" model="Tiny Latest" status="Initialised" usage="0;0;0" mode="CodeRefactorisation"/>
Je suis désolé, mais la fonction main existe déjà et contient déjà le code que vous voulez ajouter.
"##;
    chat::assert_content(buffer, BUFFER_CONTENT_4);

    let full_response = Message {
        role: Role::Assistant,
        content: "".to_string(),
        prefix: None,
        tool_calls: None,
        tool_call_id: None,
        name: None,
    };
    finalise(buffer, message_index_tool_call, full_response, state)?;
    let nvim_envelop = mistral_rx.blocking_recv().unwrap();
    let (sent_handle, sent_index, sent_message) = extract_envelop(nvim_envelop)?;
    assert_eq!(sent_handle, buffer.handle());
    assert_eq!(sent_index, message_index_tool_call);
    let is_abort = matches!(sent_message, NvimMessage::Abort);
    assert!(is_abort, "Expect finalise to sent Abort.");

    const BUFFER_CONTENT_5: &'static str = r##"<CHAT  role="Refactorisation" status="0;0;0" model="Tu es un développeur qui a des outils à ta disposition." id="00000000-0000-0000-0000-000000000000"/>
<MESSAGE  role="System" model="Tiny Latest" status="Created" usage="0;0;0"/>
Tu es un développeur qui a des outils à ta disposition.

<MESSAGE  role="User" model="Tiny Latest" status="Completed" usage="0;0;0" mode="CodeRefactorisation"/>
Peux-tu modifier la fonction main dans `tests_files/main.rs` grâce aux outils, pour qu'elle affiche "Salut\n" ?

<MESSAGE role="Assistant" model="Tiny Latest" status="Completed" usage="0;0;0" mode="CodeRefactorisation"/>

<TOOLCALL id="F7EJnRYyb" index="0" name="CodeRetriever">

```json
{"file": "tests_files/main.rs"}
```
</TOOLCALL>

<MESSAGE role="Tool" model="Tiny Latest" status="Completed" usage="0;0;0" mode="CodeRefactorisation" name="CodeRetriever" tool_call_id="F7EJnRYyb"/>
{"Ok":"fn main() {\n    println!(\"Salut\\n\");\n}\n"}

<MESSAGE role="Assistant" model="Tiny Latest" status="Completed" usage="0;0;0" mode="CodeRefactorisation"/>
Je suis désolé, mais la fonction main existe déjà et contient déjà le code que vous voulez ajouter.
"##;
    chat::assert_content(buffer, BUFFER_CONTENT_5);
    api::exec2("undo", &Default::default())?;
    chat::assert_content(buffer, BUFFER_CONTENT);

    Ok(())
}
