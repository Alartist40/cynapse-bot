# Gates: cynpase-bot Local AI Hub (Rust)

- [x] G1: Hub crate compiles cleanly and builds tests
  CHECK: cargo check --tests
  EXPECT: Finished
  EVIDENCE: Finished `dev` profile [unoptimized + debuginfo] target(s) in 33.99s

- [x] G2: Local OTA discovery endpoint serves valid WebSocket configuration JSON
  CHECK: cargo test --test ota_test test_ota_endpoint -- --nocapture
  EXPECT: test_ota_endpoint ... ok
  EVIDENCE: test test_ota_endpoint ... ok (0.02s)

- [x] G3: Xiaozhi WebSocket handshake receives client hello and returns server hello within deadline
  CHECK: cargo test --test protocol_test test_xiaozhi_handshake -- --nocapture
  EXPECT: test_xiaozhi_handshake ... ok
  EVIDENCE: test test_xiaozhi_handshake ... ok (0.10s)

- [x] G4: Xiaozhi binary Opus audio & listen event loop roundtrips correctly
  CHECK: cargo test --test protocol_test test_listen_opus_roundtrip -- --nocapture
  EXPECT: test_listen_opus_roundtrip ... ok
  EVIDENCE: test test_listen_opus_roundtrip ... ok (0.10s)

- [x] G5: Pipeline engine processes text/audio turn with Cynapse / Leafcutter seam
  CHECK: cargo test --test pipeline_test test_pipeline_turn -- --nocapture
  EXPECT: test_pipeline_turn ... ok
  EVIDENCE: test test_pipeline_turn ... ok (0.02s)

- [x] G6: Audio engine converts Opus/PCM streams and generates valid TTS audio frames
  CHECK: cargo test --test audio_test test_audio_pipeline -- --nocapture
  EXPECT: test_audio_pipeline ... ok
  EVIDENCE: test test_audio_pipeline ... ok (0.00s)

- [x] G7: Persona loader initializes system prompt and registers with Cynapse Dendrite memory
  CHECK: cargo test --test persona_test test_persona_and_memory -- --nocapture
  EXPECT: test_persona_and_memory ... ok
  EVIDENCE: test test_persona_and_memory ... ok (0.01s)

- [x] G8: Hardware action & MCP command dispatcher formats servo and face payloads
  CHECK: cargo test --test mcp_test test_mcp_action_dispatch -- --nocapture
  EXPECT: test_mcp_action_dispatch ... ok
  EVIDENCE: test test_mcp_action_dispatch ... ok (0.00s)

- [x] G9: Embedded Web Dashboard serves status and connects to monitor WebSocket
  CHECK: cargo test --test dashboard_test test_web_dashboard_and_monitor -- --nocapture
  EXPECT: test_web_dashboard_and_monitor ... ok
  EVIDENCE: test test_web_dashboard_and_monitor ... ok (0.03s)
