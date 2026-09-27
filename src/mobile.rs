use axum::{
    extract::{Extension, Json},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::animation::AnimationLibrary;
use crate::dashboard::TelemetryEvent;
use crate::server::AppState;

#[derive(Debug, Deserialize, Serialize)]
pub struct RobotControlRequest {
    pub pan: Option<i16>,
    pub tilt: Option<i16>,
    pub r: Option<u8>,
    pub g: Option<u8>,
    pub b: Option<u8>,
    pub animation: Option<String>,
    pub expression: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ChatSendRequest {
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct HubStatusResponse {
    pub status: String,
    pub device_connected: bool,
    pub session_id: Option<String>,
    pub vision_frames_count: usize,
    pub version: &'static str,
}

pub async fn handle_mobile_app_html() -> Html<&'static str> {
    Html(MOBILE_APP_HTML)
}

pub async fn handle_manifest_json() -> Response {
    (
        [(axum::http::header::CONTENT_TYPE, "application/manifest+json")],
        MANIFEST_JSON,
    )
        .into_response()
}

pub async fn handle_service_worker_js() -> Response {
    (
        [(axum::http::header::CONTENT_TYPE, "application/javascript")],
        SERVICE_WORKER_JS,
    )
        .into_response()
}

pub async fn handle_api_status(
    Extension(state): Extension<AppState>,
) -> Json<HubStatusResponse> {
    let session_guard = state.session.lock().await;
    let (connected, session_id) = match &*session_guard {
        Some(s) => (true, Some(s.id.clone())),
        None => (false, None),
    };

    Json(HubStatusResponse {
        status: "online".to_string(),
        device_connected: connected,
        session_id,
        vision_frames_count: if state.vision.get_latest_frame().await.is_some() { 1 } else { 0 },
        version: env!("CARGO_PKG_VERSION"),
    })
}

pub async fn handle_api_robot_control(
    Extension(state): Extension<AppState>,
    Json(payload): Json<RobotControlRequest>,
) -> Response {
    let mut actions = Vec::new();

    if let (Some(pan), Some(tilt)) = (payload.pan, payload.tilt) {
        actions.push(json!({
            "type": "servo_angles",
            "pan": pan.clamp(-90, 90),
            "tilt": tilt.clamp(-30, 30)
        }));
    }

    if let (Some(r), Some(g), Some(b)) = (payload.r, payload.g, payload.b) {
        actions.push(json!({
            "type": "led_color",
            "r": r,
            "g": g,
            "b": b
        }));
    }

    if let Some(ref anim_name) = payload.animation {
        if let Some(animation) = AnimationLibrary::get(anim_name) {
            actions.push(json!({
                "type": "animation_sequence",
                "name": animation.name,
                "keyframes_count": animation.keyframes.len()
            }));
        }
    }

    if let Some(ref expr) = payload.expression {
        actions.push(json!({
            "type": "expression",
            "name": expr
        }));
    }

    // Broadcast control telemetry
    let _ = state.telemetry_tx.send(TelemetryEvent {
        event_type: "mobile_robot_control".to_string(),
        payload: json!({
            "actions": actions,
            "raw": payload
        }),
        timestamp_ms: 0,
    });

    (StatusCode::OK, Json(json!({ "status": "ok", "actions_dispatched": actions.len() }))).into_response()
}

pub async fn handle_api_chat_send(
    Extension(state): Extension<AppState>,
    Json(payload): Json<ChatSendRequest>,
) -> Response {
    if payload.message.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": "empty message" }))).into_response();
    }

    match state.pipeline.process_text_turn(&payload.message).await {
        Ok(turn) => {
            for reply in &turn.messages {
                let _ = state.telemetry_tx.send(TelemetryEvent {
                    event_type: "mobile_chat_turn".to_string(),
                    payload: serde_json::to_value(reply).unwrap_or_default(),
                    timestamp_ms: 0,
                });
            }

            (StatusCode::OK, Json(json!({
                "status": "ok",
                "response": turn.spoken_text,
                "replies_count": turn.messages.len(),
                "audio_frames_count": turn.audio_frames.len()
            }))).into_response()
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))).into_response()
        }
    }
}

