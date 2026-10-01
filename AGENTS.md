# Project Local Mind — StackChan Hub (Rust Core + XiaoZhi Voice Hub)

## Architecture Overview
StackChan Local Mind is an offline, internet-independent robot perception, voice, and fleet orchestration system for M5Stack CoreS3 and ESP32-S3 robots.
- **Hub Orchestrator (Rust/Axum :8088)**: Ingests camera frames, runs YOLO tracking with hard 5–85° tilt servo clamping, coordinates Whisper.cpp STT (:8080), Ollama LLM (:11434), and Pocket-TTS (:8000).
- **XiaoZhi Voice Hub (Python :8100)**: WebSocket audio duplex server (/xiaozhi/v1/) with in-process faster-whisper, streaming Ollama, and Pocket-TTS.
- **MQTT Bus (Mosquitto :1883)**: Handles real-time telemetry, gaze commands (`stackchan/cmd/gaze`), facial expressions (`stackchan/cmd/face`), and audio speech (`stackchan/cmd/audio`).
- **Fleet Resilience**: Fallback hotspot AP (`localmind`, 192.168.50.1) and autonomous offline idle modes.

## Safety & Boundary Constraints
- **Servo Tilt Limit**: Hardware Y-axis servo must strictly remain within **5.0° to 85.0°**. Clamping is enforced in both Rust tracker and firmware.
- **Zero Cloud Runtime**: No external APIs or cloud tokens.

## Verification Procedures
1. Tracker & Gaze Clamp: `cargo test --test vision_tracker_test`
2. Voice Loop & Visual Context: `cargo test --test voice_loop_test`
3. Orchestrator API & Endpoints: `cargo test --test orchestrator_api_test`
4. Python XiaoZhi Voice Hub: `uv run pytest tests/test_hub.py -v`
5. Full Test Suite: `cargo test && uv run pytest tests/test_hub.py -v`
