# Project Local Mind — StackChan Hub (Rust Core + XiaoZhi Voice Hub)

> **Offline Robot Perception, Voice & Fleet Orchestration System for StackChan (M5Stack CoreS3 / ESP32-S3)**  
> High-performance Rust Orchestrator (`:8088`) + XiaoZhi WebSocket Voice Hub (`:8100`). Designed for Orange Pi / Linux hosts.

---

## 1. Architecture Overview

Local Mind coordinates object perception (YOLO tracking with strict 5–85° tilt servo clamping), conversational voice with low-latency streaming, and MQTT fleet orchestration with zero cloud runtime dependencies.

```
[ StackChan Robot (CoreS3) & ESP32-S3 Fleet ]
  - Gaze tracking: stackchan/cmd/gaze
  - Face emotion: stackchan/cmd/face & stackchan/cmd/emotion
  - Speaker audio: stackchan/cmd/audio
  - Telemetry: fleet/{id}/telemetry
  - Voice WebSocket: ws://hub:8100/xiaozhi/v1/
       │  ▲
       │  │  MQTT (:1883), REST (:8088), WebSocket (:8100)
       ▼  │
[ Local Mind Dual-Hub System ]
   ├── Rust Orchestrator (:8088)
   │   ├── Vision: YOLO Tracking with Strict 5.0°–85.0° Tilt Servo Clamping
   │   ├── REST API: /health, /stats, /say, /api/vision/frame
   │   └── Bus: Mosquitto MQTT Pub/Sub Client (:1883)
   │
   └── Python XiaoZhi Voice Hub (:8100)
       ├── STT: In-process Faster-Whisper (asyncio worker)
       ├── LLM: Ollama Streaming Client (:11434, qwen2.5:3b-instruct)
       ├── TTS: Pocket-TTS Serve (:8000, Cloned Personality Voice)
       └── Protocol: Full XiaoZhi Hello/Listen/Opus duplex framing
```

---

## 2. Directory Layout

```
.
├── src/
│   ├── lib.rs              # Library exports
│   ├── main.rs             # Local Mind hub entry point (:8088)
│   ├── api/
│   │   └── mod.rs          # Axum REST API (/health, /stats, /say, /api/vision/frame)
│   ├── bus/
│   │   ├── mod.rs          # Bus module exports
│   │   └── mqtt.rs         # Async MQTT client with auto-reconnect subscriptions
│   ├── vision/
│   │   ├── mod.rs          # Vision module exports
│   │   ├── detector.rs     # YOLO / RKNN detection engine
│   │   └── tracker.rs      # Track state machine & 5–85° tilt servo clamping
│   ├── voice/
│   │   ├── mod.rs          # Voice loop coordinator & rate-limited narration
│   │   ├── whisper.rs      # Whisper.cpp STT client (:8080)
│   │   ├── llm.rs          # Ollama LLM client with visual context (:11434)
│   │   └── tts.rs          # Pocket-TTS client (:8000)
│   ├── audio.rs            # Audio codec & resampling utilities
│   └── config.rs           # Unified configuration models
├── hub/
│   ├── server.py           # XiaoZhi WebSocket Server (:8100)
│   ├── audio.py            # Opus 60ms framing & codec
│   ├── llm.py              # Streaming LLM sentence chunker
│   ├── stt.py              # Faster-Whisper transcription engine
│   ├── tts.py              # Pocket-TTS client with Opus output
│   ├── config.yaml         # Unified hub configuration
│   └── persona/
│       └── system.md       # Persona definition
├── firmware/
│   └── stackchan/
│       ├── stackchan_localmind.ino  # StackChan-BSP Arduino/C++ firmware
│       └── config.h.example         # Template for Wi-Fi and MQTT credentials
├── tests/
│   ├── audio_test.rs       # Codec unit tests
│   ├── bus_test.rs         # MQTT serialization & contracts test
│   ├── orchestrator_api_test.rs # Axum REST API test
│   ├── vision_tracker_test.rs   # Tracker state & servo limit clamp test
│   ├── voice_loop_test.rs       # Voice loop & visual context test
│   └── test_hub.py         # Python audio, chunker, & fallback tests
├── docs/
│   └── ports.md            # System port mapping
├── AGENTS.md               # Tool boundaries & architecture
├── PLAN.md                 # Project roadmap
├── GATES.md                # Verification gates
├── IMPLEMENT.md            # Chronological implementation ledger
├── Cargo.toml              # Rust manifest
└── pyproject.toml          # Python manifest
```

---

## 3. Quick Start & Execution

### 1. Pre-Provision Models (Required for Zero-Internet Operation)
```bash
# Pull LLM weights
ollama pull qwen2.5:3b-instruct

# Pre-cache faster-whisper and pocket-tts models
uv run python -c "from faster_whisper import WhisperModel; WhisperModel('small', compute_type='int8')"
```

### 2. Launch Supporting Services
```bash
# Terminal 1: Ollama LLM (:11434)
ollama run qwen2.5:3b-instruct

# Terminal 2: Pocket-TTS (:8000)
uvx pocket-tts serve --port 8000

# Terminal 3: MQTT Broker (:1883)
mosquitto -p 1883
```

### 3. Launch Local Mind Hubs
```bash
# Launch Rust Orchestrator & Vision Hub (:8088)
cargo run --release -- --port 8088

# Launch Python XiaoZhi Voice Hub (:8100)
uv run python -m hub.server --port 8100
```

### 4. Run Test Suite
```bash
# Run Rust Test Suite
cargo test -- --nocapture

# Run Python Hub Test Suite
uv run pytest tests/test_hub.py -v
```
