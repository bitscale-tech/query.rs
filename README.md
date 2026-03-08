# query.rs
An AI client for the terminal.

## Features

- **Rich Markdown Rendering**: AI responses are rendered with syntax highlighting and rich formatting.
- **Auto-Wrapping & Scrolling**: Smooth terminal experience with full-text wrapping and manual/automatic scroll support.
- **Provider Support**: Works with OpenAI-compatible APIs (Groq, Ollama) and Google Gemini.
- **Model Management**: Easily add and switch between models via the `/model` command.
- **Fully Static Binaries**: Compiled against `musl` for zero-dependency deployment on Linux (x86_64 and aarch64).

#### // TODO: 

- {✓}~~**Fix aarch64 Builds** *by switching `aws-lc-rs` with `ring` and `rustls-tls-native-roots` with `rustls-tls-webpki-roots`*~~
- { }**Model removing**
- { }**Model renaming**
- { }**Mouse scroll support in Chat**
- { }**MCP access**
- { }**Config file**

## Installation

You can install `query.rs` using the provided install script:

```bash
curl -fsSL https://raw.githubusercontent.com/bitscale-tech/query.rs/main/install.sh | sh
```

Or build from source:

```bash
cargo build --release
```

## Usage

Run the binary:

```bash
./query-rs
```

### Commands

- `/model <provider> <name> <api_key> [base_url]` - Add a new model.
  - Providers: `openai`, `gemini`, `groq`, `ollama`
- `/switch <model_name>` - Switch to a different model.
- `/clear` - Clear chat history.
- `/help` - Show help message.
- `ESC` - Exit.

### Interaction

- **Sidebar**: Click a model name to switch models.

### Keybindings

- `Enter`: Send message
- `Up/Down/PgUp/PgDn`: Scroll chat history
- `Left/Right/Home/End`: Navigate input cursor
- `Delete/Backspace`: Edit text

## Configuration

Config is stored in `~/.config/query.rs/config.json`.

## License

MIT
