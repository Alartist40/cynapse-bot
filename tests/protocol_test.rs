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

    // 2. Send some mock Opus audio frames
    let mock_opus_frame = vec![0xF8, 0xFF, 0xFE, 0x01, 0x02];
    ws_stream
        .send(Message::Binary(mock_opus_frame.into()))
        .await
        .unwrap();

    // 3. Send listen stop
    let listen_stop = json!({
        "type": "listen",
        "state": "stop"
    });
    ws_stream
        .send(Message::Text(listen_stop.to_string().into()))
        .await
        .unwrap();

    // 4. Expect STT + LLM/TTS response
    let mut got_stt = false;
    let mut got_tts_start = false;

    while let Some(Ok(Message::Text(resp))) = ws_stream.next().await {
        let val: serde_json::Value = serde_json::from_str(&resp).unwrap();
        if val["type"] == "stt" {
            got_stt = true;
        }
        if val["type"] == "tts" && val["state"] == "start" {
            got_tts_start = true;
        }
        if val["type"] == "tts" && val["state"] == "stop" {
            break;
        }
    }

    assert!(got_stt, "Should receive STT message");
    assert!(got_tts_start, "Should receive TTS start message");
}

#[tokio::test]
async fn test_voice_turn_streams_binary_opus_audio() {
    let (addr, _config) = spawn_test_server().await;
    let url = format!("ws://{}/xiaozhi/ws?token=secret-token", addr);

    let (mut ws_stream, _) = connect_async(&url).await.expect("Failed to connect to WS");

    // 1. Send client hello
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

    // Consume server hello
    let _ = ws_stream.next().await;

    // 2. Send listen start
    let listen_start = json!({
        "type": "listen",
        "state": "start",
        "mode": "auto"
    });
    ws_stream
        .send(Message::Text(listen_start.to_string().into()))
        .await
        .unwrap();

    // 3. Send a client Opus audio frame
    ws_stream
        .send(Message::Binary(vec![0xF8, 0xFF, 0xFE, 0x01].into()))
        .await
        .unwrap();

    // 4. Send listen stop
    let listen_stop = json!({
        "type": "listen",
        "state": "stop"
    });
    ws_stream
        .send(Message::Text(listen_stop.to_string().into()))
        .await
        .unwrap();

    // 5. Collect downlink messages: must receive tts start, THEN binary Opus frames, THEN tts stop
    let mut got_tts_start = false;
    let mut got_tts_stop = false;
    let mut binary_frames = Vec::new();

    while let Some(Ok(msg)) = ws_stream.next().await {
        match msg {
            Message::Text(text) => {
                let val: serde_json::Value = serde_json::from_str(&text).unwrap();
                if val["type"] == "tts" && val["state"] == "start" {
                    got_tts_start = true;
                } else if val["type"] == "tts" && val["state"] == "stop" {
                    got_tts_stop = true;
                    break;
                }
            }
            Message::Binary(bin) => {
                binary_frames.push(bin.to_vec());
            }
            _ => {}
        }
    }

    assert!(got_tts_start, "Must receive tts start message");
    assert!(got_tts_stop, "Must receive tts stop message");
    assert!(
        !binary_frames.is_empty(),
        "FAIL-IF-SILENT: Hub sent 0 binary audio frames between tts start and stop! Robot will be silent."
    );
    let total_bytes: usize = binary_frames.iter().map(|f| f.len()).sum();
    assert!(total_bytes >= 10, "Total binary Opus audio bytes must be > 0 (got {})", total_bytes);
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

    // Test exact wire serialization format for Mcp tool execution
    let msg = ServerMessage::Mcp {
        tool: "self.robot.set_head_angles".to_string(),
        arguments: json!({
            "yaw": 30,
            "pitch": 15,
            "speed": 150
        }),
    };

    let serialized = serde_json::to_string(&msg).unwrap();
    let json_val: serde_json::Value = serde_json::from_str(&serialized).unwrap();

    // Verify root envelope schema
    assert_eq!(json_val["type"], "mcp", "Envelope must have type: 'mcp'");
    assert_eq!(json_val["tool"], "self.robot.set_head_angles");
    assert_eq!(json_val["arguments"]["yaw"], 30);
    assert_eq!(json_val["arguments"]["pitch"], 15);
    assert_eq!(json_val["arguments"]["speed"], 150);

    // Verify deserialization back to ServerMessage::Mcp
    let roundtrip: ServerMessage = serde_json::from_str(&serialized).unwrap();
    assert_eq!(roundtrip, msg);
}
