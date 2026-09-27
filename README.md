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
   ├── Personality & Memory:
   │     Markdown Personas (SOUL.md, IDENTITY.md, TOOLS.md)
   │     Dendrite Graph Memory (SQLite FTS5 + in-memory graph)
   └── Monitoring & Dashboard:
         Web Control Panel (GET /dashboard)
         Live Telemetry Stream (/ws/monitor)
         Camera Snapshots (GET /camera/latest.jpg)
```

---

## 2. Core Features

- **Zero Cloud Runtime:** Fully functional on an isolated LAN or fallback AP.
- **Xiaozhi Protocol Server:** Speaks native Xiaozhi binary Opus v1/v2 framing with sub-10s server hello responses.
- **Local OTA Discovery:** `GET /xiaozhi/ota/` supplies dynamic WebSocket endpoints and tokens to robot firmware.
- **Dual TTS Engine:**
  - **Pocket-TTS:** Zero-shot voice cloning from `.wav` samples directly on CPU.
  - **Piper TTS:** Low-footprint standalone ONNX neural speech synthesis.
- **Dendrite Memory Graph:** Interleaved SQLite graph memory from `cynapse-mini` retaining user interactions and facts.
- **Motion & Animation Engine:** Keyframe choreography for pan/tilt servos (-90°..90° / -30°..30°) and OLED facial expressions (`dance`, `nod`, `shake`, `wave`, `look_around`, `sleep`, `wake`).
- **Official StackChan MCP Tools:** Head angles, RGB neon lights, and reminder timer management.
- **Real-Time Web Dashboard:** Embedded single-page control panel with telemetry feeds and live camera view.

---

## 3. Quick Start

### Build & Run Locally

```bash
# Clone and build
cargo build --release

# Run hub on default port 8000
./target/release/cynpase-bot --bind-addr 0.0.0.0:8000
```

### Access Dashboard
Open your browser at:
```
http://localhost:8000/dashboard
```

---

## 4. Robot Configuration (StackChan)

Point StackChan's `CONFIG_OTA_URL` to your hub:
```
http://<HUB_LAN_IP>:8000/xiaozhi/ota/
```

When StackChan boots, it requests OTA discovery, receives the WebSocket URL (`ws://<HUB_LAN_IP>:8000/xiaozhi/ws`) and authentication token, and establishes a persistent local session.

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
│   ├── mcp.rs            # StackChan HAL MCP tools & custom registry
│   ├── ota.rs            # Local OTA JSON discovery endpoint
│   ├── persona.rs        # SOUL / IDENTITY / Dendrite memory context
│   ├── pipeline.rs       # 3-tier cascade and turn execution
│   ├── protocol.rs       # Xiaozhi framing & message types
│   ├── server.rs         # Axum WebSocket router & stream demuxer
│   ├── session.rs        # Device session state & audio buffers
│   └── vision.rs         # 0.3MP JPEG frame ingestion & snapshot API
├── data/persona/         # Markdown persona definition files
├── deploy/               # Systemd units & launcher scripts
├── tests/                # Deterministic integration test suite
└── GATES.md              # Observable completion verification gates
```

---

## 7. Roadmap

- [ ] **Desktop Control Panel:** Native GUI window application for desktop control and telemetry.
- [ ] **Mobile App:** Cross-platform companion app (Flutter / Slint) for phone-based robot control and avatar streaming.
- [ ] **Physical Hardware Verification:** Live on-device validation once StackChan hardware arrives.
