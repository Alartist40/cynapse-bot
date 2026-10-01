# Project Local Mind — Plan

**Goal:** Unified dual-hub system: Rust orchestrator for perception & MQTT fleet control (`:8088`), and Python voice hub speaking XiaoZhi protocol (`:8100`).

## Milestones

- [x] **Stage 1: Workspace & Core Configuration**
  OWNS: `Cargo.toml`, `pyproject.toml`, `AGENTS.md`, `PLAN.md`, `GATES.md`, `IMPLEMENT.md`
  GATE: G1 (MET)

- [x] **Stage 2: Vision Tracker with 5–85° Tilt Clamp & Association Guard**
  OWNS: `src/vision/mod.rs`, `src/vision/tracker.rs`, `src/vision/detector.rs`
  STATUS: Tracker state machine, within-threshold association guard, streak decay, and hardware clamp verified; real RKNN NPU model execution [DEFERRED to hardware bench].
  GATE: G2 (MET)

- [x] **Stage 3: MQTT Bus & Auto-Reconnect Contracts**
  OWNS: `src/bus/mod.rs`, `src/bus/mqtt.rs`
  STATUS: Auto-resubscription on ConnAck, topic publish/subscribe verified.
  GATE: G3 (MET)

- [x] **Stage 4: Voice Loop & Audio Synthesis Clients**
  OWNS: `src/voice/mod.rs`, `src/voice/whisper.rs`, `src/voice/llm.rs`, `src/voice/tts.rs`
  STATUS: Rust voice synthesis client & rate-limited narration verified in tests [live voice loop handled via XiaoZhi WebSocket Hub].
  GATE: G4 (MET)

- [x] **Stage 5: Axum Orchestrator API & Firmware Contracts**
  OWNS: `src/api/mod.rs`, `src/main.rs`, `src/lib.rs`, `firmware/stackchan/stackchan_localmind.ino`
  STATUS: Non-mutating `/stats`, `/say` audio broadcast & speaking trigger, `/api/vision/frame`, MAC-based MQTT firmware client ID verified.
  GATE: G5 (MET)

- [x] **Stage 6: XiaoZhi Protocol Voice Hub & Audio Pacing**
  OWNS: `hub/server.py`, `hub/audio.py`, `hub/llm.py`, `hub/stt.py`, `hub/tts.py`
  STATUS: WebSocket duplex Opus framing, non-blocking STT, earliest delimiter sentence chunker with ellipsis support, and disconnect safety verified.
  GATE: G6 (MET)
