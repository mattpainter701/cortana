# Cortana Terminal AI: Implementation Plan and Expectations

## 1. Project Goal

Cortana is a terminal-first AI assistant designed to feel alive inside a modern TUI. The goal is not only to provide text chat, but to combine voice input, spoken output, agentic coding workflows, document recaps, and a fluid animated facial presence that reacts to the conversation in real time.

The implementation targets a Rust-first architecture because Rust gives us fast startup, low memory overhead, strong typing for complex agent state, and a single deployable binary for terminal users.

## 2. Core Product Expectations

Cortana should support four primary interaction modes:

1. **Conversational agent mode**: The user talks or types, Cortana responds with text and optional speech.
2. **Voice-native assistant mode**: Cortana listens through speech-to-text, answers through text-to-speech, and syncs facial animation to the spoken output.
3. **Rapid recap mode**: Cortana produces fast text banners and short spoken summaries without spending LLM tokens.
4. **Coding integration mode**: Cortana works alongside Claude Code and OpenCode by exposing CLI commands, JSON output, and file-oriented summaries.

The user experience should stay terminal-native wherever possible. Graphical animation is rendered inline through terminal graphics protocols rather than through a separate desktop window.

## 3. Technology Stack

| Capability | Rust Implementation Choice | Purpose |
| --- | --- | --- |
| Terminal UI | `ratatui` + `crossterm` | Layout, keyboard handling, panels, banners, logs |
| Inline terminal graphics | Kitty graphics protocol, with Sixel/ASCII fallback | Animated Cortana face inside supported terminals |
| Procedural face rendering | `tiny-skia` | Render a parametric 2D face into small RGBA frames |
| Async runtime | `tokio` | Concurrent STT, TTS, LLM calls, UI rendering, and tool execution |
| HTTP/API calls | `reqwest` | Cloud LLM providers, ElevenLabs, and future APIs |
| Speech-to-text | `whisper-rs` or `vosk-rs` | Offline voice transcription |
| Text-to-speech | System TTS through `tts`, optional ElevenLabs | Spoken Cortana responses |
| Local LLM option | `llama-cpp-rs`, `mistralrs`, or Ollama CLI bridge | Offline model support |
| Fast summarization | Extractive summarization, keyword extraction, TextRank/TF-IDF | Recaps without LLM token usage |
| CLI parsing | `clap` | Commands for summarize, speak, recap, daemon, and integrations |

## 4. High-Level Architecture

```text
User input
  ├─ typed terminal input
  └─ microphone input
        ↓
Input layer
  ├─ STT transcription
  ├─ command parser
  └─ safety classifier for shell/tool requests
        ↓
Agent core
  ├─ persona and policy
  ├─ memory window
  ├─ tool planner
  ├─ LLM provider adapter
  └─ local no-token summarizer
        ↓
Output layer
  ├─ text response panel
  ├─ recap banner
  ├─ TTS speech output
  └─ facial animation parameter stream
        ↓
TUI renderer
  ├─ ratatui layout
  ├─ terminal graphics frame renderer
  ├─ ASCII fallback face
  └─ keyboard/status widgets
```

The agent core should not know whether the face is rendered through Kitty graphics, Sixel, or ASCII. It should emit animation parameters and let the visualizer backend decide how to display them.

## 5. Visual Identity and Facial Animation

The visualizer should go beyond static states like happy, sad, and neutral. Cortana should appear to flow naturally through micro-expressions, breathing, eye motion, and speech-linked mouth movement.

### 5.1 Boot and Appearance Visuals

At process boot, Cortana should first render a minimal cyan activation ring with the greeting `Hi, I’m Cortana.`. When a session starts, or when Cortana intentionally appears to speak, the TUI should transition into a grainy, pixelated blue hologram silhouette: clearly feminine, beautiful, terminal-native, and compatible with ANSI/ASCII fallback rendering before higher-fidelity Kitty/Sixel animation lands.

### 5.2 Parametric Face Model

The face should be represented by continuous blend parameters instead of a small set of fixed emotion frames.

| Parameter | Range | Meaning |
| --- | --- | --- |
| `mouth_open` | `0.0..1.0` | Lip separation and speech openness |
| `smile` | `-1.0..1.0` | Frown to smile curve |
| `brow_raise` | `-1.0..1.0` | Furrowed brow to surprised brow |
| `eye_squint` | `0.0..1.0` | Open eyes to squinted/blinking eyes |
| `head_tilt` | `-10.0..10.0` degrees | Subtle attitude and conversational motion |
| `gaze_x` | `-1.0..1.0` | Horizontal eye focus |
| `gaze_y` | `-1.0..1.0` | Vertical eye focus |
| `breathing_phase` | cyclical | Subtle idle scale/offset |
| `arousal` | `0.0..1.0` | Emotional intensity |
| `valence` | `-1.0..1.0` | Negative to positive tone |