pub async fn handle_api_celestial_memory() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "nodes": [
            { "id": "sys_core", "label": "Core Identity", "tier": "System", "coords": [0.0, 0.0, 0.0], "radius": 14.0, "color": "#f59e0b" },
            { "id": "sem_robot", "label": "StackChan HAL", "tier": "Semantic", "coords": [-35.0, 20.0, 15.0], "radius": 10.0, "color": "#06b6d4" },
            { "id": "sem_vision", "label": "Vision Mesh", "tier": "Semantic", "coords": [30.0, 25.0, -10.0], "radius": 9.0, "color": "#06b6d4" },
            { "id": "ep_chat", "label": "Recent Dialogue", "tier": "Episodic", "coords": [15.0, -30.0, 25.0], "radius": 7.0, "color": "#10b981" },
            { "id": "wk_state", "label": "Working Buffer", "tier": "Working", "coords": [-20.0, -25.0, -20.0], "radius": 6.0, "color": "#8b5cf6" }
        ],
        "links": [
            { "from": "sys_core", "to": "sem_robot", "strength": 0.9 },
            { "from": "sys_core", "to": "sem_vision", "strength": 0.8 },
            { "from": "sys_core", "to": "ep_chat", "strength": 0.75 },
            { "from": "ep_chat", "to": "wk_state", "strength": 0.65 }
        ]
    }))
}

const MANIFEST_JSON: &str = r###"{
  "name": "CynapseBot Mobile Companion",
  "short_name": "CynapseBot",
  "start_url": "/mobile",
  "display": "standalone",
  "background_color": "#090d16",
  "theme_color": "#38bdf8",
  "orientation": "portrait-primary",
  "icons": [
    {
      "src": "/manifest.json",
      "sizes": "192x192 512x512",
      "type": "application/json",
      "purpose": "any maskable"
    }
  ]
}"###;

const SERVICE_WORKER_JS: &str = r###"
const CACHE_NAME = 'cynpase-mobile-v1';
const ASSETS = ['/mobile', '/manifest.json'];

self.addEventListener('install', (e) => {
    e.waitUntil(caches.open(CACHE_NAME).then((cache) => cache.addAll(ASSETS)));
});

self.addEventListener('fetch', (e) => {
    if (e.request.url.includes('/api/') || e.request.url.includes('/ws/')) {
        return;
    }
    e.respondWith(
        fetch(e.request).catch(() => caches.match(e.request))
    );
});
"###;

