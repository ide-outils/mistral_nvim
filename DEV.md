# How It Works

## **1. Global Structure**
### **Main Modules Tree**
- **`src/`**: Core of the plugin.
  - **`mistral/`**: Interaction with the Mistral API.
    - **`client.rs`**: HTTP client for requests to the Mistral API (uses `reqwest`).
    - **`controlleur/`**: Business logic for building requests (e.g., FIM, chat).
    - **`model/`**: Data models for interacting with the Mistral API (e.g., `Completion`, `ToolCall`).
  - **`nvim/`**: Integration with Neovim.
    - **`controlleur/`**: Entry points for adding commands, keymaps, and auto-commands.
    - **`model/`**: Data representation for interacting with Neovim (e.g., `Buffer`, `Cursor`).
    - **`vue/`**: Receives messages from `src/mistral/` and updates Neovim.
  - **`utils/`**: Common utilities (logger, shared types, etc.).
  - **`lib.rs`**: Main entry point of the plugin.
  - **`messages.rs`**: Definition of messages exchanged between `src/nvim/` and `src/mistral/`.

- **`code_modifier/`**: Internal library for manipulating code (e.g., parsing, injections).
- **`mistral_nvim_derive/`**: Procedural macros for generating `Forms` and `Tools`.

---

### **Role of Key Modules**
| Module                     | Role                                                                                     |
|----------------------------|------------------------------------------------------------------------------------------|
| **`mistral/client`**       | HTTP client for interacting with the Mistral API (uses `reqwest` for requests).     |
| **`mistral/model`**        | Data models for API requests/responses (e.g., `Completion`, `ToolCall`).               |
| **`nvim/model`**           | Represents data for modifying Neovim's state/display (`Buffer`, `Cursor`).            |
| **`nvim/controlleur`**     | Entry points for initializing the plugin (commands, keymaps, auto-commands).         |
| **`nvim/vue`**             | Interaction logic between Mistral and Neovim (e.g., updating buffers).                 |
| **`nvim/model/tool_mode`** | Definition of tools available to Mistral (e.g., `CodeRefactorisation`).               |

---

## **2. Key Entry Points**
### **Neovim API Commands**
- **`src/nvim/controlleur/mod.rs`**: Defines commands and keymaps.
  - Example command:
    ```rust
    ncmd(s, n!(FimFunction), "MistralFIMFunction", c_opts().desc("Applies FIM to the function under the cursor."))?;
    ```
  - Example keymap:
    ```rust
    nmap(s, n!(FimFunction), "<Leader>mff", k_opts().desc(d).noremap(true))?;
    ```

### **Main Rust Functions**
- **`src/lib.rs`**: Entry point of the plugin.
  - Initializes communication channels (`mpsc`) between Neovim and Mistral.
  - Launches the `mistral_loop` thread to handle API requests.

- **`src/nvim/vue/chat.rs`**: Handles messages received from Mistral for chats.
  - Example of message processing:
    ```rust
    match message {
        MistralMessage::UpdateContent(chunk) => chat.lock().insert(chunk, Some(assistant_index))?,
        MistralMessage::RunTool(tool_calls) => mode.run_tool(state, run_tool_message),
    }
    ```

---

## **3. Tools**
### **How Tools Work**
- **Definition**: Tools are defined in `src/mistral/model/tools.rs` and `src/nvim/model/tool_mode/`.
  - Example: `CodeRefactorisation` allows code modification via tools like `CodeModifier`.
- **Execution**:
  - Mistral sends a `ToolCall` (e.g., `CodeRetriever`).
  - The plugin executes the tool and returns the result to Mistral.
  - Example code:
    ```rust
    for tool in tool_calls {
        let tool_id = crate::utils::tool_id::tool_id_to_usize(tool.id.as_ref().unwrap());
        let message = mode.run_tool(state, run_tool_message);
    }
    ```

### **Creating a New Tool**
1. **Define the Structure**:
   - Use the `Tool` derive to automatically generate the JSON schema.
   - Example:
     ```rust
     #[derive(Tool)]
     #[description("Get a file's content.")]
     struct CodeRetriever {
         #[description("Path to the file.")]
         file: String,
     }
     ```
2. **Implement `Runnable`**:
   - Define the logic in the `run` method.
   - Example:
     ```rust
     impl Runnable for CodeRetriever {
         type Ok = String;
         type Err = String;
         fn run(&mut self, _state: SharedState, _msg: RunToolMessage) -> Result<Self::Ok, Self::Err> {
             std::fs::read_to_string(&self.file).map_err(|e| e.to_string())
         }
     }
     ```
