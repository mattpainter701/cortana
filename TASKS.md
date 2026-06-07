# Cortana — Task List

## Active Tasks

### 1. Cortana-like face renderer — COMPLETED
- Redesigned canvas face renderer with heart-shaped face, Cortana's signature crown (5 overlapping arc bands), better eyes/brows/nose/mouth, hair silhouette, cheek glow, enhanced holographic aura
- Added Kitty graphics protocol renderer using tiny-skia (gated behind `kitty-render` feature)
- Fixed `rand_f64()` to use actual randomness instead of deterministic hash

### 2. OpenCode integration — COMPLETED
- Created `.opencode/skills/cortana/SKILL.md` — skill definition loaded by OpenCode
- LLM can invoke `cortana summarize`, `cortana speak`, `cortana session`, etc.

### 3. Build verification — COMPLETED
- `cargo build` compiles with both default and `kitty-render` features
- `cargo test` — all 13 tests pass

### 4. Kitty protocol face renderer — COMPLETED
- Implemented full tiny-skia `render_pixmap()` replacing the blank stub
- Renders all face components: heart-shaped face, Cortana crown, eyes, brows, nose, Cupid's-bow mouth, neck, hair silhouette, collar, cheek glow, aura rings, particles, scanline
- Uses anti-aliased vector paths and filled shapes instead of coarse dot-matrix
- Frame-count animation via wall clock

## Future Tasks

- Implement proper tiny-skia Kitty protocol face rendering
- Add speech-to-text (whisper-rs)
- Add ElevenLabs premium TTS provider
- Add LLM agent core with tool execution
- Create Claude Code integration hooks
