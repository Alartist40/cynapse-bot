use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Html,
    Extension,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryEvent {
    pub event_type: String, // "turn", "state", "audio_level", "system"
    pub payload: serde_json::Value,
    pub timestamp_ms: u64,
}

pub type TelemetrySender = broadcast::Sender<TelemetryEvent>;

pub async fn handle_dashboard_html() -> Html<&'static str> {
    Html(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>CynapseBot Dashboard</title>
    <style>
        :root { --bg: #0f172a; --card: #1e293b; --text: #f8fafc; --accent: #38bdf8; --green: #4ade80; }
        body { margin: 0; padding: 2rem; background: var(--bg); color: var(--text); font-family: system-ui, sans-serif; }
        .container { max-width: 900px; margin: 0 auto; }
        header { display: flex; justify-content: space-between; align-items: center; border-bottom: 1px solid #334155; padding-bottom: 1rem; margin-bottom: 2rem; }
        h1 { margin: 0; font-size: 1.5rem; display: flex; align-items: center; gap: 0.5rem; }
        .badge { background: var(--green); color: #000; font-size: 0.75rem; padding: 0.2rem 0.6rem; border-radius: 9999px; font-weight: bold; }
        .grid { display: grid; grid-template-columns: 1fr 1fr; gap: 1.5rem; margin-bottom: 2rem; }
        .card { background: var(--card); border-radius: 0.75rem; padding: 1.5rem; border: 1px solid #334155; }
        .card h2 { margin-top: 0; font-size: 1.1rem; color: var(--accent); }
        .feed { height: 300px; overflow-y: auto; background: #0b0f19; border-radius: 0.5rem; padding: 1rem; font-family: monospace; font-size: 0.9rem; }
        .feed-entry { margin-bottom: 0.5rem; border-bottom: 1px solid #1e293b; padding-bottom: 0.5rem; }
        .ts { color: #64748b; font-size: 0.8rem; }
    </style>
</head>
<body>
    <div class="container">
        <header>
            <h1>🤖 CynapseBot Hub <span class="badge" id="status">CONNECTING</span></h1>
            <div id="uptime">StackChan Local Mesh</div>
        </header>
        <div class="grid">
            <div class="card">
                <h2>Robot State</h2>
                <p><strong>Device:</strong> <span id="device-id">stackchan-esp32s3</span></p>
                <p><strong>Protocol:</strong> Xiaozhi v1 (Opus 16k up / 24k down)</p>
                <p><strong>Engine:</strong> Leafcutter Qwen 2.5-7B (Offline)</p>
            </div>
            <div class="card">
                <h2>Memory & Graph</h2>
                <p><strong>Dendrite Nodes:</strong> Active</p>
                <p><strong>Intent Cascade:</strong> Tier 1 Fast Rules Active</p>
            </div>
        </div>
        <div class="card">
            <h2>Real-time Telemetry & Voice Transcript</h2>
            <div class="feed" id="log-feed">
                <div class="feed-entry"><span class="ts">[System]</span> Dashboard initialized. Listening to /ws/monitor...</div>
            </div>
        </div>
    </div>
    <script>
        const wsProtocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const wsUrl = `${wsProtocol}//${window.location.host}/ws/monitor`;
        const ws = new WebSocket(wsUrl);
        const statusEl = document.getElementById('status');
        const feedEl = document.getElementById('log-feed');

        ws.onopen = () => {
            statusEl.innerText = 'ONLINE';
            statusEl.style.background = '#4ade80';
        };

        ws.onclose = () => {
            statusEl.innerText = 'OFFLINE';
            statusEl.style.background = '#f87171';
        };

        ws.onmessage = (e) => {
            try {
                const data = JSON.parse(e.data);
                const entry = document.createElement('div');
                entry.className = 'feed-entry';
                entry.innerHTML = `<span class="ts">[${new Date().toLocaleTimeString()}]</span> <strong>${data.event_type}:</strong> ${JSON.stringify(data.payload)}`;
                feedEl.appendChild(entry);
                feedEl.scrollTop = feedEl.scrollHeight;
            } catch(err) {
                console.error(err);
            }
        };
    </script>
</body>
</html>"#)
}

pub async fn handle_monitor_ws(
    Extension(tx): Extension<TelemetrySender>,
    ws: WebSocketUpgrade,
) -> axum::response::Response {
    ws.on_upgrade(move |socket| handle_monitor_socket(socket, tx))
}

async fn handle_monitor_socket(socket: WebSocket, tx: TelemetrySender) {
    let (mut sender, mut _receiver) = socket.split();
    let mut rx = tx.subscribe();

    info!("New dashboard client connected to /ws/monitor");

    while let Ok(event) = rx.recv().await {
        if let Ok(json_str) = serde_json::to_string(&event) {
            if let Err(e) = sender.send(Message::Text(json_str.into())).await {
                warn!("Dashboard monitor client disconnected: {}", e);
                break;
            }
        }
    }
}
