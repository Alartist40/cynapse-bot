# Changelog

All notable changes to `cynpase-bot` will be documented in this file.

## [0.3.0] - 2026-09-27

### Added
- **Native Desktop GUI Application (`src/gui/`, `src/gui_main.rs`):**
  - High-performance native control panel binary (`cynpase-gui`) powered by `eframe`/`egui`.
  - Pan/Tilt servo manual positioning sliders (-90°..+90° / -30°..+30°).
  - Quick-trigger choreography panel (dance, nod, shake, wave, sleep, wake) and RGB LED controls.
  - Real-time robot vision camera feed and conversation transcript.
  - Interactive 3D Mazzaroth constellation viewport with orbit controls and node selection.
- **Mobile Companion PWA & REST Control Suite (`src/mobile.rs`):**
  - Zero-install responsive mobile controller served directly at `GET /mobile` and `GET /app`.
  - 360° circular virtual touch joystick with real-time angle labels and haptic feedback.
  - Animated StackChan avatar face mode rendered on HTML5 Canvas (kawaii blinking eyes, gaze following touch, emotional states).
  - Push-to-talk voice bridge and real-time conversation feed.
  - REST endpoints: `POST /api/robot/control`, `POST /api/chat/send`, `GET /api/status`, `GET /api/memory/celestial`.
  - PWA manifest (`/manifest.json`) and service worker (`/service-worker.js`) for standalone home-screen app mode.
  - Standalone packaging scaffolding and instructions in `mobile/`.
- **Quality Gates Verification:**
  - Added and verified gates G14–G18 in `GATES.md` with 100% automated test pass rate across 12 test suites.

## [0.2.0] - 2026-09-27

### Added
- **Mazzaroth Unified Memory Engine (`src/mazzaroth/`):**
  - **4-Tier Knowledge Taxonomy:** Core (L4), Semantic (L3), Episodic (L2), Working (L1) with `Identity`, `Person`, `Concept`, `Project`, `Procedure`, `Lesson`, `Event`, and `AtomicFact` classifications.
  - **Cognitive Decay & Hebbian Learning:** Ebbinghaus exponential forgetting curve with strength reinforcement and automatic co-occurrence link association.
  - **Relational & Semantic FTS5 Store:** High-performance SQLite backing with Porter Unicode full-text search.
  - **Celestial 3D Spatial Layout:** N-body gravitational clustering mapping memories into planetary constellations for visual rendering.
  - **Episodic-to-Semantic Consolidation:** Automatic distillation of raw conversation turns into long-term atomic facts.

## [0.1.0] - 2026-09-27

### Added
- **Xiaozhi Protocol Engine:** Implemented native Xiaozhi WebSocket server in Rust with binary Opus framing, hello/listen/abort event handling, and sub-10s server hello responses.
- **Local OTA Discovery:** Added `GET /xiaozhi/ota/` endpoint returning dynamic WebSocket URLs and authentication tokens to StackChan firmware without cloud dependencies.
- **3-Tier Intent Cascade:**
  - Tier 1: Fast rules executing sub-50ms hardware actions (`stop`, `wave`, `look_up`, `look_down`) without LLM latency.
  - Tier 2: StackChan MCP tools (`self.robot.set_head_angles`, `set_led_color`, `create_reminder`, `get_reminders`).
  - Tier 3: Leafcutter / OpenAI-compatible `/v1/chat/completions` engine integration (Qwen 2.5-7B / Ornith 1.5-9B).
- **Audio Pipeline & Pocket-TTS:**
  - Direct support for Kyutai Pocket-TTS for zero-shot voice cloning on CPU.
  - Piper TTS ONNX binary fallback with 24 kHz Opus chunking.
  - Whisper STT transcription pipeline.
- **Persona & Dendrite Memory:**
  - Auto-loading markdown personas from `data/persona/` (`SOUL.md`, `IDENTITY.md`, `TOOLS.md`).
  - SQLite Dendrite graph memory integration via `cynapse-memory`.
- **Vision Ingestion:**
  - 0.3MP JPEG frame detector and caching in `VisionManager`.
  - HTTP snapshot endpoint `GET /camera/latest.jpg`.
- **Animation Choreography:**
  - Keyframed servo and facial animations (`dance`, `nod`, `shake`, `wave`, `look_around`, `sleep`, `wake`).
- **Web Dashboard & Telemetry:**
  - Embedded single-page dashboard at `GET /dashboard`.
  - Real-time WebSocket telemetry channel at `GET /ws/monitor`.
- **Orange Pi 6 Plus Deployment:**
  - Systemd unit `deploy/cynpase-bot.service`.
  - Installer script `deploy/install-opi.sh` and launcher `deploy/start.sh`.
- **Verification Gates:**
  - Completed all 9 verifiable completion gates in `GATES.md` with 100% test pass rate.
