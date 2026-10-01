# Implementation Ledger — Project Local Mind

## 2026-10-01 — Local Mind Architecture & Autopsy Remediation
- Remediated confirmed P0, P1, and P2 audit items across Rust orchestrator, Python XiaoZhi voice hub, and Arduino firmware [DEFERRED: /say end-to-end audio playback — firmware cmd/audio is visual talking indicator only, as ESP32 cannot receive raw WAV over MQTT (audio streaming handled via WebSocket :8100); real RKNN NPU model execution deferred to physical hardware bench; external camera frame producer for /api/vision/frame deferred to video pipeline setup].
- `hub/server.py`: Resolved mid-turn client disconnect traceback (`ConnectionClosed` caught safely in `send_json`, `send_audio_frames`, and `cancel_active_turn`), added 15s incoming audio buffer limit to prevent unbounded RAM growth, offloaded file logs to `asyncio.to_thread`, logged unexpected errors with stack traces.
- `hub/llm.py`: Implemented earliest-delimiter sentence splitting with ellipsis (`...`) support and non-alphanumeric token protection.
- `src/vision/tracker.rs`: Built target state machine with strict hardware clamp [5.0°, 85.0°], `!matched_track_ids.contains(&id)` association guard (tested with proximate detections within threshold), and streak decay across frame gaps.
- `src/bus/mqtt.rs`: Integrated `rumqttc` client with auto-resubscription on broker `ConnAck`, audio speak publishing, and face emotion commands.
- `src/api/mod.rs`: Implemented non-mutating `/stats` via `peek_tracks()`, direct detector status reporting (`idle_waiting_npu` vs `simulated`), and synchronized `/say` (synthesize before face/audio broadcast).
- `firmware/stackchan/stackchan_localmind.ino`: Dynamic MAC-derived MQTT client ID (`stackchan-{MAC}`), external `config.h` credentials without hardcoded passwords in git, `stackchan/cmd/audio` visual speaking indicator, and hardware tilt clamping.
- Resolved all Clippy warnings and removed unused dependencies (`tokio-tungstenite`, `tower-http`, `futures-util`, `uuid`, `av`) and orphaned tools.
- All 6 verification gates passed cleanly (`cargo test` and `uv run pytest tests/test_hub.py`).
