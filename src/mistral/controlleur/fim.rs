use std::{
    collections::{HashMap, hash_map::Entry},
    sync::{Arc, atomic::AtomicBool},
};

use tokio::sync::{Mutex, mpsc::UnboundedSender};
use tree_sitter::{Query, QueryCursor, StreamingIterator as _};

use crate::{
    messages::{self, IdMessage, MistralEnveloppe, MistralMessage},
    mistral::{
        client::MistralClient,
        model::{
            completion::{ChatRequest, CompletionParams, FimCompletion, FimRequest, Model},
            stream::StreamResponse,
        },
    },
    notify::NotifyLevel,
    nvim::{self, model::Cursor},
};

pub struct AbortHandle {
    atomic: Arc<AtomicBool>,
    handle: tokio::task::AbortHandle,
}
impl AbortHandle {
    pub fn new<Fut>(atomic: Arc<AtomicBool>, task: tokio::task::JoinHandle<Fut>) -> Self {
        let handle = task.abort_handle();
        Self { atomic, handle }
    }
    pub fn soft_abort(&mut self) {
        self.atomic
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
    pub fn hard_abort(&mut self) {
        self.handle.abort()
    }
}
#[derive(Clone)]
pub struct SenderHandle {
    tx_nvim: UnboundedSender<messages::MistralEnveloppe>,
    handle_nvim: nvim_oxi::libuv::AsyncHandle,
}
impl SenderHandle {
    pub fn send_enveloppe(&self, message: messages::MistralEnveloppe) {
        if let Err(err) = self.tx_nvim.send(message) {
            crate::log_tokio!(Error, "Can't send MistralEnveloppe to nvim : {err}");
            return;
        }
        if let Err(err) = self.handle_nvim.send() {
            crate::log_tokio!(Error, "Can't send AsyncHandle : {err}");
        }
    }
    pub fn send(&self, id: messages::IdMessage, message: messages::MistralMessage) {
        self.send_enveloppe(messages::MistralEnveloppe { id, message })
    }
    pub fn notify_warn(&self, msg: impl ToString) {
        self.send_enveloppe(MistralEnveloppe::notify_warn(messages::IdMessage::FIM(0, 0), msg));
    }
    pub fn notify_error(&self, msg: impl ToString) {
        self.send_enveloppe(MistralEnveloppe::notify_error(messages::IdMessage::FIM(0, 0), msg));
    }
}
pub type SharedContext = Arc<Context>;
pub struct Context {
    pub nvim_sendle: SenderHandle,
    pub tasks: Mutex<HashMap<messages::IdMessage, AbortHandle>>,
    pub client: MistralClient,
}
impl Context {
    pub fn new(
        tx_nvim: UnboundedSender<messages::MistralEnveloppe>,
        handle_nvim: nvim_oxi::libuv::AsyncHandle,
    ) -> Self {
        let nvim_sendle = SenderHandle { tx_nvim, handle_nvim };
        let client = MistralClient::new(SenderHandle::clone(&nvim_sendle));
        Self {
            nvim_sendle,
            tasks: Default::default(),
            client,
        }
    }
    // pub fn send(&self, id: IdMessage, message: MistralMessage) {
    //     self.nvim_sendle.send(MistralEnveloppe { id, message });
    // }
}

macro_rules! pipe {
    ($self:ident -> $args:expr) => {
        Pipe::new($args, $self.context, $self.id)
    };
}

pub struct Pipe<T> {
    args: T,
    id: messages::IdMessage,
    context: SharedContext,
}

#[allow(dead_code)]
impl<T> Pipe<T> {
    pub fn notify_error(&self, message: impl ToString) {
        crate::log_tokio!(Error, "{}", message.to_string());
        self.send(MistralMessage::Notify {
            level: NotifyLevel::Error,
            message: message.to_string(),
        });
    }
    pub fn notify_warn(&self, message: impl ToString) {
        crate::log_tokio!(Warn, "{}", message.to_string());
        self.send(MistralMessage::Notify {
            level: NotifyLevel::Warn,
            message: message.to_string(),
        });
    }
    pub fn send(&self, message: MistralMessage) {
        self.context.nvim_sendle.send(self.id, message);
    }
    fn new(args: T, context: Arc<Context>, id: messages::IdMessage) -> Self {
        Self { args, context, id }
    }

    #[must_use]
    fn tap<F>(self, f: F) -> Self
    where
        F: FnOnce(&T, &Context),
    {
        f(&self.args, &self.context);
        self
    }

    #[must_use]
    fn map<U, F>(self, f: F) -> Pipe<U>
    where
        F: FnOnce(T, &Context) -> U,
    {
        pipe!(self -> f(self.args, &self.context))
    }

