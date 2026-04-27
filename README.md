# ϟ Skycode

**Offline-first AI coding agent. Works with any model. No subscriptions.**

Skycode brings the Claude Code experience to local LLMs — interactive CLI, tool calling, file operations, git commands, multi-agent workflows — all running 100% on your machine via [Ollama](https://ollama.ai).

The key idea: integrate with **any AI**, especially free open-source models. Internally uses the Anthropic SDK format, translated to Ollama's API by SkyBridge — swap models with a single env var.

---

## Monorepo Structure

```
skycode/
├── sky-code/          # Rust workspace — main CLI engine (11 crates)
├── skybridge/         # Rust Axum proxy — Anthropic ↔ Ollama translator
├── sky-code-npm/      # npm distribution package + pre-built binaries
└── sky-code-gui/      # Tauri 2 desktop GUI (React + TypeScript)
```

## How It Works

```
You → sky (CLI or GUI) → SkyBridge :4000 → Ollama :11434 → local model
```

SkyBridge is the key piece: it speaks Anthropic API inward, Ollama API outward. This means any Ollama-compatible model (llama3, deepseek-coder, qwen, phi, mistral, gemma…) works out of the box.

---

## Quick Start

### Requirements
- [Ollama](https://ollama.ai/download) with at least one model pulled
- Windows 10/11 x64 (Linux also supported, macOS coming)

### Install a model
```bash
ollama pull llama3.1:8b
# or lighter:
ollama pull llama3.2:1b
```

### Run (no build needed — pre-built binaries included)
```powershell
cd sky-code
.\sky.bat        # interactive terminal chat
.\skyui.bat      # web UI at http://localhost:4321
```

### Or install via npm
```bash
npm install -g sky-code
skycode prompt "What is 2+2?"
```

---

## Three Modes

| Mode | How to enter | What it does |
|------|-------------|--------------|
| **Chat** | Default | Pure conversation |
| **Agent** | `--permission-mode workspace-write` | Tool calling: files, git, terminal, web search, tasks |
| **Team Agents** | `/agents` slash command | Multi-agent parallel workflows |

---

## Slash Commands

| Command | Description |
|---------|-------------|
| `/model` | Switch active model |
| `/agents` | Configure agent team |
| `/skills` | List available skills |
| `/doctor` | Health check |
| `/config` | Show configuration |

---

## Configuration

| Env Var | Default | Description |
|---------|---------|-------------|
| `FILANTHROPIC_BASE_URL` | `http://localhost:4000` | SkyBridge endpoint |
| `FILANTHROPIC_API_KEY` | `ollama` | Any value works locally |
| `FILANTHROPIC_MODEL` | `cloud-apus-4-6` | Model alias or Ollama model name |
| `OLLAMA_BASE_URL` | `http://localhost:11434` | Ollama server (used by SkyBridge) |

**Model aliases:**

| Alias | Maps to |
|-------|---------|
| `cloud-apus-4-6` | `llama3.1:8b` |
| `sky-sannet-4-6` | balanced mid-size model |
| `sky-haiku-4` | fast/lightweight model |

Or use any Ollama model name directly.

---

## Building from Source

Requires: Rust 1.83+, Node.js 18+ (GUI only)

```powershell
# CLI
cd sky-code && cargo build --release

# Proxy
cd skybridge && cargo build --release

# Desktop GUI
cd sky-code-gui && npm install && npm run tauri dev
```

---

## Status

| Component | Status |
|-----------|--------|
| sky-code CLI | ✅ Production-ready |
| skybridge proxy | ✅ Production-ready |
| npm package | ✅ Functional (win/linux binaries bundled) |
| Tauri GUI | 🔶 Beta (chat works, sessions/settings pending) |
| Team Agents | 🔶 Partial |

---

## Privacy

- Zero telemetry, zero external API calls
- All data stays in `.sky/` (local, gitignored)
- GDPR/CCPA compliant by design

---

## License

MIT — see [LICENSE](LICENSE)

© 2026 — Independent project, no affiliation with Anthropic PBC
