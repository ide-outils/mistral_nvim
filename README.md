# Mistral Neovim Plugin

A plugin for conversing or using Fill-In-The-Middle (FIM) with Mistral.
The conversational chat is file-based, allowing you to modify conversations as needed.
This plugin allows you to select a set of tools that Mistral can use. Currently, only the `CodeRefactorisation` toolset exists.
You can also switch models during a conversation or modify received responses to correct any errors or inaccuracies.

[French Version](README_fr.md)

## **Features**

- **Fill In the Middle (FIM)**: Smart code completion based on context.
- **Interactive Chat**: Generate content with Mistral to refactor, document, or generate code.
- **Built-in Tools**: Use tools like `CodeModifier`, `CodeRetriever`, or `CodeTree` to interact with your project.
- **Customization**: Create your own tools and modes to extend the plugin's capabilities.

## **Installation**

### **Prerequisites**
- Neovim 0.11 or higher (check `nvim_oxi` features in `Cargo.toml`).
- Neovim tree-sitter installed with at least (`markdown`, `ron`, and any language that appears in a conversation).
- Tested with Rust nightly 1.94 (for compilation).
- A Mistral API key (to be configured in your environment).

### **Manual Compilation**

Currently, only manual compilation is supported.

1. Clone the repository:
   ```sh
   git clone https://github.com/[your-user]/mistral.nvim.git
   cd mistral.nvim
   ```

2. Compile the plugin:
   ```sh
   cargo build --release --features=prod_mode
   plugin_path=`pwd`
   ```

3. Add a symbolic link in your Neovim configuration directory:
    ```sh
    cd ~/.config/nvim
    ln -s "$plugin_path/target/release/libmistral.so" mistral_nvim.so
    ```

4. Add the plugin to your Neovim configuration:
    ```lua
    require("mistral_nvim")
    ```

## **Configuration**

### **Environment Variables**
- `MISTRAL_API_KEY`: Your Mistral API key.

### **WIP: Configuration**

Add this to your `init.lua`:

```lua
require('mistral').setup {
    -- Example configuration
    log_level = "info",  -- Available levels: "trace", "debug", "info", "warn", "error", "off"
    keymaps = {
        fim_function = "<Leader>mff",  -- Shortcut for FIM on a function
        fim_visual = "<Leader>mf",    -- Shortcut for FIM in visual mode
    },
}
```

## **Usage**

### **Fill In the Middle (FIM) (FIXME: minor regression)**

1. **On a function**: Place the cursor on a function and execute `:MistralFIMFunction` or use the shortcut `<Leader>mff`.
2. **On a visual selection**: Select code in visual mode and execute `:MistralFIMVisual` or use the shortcut `<Leader>mf`.
3. **On the cursor line**: Execute `:MistralFIMCursor` or use the shortcut `<Leader>mfc`.

### **Interactive Chat**

1. **Create a chat**: Open a `*.chat` buffer, then execute `:MistralNewChat`, fill out the form (use `<tab>` to switch fields, `<CR>` to confirm, `<Esc>` to cancel).
2. **Send a prompt**: Write your prompt and execute `:MistralChatSendPrompt` or use `<CR><CR>`.
3. **Use tools**: Mistral can call tools like `CodeRefactorisation` to interact with your code. Activate tools with `:MistralChatChangeMode`.
4. **Change model**: You can change the model for the next prompt with `:MistralChatChangeModel`. Thus, a conversation can be managed by different models.
5. **Adjust responses**: If a response doesn't suit you, modify it to align with your project's reality.
6. **Track token usage**: Monitor token consumption during the conversation.
7. **Add a new prompt**: For now, you need to manually add a new prompt after a completion: `:MistralChatNewPrompt`.

### **Example Workflow**

1. Open a Rust file.
2. Select a function in visual mode.
3. Execute `:MistralFIMVisual`.
4. Mistral completes the code based on context.

## **Funding**

This project is for personal use. Without funding, maintenance and feature additions will be based on my needs.
If you are interested in specific features, you can always open a ticket. Since I am currently unemployed, I have free time.

## **Technical Documentation**

For more details on the project architecture, contributions, or developing new features, refer to the [`DEV.md`](DEV.md) file.

## **License**

This project is licensed under the **MIT** License.