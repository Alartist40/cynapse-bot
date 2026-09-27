use cynpase_bot::{create_router, AppState, HubConfig, PipelineEngine};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

async fn spawn_test_server() -> (SocketAddr, Arc<HubConfig>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let config = Arc::new(HubConfig {
        bind_addr: addr,
        public_ws_url: format!("ws://{}/xiaozhi/ws", addr),
        auth_token: "secret-token".to_string(),
        leafcutter_url: "http://127.0.0.1:9999/mock-llm".to_string(),
        model_name: "test-model".to_string(),
    });

    let pipeline = PipelineEngine::new(config.clone());
    let (telemetry_tx, _) = tokio::sync::broadcast::channel(10);
    let (device_cmd_tx, _) = tokio::sync::broadcast::channel(10);
    let state = AppState {
        config: config.clone(),
        pipeline,
        session: Arc::new(Mutex::new(None)),
        telemetry_tx,
        device_cmd_tx,
        vision: cynpase_bot::VisionManager::new(),
    };

    let app = create_router(state);
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (addr, config)
}

#[tokio::test]
async fn test_xiaozhi_handshake() {
    let (addr, _config) = spawn_test_server().await;
    let url = format!("ws://{}/xiaozhi/ws?token=secret-token", addr);

    let (mut ws_stream, _) = connect_async(&url).await.expect("Failed to connect to WS");

    // Send client hello
    let client_hello = json!({
        "type": "hello",
        "version": 1,
        "features": { "mcp": true },
        "transport": "websocket",
        "audio_params": {
            "format": "opus",
            "sample_rate": 16000,
            "channels": 1,
            "frame_duration": 60
        }
    });

    ws_stream
        .send(Message::Text(client_hello.to_string().into()))
        .await
        .unwrap();

    // Expect server hello
    if let Some(Ok(Message::Text(resp))) = ws_stream.next().await {
        let val: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(val["type"], "hello");
        assert_eq!(val["transport"], "websocket");
        assert_eq!(val["audio_params"]["sample_rate"], 24000);
        assert!(val["session_id"].is_string());
    } else {
        panic!("Expected server hello text message");
    }
}

#[tokio::test]
async fn test_listen_opus_roundtrip() {
    let (addr, _config) = spawn_test_server().await;
    let url = format!("ws://{}/xiaozhi/ws?token=secret-token", addr);

    let (mut ws_stream, _) = connect_async(&url).await.expect("Failed to connect to WS");

    // 1. Send listen start
    let listen_start = json!({
        "type": "listen",
        "state": "start",
        "mode": "auto"
    });
    ws_stream
        .send(Message::Text(listen_start.to_string().into()))
        .await
        .unwrap();

    // 2. Send listen stop
    let listen_stop = json!({
        "type": "listen",
        "state": "stop"
    });
    ws_stream
        .send(Message::Text(listen_stop.to_string().into()))
        .await
        .unwrap();

    // 3. Receive with timeout — since no speech was in the buffer, expect clean untranscribed completion (0 fake turns)
    let timeout_res = tokio::time::timeout(tokio::time::Duration::from_millis(300), ws_stream.next()).await;
    assert!(timeout_res.is_err() || timeout_res.unwrap().is_none(), "Empty audio buffer must not emit fabricated messages");
}

#[tokio::test]
async fn test_voice_turn_streams_binary_opus_audio() {
    let config = Arc::new(HubConfig::default());
    let pipeline = PipelineEngine::new(config);

    // Verify that turn execution generates TTS start, binary Opus audio frames, and TTS stop
    let turn = pipeline.process_text_turn("Stop robot movement").await.unwrap();

    assert_eq!(turn.messages.len(), 4, "Turn must produce STT, MCP, TTS start, and TTS stop");
    assert!(!turn.audio_frames.is_empty(), "Turn must generate binary Opus audio frames");
    assert!(turn.audio_frames.len() >= 2, "Must contain at least 2 Opus audio frames");

    let has_stt = turn.messages.iter().any(|m| matches!(m, cynpase_bot::protocol::ServerMessage::Stt { .. }));
    let has_mcp = turn.messages.iter().any(|m| matches!(m, cynpase_bot::protocol::ServerMessage::Mcp { .. }));
    let has_tts_start = turn.messages.iter().any(|m| matches!(m, cynpase_bot::protocol::ServerMessage::Tts { state, .. } if state == "start"));
    let has_tts_stop = turn.messages.iter().any(|m| matches!(m, cynpase_bot::protocol::ServerMessage::Tts { state, .. } if state == "stop"));

    assert!(has_stt, "Turn must contain STT acknowledgment");
    assert!(has_mcp, "Turn must contain native Mcp tool execution");
    assert!(has_tts_start, "Turn must contain TTS start");
    assert!(has_tts_stop, "Turn must contain TTS stop");

    // Decode all frames to verify 24 kHz Opus integrity
    let mut decoder = opus::Decoder::new(24000, opus::Channels::Mono).unwrap();
    let mut decoded_buf = vec![0i16; 1440];
    for frame in &turn.audio_frames {
        let decoded = decoder.decode(frame, &mut decoded_buf, false).unwrap();
        assert_eq!(decoded, 1440, "Each frame must decode to exactly 1440 samples (60ms @ 24kHz)");
    }
}

