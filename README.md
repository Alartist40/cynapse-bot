# cynpase-bot

> **Local-First, Offline AI Hub for the StackChan Robot (M5Stack CoreS3 / ESP32-S3)**  
> Written in Rust (`tokio` + `axum`). Designed for Orange Pi 6 Plus SBC / Linux Host.

---

## 1. Overview

`cynpase-bot` is an offline-first AI brain and WebSocket hub for the StackChan desktop companion robot. It replaces cloud-dependent servers with a local Rust daemon that provides real-time voice, vision, memory, motion, and MCP tool execution over LAN.

```
[ StackChan Robot (ESP32-S3 CoreS3) ]
        │  ▲
        │  │  Xiaozhi WebSocket Protocol
        │  │  - 16 kHz Opus audio (in) / 24 kHz Opus audio (out)
        │  │  - 0.3MP JPEG video stream
        │  │  - JSON events (hello, listen, abort, mcp, config)
        ▼  │
[ cynpase-bot Hub (Orange Pi 6 Plus / Rust) ]
   ├── OTA & Discovery Endpoint (GET /xiaozhi/ota/)
   ├── Protocol Demuxer (Opus Audio & JPEG Video)
   ├── 3-Tier Intent Cascade:
   │     Tier 1: Hardware Fast Rules (<50ms: stop, wave, look_up)
   │     Tier 2: StackChan MCP Tools (hal_mcp: servos, LEDs, timers)
   │     Tier 3: Local LLM Engine (Leafcutter / Qwen 2.5-7B / Ornith 1.5-9B)
   ├── Audio Engine:
   │     ASR: Whisper.cpp
   │     TTS: Pocket-TTS (Zero-shot voice cloning) & Piper (ONNX)
   ├── Personality & Mazzaroth Memory:
   │     Markdown Personas (SOUL.md, IDENTITY.md, TOOLS.md)
   │     Mazzaroth 4-Tier Memory Engine (Core, Semantic, Episodic, Working)
   │     Cognitive Decay (Ebbinghaus) & Hebbian Synapse Reinforcement
   │     3D Celestial Galaxy Spatial Clustering (SQLite FTS5)
   ├── Monitoring, GUI & Mobile:
   │     Native Desktop GUI (cynpase-gui) with 3D Galaxy Viewport
   │     Mobile Companion PWA (GET /mobile) with Touch Joystick & Virtual Avatar
   │     Web Control Panel (GET /dashboard) & Live Telemetry (/ws/monitor)
   │     Camera Snapshots (GET /camera/latest.jpg)
```

---

## 2. Core Features

- **Zero Cloud Runtime:** Fully functional on an isolated LAN or fallback AP.
- **Xiaozhi Protocol Server:** Speaks native Xiaozhi binary Opus v1/v2 framing with sub-10s server hello responses.
- **Local OTA Discovery:** `GET /xiaozhi/ota/` supplies dynamic WebSocket endpoints and tokens to robot firmware.
- **Dual TTS Engine:**
  - **Pocket-TTS:** Zero-shot voice cloning from `.wav` samples directly on CPU.
  - **Piper TTS:** Low-footprint standalone ONNX neural speech synthesis.
- **Mazzaroth Unified Memory Engine:** 4-tier cognitive taxonomy (Core, Semantic, Episodic, Working), Ebbinghaus retention decay, Hebbian links, SQLite FTS5 search, and 3D celestial orbital layout.
- **Motion & Animation Engine:** Keyframe choreography for pan/tilt servos (-90°..90° / -30°..30°) and OLED facial expressions (`dance`, `nod`, `shake`, `wave`, `look_around`, `sleep`, `wake`).
- **Official StackChan MCP Tools:** Head angles, RGB neon lights, and reminder timer management.
- **Native Desktop GUI (`cynpase-gui`):** Real-time hardware control panel, pan/tilt sliders, live camera feed, transcript stream, and interactive 3D Mazzaroth galaxy visualizer.
- **Mobile Companion PWA (`GET /mobile`):** Zero-install responsive mobile controller with 360° touch joystick, haptic vibration, push-to-talk voice bridge, and animated StackChan virtual face avatar mode.