const MOBILE_APP_HTML: &str = r###"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no, viewport-fit=cover">
    <meta name="apple-mobile-web-app-capable" content="yes">
    <meta name="apple-mobile-web-app-status-bar-style" content="black-translucent">
    <meta name="theme-color" content="#090d16">
    <link rel="manifest" href="/manifest.json">
    <title>CynapseBot Mobile</title>
    <style>
        :root {
            --bg: #090d16;
            --surface: #131b2e;
            --surface-hover: #1c2742;
            --border: #23314f;
            --accent: #38bdf8;
            --accent-glow: rgba(56, 189, 248, 0.25);
            --green: #4ade80;
            --amber: #f59e0b;
            --crimson: #f43f5e;
            --text-main: #f8fafc;
            --text-muted: #94a3b8;
            --radius: 16px;
        }

        * {
            box-sizing: border-box;
            -webkit-tap-highlight-color: transparent;
            user-select: none;
            -webkit-user-select: none;
        }

        body {
            margin: 0;
            padding: env(safe-area-inset-top, 16px) 16px env(safe-area-inset-bottom, 16px) 16px;
            background: var(--bg);
            color: var(--text-main);
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
            min-height: 100vh;
            display: flex;
            flex-direction: column;
            overflow-x: hidden;
        }

        header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            padding: 8px 4px 14px 4px;
        }

        .brand {
            display: flex;
            align-items: center;
            gap: 8px;
            font-weight: 700;
            font-size: 1.15rem;
            letter-spacing: -0.02em;
        }

        .brand-dot {
            width: 10px;
            height: 10px;
            border-radius: 50%;
            background: var(--green);
            box-shadow: 0 0 10px var(--green);
            transition: all 0.3s ease;
        }

        .brand-dot.disconnected {
            background: var(--crimson);
            box-shadow: 0 0 10px var(--crimson);
        }

        .top-meta {
            font-size: 0.75rem;
            color: var(--text-muted);
            background: var(--surface);
            padding: 4px 10px;
            border-radius: 9999px;
            border: 1px solid var(--border);
        }

        .tab-bar {
            display: grid;
            grid-template-columns: repeat(4, 1fr);
            gap: 6px;
            background: var(--surface);
            padding: 4px;
            border-radius: 12px;
            margin-bottom: 14px;
            border: 1px solid var(--border);
        }

        .tab-btn {
            background: transparent;
            border: none;
            color: var(--text-muted);
            padding: 8px 4px;
            border-radius: 8px;
            font-size: 0.8rem;
            font-weight: 600;
            display: flex;
            flex-direction: column;
            align-items: center;
            gap: 3px;
            cursor: pointer;
            transition: all 0.2s;
        }

        .tab-btn.active {
            background: var(--accent);
            color: #090d16;
            box-shadow: 0 2px 8px var(--accent-glow);
        }

        .tab-pane {
            display: none;
            flex-direction: column;
            gap: 14px;
            flex: 1;
        }

        .tab-pane.active {
            display: flex;
        }

        .card {
            background: var(--surface);
            border: 1px solid var(--border);
            border-radius: var(--radius);
            padding: 14px;
        }

        .card-title {
            font-size: 0.85rem;
            text-transform: uppercase;
            letter-spacing: 0.05em;
            color: var(--text-muted);
            margin-top: 0;
            margin-bottom: 10px;
            display: flex;
            justify-content: space-between;
            align-items: center;
        }

        .avatar-container {
            width: 100%;
            aspect-ratio: 16/10;
            background: #020617;
            border-radius: calc(var(--radius) - 4px);
            position: relative;
            overflow: hidden;
            display: flex;
            justify-content: center;
            align-items: center;
            border: 1px solid #1e293b;
        }

        #avatarCanvas {
            width: 100%;
            height: 100%;
            touch-action: none;
        }

        .joystick-container {
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            padding: 10px 0;
        }

        .joystick-base {
            width: 190px;
            height: 190px;
            background: radial-gradient(circle, #1e293b 0%, #0f172a 100%);
            border: 2px solid var(--border);
            border-radius: 50%;
            position: relative;
            touch-action: none;
            display: flex;
            align-items: center;
            justify-content: center;
            box-shadow: inset 0 2px 8px rgba(0,0,0,0.5);
        }

        .joystick-thumb {
            width: 68px;
            height: 68px;
            background: linear-gradient(135deg, var(--accent) 0%, #0284c7 100%);
            border-radius: 50%;
            position: absolute;
            box-shadow: 0 4px 14px var(--accent-glow);
            pointer-events: none;
            transform: translate(0px, 0px);
            transition: transform 0.05s ease-out;
        }

        .joystick-meta {
            display: flex;
            gap: 16px;
            margin-top: 10px;
            font-size: 0.85rem;
            color: var(--text-muted);
            font-family: monospace;
        }

        .action-grid {
            display: grid;
            grid-template-columns: repeat(3, 1fr);
            gap: 8px;
        }

        .btn {
            background: var(--surface-hover);
            color: var(--text-main);
            border: 1px solid var(--border);
            padding: 10px 6px;
            border-radius: 10px;
            font-size: 0.82rem;
            font-weight: 600;
            display: flex;
            flex-direction: column;
            align-items: center;
            gap: 4px;
            cursor: pointer;
            transition: all 0.15s;
        }

        .btn:active {
            transform: scale(0.96);
            background: var(--accent);
            color: #090d16;
        }

        .color-row {
            display: flex;
            gap: 8px;
            margin-top: 8px;
        }

        .color-chip {
            flex: 1;
            height: 32px;
            border-radius: 8px;
            border: 2px solid transparent;
            cursor: pointer;
        }

        .color-chip:active {
            border-color: #fff;
            transform: scale(0.95);
        }

        .chat-feed {
            height: 180px;
            overflow-y: auto;
            background: #060911;
            border-radius: 10px;
            padding: 10px;
            font-size: 0.85rem;
            display: flex;
            flex-direction: column;
            gap: 8px;
            border: 1px solid #1a233a;
        }

        .msg-bubble {
            padding: 8px 12px;
            border-radius: 12px;
            max-width: 85%;
            word-break: break-word;
            line-height: 1.35;
        }

        .msg-user {
            align-self: flex-end;
            background: #0284c7;
            color: #fff;
            border-bottom-right-radius: 2px;
        }

        .msg-bot {
            align-self: flex-start;
            background: #1e293b;
            color: #f1f5f9;
            border-bottom-left-radius: 2px;
        }

        .chat-input-bar {
            display: flex;
            gap: 8px;
            margin-top: 8px;
        }

        .text-input {
            flex: 1;
            background: #060911;
            border: 1px solid var(--border);
            border-radius: 10px;
            padding: 10px 12px;
            color: #fff;
            font-size: 0.9rem;
            outline: none;
        }

        .text-input:focus {
            border-color: var(--accent);
        }

        .ptt-btn {
            width: 100%;
            height: 52px;
            border-radius: 14px;
            background: linear-gradient(135deg, #0284c7 0%, #0369a1 100%);
            border: none;
            color: #fff;
            font-weight: 700;
            font-size: 1rem;
            display: flex;
            align-items: center;
            justify-content: center;
            gap: 8px;
            box-shadow: 0 4px 14px rgba(2, 132, 199, 0.4);
            cursor: pointer;
            margin-top: 8px;
        }

        .ptt-btn.recording {
            background: linear-gradient(135deg, var(--crimson) 0%, #be123c 100%);
            animation: pulse 1s infinite;
        }

        @keyframes pulse {
            0% { box-shadow: 0 0 0 0 rgba(244, 63, 94, 0.7); }
            70% { box-shadow: 0 0 0 15px rgba(244, 63, 94, 0); }
            100% { box-shadow: 0 0 0 0 rgba(244, 63, 94, 0); }
        }

        .camera-box {
            width: 100%;
            aspect-ratio: 4/3;
            background: #000;
            border-radius: 10px;
            overflow: hidden;
            display: flex;
            align-items: center;
            justify-content: center;
            position: relative;
        }

        .camera-img {
            width: 100%;
            height: 100%;
            object-fit: cover;
        }

        .celestial-box {
            width: 100%;
            aspect-ratio: 1/1;
            background: #020617;
            border-radius: 10px;
            position: relative;
            overflow: hidden;
        }

        #celestialCanvas {
            width: 100%;
            height: 100%;
            touch-action: none;
        }
    </style>
</head>
<body>

    <header>
        <div class="brand">
            <div class="brand-dot" id="netDot"></div>
            <span>CynapseBot</span>
        </div>
        <div class="top-meta" id="hubStateLabel">OPI 6 Plus Hub</div>
    </header>

    <div class="tab-bar">
        <button class="tab-btn active" onclick="switchTab('control', event)">
            <span>🕹️</span> Control
        </button>
        <button class="tab-btn" onclick="switchTab('avatar', event)">
            <span>🤖</span> Face
        </button>
        <button class="tab-btn" onclick="switchTab('voice', event)">
            <span>💬</span> Voice
        </button>
        <button class="tab-btn" onclick="switchTab('galaxy', event)">
            <span>🌌</span> Memory
        </button>
    </div>

    <!-- TAB 1: Control & Camera -->
    <div id="tab-control" class="tab-pane active">
        <div class="card">
            <div class="card-title">
                <span>Pan / Tilt Servo Control</span>
                <button class="btn" style="padding: 2px 8px; font-size: 0.7rem;" onclick="resetServos()">Center</button>
            </div>
            <div class="joystick-container">
                <div class="joystick-base" id="joyBase">
                    <div class="joystick-thumb" id="joyThumb"></div>
                </div>
                <div class="joystick-meta">
                    <span>PAN: <strong id="panVal">0°</strong></span>
                    <span>TILT: <strong id="tiltVal">0°</strong></span>
                </div>
            </div>
        </div>

        <div class="card">
            <div class="card-title">Quick Actions & Choreography</div>
            <div class="action-grid">
                <button class="btn" onclick="triggerAnim('dance')">💃 Dance</button>
                <button class="btn" onclick="triggerAnim('nod')">点头 Nod</button>
                <button class="btn" onclick="triggerAnim('shake')">摇头 Shake</button>
                <button class="btn" onclick="triggerAnim('wave')">👋 Wave</button>
                <button class="btn" onclick="triggerAnim('look_around')">👀 Look</button>
                <button class="btn" onclick="triggerAnim('wake')">⚡ Wake</button>
            </div>
            <div class="color-row">
                <div class="color-chip" style="background: #38bdf8;" onclick="setLed(56, 189, 248)"></div>
                <div class="color-chip" style="background: #4ade80;" onclick="setLed(74, 222, 128)"></div>
                <div class="color-chip" style="background: #f59e0b;" onclick="setLed(245, 158, 11)"></div>
                <div class="color-chip" style="background: #f43f5e;" onclick="setLed(244, 63, 94)"></div>
                <div class="color-chip" style="background: #a855f7;" onclick="setLed(168, 85, 247)"></div>
            </div>
        </div>

        <div class="card">
            <div class="card-title">Robot Vision Viewfinder</div>
            <div class="camera-box">
                <img id="camImg" class="camera-img" src="/camera/latest.jpg" onerror="this.style.display='none'" onload="this.style.display='block'" alt="Camera stream">
            </div>
        </div>
    </div>

    <!-- TAB 2: Virtual StackChan Face Screen -->
    <div id="tab-avatar" class="tab-pane">
        <div class="card">
            <div class="card-title">StackChan Screen Mode</div>
            <div class="avatar-container" id="avatarBox">
                <canvas id="avatarCanvas" width="480" height="300"></canvas>
            </div>
        </div>

        <div class="card">
            <div class="card-title">Face Expression</div>
            <div class="action-grid">
                <button class="btn" onclick="setEmotion('happy')">😄 Happy</button>
                <button class="btn" onclick="setEmotion('neutral')">😐 Neutral</button>
                <button class="btn" onclick="setEmotion('thinking')">🤔 Thinking</button>
                <button class="btn" onclick="setEmotion('angry')">😠 Angry</button>
                <button class="btn" onclick="setEmotion('surprised')">😲 Surprised</button>
                <button class="btn" onclick="setEmotion('sleepy')">😴 Sleepy</button>
            </div>
        </div>
    </div>

    <!-- TAB 3: Voice & Chat -->
    <div id="tab-voice" class="tab-pane">
        <div class="card" style="flex: 1; display: flex; flex-direction: column;">
            <div class="card-title">Live Conversation Feed</div>
            <div class="chat-feed" id="chatFeed">
                <div class="msg-bubble msg-bot">Hello! I am CynapseBot. You can speak or type to me here.</div>
            </div>

            <div class="chat-input-bar">
                <input type="text" id="textInput" class="text-input" placeholder="Type a message..." onkeydown="if(event.key==='Enter') sendText()">
                <button class="btn" style="padding: 0 16px;" onclick="sendText()">Send</button>
            </div>

            <button class="ptt-btn" id="pttButton" onpointerdown="startPtt(event)" onpointerup="stopPtt(event)" onpointercancel="stopPtt(event)">
                <span id="pttIcon">🎤</span> <span id="pttLabel">Push & Hold to Speak</span>
            </button>
        </div>
    </div>

    <!-- TAB 4: Mazzaroth Galaxy Viewport -->
    <div id="tab-galaxy" class="tab-pane">
        <div class="card">
            <div class="card-title">Mazzaroth 3D Memory Constellation</div>
            <div class="celestial-box">
                <canvas id="celestialCanvas" width="400" height="400"></canvas>
            </div>
        </div>
        <div class="card">
            <div class="card-title">Memory Metrics</div>
            <div style="font-size: 0.85rem; color: var(--text-muted); line-height: 1.6;">
                <div>🌟 Core System Nodes: <strong>Active</strong></div>
                <div>🌿 Semantic Knowledge: <strong>Indexed (FTS5)</strong></div>
                <div>⏳ Ebbinghaus Retention: <strong>Calculated</strong></div>
                <div>🔗 Hebbian Synapses: <strong>Dynamic Weighting</strong></div>
            </div>
        </div>
    </div>

    <script>
        if ('serviceWorker' in navigator) {
            navigator.serviceWorker.register('/service-worker.js').catch(console.error);
        }

        let currentPan = 0;
        let currentTilt = 0;
        let lastSentPan = 0;
        let lastSentTilt = 0;
        let sendThrottle = null;
        let emotion = 'neutral';
        let gazeX = 0;
        let gazeY = 0;
        let blinkPhase = 0;

        function switchTab(tabId, ev) {
            document.querySelectorAll('.tab-btn').forEach(b => b.classList.remove('active'));
            document.querySelectorAll('.tab-pane').forEach(p => p.classList.remove('active'));
            if (ev && ev.currentTarget) {
                ev.currentTarget.classList.add('active');
            }
            document.getElementById('tab-' + tabId).classList.add('active');
        }

        function haptic() {
            if (navigator.vibrate) navigator.vibrate(12);
        }

        const joyBase = document.getElementById('joyBase');
        const joyThumb = document.getElementById('joyThumb');
        const panValEl = document.getElementById('panVal');
        const tiltValEl = document.getElementById('tiltVal');
        const maxRadius = 65;

        let joyActive = false;
        let joyRect = null;

        joyBase.addEventListener('pointerdown', (e) => {
            joyActive = true;
            joyBase.setPointerCapture(e.pointerId);
            joyRect = joyBase.getBoundingClientRect();
            handleJoyMove(e);
            haptic();
        });

        joyBase.addEventListener('pointermove', (e) => {
            if (!joyActive) return;
            handleJoyMove(e);
        });

        const endJoy = (e) => {
            if (!joyActive) return;
            joyActive = false;
            joyThumb.style.transform = 'translate(0px, 0px)';
            currentPan = 0;
            currentTilt = 0;
            updateAngleLabels();
            sendControl();
            haptic();
        };

        joyBase.addEventListener('pointerup', endJoy);
        joyBase.addEventListener('pointercancel', endJoy);

        function handleJoyMove(e) {
            const centerX = joyRect.left + joyRect.width / 2;
            const centerY = joyRect.top + joyRect.height / 2;
            let dx = e.clientX - centerX;
            let dy = e.clientY - centerY;
            const dist = Math.hypot(dx, dy);

            if (dist > maxRadius) {
                dx = (dx / dist) * maxRadius;
                dy = (dy / dist) * maxRadius;
            }

            joyThumb.style.transform = `translate(${dx}px, ${dy}px)`;

            currentPan = Math.round((dx / maxRadius) * 90);
            currentTilt = Math.round((-dy / maxRadius) * 30);
            gazeX = dx / maxRadius;
            gazeY = dy / maxRadius;

            updateAngleLabels();
            throttledSendControl();
        }

        function updateAngleLabels() {
            panValEl.innerText = `${currentPan}°`;
            tiltValEl.innerText = `${currentTilt}°`;
        }

        function throttledSendControl() {
            if (sendThrottle) return;
            sendThrottle = setTimeout(() => {
                sendControl();
                sendThrottle = null;
            }, 80);
        }

        async function sendControl(extra = {}) {
            if (currentPan === lastSentPan && currentTilt === lastSentTilt && Object.keys(extra).length === 0) return;
            lastSentPan = currentPan;
            lastSentTilt = currentTilt;

            const payload = {
                pan: currentPan,
                tilt: currentTilt,
                ...extra
            };

            try {
                await fetch('/api/robot/control', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(payload)
                });
            } catch(e) {
                console.error(e);
            }
        }

        function resetServos() {
            currentPan = 0;
            currentTilt = 0;
            gazeX = 0;
            gazeY = 0;
            joyThumb.style.transform = 'translate(0px, 0px)';
            updateAngleLabels();
            sendControl();
            haptic();
        }

        function triggerAnim(name) {
            haptic();
            sendControl({ animation: name });
        }

        function setLed(r, g, b) {
            haptic();
            sendControl({ r, g, b });
        }

        function setEmotion(em) {
            emotion = em;
            haptic();
            sendControl({ expression: em });
        }

        const avatarCanvas = document.getElementById('avatarCanvas');
        const actx = avatarCanvas.getContext('2d');

        function renderAvatar() {
            actx.clearRect(0, 0, avatarCanvas.width, avatarCanvas.height);
            const w = avatarCanvas.width;
            const h = avatarCanvas.height;
            const eyeSpacing = 110;
            const eyeY = h / 2 - 10 + (gazeY * 15);
            const leftEyeX = w / 2 - eyeSpacing + (gazeX * 25);
            const rightEyeX = w / 2 + eyeSpacing + (gazeX * 25);

            blinkPhase += 0.05;
            const isBlink = Math.sin(blinkPhase) > 0.96;

            actx.fillStyle = '#38bdf8';
            actx.shadowColor = 'rgba(56, 189, 248, 0.6)';
            actx.shadowBlur = 15;

            function drawEye(cx, cy) {
                actx.save();
                actx.translate(cx, cy);

                if (isBlink || emotion === 'sleepy') {
                    actx.beginPath();
                    actx.arc(0, 0, 30, 0.2 * Math.PI, 0.8 * Math.PI, false);
                    actx.lineWidth = 10;
                    actx.strokeStyle = '#38bdf8';
                    actx.stroke();
                } else if (emotion === 'happy') {
                    actx.beginPath();
                    actx.arc(0, 10, 32, 1.15 * Math.PI, 1.85 * Math.PI, false);
                    actx.lineWidth = 12;
                    actx.strokeStyle = '#38bdf8';
                    actx.stroke();
                } else if (emotion === 'angry') {
                    actx.beginPath();
                    actx.ellipse(0, 0, 32, 24, cx < w/2 ? 0.4 : -0.4, 0, Math.PI * 2);
                    actx.fill();
                } else if (emotion === 'surprised') {
                    actx.beginPath();
                    actx.arc(0, 0, 38, 0, Math.PI * 2);
                    actx.fill();
                } else {
                    actx.beginPath();
                    actx.ellipse(0, 0, 26, 36, 0, 0, Math.PI * 2);
                    actx.fill();
                }
                actx.restore();
            }

            drawEye(leftEyeX, eyeY);
            drawEye(rightEyeX, eyeY);

            actx.beginPath();
            actx.lineWidth = 6;
            actx.strokeStyle = '#38bdf8';
            if (emotion === 'happy') {
                actx.arc(w / 2 + (gazeX * 10), h / 2 + 55, 18, 0.1 * Math.PI, 0.9 * Math.PI, false);
                actx.stroke();
            } else if (emotion === 'surprised') {
                actx.arc(w / 2 + (gazeX * 10), h / 2 + 60, 10, 0, Math.PI * 2);
                actx.stroke();
            }

            requestAnimationFrame(renderAvatar);
        }
        renderAvatar();

        async function sendText() {
            const input = document.getElementById('textInput');
            const text = input.value.trim();
            if (!text) return;
            input.value = '';

            addChatBubble(text, 'user');

            try {
                const res = await fetch('/api/chat/send', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ message: text })
                });
                const data = await res.json();
                if (data.response) {
                    addChatBubble(data.response, 'bot');
                }
            } catch(e) {
                addChatBubble('Error: Hub unreachable', 'bot');
            }
        }

        function addChatBubble(text, sender) {
            const feed = document.getElementById('chatFeed');
            const bubble = document.createElement('div');
            bubble.className = `msg-bubble msg-${sender}`;
            bubble.innerText = text;
            feed.appendChild(bubble);
            feed.scrollTop = feed.scrollHeight;
        }

        let isRecording = false;
        let speechRecognizer = null;
        let recognizedSpeech = '';

        if ('SpeechRecognition' in window || 'webkitSpeechRecognition' in window) {
            const SpeechRec = window.SpeechRecognition || window.webkitSpeechRecognition;
            speechRecognizer = new SpeechRec();
            speechRecognizer.continuous = true;
            speechRecognizer.interimResults = true;
            speechRecognizer.lang = 'en-US';

            speechRecognizer.onresult = (event) => {
                let interim = '';
                for (let i = event.resultIndex; i < event.results.length; ++i) {
                    if (event.results[i].isFinal) {
                        recognizedSpeech += event.results[i][0].transcript;
                    } else {
                        interim += event.results[i][0].transcript;
                    }
                }
                if (interim) {
                    document.getElementById('pttLabel').innerText = `"${interim}"`;
                }
            };

            speechRecognizer.onerror = (e) => {
                console.warn('Speech recognition error:', e.error);
            };
        }

        function startPtt(e) {
            e.preventDefault();
            isRecording = true;
            recognizedSpeech = '';
            document.getElementById('pttButton').classList.add('recording');
            document.getElementById('pttLabel').innerText = 'Listening... (Release to send)';
            haptic();

            if (speechRecognizer) {
                try {
                    speechRecognizer.start();
                } catch(err) {}
            }
        }

        async function stopPtt(e) {
            if (!isRecording) return;
            e.preventDefault();
            isRecording = false;
            document.getElementById('pttButton').classList.remove('recording');
            document.getElementById('pttLabel').innerText = 'Push & Hold to Speak';
            haptic();

            if (speechRecognizer) {
                try {
                    speechRecognizer.stop();
                } catch(err) {}
            }

            // Give recognition a moment to finalize
            setTimeout(async () => {
                const textToSend = recognizedSpeech.trim();
                if (textToSend.length > 0) {
                    addChatBubble(textToSend, 'user');
                    try {
                        const res = await fetch('/api/chat/send', {
                            method: 'POST',
                            headers: { 'Content-Type': 'application/json' },
                            body: JSON.stringify({ message: textToSend })
                        });
                        const data = await res.json();
                        if (data.response) addChatBubble(data.response, 'bot');
                    } catch(err) {
                        addChatBubble('Error: Hub unreachable', 'bot');
                    }
                }
            }, 300);
        }

        const celCanvas = document.getElementById('celestialCanvas');
        const cctx = celCanvas.getContext('2d');
        let rotAngle = 0;
        let galaxyNodes = [];
        let galaxyLinks = [];

        async function loadGalaxy() {
            try {
                const res = await fetch('/api/memory/celestial');
                const data = await res.json();
                galaxyNodes = data.nodes || [];
                galaxyLinks = data.links || [];
            } catch(e) {
                console.error(e);
            }
        }
        loadGalaxy();

        function renderGalaxy() {
            cctx.clearRect(0, 0, celCanvas.width, celCanvas.height);
            const cx = celCanvas.width / 2;
            const cy = celCanvas.height / 2;
            rotAngle += 0.008;

            const cosA = Math.cos(rotAngle);
            const sinA = Math.sin(rotAngle);

            const projected = {};
            galaxyNodes.forEach(node => {
                const [x, y, z] = node.coords;
                const rx = x * cosA - z * sinA;
                const rz = x * sinA + z * cosA + 120;
                const scale = 220 / (rz || 1);
                const px = cx + rx * scale;
                const py = cy + y * scale;
                projected[node.id] = { px, py, scale, ...node };
            });

            cctx.strokeStyle = 'rgba(56, 189, 248, 0.25)';
            cctx.lineWidth = 1.5;
            galaxyLinks.forEach(link => {
                const p1 = projected[link.from];
                const p2 = projected[link.to];
                if (p1 && p2) {
                    cctx.beginPath();
                    cctx.moveTo(p1.px, p1.py);
                    cctx.lineTo(p2.px, p2.py);
                    cctx.stroke();
                }
            });

            Object.values(projected).forEach(p => {
                cctx.beginPath();
                cctx.arc(p.px, p.py, Math.max(4, p.radius * (p.scale / 2)), 0, Math.PI * 2);
                cctx.fillStyle = p.color || '#38bdf8';
                cctx.shadowColor = p.color || '#38bdf8';
                cctx.shadowBlur = 10;
                cctx.fill();

                cctx.shadowBlur = 0;
                cctx.fillStyle = '#cbd5e1';
                cctx.font = '10px monospace';
                cctx.fillText(p.label, p.px + 8, p.py + 3);
            });

            requestAnimationFrame(renderGalaxy);
        }
        renderGalaxy();

        const wsProto = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const ws = new WebSocket(`${wsProto}//${window.location.host}/ws/monitor`);
        const netDot = document.getElementById('netDot');
        const hubStateLabel = document.getElementById('hubStateLabel');

        ws.onopen = () => {
            netDot.classList.remove('disconnected');
            hubStateLabel.innerText = 'Connected';
        };

        ws.onclose = () => {
            netDot.classList.add('disconnected');
            hubStateLabel.innerText = 'Disconnected';
        };

        setInterval(() => {
            const cam = document.getElementById('camImg');
            if (cam && document.getElementById('tab-control').classList.contains('active')) {
                cam.src = `/camera/latest.jpg?t=${Date.now()}`;
            }
        }, 2000);
    </script>
</body>
</html>"###;
