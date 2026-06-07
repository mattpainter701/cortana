---
name: cortana
description: Terminal-native AI assistant — summarize files/code/diffs, speak text aloud, manage session recaps, and render a Cortana hologram face. Invoke via `cortana` CLI in the project workspace.
---

## What Cortana does

Cortana is a Rust-based terminal AI assistant. It runs as a CLI binary (`cortana`) in this project. It provides:

- **Zero-token text/code/diff summaries** via extractive algorithms (no LLM cost)
- **System TTS speech** — speak text aloud
- **Session tracking** — recaps of what you worked on
- **TUI mode** — interactive terminal UI with animated Cortana hologram face
- **JSON output** — machine-readable summaries for agent handoff

## How to invoke

All commands are run from the project root (`I:\deepseek\cortana\cortana`):

### Summarize a file
```
cortana summarize --fast --mode text <filepath>
cortana summarize --fast --mode code <filepath>
```

### Summarize a git diff
```
git diff | cortana summarize --fast --mode diff
```

### Get JSON output (for agents)
```
cortana summarize --fast --mode code --json src/main.rs
git diff | cortana summarize --fast --mode diff --json
```

### Speak text
```
cortana speak "Build completed successfully."
```

### Session recap
```
cortana session recap
cortana session export
```

### Launch TUI
```
cortana tui
```

### Boot splash / appearance
```
cortana boot
cortana appear
```

## When to use

Use Cortana when:
- You need a fast, token-free summary of a file or diff
- You want text spoken aloud via system TTS
- You need a session recap or structured JSON output
- You want to see the animated Cortana hologram face react to conversation