---

## 3. Quick Start

### Build & Run Hub Daemon

```bash
# Build release binaries
cargo build --release

# Run the hub daemon on port 8000
./target/release/cynpase-bot --bind-addr 0.0.0.0:8000
```

### Launch Desktop GUI Application

```bash
# Run native desktop control panel
cargo run --bin cynpase-gui
```

### Access Mobile & Web Dashboards
- **Mobile Companion App:** `http://<HUB_LAN_IP>:8000/mobile` (Tap *Add to Home Screen*)
- **Web Dashboard:** `http://<HUB_LAN_IP>:8000/dashboard`

---

## 4. Robot Configuration (StackChan)

Point StackChan's `CONFIG_OTA_URL` to your hub with your authentication token:
```
http://<HUB_LAN_IP>:8000/xiaozhi/ota/?token=cynpase-secret-token
```
*(Or pass `Authorization: Bearer <TOKEN>` in the firmware HTTP request headers).*

When StackChan boots, it requests authenticated OTA discovery, receives the WebSocket URL (`ws://<HUB_LAN_IP>:8000/xiaozhi/ws`) along with connection parameters, and establishes a persistent local session.

---

## 5. Orange Pi 6 Plus Deployment

```bash
# Run one-step systemd installer
sudo bash deploy/install-opi.sh

# Service commands
sudo systemctl start cynpase-bot
sudo systemctl status cynpase-bot
journalctl -u cynpase-bot -f
```

---

## 6. Project Structure

```
cynpase-bot/
├── src/
│   ├── animation.rs      # Servo and face choreographies
│   ├── audio.rs          # Pocket-TTS & Piper synthesis engines
│   ├── config.rs         # Hub CLI parameters & default ports
│   ├── dashboard.rs      # Web UI & /ws/monitor telemetry
│   ├── gui/              # Native Desktop GUI (eframe / egui)
│   ├── gui_main.rs       # cynpase-gui binary entrypoint
│   ├── mazzaroth/        # Unified memory engine (4 tiers, FTS5, 3D gravity)
│   ├── mcp.rs            # StackChan HAL MCP tools & custom registry
│   ├── mobile.rs         # Mobile PWA generator & REST control APIs
│   ├── ota.rs            # Local OTA JSON discovery endpoint
│   ├── persona.rs        # SOUL / IDENTITY / Memory context
│   ├── pipeline.rs       # 3-tier cascade and turn execution
│   ├── protocol.rs       # Xiaozhi framing & message types
│   ├── server.rs         # Axum WebSocket router & stream demuxer
│   ├── session.rs        # Device session state & audio buffers
│   └── vision.rs         # 0.3MP JPEG frame ingestion & snapshot API
├── data/persona/         # Markdown persona definition files
├── deploy/               # Systemd units & launcher scripts
├── mobile/               # Standalone mobile wrapper & Android packaging
├── tests/                # 11 integration test suites (21 verified gates in GATES.md)
└── GATES.md              # Quality gates & test verification ledger
```

---

## 7. Roadmap

- [x] **Local OTA Discovery & Xiaozhi WebSocket Framing:** Sub-10s handshake, 16k in / 24k out Opus audio.
- [x] **Audio & TTS Pipeline:** Pocket-TTS voice cloning + Piper ONNX synthesis on CPU.
- [x] **Mazzaroth Memory Engine:** 4-tier taxonomy, SQLite FTS5, Ebbinghaus decay, 3D celestial layout.
- [x] **Native Desktop Control Panel:** `cynpase-gui` with 3D constellation orbital viewport.
- [x] **Mobile Companion App:** PWA with touch joystick, virtual avatar face, and push-to-talk.
- [ ] **Physical Hardware Pre-Flight Verification:** Live robot check when hardware package is delivered.
