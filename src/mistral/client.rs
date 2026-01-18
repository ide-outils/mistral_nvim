use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use futures::StreamExt;
use reqwest::Client as ReqwestClient;
use serde_json::Value;
use tokio::time;

use crate::{
    log_tokio,
    messages::{self, IdMessage, MistralEnveloppe, MistralMessage},
    mistral::{
        controlleur::fim::SenderHandle,
        model::stream::{ErrorMessageType, Status, StreamError, StreamEvent, StreamParam, StreamResponse},
    },
};

#[macro_export]
macro_rules! json_values {
    [$($ser_object:ident),*] => {
        vec![$(serde_json::to_value(&$ser_object)?),*]
    }
}

pub fn json_merge_values(data: Vec<Value>) -> crate::Result<String> {
    let merged_map = data
        .into_iter()
        .filter_map(|v| if let Value::Object(map) = v { Some(map) } else { None })
        .flatten()
        .collect();
    Ok(serde_json::to_string_pretty(&Value::Object(merged_map))?)
}

#[derive(Clone)]
pub struct MistralClient(Arc<MistralClientInner>);
struct MistralClientInner {
    client: ReqwestClient,
    api_key: String,
    sendle_nvim: SenderHandle,
}

impl MistralClient {
    pub fn send(&self, id: messages::IdMessage, message: MistralMessage) {
        self.0
            .sendle_nvim
            .send_enveloppe(MistralEnveloppe { id, message });
    }
    pub fn notify_warn(&self, msg: impl ToString) {
        log_tokio!(Warn, "{}", msg.to_string());
        self.0.sendle_nvim.notify_warn(msg);
    }
    pub fn notify_error(&self, msg: impl ToString) {
        log_tokio!(Error, "{}", msg.to_string());
        self.0.sendle_nvim.notify_error(msg);
    }

    pub fn new(sendle_nvim: SenderHandle) -> Self {
        let client = ReqwestClient::new();
        let api_key = std::env::var("MISTRAL_API_KEY").expect("No env var MISTRAL_API_KEY.");
        // logs!("Mistral API : '{}'", api_key);
        Self(Arc::new(MistralClientInner {
            client,
            api_key,
            sendle_nvim,
        }))
    }

    pub fn request(&self, method: reqwest::Method, endpoint: &str) -> reqwest::RequestBuilder {
        self.0
            .client
            .request(method, format!("https://api.mistral.ai/v1/{}", endpoint))
            .header("Authorization", format!("Bearer {}", self.0.api_key))
    }