3. **Add the Tool to a Mode**:
   - In `src/nvim/model/tool_mode/mod.rs`, add the tool to the list of available tools.
   - Example:
     ```rust
     impl Mode {
         pub fn current_tools(&self) -> Vec<Tool> {
             match self {
                 Self::CodeRefactorisation => CodeRefactorisation::get_tools(),
             }
         }
     }
     ```

---

## **4. Data Flow**
### **Data Circulation**
1. **Neovim → Rust**:
   - A Neovim command (e.g., `:MistralFIMFunction`) triggers a call to a Rust function via `nvim_oxi`.
   - Example:
     ```rust
     ncmd(s, n!(FimFunction), "MistralFIMFunction", c_opts().desc(d))?;
     ```
2. **Rust → Tokio**:
   - The Rust function sends a message via an `mpsc` channel to a Tokio thread.
   - Example:
     ```rust
     state.tx_mistral.send(envelop).unwrap();
     ```
3. **Tokio → Mistral API**:
   - The Tokio thread sends an HTTP request to the Mistral API via `reqwest`.
   - Example:
     ```rust
     self.request(reqwest::Method::POST, endpoint).body(body).send().await
     ```
4. **Mistral API → Tokio**:
   - The Mistral API responds with a stream (`Stream`) managed by Tokio.
   - Example:
     ```rust
     let mut stream = response.bytes_stream();
     while let Some(chunk) = stream.next().await { ... }
     ```
5. **Tokio → Libuv**:
   - Tokio sends the received chunks to a Libuv thread via an `mpsc` channel.
   - Example:
     ```rust
     self.0.sendle_nvim.send_enveloppe(MistralEnveloppe { id, message });
     ```
6. **Libuv → Neovim**:
   - The Libuv thread updates the Neovim buffer via `nvim_oxi`.
   - Example:
     ```rust
     oxi::schedule(move |_| nvim::vue::chat::handle_nvim_message(...));
     ```

---

## **5. External Dependencies**
### **Rust Libraries Used**
| Library               | Role                                                                                     |
|-----------------------|------------------------------------------------------------------------------------------|
| `nvim_oxi`            | Integration with Neovim (nvim API, buffer management, etc.).                            |
| `tokio`               | Async runtime for managing HTTP requests and streams.                                    |
| `reqwest`             | HTTP client for interacting with the Mistral API.                                       |
| `serde` + `serde_json`| Serialization/deserialization of data (JSON).                                           |
| `schemars`            | JSON schema generation for tools.                                                       |
| `futures`             | Async stream management.                                                                 |

---

## **6. Concrete Example: `:RunTests` Workflow**
### **Workflow Steps**
1. **Triggering**:
   - The user creates a chat with `:MistralNewChat`.
   - The user fills out the chat creation form.
   - The chat is initialized.
   - The user executes `:MistralChatSendPrompt` or uses the shortcut `<CR><CR>`.

2. **Data Preparation**:
   - The `controlleur` retrieves the context (buffer, selection, etc.) and sends a message to `src/mistral/`:
     ```rust
     let envelop = NvimEnveloppe {
         id: IdMessage::Chat(buffer.handle(), message_index),
         message: NvimMessage::Chat(request),
     };
     state.tx_mistral.send(envelop).unwrap();
     ```

3. **Sending to Mistral**:
   - The Tokio thread receives the message and sends a request to the Mistral API:
     ```rust
     let response = client.request(reqwest::Method::POST, "chat/completions").body(body).send().await;
     ```

4. **Receiving the Stream**:
   - Mistral responds with a stream of chunks. Each chunk is processed and sent to Neovim:
     ```rust
     while let Some(chunk) = stream.next().await {
         self.send(id, MistralMessage::UpdateContent(chunk));
     }
     ```

5. **Updating Neovim**:
   - The Libuv thread receives the chunks and updates the Neovim buffer:
     ```rust
     match message {
         MistralMessage::UpdateContent(chunk) => chat.lock().insert(chunk, Some(assistant_index))?,
     }
     ```

6. **Executing Tools**:
   - If Mistral calls a tool (e.g., `CodeModifier`), the plugin executes it and returns the result:
     ```rust
     for tool in tool_calls {
         let message = mode.run_tool(state, run_tool_message);
         chat.push_message(message, Some(assistant_index))?;
     }
     ```

---

## **Summary for Integration with Mistral**
- **Entry Points**: Neovim commands (e.g., `:MistralFIMFunction`) and auto-commands (e.g., on `*.chat`) are the entry points.
- **Tools**: Defined in `mistral/model/tools.rs` and executed via `nvim/model/tool_mode/`.
- **Data Flow**:
  - Neovim → Libuv → Tokio → Mistral API → Tokio → Libuv → Neovim.
- **Extensions**:
  - Use `mistral_nvim_derive` to simplify the creation of tools and forms.
  - Add new tools in `src/nvim/model/tool_mode/`.