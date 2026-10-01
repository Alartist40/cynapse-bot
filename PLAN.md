# Project Local Mind — Plan

**Goal:** High-performance Rust orchestrator for perception, voice loop, and MQTT fleet control for StackChan.

## Milestones

- [x] **Stage 1: Workspace & Cargo Setup**
  OWNS: `Cargo.toml`, `AGENTS.md`, `PLAN.md`, `GATES.md`, `IMPLEMENT.md`
  GATE: G1 (MET)

- [x] **Stage 2: Vision Detector & Tracker with 5–85° Tilt Clamp**
  OWNS: `src/vision/mod.rs`, `src/vision/tracker.rs`, `src/vision/detector.rs`
  GATE: G2 (MET)

- [x] **Stage 3: MQTT Bus & Fleet Contract**
  OWNS: `src/bus/mod.rs`, `src/bus/mqtt.rs`
  GATE: G3 (MET)

- [x] **Stage 4: Voice Loop & Visual Context Injection**
  OWNS: `src/voice/mod.rs`, `src/voice/whisper.rs`, `src/voice/llm.rs`, `src/voice/tts.rs`
  GATE: G4 (MET)

- [x] **Stage 5: Axum REST/WebSocket Orchestrator & Firmware Contracts**
  OWNS: `src/api/mod.rs`, `src/main.rs`, `src/lib.rs`, `firmware/stackchan/stackchan_localmind.ino`
  GATE: G5 (MET)
