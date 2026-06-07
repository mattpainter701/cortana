# Changelog

## [0.2.0] — Unreleased

### Added
- Cortana-like canvas face renderer: heart-shaped feminine face, signature crown (5 overlapping arc bands), hair silhouette, neck, cheek glow, enhanced holographic aura/particles
- Kitty graphics protocol renderer using tiny-skia (optional, gated behind `kitty-render` feature)
  - Full `render_pixmap()` implementation with anti-aliased vector paths
  - Renders heart-shaped face fill, jawline, eyes (almond + iris + pupil + highlights), arched brows,
    nose (bridge + tip + nostrils), Cupid's-bow mouth with speech/closed states,
    neck, hair silhouette, 5-arc Cortana crown with glow aura, collar, cheek glow,
    4-layer pulsing aura rings, particles, and scanline overlay
  - Animation driven by wall-clock frame count and FaceParams expressions
- OpenCode integration via `.opencode/skills/cortana/SKILL.md`

### Changed
- Canvas face renderer completely redesigned — generic abstract face replaced with Cortana hologram identity
- Face proportions improved: heart-shaped jawline, Cupid's-bow mouth, almond eyes with highlights, arched brows, better nose
- `rand_f64()` now uses actual system time-based randomness for particle variation

## [0.1.0] — Initial Release

- CLI commands: boot, appear, session, summarize, recap, speak, daemon, config
- Zero-token extractive summarization (text, code, diff)
- JSON output contract for agent integration
- Basic canvas face renderer (generic abstract face)
- FaceParams/FaceState animation parameter system
- TUI mode with ratatui layout
- System TTS speech support
