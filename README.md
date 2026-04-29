# Skycode

Offline AI coding agent — tool calling, file operations, git, and multi-agent workflows via local Ollama models

## Overview

Replicates the interactive coding agent experience using local LLMs instead of a cloud API. The CLI uses the Anthropic SDK message format internally; SkyBridge translates that to Ollama's API at runtime. Swapping models requires one env var change. No external API calls, no telemetry, no subscription.

## Features

- Interactive terminal chat with tool calling — file read/write, shell execution, git commands, web search
- Three operating modes: chat, agent (workspace-write), and multi-agent team workflows
- Slash commands: `/model`, `/agents`, `/skills`, `/doctor`, `/config`
- Model alias system — maps short names (`cloud-apus-4-6`, `sky-haiku-4`) to Ollama model names; any Ollama model name works directly
- Pre-built binaries distributed via npm — no Rust toolchain needed for end users
- Desktop GUI via Tauri 2 (beta) — chat functional; sessions and settings panel pending
- Zero telemetry — all state stored locally in `.sky/` (gitignored)
- `unsafe_code = "forbid"` enforced across all 11 Rust crates at compile time

## Architecture

```
CLI / GUI  →  SkyBridge :4000  →  Ollama :11434  →  local model
```

SkyBridge is an Axum HTTP proxy that speaks Anthropic API inward and Ollama API outward. The CLI engine (`sky-code`) is built as a Rust workspace of 11 crates. The Tauri GUI wraps the CLI in a desktop shell. The npm package bundles pre-built Windows and Linux binaries.

### Components

| Component | Role |
|---|---|
| `sky-code/` | Rust workspace (11 crates) — main CLI engine |
| `skybridge/` | Rust Axum proxy — Anthropic ↔ Ollama API translation; listens on :4000 |
| `sky-code-npm/` | npm distribution package — pre-built binaries for Windows and Linux |
| `sky-code-gui/` | Tauri 2 desktop GUI — React + TypeScript frontend over the CLI |

## Tech Stack

| Technology | Role |
|---|---|
| Rust 1.83+ | CLI engine and SkyBridge proxy |
| Axum | HTTP framework for SkyBridge |
| Tauri 2 | Desktop GUI shell |
| React + TypeScript | GUI frontend |
| Ollama | Local LLM runtime (external dependency) |
| npm | Binary distribution |

## Installation

### Via npm (no Rust required)

```bash
npm install -g sky-code
```

### From source

```bash
# Requirements: Rust 1.83+, Ollama running

# CLI
cd sky-code
cargo build --release

# Proxy
cd skybridge
cargo build --release

# Desktop GUI (requires Node.js 18+)
cd sky-code-gui
npm install && npm run tauri dev
```

Pull at least one model before first run:

```bash
ollama pull llama3.1:8b
```

## Usage

```powershell
# Interactive terminal agent (Windows)
cd sky-code
.\sky.bat

# Web UI at http://localhost:4321
.\skyui.bat
```

```bash
# One-shot prompt (npm install)
skycode prompt "Explain this function"
```

| Env Var | Default | Description |
|---|---|---|
| `FILANTHROPIC_BASE_URL` | `http://localhost:4000` | SkyBridge endpoint |
| `FILANTHROPIC_API_KEY` | `ollama` | Any non-empty value |
| `FILANTHROPIC_MODEL` | `cloud-apus-4-6` | Model alias or Ollama model name |
| `OLLAMA_BASE_URL` | `http://localhost:11434` | Ollama server |

To switch models, set `FILANTHROPIC_MODEL` to any Ollama model name or a defined alias.

## Project Structure

```
skycode/
├── sky-code/             — Rust workspace (11 crates); main CLI engine
├── skybridge/            — Rust Axum proxy; Anthropic ↔ Ollama translation
├── sky-code-npm/         — npm package; pre-built binaries
├── sky-code-gui/         — Tauri 2 desktop GUI (beta)
└── sky-code/
    └── .sky/             — local state (gitignored)
```

## Notes

SkyBridge's proxy layer means the CLI is not coupled to Ollama specifically — any backend that speaks the Anthropic Messages API (including the real Anthropic API) works by pointing `FILANTHROPIC_BASE_URL` at it.

Team Agents (`/agents`) are partially implemented. Multi-agent workflow coordination compiles and initializes but some workflow types are incomplete.

macOS support is not currently included in the pre-built binaries. Building from source on macOS should work but is untested.

## License

MIT — see [LICENSE](LICENSE)