#[tokio::test]
async fn test_device_path_fail_if_hello_on_untranscribed_audio() {
    let (addr, _config) = spawn_test_server().await;
    let url = format!("ws://{}/xiaozhi/ws?token=secret-token", addr);

    let (mut ws_stream, _) = connect_async(&url).await.expect("Failed to connect to WS");

    // 1. Send listen start
    let listen_start = json!({
        "type": "listen",
        "state": "start",
        "mode": "auto"
    });
    ws_stream
        .send(Message::Text(listen_start.to_string().into()))
        .await
        .unwrap();

    // 2. Send empty / silence packets (untranscribable)
    let silent_frame = vec![0x00, 0x00, 0x00, 0x00];
    ws_stream.send(Message::Binary(silent_frame.into())).await.unwrap();

    // 3. Send listen stop
    let listen_stop = json!({
        "type": "listen",
        "state": "stop"
    });
    ws_stream
        .send(Message::Text(listen_stop.to_string().into()))
        .await
        .unwrap();

    // 4. Collect any responses with a 300ms timeout
    let mut received_messages = Vec::new();
    while let Ok(Some(Ok(msg))) = tokio::time::timeout(tokio::time::Duration::from_millis(300), ws_stream.next()).await {
        if let Message::Text(text) = msg {
            let val: serde_json::Value = serde_json::from_str(&text).unwrap();
            received_messages.push(val);
        }
    }

    // 5. FAIL-IF-HELLO GATE: Ensure no STT/LLM turn with text="Hello" was fabricated
    for msg in &received_messages {
        if msg["type"] == "stt" {
            assert_ne!(msg["text"], "Hello", "FAIL-IF-HELLO: Device path fabricated 'Hello' STT transcript on empty speech!");
        }
        if msg["type"] == "tts" {
            if let Some(text) = msg["text"].as_str() {
                assert!(!text.to_lowercase().contains("how can i help you"), "FAIL-IF-HELLO: Fabricated conversational turn occurred on empty audio");
            }
        }
    }
}

#[test]
fn test_mcp_wire_envelope_conformance() {
    use cynpase_bot::protocol::ServerMessage;

    // Test exact wire serialization format matching 78/xiaozhi-esp32 application.cc:565-568 & mcp_server.cc:353-434
    let msg = ServerMessage::mcp(
        "self.robot.set_head_angles",
        json!({
            "yaw": 30,
            "pitch": 15,
            "speed": 150
        }),
    );

    let serialized = serde_json::to_string(&msg).unwrap();
    let json_val: serde_json::Value = serde_json::from_str(&serialized).unwrap();

    // 1. Root envelope must be {"type": "mcp", "payload": { ... }}
    assert_eq!(json_val["type"], "mcp", "Envelope must have type: 'mcp'");
    assert!(json_val["payload"].is_object(), "application.cc:565 requires payload object");

    // 2. Payload must be JSON-RPC 2.0 tools/call object
    assert_eq!(json_val["payload"]["jsonrpc"], "2.0", "mcp_server.cc:358 requires jsonrpc: '2.0'");
    assert_eq!(json_val["payload"]["method"], "tools/call", "mcp_server.cc:362 requires method: 'tools/call'");
    assert_eq!(json_val["payload"]["id"], 1);

    // 3. Params must have name & arguments
    assert_eq!(json_val["payload"]["params"]["name"], "self.robot.set_head_angles");
    assert_eq!(json_val["payload"]["params"]["arguments"]["yaw"], 30);
    assert_eq!(json_val["payload"]["params"]["arguments"]["pitch"], 15);
    assert_eq!(json_val["payload"]["params"]["arguments"]["speed"], 150);

    // 4. Verify deserialization back to ServerMessage::Mcp
    let roundtrip: ServerMessage = serde_json::from_str(&serialized).unwrap();
    assert_eq!(roundtrip, msg);
}