    pub async fn send_request<ReqBuilder>(&self, mut request: ReqBuilder) -> Result<reqwest::Response, Status>
    where
        ReqBuilder: FnMut(&Self) -> reqwest::RequestBuilder,
    {
        let response;
        let mut attempts = 0;
        'retry: loop {
            // logs!("Send requests");
            let result_send = request(&self).send().await;
            match result_send {
                Ok(response_unwrapped) => {
                    response = response_unwrapped;
                    break 'retry;
                }
                Err(err) => {
                    attempts += 1;
                    let secs = 4u64.pow(attempts);
                    let msg = format!(
                        "Fail to join mistral (attempt n°{}). Will attempt again in {} seconds. Error: {}",
                        attempts, secs, err
                    );
                    self.notify_warn(msg);
                    if attempts > 4 {
                        return Err(Status::Failed(
                            format!("~Error: Request failed. ({})~", err),
                            ErrorMessageType::default(),
                        ));
                    }
                    time::sleep(std::time::Duration::from_secs(secs)).await;
                }
            };
        }
        Ok(response)
    }
    #[cfg(feature = "prod_mode")]
    pub async fn stream<Callback>(
        &self,
        endpoint: &str,
        body: serde_json::Value,
        callback: Callback,
        should_abort: Arc<AtomicBool>,
        id: messages::IdMessage,
    ) where
        Callback: Fn(StreamResponse) + Send + Sync,
    {
        self.stream_inner(endpoint, body, callback, should_abort, id)
            .await;
    }
    #[cfg(not(feature = "prod_mode"))]
    pub async fn stream<Callback>(
        &self,
        _endpoint: &str,
        body: serde_json::Value,
        _callback: Callback,
        should_abort: Arc<AtomicBool>,
        id: messages::IdMessage,
    ) where
        Callback: Fn(StreamResponse) + Send + Sync,
    {
        // logs!("TEST STREAM BODY : {body}");
        let chunks = vec![
            vec!["```rust"],
            vec!["", "", ""],
            vec!["#["],
            vec!["cfg(test)]", "mod"],
            vec![" testé {", "    use"],
            vec![" super::*;", "", ""],
            vec!["    #[test]", ""],
            vec!["    fn test"],
            vec!["_fibonaccià() {", ""],
            vec!["        assert_eq!("],
            vec!["fibonacci(1"],
            vec!["), vec"],
            vec!["![1]);", "       "],
            vec![" assert_eq!(fibè"],
            vec!["onacci(2),"],
            vec![" vec![1,"],
            vec![" 1]);", "       "],
            vec![" assert_eq!(fib"],
            vec!["onacci(3),"],
            vec![" vec![1,"],
            vec![" 1, "],
            vec!["2]);", "        assert"],
            vec!["_eq!(fibonacci"],
            vec!["(4), vec"],
            vec!["![1, "],
            vec!["1, 2"],
            vec![", 3]);", ""],
            vec!["        assert_eq!("],
            vec!["fibonacci(5"],
            vec!["), vec![1"],
            vec![", 1,"],
            vec![" 2, "],
            vec!["3, 5"],
            vec!["]);", "   "],
            vec![" }", ""],
            vec!["}", "```"],
        ];
        let message = format!("{body:#?}");
        let level = crate::notify::NotifyLevel::Debug;
        self.send(id, MistralMessage::Notify { message, level });
        let _ = body;
        let content = chunks
            .iter()
            .map(|chunk| chunk.join("\n"))
            .collect::<Vec<_>>()
            .join("");
        for chunk in chunks {
            if should_abort.load(Ordering::Relaxed) {
                break;
            }
            let chunk = chunk.into_iter().map(ToString::to_string).collect();
            self.send(id, MistralMessage::UpdateContent(chunk));
            #[cfg(not(test))]
            tokio::time::sleep(std::time::Duration::from_millis(115)).await;
        }
        let mut response = StreamResponse::new();
        response.message.content = content;
        response.message.role = crate::mistral::model::Role::Assistant;
        response.usage.completion_tokens = 89;
        response.usage.prompt_tokens = 12;
        response.usage.total_tokens = 101;
        self.send(id, MistralMessage::FinalizeTask(response));
    }
    #[allow(dead_code)]
    async fn stream_inner<Callback>(
        &self,
        endpoint: &str,
        body: serde_json::Value,
        callback: Callback,
        should_abort: Arc<AtomicBool>,
        id: IdMessage,
    ) where
        Callback: Fn(StreamResponse) + Send + Sync,
    {
        // logs!("Start STREAM :");
        let stream_param = StreamParam { stream: true };
        let body = json_merge_values(vec![
            body,
            serde_json::to_value(stream_param).expect("Should not failed to parse my own struct."),
        ])
        .expect("Already parsed before, with just stream param added, it should never fail.");
        let request = move |client: &Self| {
            client
                .request(reqwest::Method::POST, endpoint)
                .body(body.clone())
        };
        let response = match self.send_request(request).await {
            Ok(r) => r,
            Err(status) => {
                let mut stream_response = StreamResponse::new();
                stream_response.status = status;
                callback(stream_response);
                return;
            }
        };

        let mut stream_response = StreamResponse::new();
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        log_tokio!(Debug, "\n\nSTART STREAM\n\n");
        // let mut role = Role::Assistant;
        // let mut partial_line = String::new();
        // logs!("Start receiving Stream");
        let mut prev_chunk: Option<Vec<u8>> = None;
        'stream: while let Some(chunk_result) = stream.next().await {
            if should_abort.load(Ordering::Relaxed) {
                // logs!("Task abort by user.");
                buffer.clear();
                break;
            }
            log_tokio!(Trace, "{:?}\n", chunk_result);

            let chunk = match chunk_result {
                Ok(chunk) => chunk,
                Err(_) => {
                    self.notify_error("Error: Chunk can't be retrieved.");
                    continue;
                }
            };
            let chunk = match prev_chunk.take() {
                Some(prev_chunk) => match chunk.try_into_mut() {
                    Ok(mut chunk_mut) => {
                        chunk_mut.extend_from_slice(prev_chunk.as_slice());
                        chunk_mut.into()
                    }
                    Err(chunk) => chunk,
                },
                None => chunk,
            };
            // logs!("\n-- Chunk received. --");
            let chunk_str = match std::str::from_utf8(&chunk) {
                Ok(c) => c.to_string(),
                Err(_err) => {
                    if chunk.len() < 50 {
                        // We probably are in the middle of a graphem, let's wait next chunk
                        prev_chunk = Some(chunk.into());
                        continue;
                    } else {
                        // Too many chunks inrow have failed, let's assume that the answer contains
                        // an utf-8 error.
                        let cow = String::from_utf8_lossy(&chunk);
                        self.notify_warn("Chunk can't be parsed to utf8.");
                        cow.to_string()
                    }
                }
            };
            let mut first_new_line = true;
            for char in chunk_str.chars() {
                if char == '\n' && first_new_line {
                    first_new_line = false
                } else if char == '\n' && !first_new_line {
                    first_new_line = true;
                    let next_message: String = buffer.drain(..).collect();
                    if next_message.starts_with("data: ") {
                        let data = &next_message[6..];
                        if data == "[DONE]" {
                            break 'stream;
                        }
                        match serde_json::from_str::<StreamEvent>(&data) {
                            Ok(event) => {
                                if let Some(usage) = event.usage {
                                    stream_response.usage += usage;
                                }
                                for choice in event.choices {
                                    if let Err(err) = stream_response
                                        .add_delta(choice.delta, SenderHandle::clone(&self.0.sendle_nvim), id)
                                        .await
                                    {
                                        stream_response.status = Status::Failed(
                                            format!("Failed to write FIFO : {err}"),
                                            ErrorMessageType::default(),
                                        );
                                        callback(stream_response);
                                        return;
                                    }
                                }
                            }
                            Err(error) => {
                                self.notify_error(format!("Error: Stream Json Parsing. {error}"));
                            }
                        }
                    } else {
                        self.notify_error("Line does not start with 'data: '.");
                        break 'stream;
                    }
                } else {
                    if char == '\0' {
                        // two \0 in row is reserved for telling the end of the file.
                        buffer.extend(&['\0', '0']);
                    } else {
                        buffer.push(char);
                    }
                }
            }
        }
        if !buffer.is_empty() {
            if let Ok(error) = serde_json::from_str::<StreamError>(&buffer) {
                let StreamError {
                    object,
                    type_,
                    message,
                    param,
                    code,
                } = error;
                let param = if let Some(param) = param {
                    &format!(" (param '{param}'")
                } else {
                    ""
                };
                let code = if let Some(code) = code {
                    &format!("(n° {code})")
                } else {
                    ""
                };
                stream_response.status = Status::Failed(format!("~{object}{code}{param} '{type_}'"), message);
                callback(stream_response);
                return;
            } else {
                self.notify_error(format!("Unknown error during stream. buffer left : {buffer}"));
            }
        }
        callback(stream_response);
    }
}

