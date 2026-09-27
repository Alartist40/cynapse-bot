# Changelog

All notable changes to `cynpase-bot` will be documented in this file.

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
