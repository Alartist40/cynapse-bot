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

- [x] G10: Mazzaroth graph core compiles and persists 4-tier knowledge nodes with SQLite FTS5
  CHECK: cargo test --test mazzaroth_test test_mazzaroth_core_persistence -- --nocapture
  EXPECT: test_mazzaroth_core_persistence ... ok
  EVIDENCE: test test_mazzaroth_core_persistence ... ok (0.01s)

- [x] G11: Mazzaroth cognitive activation & decay calculates memory strength and Hebbian link reinforcement
  CHECK: cargo test --test mazzaroth_test test_cognitive_decay_and_hebbian -- --nocapture
  EXPECT: test_cognitive_decay_and_hebbian ... ok
  EVIDENCE: test test_cognitive_decay_and_hebbian ... ok (0.00s)

- [x] G12: Mazzaroth multi-tier hierarchy migrates working turns to episodic logs and consolidates semantic knowledge
  CHECK: cargo test --test mazzaroth_test test_hierarchy_consolidation -- --nocapture
  EXPECT: test_hierarchy_consolidation ... ok
  EVIDENCE: test test_hierarchy_consolidation ... ok (0.00s)

- [x] G13: Mazzaroth celestial spatial engine computes 3D gravity coordinates and constellation clusters
  CHECK: cargo test --test mazzaroth_test test_celestial_spatial_layout -- --nocapture
  EXPECT: test_celestial_spatial_layout ... ok
  EVIDENCE: test test_celestial_spatial_layout ... ok (0.00s)

- [x] G14: Desktop GUI crate and binary compiles cleanly
  CHECK: cargo check --tests
  EXPECT: Finished
  EVIDENCE: Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.52s

- [x] G15: Desktop GUI state manager processes control events, telemetry, and pan/tilt updates
  CHECK: cargo test --test gui_test test_gui_state_manager -- --nocapture
  EXPECT: test_gui_state_manager ... ok
  EVIDENCE: test test_gui_state_manager ... ok (0.00s)

- [x] G16: Mazzaroth constellation 2D/3D viewport projection calculates screen positions
  CHECK: cargo test --test gui_test test_constellation_viewport_projection -- --nocapture
  EXPECT: test_constellation_viewport_projection ... ok
  EVIDENCE: test test_constellation_viewport_projection ... ok (0.00s)

- [x] G17: Mobile PWA endpoint serves responsive app bundle, manifest, and service worker
  CHECK: cargo test --test mobile_test test_mobile_pwa_endpoints -- --nocapture
  EXPECT: test_mobile_pwa_endpoints ... ok
  EVIDENCE: test test_mobile_pwa_endpoints ... ok (0.01s)

- [x] G18: Mobile robot control API dispatches pan/tilt, choreographies, and broadcasts telemetry
  CHECK: cargo test --test mobile_test test_mobile_robot_control_and_telemetry -- --nocapture
  EXPECT: test_mobile_robot_control_and_telemetry ... ok
  EVIDENCE: test test_mobile_robot_control_and_telemetry ... ok (0.00s)

- [x] G19: Voice turn over WebSocket streams binary Opus audio frames (24 kHz, 60 ms) between TTS start and stop
  CHECK: cargo test --test protocol_test test_voice_turn_streams_binary_opus_audio -- --nocapture
  EXPECT: test_voice_turn_streams_binary_opus_audio ... ok
  EVIDENCE: test test_voice_turn_streams_binary_opus_audio ... ok (0.00s)

- [x] G20: Negative authentication & OTA security verification (unauthenticated requests rejected with 401)
  CHECK: cargo test --test ota_test test_ota_endpoint -- --nocapture
  EXPECT: test_ota_endpoint ... ok
  EVIDENCE: test test_ota_endpoint ... ok (0.02s)

- [x] G21: REST/GUI/Mobile command dispatch reaches connected robot WebSocket as ServerMessage::Mcp (fail-if-dead-end)
  CHECK: cargo test --test mobile_test test_mobile_control_reaches_connected_robot_socket -- --nocapture
  EXPECT: test_mobile_control_reaches_connected_robot_socket ... ok
  EVIDENCE: test test_mobile_control_reaches_connected_robot_socket ... ok (0.05s)

- [x] G22: Linear audio resampling (22050 Hz -> 24000 Hz) & Firmware-Conformant MCP mapping (zero unhandled tool names)
  CHECK: cargo test --test audio_test --test mcp_test -- --nocapture
  EXPECT: test_audio_pipeline ... ok && test_mcp_action_dispatch ... ok
  EVIDENCE: test test_audio_pipeline ... ok (0.02s) / test test_mcp_action_dispatch ... ok (0.00s)

- [x] G23: Audio PTT endpoint container verification (rejects non-RIFF/WAV with 400) & Fail-if-Hello gate on untranscribed audio
  CHECK: cargo test --test mobile_test test_mobile_chat_and_status -- --nocapture
  EXPECT: test_mobile_chat_and_status ... ok
  EVIDENCE: test test_mobile_chat_and_status ... ok (0.00s)

- [x] G24: Robot WebSocket path untranscribed audio yields zero fabricated "Hello" turns (device fail-if-Hello gate)
  CHECK: cargo test --test protocol_test test_device_path_fail_if_hello_on_untranscribed_audio -- --nocapture
  EXPECT: test_device_path_fail_if_hello_on_untranscribed_audio ... ok
  EVIDENCE: test test_device_path_fail_if_hello_on_untranscribed_audio ... ok (0.30s)

- [x] G25: ServerMessage::Mcp JSON wire envelope exact serialization & deserialization conformance
  CHECK: cargo test --test protocol_test test_mcp_wire_envelope_conformance -- --nocapture
  EXPECT: test_mcp_wire_envelope_conformance ... ok
  EVIDENCE: test test_mcp_wire_envelope_conformance ... ok (0.00s)





