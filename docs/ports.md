# Network Ports Configuration

| Service | Port | Protocol | Description |
|---|---|---|---|
| **LocalBrain Hub (Rust Core)** | `8100` | WebSocket (`ws://0.0.0.0:8100/xiaozhi/v1/`) | StackChan robot audio & control stream |
| **Pocket TTS** | `8000` | HTTP (`http://127.0.0.1:8000`) | Pocket-TTS local synthesis server (`uvx pocket-tts serve`) |
| **Faster-Whisper STT** | `8200` | HTTP (`http://127.0.0.1:8200/transcribe`) | Local speech-to-text service (`python tools/stt_server.py`) |
| **Ollama LLM** | `11434` | HTTP (`http://127.0.0.1:11434`) | Local LLM inference server (`ollama run qwen2.5:3b-instruct`) |
