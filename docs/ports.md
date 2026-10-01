# Network Ports Configuration

| Service | Port | Protocol | Description |
|---|---|---|---|
| **Voice Hub (Python XiaoZhi)** | `8100` | WebSocket (`ws://0.0.0.0:8100/xiaozhi/v1/`) | StackChan robot audio stream, STT, streaming LLM, Pocket-TTS & barge-in |
| **Vision & Fleet Hub (Rust Core)** | `8088` | HTTP REST (`http://0.0.0.0:8088`) | `/health`, `/stats`, `/say`, `/api/vision/frame`, YOLO tracking & gaze |
| **Mosquitto MQTT Broker** | `1883` | MQTT (`tcp://127.0.0.1:1883`) | Robot gaze commands, facial expressions, and fleet telemetry bus |
| **Pocket TTS Serve** | `8000` | HTTP (`http://127.0.0.1:8000`) | Pocket-TTS zero-shot neural speech synthesis |
| **Whisper.cpp STT** | `8080` | HTTP (`http://127.0.0.1:8080/inference`) | Local speech-to-text inference |
| **Ollama LLM** | `11434` | HTTP (`http://127.0.0.1:11434/api/chat`) | Local LLM inference server (`qwen2.5:3b-instruct`) |