All parameters should be smoothed with interpolation or low-pass filtering to prevent abrupt jumps.

### 5.3 Animation Sources

Cortana's expression should be driven by multiple lightweight signals:

- **Speech amplitude** controls mouth openness, head bob, and emphasis.
- **TTS phoneme or viseme timing** controls more accurate mouth shapes when available.
- **Response tone** controls valence and arousal.
- **Conversation state** controls skepticism, confidence, disagreement, listening posture, and idle behavior.
- **Randomized micro-behavior** controls blinks, eye saccades, small head tilts, and breathing.

### 5.4 Rendering Strategy

The first production renderer should use `tiny-skia` to draw a small procedural 2D face, then stream the resulting frames to the terminal through the Kitty graphics protocol. This avoids maintaining a large sprite sheet and allows continuous expression blending.

Fallback modes should be supported in this order:

1. Kitty graphics protocol for terminals like Kitty and WezTerm.
2. Sixel or iTerm inline image support where available.
3. ASCII expression frames for basic terminals.

## 6. Speech Output and Lip Sync

Cortana should be able to speak, not merely post text recaps. Text-to-speech is a first-class output path.

### 6.1 Speech Output Modes

| Mode | Cost | Use Case |
| --- | --- | --- |
| System TTS | Free | Default local speech output and development |
| ElevenLabs | Paid API usage | High-quality voice for demos or premium mode |
| Local neural TTS | Hardware-dependent | Offline high-quality voice when available |
| Text-only | Free | Silent environments, logs, and accessibility |

### 6.2 Lip Sync Strategy

The TTS pipeline should emit an `AudioEvent` stream while speech plays:

```text
SpeechStarted(response_id)
Viseme { time_ms, mouth_shape, intensity }
Amplitude { time_ms, rms }
SpeechEnded(response_id)
```

If phoneme or viseme timing is unavailable, Cortana should fall back to amplitude-based mouth movement. The fallback is less precise, but still creates the perception that the face is speaking.

## 7. Rapid No-Token Summaries

Rapid summaries should not require an LLM call. They should use deterministic local algorithms that are fast enough to run continuously for recap banners and spoken mini-summaries.

### 7.1 Summary Types

| Summary Type | Output | Token Usage | Intended Use |
| --- | --- | --- | --- |
| Recap banner | One-line status summary | Zero LLM tokens | Always-visible TUI banner |
| Spoken mini-recap | One to three selected sentences | Zero LLM tokens | Quick verbal catch-up |
| Document skim | Key sentences and keywords | Zero LLM tokens | Fast first pass over files |
| Deep recap | Generated analysis | Uses LLM tokens | Explicit user request only |

### 7.2 No-Token Algorithms

The local summarizer should combine several extractive techniques:

- **Keyword extraction** for topic labels and banner terms.
- **TF-IDF scoring** for important terms and sentences.
- **TextRank-style sentence ranking** for more coherent multi-sentence summaries.
- **Code-aware heuristics** for function names, comments, docstrings, exports, imports, and changed files.

This gives Cortana free rapid summaries for both text output and speech output. The system should only use an LLM for summaries when the user explicitly asks for a deep recap, research-grade analysis, or rewrite-quality prose.

### 7.3 Banner Examples

```text
RECAP: Building Rust TUI Cortana with speech, animated face, and no-token summaries.
CODE: Modified visualizer pipeline; pending Kitty protocol integration tests.
DOC: Paper focuses on contrastive pretraining, evaluation gaps, and deployment risks.
```

## 8. Claude Code and OpenCode Support

Cortana should support Claude Code and OpenCode through simple command-line and machine-readable integration points. The goal is to let those tools request rapid recaps, file summaries, and project state snapshots without forcing Cortana into a specific editor or IDE.

### 8.1 CLI Commands

Recommended commands:

```bash
cortana summarize --fast --mode text README.md
cortana summarize --fast --mode code src/main.rs
cortana summarize --fast --mode diff --json
cortana recap --session current --format markdown
cortana recap --session current --speak
cortana speak "Build completed successfully. Two warnings remain."
cortana daemon --stdio
```

### 8.2 JSON Output Contract

Claude Code and OpenCode integrations should be able to request structured output:

```json
{
  "mode": "code",
  "summary": "Updates the visualizer pipeline and adds no-token recap support.",
  "keywords": ["visualizer", "recap", "speech", "kitty"],
  "files": [
    {
      "path": "src/visualizer.rs",
      "kind": "modified",
      "highlights": ["Adds FaceParams smoothing", "Emits terminal graphics frames"]
    }
  ],
  "token_usage": 0
}
```

