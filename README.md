# Project Local Mind — StackChan Hub (Rust Core)

> **Offline Robot Perception, Voice & Fleet Orchestration System for StackChan (M5Stack CoreS3 / ESP32-S3)**  
> Written in pure Rust (`axum` + `rumqttc` + `libopus`). Designed for Orange Pi 6 Plus (RK3588) or Linux Host.

---

## 1. Architecture Overview

Local Mind coordinates object perception (YOLO tracking with hard 5–85° tilt servo clamping), conversational voice with live visual context, and MQTT fleet orchestration with zero internet dependency.

```
[ StackChan Robot (CoreS3) & ESP32-S3 Fleet ]
  - Camera MJPEG stream
  - Dual Mics (VAD audio)
  - Speaker + OLED Face + Pan/Tilt Servos
       │  ▲
       │  │  MQTT (:1883) & REST (:8088)
       │  │  - Gaze tracking: stackchan/cmd/gaze
       │  │  - Face emotion: stackchan/cmd/face
       │  │  - Telemetry: fleet/{id}/telemetry
       ▼  │
[ Local Mind Hub (Rust Core :8088) ]
   ├── Vision: YOLO Tracking with Strict 5.0°–85.0° Tilt Servo Clamping
   ├── STT: Whisper.cpp HTTP Server (:8080)
   ├── LLM: Ollama (:11434, qwen2.5:3b-instruct with Visual Context Injection)
   ├── TTS: Pocket-TTS Serve (:8000, Cloned Personality Voice)
   └── Bus: Mosquitto MQTT Pub/Sub (:1883)
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
│   │   └── mqtt.rs         # Async MQTT client (rumqttc)
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
│   ├── config.rs           # Configuration models
│   └── bin/
│       └── fake_device.rs  # StackChan simulator binary
├── firmware/
│   └── stackchan/
│       └── stackchan_localmind.ino  # StackChan-BSP Arduino/C++ firmware
├── tests/
│   ├── audio_test.rs       # Codec unit tests
│   ├── bus_test.rs         # MQTT serialization & contracts test
│   ├── orchestrator_api_test.rs # Axum REST API test
│   ├── vision_tracker_test.rs   # Tracker state & servo limit clamp test
│   └── voice_loop_test.rs       # Voice loop & visual context test
├── AGENTS.md               # Tool boundaries & architecture
├── PLAN.md                 # Project roadmap
├── GATES.md                # Verification gates
├── IMPLEMENT.md            # Chronological implementation ledger
└── Cargo.toml              # Rust manifest
```

---

## 3. Quick Start & Execution

### 1. Launch Ollama (LLM)
```bash
ollama run qwen2.5:3b-instruct
```

### 2. Launch Pocket-TTS (TTS)
```bash
uvx pocket-tts serve --port 8000
```

### 3. (Optional) Launch Whisper.cpp STT
```bash
# In whisper.cpp repo
./server -m models/ggml-small.bin --port 8080
```

### 4. Launch Local Mind Rust Orchestrator
```bash
cargo run --release -- --port 8088
```

### 5. Verify Health & Trigger Speech
```bash
curl http://localhost:8088/health
curl -X POST http://localhost:8088/say -H "Content-Type: application/json" -d '{"text":"Hello from StackChan Local Mind!","expression":"happy"}'
```

### 6. Run Test Suite
```bash
cargo test -- --nocapture
```
