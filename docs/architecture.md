# Architecture Overview

This document describes the internal structure of `query-rs`.

## Module Structure

- `main.rs`: The entry point. Manages the TUI event loop, state, and UI rendering (using `ratatui` and `crossterm`).
- `api.rs`: The AI provider abstraction. Handles request formatting, response parsing, and tool-call detection for OpenAI and Gemini.
- `mcp.rs`: The Model Context Protocol (MCP) manager. Manages child processes for MCP servers and handles tool listing/invocation via `rmcp`.
- `config.rs`: Handles persistent storage of settings, model credentials, and MCP server definitions.

## The Chat Loop

`query-rs` implements an asynchronous multi-turn chat loop:

1. **User Input**: The user submits a message via the TUI.
2. **Persistence**: The message is added to the in-memory history.
3. **API Request**: `api.rs` constructs a payload including history and available MCP tools.
4. **Tool Call Detection**:
   - If the AI returns text, it's displayed.
   - If the AI returns a **Tool Call** (e.g., `filesystem:read_file`), the loop enters a sub-step.
5. **Tool Execution**:
   - `mcp.rs` identifies the owner of the tool.
   - The tool is executed via the MCP protocol.
   - The result is sent back to the AI provider.
6. **Final Response**: The AI provides a final text response based on the tool's output.

## Concurrency

- **UI Thread**: Runs the TUI loop, handling drawing and user input synchronously to ensure responsiveness.
- **Async Runtime (`tokio`)**: Offloads network requests and MCP child process management to background tasks.
- **Message Channel**: Background tasks communicate results back to the UI thread via a `tokio::sync::mpsc` channel.

## Rendering

The UI is built with `ratatui` components:
- **Chat Area**: Uses `termimad` to render Markdown content and `ansi-to-tui` for status coloring.
- **Input Area**: A custom text buffer with cursor management.
- **Status Line**: Shows immediate feedback on command execution and backend activity.
