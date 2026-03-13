# Configuration

`query-rs` stores its configuration in a JSON file.

## Config Location
`$HOME/.config/query.rs/config.json`

## Configuration Structure

The file is automatically created and managed by the application, but it can be edited manually.

```json
{
  "models": {
    "my-gpt4": {
      "name": "gpt-4o",
      "api_key": "sk-...",
      "base_url": "https://api.openai.com/v1",
      "provider": "OpenAICompat"
    },
    "my-gemini": {
      "name": "gemini-1.5-pro",
      "api_key": "AIza...",
      "base_url": "https://generativelanguage.googleapis.com",
      "provider": "Gemini"
    }
  },
  "current_model": "gemini-2.5-pro",
  "mcp_servers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/home/femboi/Projects"],
      "env": {}
    }
  }
}
```

### Fields

- `models`: A map of friendly names to model configurations.
    - `provider`: Either `OpenAICompat` or `Gemini`.
    - `base_url`: The endpoint for the API (defaults to standard endpoints if omitted during `/model` command).
- `current_model`: The key of the model currently used for chat.
- `mcp_servers`: A map of server names to their executable configuration.
    - `command`: The binary to run (e.g., `npx`, `python3`, `node`).
    - `args`: List of arguments to pass to the command.
    - `env`: Environment variables for the server process.

## Using Local LLMs (Ollama)

To use Ollama, use the `openai` provider and point it to the Ollama API:
```bash
/model openai my-llama-3 empty http://localhost:11434/v1
```
(Ollama does not require an API key, so `empty` or any string can be used).
