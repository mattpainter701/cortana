# Cortana

Cortana is a Rust-first, terminal-native AI assistant prototype. This repository now contains the first implementation slice: a command-line surface for no-token summaries, coding-agent integration outputs, speech scaffolding, and the facial-animation parameter protocol.

## Current Features

- `boot`: renders the minimal cyan activation ring and `Hi, I’m Cortana.` greeting at startup.
- `appear` / `appearance`: renders the grainy blue pixel-hologram presence reveal used when Cortana appears to speak.
- `session start`: renders the full session-start sequence by combining the boot ring, hologram reveal, and online status.
- `summarize --fast --mode text`: creates a zero-token extractive recap from text.
- `summarize --fast --mode code`: extracts code symbols and comments for rapid file recaps.
- `summarize --fast --mode diff`: summarizes changed files from unified git diffs.
- `--json`: emits machine-readable output for Claude Code, OpenCode, or other terminal agents.
- `speak`: provides the placeholder command that will later route text into system TTS or ElevenLabs.
- `daemon --stdio`: starts with the session visual sequence, then emits an initial `FACE ...` protocol line for the renderer.
- `FaceParams`/`FaceState`: models continuous mouth, gaze, brow, valence, arousal, and smoothing values for the future TUI renderer.

## Examples

```bash
cargo run -- boot
cargo run -- appear
cargo run -- session start
cargo run -- summarize --fast --mode text CORTANA_IMPLEMENTATION.md
cargo run -- summarize --fast --mode code --json src/main.rs
git diff | cargo run -- summarize --fast --mode diff --json
cargo run -- speak "Build completed successfully."
cargo run -- daemon --stdio
```

## Token Usage

Fast summaries are local and extractive. They do not call an LLM and report `token_usage: 0` in JSON output.

## Next Implementation Targets

1. Add persistent session storage for `recap --session current`.
2. Replace the `speak` placeholder with a system TTS provider.
3. Add a `ratatui` layout around the CLI core.
4. Convert the boot and appearance sequences into frame-based Kitty/Sixel/ASCII visualizer animations driven by `FaceParams`.
5. Add Claude Code/OpenCode hook examples once the JSON contract stabilizes.
