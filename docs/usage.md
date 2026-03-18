# Usage Guide

`query-rs` is a high-performance terminal chat interface (TUI) for interacting with AI models.

## Keyboard Shortcuts

- **Enter**: Send message or execute command.
- **Tab**: Switch between input area and model selection (sidebar).
- **Up / Down / PageUp / PageDown**: Scroll through chat history or the help menu.
- **Esc**: Close the help menu or exit the application.
- **Ctrl+C**: Force exit the application.
- **Mouse Scroll**: Scroll the chat history.
- **Mouse Click (Sidebar)**: Select an AI model.

## TUI Commands

Commands are entered in the input box starting with a forward slash `/`.

### Model Management
- `/add <provider> <name> <api_key> [base_url]`: Add a new model.
    - Providers: `gemini`, `openai`, `anthropic`, `groq`, `ollama`.
- `/model <model_name>`: Switch to a different configured model.
- `/sidebar`: Toggle the visibility of the models list.
- `/remove <model_name>`: Remove a model from your configuration.
- `/rename <old_name> <new_name>`: Rename an existing model.

### MCP Management
- `/mcp add <name> <command> [args...]`: Add and connect a new MCP server.
- `/mcp list`: List all currently configured MCP servers.

### General
- `/help`: Show the help menu overlay.
- `/clear`: Clear the current chat history.

## Chatting
Simply type your message and press **Enter**. If a model and MCP tools are configured, the AI will respond. If it needs to use a tool (like reading a file or searching), it will do so automatically and show the tool execution progress in the status bar.
