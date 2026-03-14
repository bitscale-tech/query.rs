# query.rs
An AI client for the terminal.

## Features

- **Rich Markdown Rendering**: AI responses are rendered with syntax highlighting and rich formatting.
- **Model Context Protocol (MCP)**: Native support for MCP servers to give the AI access to tools.
- **Provider Support**: Works with OpenAI-compatible APIs (Groq, Ollama), Google Gemini, Anthropic Claude, and more.
- **Full Mouse Support**: Selection, scrolling, and interaction via mouse.

**// TODO for v1:**
- Better MCP support
- New Chat UI
- Some more providers
- Better error handling
- Adding the other parts of docs into the `/help` menu
- .env support for the API Configuration
- Memory at `~/.config/query.rs/memory.json`
- Token count menu

## Documentation

Comprehensive documentation is available in the `docs/` directory:
- [Installation](docs/installation.md) - Building and setup.
- [Usage Guide](docs/usage.md) - Commands and keyboard shortcuts.
- [Configuration](docs/configuration.md) - JSON structure and settings.
- [MCP Integration](docs/mcp.md) - How to use Model Context Protocol.
- [Architecture](docs/architecture.md) - Internal design overview.

## Installation

You can build from source:

```bash
cargo build --release
```

Or run the build script:

```bash
bash build.sh
```

## Usage

Run the binary:

```bash
./query-rs-x86_64-linux
```

### Quick Commands

- `/model <provider> <name> <api_key> [base_url]` - Add a new model.
- `/switch <model_name>` - Switch models.
- `/mcp add <name> <cmd> [args]` - Add an MCP tool.
- `/help` - Show all commands in an overlay.

## Configuration

Settings and models are stored in `~/.config/query.rs/config.json`.

## License

MIT