The `token_usage` field should be explicit so downstream tools can distinguish free extractive recaps from LLM-generated analysis.

### 8.3 Integration Behavior

Claude Code and OpenCode should be treated as callers, not hard dependencies. Cortana should expose stable commands and output formats that any coding agent can use.

Expected integration features:

- Summarize the current file.
- Summarize the current git diff.
- Summarize recent terminal output.
- Produce a spoken build/test result.
- Produce a recap banner after a coding agent finishes work.
- Generate a markdown session recap for handoff between coding tools.

## 9. Agentic Reasoning and Safety

Cortana should have an agentic core, but destructive actions must be guarded.

### 9.1 Agent Capabilities

- Maintain a sliding memory window.
- Read project files when authorized by the user's workflow.
- Summarize code, documents, diffs, terminal output, and conversations.
- Use tools for shell commands, file operations, and project inspection.
- Speak responses aloud when speech mode is enabled.
- Show facial reactions while thinking, speaking, listening, and disagreeing.

### 9.2 Safety Expectations

Cortana should require confirmation before destructive or sensitive actions, including:

- Deleting files or directories.
- Overwriting user data.
- Running commands with broad filesystem impact.
- Installing dependencies globally.
- Sending sensitive content to cloud APIs.
- Using paid APIs when the user has not enabled them.

The assistant can be sassy or argumentative in personality, but the implementation should remain predictable, auditable, and safe.

## 10. Configuration

A default configuration file should let users select providers and fallback modes.

```toml
[ui]
renderer = "kitty"
fallback_renderer = "ascii"
frame_rate = 30
show_recap_banner = true

[speech]
stt_provider = "whisper-rs"
tts_provider = "system"
enable_lip_sync = true
speak_by_default = false

[summary]
default_mode = "fast"
allow_llm_deep_recaps = true
banner_max_chars = 120
speech_recap_sentences = 2

[integrations]
claude_code = true
opencode = true
json_output = true

[llm]
provider = "openai-compatible"
local_provider = "ollama"
require_confirmation_for_cloud = true

[paid_apis]
elevenlabs_enabled = false
monthly_budget_usd = 0
```

## 11. Milestone Plan

### Milestone 1: TUI Skeleton

- Build the `ratatui` layout.
- Add conversation log, input bar, status panel, and recap banner.
- Add CLI commands through `clap`.
- Add ASCII face fallback.

### Milestone 2: No-Token Recaps

- Implement keyword extraction and extractive sentence ranking.
- Add `summarize --fast --mode text`.
- Add `summarize --fast --mode code`.
- Add `summarize --fast --mode diff --json` for coding agents.

### Milestone 3: Speech

- Add system TTS.
- Add optional ElevenLabs provider behind explicit config.
- Add speech event stream.
- Add amplitude-based lip sync fallback.

### Milestone 4: Living Face Renderer

- Implement `FaceParams`.
- Render procedural face frames with `tiny-skia`.
- Stream frames through Kitty graphics.
- Add blinking, breathing, gaze, head tilt, and mouth motion.

### Milestone 5: Agent Core

- Add LLM provider abstraction.
- Add memory and persona prompts.
- Add guarded tool execution.
- Route response tone into facial animation.

### Milestone 6: Claude Code and OpenCode Workflows

- Add stable JSON output contracts.
- Add session recap files.
- Add current file and current diff summarizers.
- Document shell hooks, editor tasks, and agent handoff commands.

## 12. Success Criteria

The project is successful when Cortana can:

- Start quickly as a terminal application.
- Accept typed input and optionally spoken input.
- Speak responses aloud when speech mode is enabled.
- Animate a face inline in the terminal with mouth movement and emotional flow.
- Produce recap banners without LLM tokens.
- Produce spoken mini-recaps without LLM tokens.
- Integrate with Claude Code and OpenCode through CLI and JSON outputs.
- Use paid APIs only when explicitly enabled.
- Fall back gracefully when terminal graphics, speech, or cloud services are unavailable.

## 13. Non-Goals for the First Version

The first version should not attempt to ship a full 3D avatar, a separate desktop overlay, or perfect phoneme-level lip sync. The first version should prioritize a strong terminal experience, reliable speech output, fast no-token summaries, and a face that feels alive through procedural motion.

## 14. Final Expectation

Cortana should feel like a terminal-native AI companion: fast, expressive, useful, and opinionated without being reckless. It should be able to talk, listen, summarize, code-assist, and visually react in the same TUI session.

The practical path is to build the no-token recap system and TUI first, add speech second, add the living face third, and then connect Claude Code/OpenCode workflows once the CLI contract is stable.
