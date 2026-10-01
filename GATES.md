# Completion Gates Ledger — Local Mind

- [x] G1: Cargo manifest and dependencies compile cleanly
  CHECK: cargo check && echo "G1_PASS"
  EXPECT: G1_PASS
  EVIDENCE: G1_PASS (Verified 2026-10-01)

- [x] G2: Vision tracker state machine, multi-object association, and 5–85° tilt servo clamping verified
  CHECK: cargo test --test vision_tracker_test -- --nocapture
  EXPECT: VISION_TRACKER_PASS
  EVIDENCE: VISION_TRACKER_PASS (Verified 2026-10-01)

- [x] G3: MQTT bus models, auto-reconnect subscription, and topic pub/sub contracts verified
  CHECK: cargo test --test bus_test -- --nocapture
  EXPECT: BUS_TEST_PASS
  EVIDENCE: BUS_TEST_PASS (Verified 2026-10-01)

- [x] G4: Voice loop with honest error propagation and rate-limited narration verified
  CHECK: cargo test --test voice_loop_test -- --nocapture
  EXPECT: VOICE_LOOP_PASS
  EVIDENCE: VOICE_LOOP_PASS (Verified 2026-10-01)

- [x] G5: Orchestrator REST endpoints (/health, /stats, /say, /api/vision/frame) and full integration verified
  CHECK: cargo test --test orchestrator_api_test -- --nocapture
  EXPECT: ORCHESTRATOR_API_PASS
  EVIDENCE: ORCHESTRATOR_API_PASS (Verified 2026-10-01)

- [x] G6: Python XiaoZhi Voice Hub streaming audio, earliest boundary chunking, and disconnect safety verified
  CHECK: uv run pytest tests/test_hub.py -v
  EXPECT: 5 passed
  EVIDENCE: 5 passed (Verified 2026-10-01)