    fn next<Next>(self, value: Next) -> Pipe<Next> {
        pipe!(self -> value)
    }
}

pub struct TreeSitterContext {
    language: tree_sitter::Language,
    tree: tree_sitter::Tree,
    content: String,
    cursor: nvim::model::Cursor,
}
impl TreeSitterContext {
    pub fn from_bufferdata(data: nvim::model::BufferData) -> crate::Result<Self> {
        let nvim::model::BufferData {
            filetype,
            cursor,
            content,
            ..
        } = data;
        let content = content.join("\n");

        let mut parser = tree_sitter::Parser::new();
        let language_type = match filetype.as_str() {
            "rust" => tree_sitter_rust::LANGUAGE,
            _ => return Err("Filetype not supported.".into()),
        };
        let language = language_type.into();
        parser.set_language(&language)?;

        let Some(tree) = parser.parse(&content, None) else {
            return Err("Can't parse with tree-sitter.".into());
        };
        Ok(TreeSitterContext {
            language,
            tree,
            content,
            cursor,
        })
    }
    pub fn extract_query_vec(self, query: &str) -> crate::Result<Vec<String>> {
        let TreeSitterContext {
            language,
            tree,
            content,
            ..
        } = &self;
        let query = Query::new(&language, query)?;
        let mut query_cursor = QueryCursor::new();
        // let mut matches = cursor.matches(&query, tree.root_node(), content.as_bytes());
        // let captures_names = query.capture_names();
        let mut matches = query_cursor.matches(&query, tree.root_node(), content.as_bytes());

        let mut targets = Vec::new();
        while let Some(m) = matches.next() {
            // for capture in m.captures {
            //     let node = capture.node;
            //     let start: nvim::model::Cursor = node.start_position().into();
            //     let end: nvim::model::Cursor = node.end_position().into();
            //     // crate::log_libuv!(Trace,"-- {:?}({start}, {end}) --", captures_names.get(capture.index as usize));
            // }
            // let mut docstring_selction = if let Some(c) = m.captures.first() {
            //     let node = c.node;
            //     let start: nvim::model::Cursor = node.start_position().into();
            //     let end: nvim::model::Cursor = node.end_position().into();
            //     nvim::model::Selection { start, end }
            // } else {
            //     continue;
            // };
            let mut all_captured = String::new();
            for capture in m.captures.iter() {
                let node = capture.node;
                // if node.grammar_name == ""
                let start_byte = node.start_byte();
                let end_byte = node.end_byte();
                all_captured += &content[start_byte..end_byte];
            }
            targets.push(all_captured);
        }
        Ok(targets)
    }
    pub fn extract_query_vec_cursor(self, query: &str) -> crate::Result<Vec<Cursor>> {
        let TreeSitterContext {
            language,
            tree,
            content,
            ..
        } = &self;
        let query = Query::new(&language, query)?;
        let mut query_cursor = QueryCursor::new();
        // let mut matches = cursor.matches(&query, tree.root_node(), content.as_bytes());
        // let captures_names = query.capture_names();
        let mut matches = query_cursor.matches(&query, tree.root_node(), content.as_bytes());

        let mut targets = Vec::new();
        while let Some(m) = matches.next() {
            if let Some(c) = m.captures.first() {
                let node = c.node;
                let start: nvim::model::Cursor = node.start_position().into();
                targets.push(start);
            }
        }
        Ok(targets)
    }
}

// Example : Compter les voyelles
impl Pipe<messages::Visual> {
    fn extract_selection(self) -> Pipe<(String, Cursor)> {
        let (selected_content, cursor) = self.args.get_selected_content();
        // crate::log_libuv!(Trace,".........................................Init  : {cursor:?}\n\n”");
        self.next((selected_content, cursor))
        // pipe!(self -> selected_content)
    }
}

impl Pipe<messages::Normal> {
    fn lines_split_at_cursor(self) -> Pipe<(String, Option<String>, Cursor)> {
        let messages::Normal {
            data: nvim::model::BufferData { cursor, content, .. },
            ..
        } = &self.args;
        let ctx = if content.len() <= *cursor.row {
            ("".to_string(), Some("".to_string()), cursor.clone())
        } else {
            let (prefix, suffix) = content.split_at(*cursor.row);
            (prefix.join("\n"), Some(suffix.join("\n")), cursor.clone())
        };
        pipe!(self -> ctx)
    }
    fn tree_sitter(self) -> crate::Result<Pipe<TreeSitterContext>> {
        let ctx = TreeSitterContext::from_bufferdata(self.args.data)?;
        Ok(pipe!(self -> ctx))
    }
}

impl Pipe<TreeSitterContext> {
    fn extract_query_under_cursor(self, query: &str) -> crate::Result<Pipe<(String, Cursor)>> {
        let TreeSitterContext {
            language,
            tree,
            content,
            cursor,
        } = &self.args;
        let query = Query::new(&language, query)?;
        let mut query_cursor = QueryCursor::new();
        // let mut matches = cursor.matches(&query, tree.root_node(), content.as_bytes());
        // let captures_names = query.capture_names();
        let mut matches = query_cursor.matches(&query, tree.root_node(), content.as_bytes());

        while let Some(m) = matches.next() {
            for capture in m.captures {
                let node = capture.node;
                let start: nvim::model::Cursor = node.start_position().into();
                let end: nvim::model::Cursor = node.end_position().into();
                // crate::log_libuv!(Trace,"-- {:?}({start}, {end}) --", captures_names.get(capture.index as usize));
                if start <= *cursor && *cursor <= end {
                    let mut all_captured = String::new();
                    for capture in m.captures.iter() {
                        let node = capture.node;
                        let start_byte = node.start_byte();
                        let end_byte = node.end_byte();
                        all_captured += &content[start_byte..end_byte];
                    }
                    // let cursor_to_send = nvim::model::Cursor {
                    //     row: end.row,
                    //     col: end.col,
                    // };
                    // self.send(MistralMessage::InitializeTask(end));
                    return Ok(pipe!(self -> (all_captured, end)));
                }
            }
        }
        Err("Not found".into())
    }
}

impl Pipe<(String, Cursor)> {
    fn create_fim_payload(self) -> Pipe<FimRequest> {
        self.send(MistralMessage::InitializeTask(self.args.1.clone()));
        let request = FimRequest {
            completion: FimCompletion {
                model: Model::fim(),
                prompt: self.args.0,
                suffix: None,
            },
            params: CompletionParams::default(),
        };
        pipe!(self -> request)
    }
}
impl Pipe<(String, Option<String>, Cursor)> {
    fn create_fim_payload(self) -> Pipe<FimRequest> {
        self.send(MistralMessage::InitializeTask(self.args.2.clone()));
        let request = FimRequest {
            completion: FimCompletion {
                model: Model::fim(),
                prompt: self.args.0,
                suffix: self.args.1,
            },
            params: CompletionParams::default(),
        };
        pipe!(self -> request)
    }
}

impl<Serializable> Pipe<Serializable>
where
    Serializable: serde::Serialize,
{
    fn to_json_value(self) -> crate::Result<Pipe<serde_json::Value>> {
        Ok(pipe!(self -> serde_json::to_value(self.args)?))
    }
}

impl Pipe<serde_json::Value> {
    fn initialize_task_default(self) -> Self {
        self.send(messages::MistralMessage::InitializeTask(nvim::model::Cursor::zero()));
        self
    }
    async fn send_stream_request(self, route: &str) {
        let sendle = SenderHandle::clone(&self.context.nvim_sendle);
        let id = self.id.clone();
        let callback = move |response: StreamResponse| {
            sendle.send(id, MistralMessage::FinalizeTask(response));
        };

        let client = MistralClient::clone(&self.context.client);
        let mut lock = self.context.tasks.lock().await;
        let task_entry = lock.entry(self.id);
        let route = route.to_string();
        match task_entry {
            Entry::Occupied(_) => {
                self.notify_error("Task already running for given ID.");
            }
            Entry::Vacant(vacant) => {
                let should_abort = Arc::new(AtomicBool::new(false));
                let should_abort_clone = Arc::clone(&should_abort);
                crate::log_tokio!(Error, "Send Request : {}", self.args);
                let task = tokio::task::spawn(async move {
                    client
                        .stream(&route, self.args, callback, should_abort, self.id)
                        .await
                });
                vacant.insert(AbortHandle::new(should_abort_clone, task));
            }
        }
    }
}

pub async fn cursor(id: IdMessage, message: messages::Normal, context: SharedContext) -> crate::Result<()> {
    Pipe::new(message, context, id)
        .lines_split_at_cursor()
        .create_fim_payload()
        .to_json_value()?
        .send_stream_request("fim/completions")
        .await;
    Ok(())
}

pub async fn function(id: IdMessage, message: messages::Normal, context: SharedContext) -> crate::Result<()> {
    Pipe::new(message, context, id)
        .tree_sitter()?
        // .map_err(|err| err.to_string())?
        .extract_query_under_cursor(
            "([(block_comment(doc_comment)) (line_comment(doc_comment))]* @docstring . (attribute_item)* @attribute . (function_item) @function)",
        )?
        .create_fim_payload()
        .to_json_value()?
        .send_stream_request("fim/completions")
        .await;
    Ok(())
}

pub async fn visual(id: IdMessage, message: messages::Visual, context: SharedContext) -> crate::Result<()> {
    Pipe::new(message, context, id)
        .extract_selection()
        .create_fim_payload()
        .to_json_value()?
        .send_stream_request("fim/completions")
        .await;
    Ok(())
}

pub async fn chat_completion(id: IdMessage, message: ChatRequest, context: SharedContext) -> crate::Result<()> {
    Pipe::new(message, context, id)
        .to_json_value()?
        .initialize_task_default()
        .send_stream_request("chat/completions")
        .await;
    Ok(())
}

pub async fn abort_task(id: IdMessage, context: SharedContext) -> crate::Result<()> {
    let mut tasks = context.tasks.lock().await;
    match tasks.entry(id) {
        Entry::Occupied(mut occupied_entry) => {
            let task: &mut AbortHandle = occupied_entry.get_mut();
            task.soft_abort();
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            task.hard_abort();
            occupied_entry.remove();
        }
        Entry::Vacant(_) => (),
    }
    Ok(())
}
