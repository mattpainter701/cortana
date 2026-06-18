# Cortana

Cortana is a Rust-first, terminal-native AI assistant prototype: a live terminal companion with a holographic avatar, no-token summaries, speech scaffolding, session memory, and optional OpenCode integration.

![Cortana terminal avatar](assets/cortana-avatar.png)

## Current Features

- `cortana` / `cortana tui`: launches the terminal-native Cortana interface with a live image-backed hologram avatar.
- `boot`: renders the minimal cyan activation ring and `Hi, I’m Cortana.` greeting at startup.
- `appear` / `appearance`: renders the grainy blue pixel-hologram presence reveal used when Cortana appears to speak.
- `session start`: renders the full session-start sequence by combining the boot ring, hologram reveal, and online status.
- `summarize --fast --mode text`: creates a zero-token extractive recap from text.
- `summarize --fast --mode code`: extracts code symbols and comments for rapid file recaps.
- `summarize --fast --mode diff`: summarizes changed files from unified git diffs.
- `--json`: emits machine-readable output for Claude Code, OpenCode, or other terminal agents.
- `speak`: routes text into system TTS and drives mouth/cheek expression events for the avatar.
- `daemon --stdio`: starts with the session visual sequence, then emits an initial `FACE ...` protocol line for the renderer.
- `opencode install`: installs the optional OpenCode addon while keeping `cortana` available as a standalone terminal TUI.
- `FaceParams`/`FaceState`: models continuous mouth, gaze, brow, cheek lift, valence, arousal, and smoothing values for avatar animation.
- Expression controller: maps idle, attentive, thinking, and speaking states into facial gestures.

## Examples

```bash
cortana
cortana tui
cargo run -- boot
cargo run -- appear
cargo run -- session start
cargo run -- summarize --fast --mode text CORTANA_IMPLEMENTATION.md
cargo run -- summarize --fast --mode code --json src/main.rs
git diff | cargo run -- summarize --fast --mode diff --json
cargo run -- speak "Build completed successfully."
cargo run -- daemon --stdio
cargo run -- opencode install --project
```

## Terminal-first with optional OpenCode addon

Run `cortana` or `cortana tui` for the full terminal-native experience. The OpenCode integration is intentionally an addon, not a replacement: install it with `cortana opencode install --project` to copy `integrations/opencode/cortana.ts` into `.opencode/plugins/cortana.ts`, or use `--global` for `~/.config/opencode/plugins/cortana.ts`. OpenCode auto-loads local plugins from those directories, and the addon exposes Cortana tools for the banner, zero-token recaps, and zero-token summaries. If the binary is not on `PATH`, set `CORTANA_BIN=/absolute/path/to/cortana` before launching OpenCode.

## Token Usage

Fast summaries are local and extractive. They do not call an LLM and report `token_usage: 0` in JSON output.

## Next Implementation Targets

1. Replace estimated speech amplitude with real audio/viseme timing.
2. Add bidirectional voice input.
3. Add richer facial landmarks for eyes, mouth, cheeks, and brows.
4. Add image-protocol rendering for terminals that support higher fidelity than cell pixels.
5. Extend the OpenCode addon with richer TUI commands once OpenCode exposes more stable sidebar/panel APIs.