// pub async fn query<Callback>(endpoint: &str, body: serde_json::Value, callback: Callback, should_abort:
// Arc<AtomicBool>) where
//     Callback: Fn(StreamResponse) + Send + Sync,
// {
//     let stream_param = StreamParam { stream: true };
//     let body = json_merge_values(vec![body, serde_json::to_value(stream_param).unwrap()]);
//     let client = ReqwestClient::new();
//     let api_key = std::env::var("MISTRAL_API_KEY").expect("No env var MISTRAL_API_KEY.");
//     logs!("Mistral API : '{}'", api_key);
//     let result_send = client
//         .post(format!("https://api.mistral.ai/v1/{}", endpoint))
//         .header("Authorization", format!("Bearer {}", api_key))
//         .body(body.clone())
//         .send();

//     let mut stream_response = StreamResponse::new();

//     let mut stream = response.bytes_stream();
//     let mut buffer = String::new();
//     // let mut role = Role::Assistant;
//     // let mut partial_line = String::new();
//     crate::log_libuv!(Trace,"Start receiving Stream");
//     'stream: while let Some(chunk_result) = stream.next().await {
//         if should_abort.load(Ordering::Relaxed) {
//             crate::log_libuv!(Trace,"Task abort by user.");
//             buffer.clear();
//             break;
//         }

//         let chunk = match chunk_result {
//             Ok(chunk) => chunk,
//             Err(_) => {
//                 crate::log_libuv!(Trace,"Error: Chunk can't be retrieved.");
//                 continue;
//             }
//         };
