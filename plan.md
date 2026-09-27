# cynpase-bot — Master Plan

**Goal:** an offline, local-first AI ecosystem for the StackChan robot (ESP32-S3). AI runs on a
computer on your LAN; the robot connects to it instead of the cloud. Zero internet required at runtime.

**Status:** planning. Reference audits complete (read-only, nothing edited).
**Repo rule:** all build work happens here. `/home/xander/Documents/reference/robots/*` and
`/home/xander/Documents/portfolio/pi/` are read-only references.

---

## 0. How we work (adopted from `prompt personality/enhancement/`)

- **i-have-adhd** — lead with the next action, number steps, restate state every turn, ≤5 visible
  items per list, no preamble/recap/closer, specific time estimates.
- **ponytail** — YAGNI 7-rung ladder: does it need to exist → already in repo → stdlib → platform
  feature → installed dep → one line → minimum code. Bug fix = root cause. One runnable check for
  non-trivial logic. `# ponytail: <ceiling>, <upgrade path>` for deliberate corners.
- **unlazy** — `GATES.md` (CHECK/EXPECT) before substantial builds; impossible gates get `ABANDON:`;
  never report done while a required gate is unmet.
- **awesome-harness** — pull context when needed, keep contracts explicit, verify before claiming done.

---

## 1. The four references — what each gives us

| Reference | What it is | What cynpase-bot takes |
|---|---|---|
| **StackChan** | Our robot. M5Stack CoreS3 (ESP32-S3, 16 MB flash, 8 MB PSRAM), camera, dual mics, speaker, 2 servos, OLED. ESP-IDF firmware + Go server + Flutter app | The **device** and its **xiaozhi audio protocol**. Already 80% LAN-capable. Stays unmodified (config-only) |
| **OmniBot** | PC hub (FastAPI) + thin ESP32 Pixel | The **hub topology**: device socket / dashboard socket / session objects / runtime config push / persona files |
| **pi/ (Pathfinder Eye)** | Your offline Pi 5 robot: Go brain + Leafcutter + whisper + TTS + memory | The **local AI guts**: Leafcutter LLM, whisper STT, TTS, 3-tier intent cascade, Dendrite memory, Cynapse orchestration concept |
| **sesame-robot** | Your ESP32 quadruped, fully offline, JSON API | **PC-brain → ESP32-body over LAN proven**; the `{"command","face"}` two-channel contract; interruptible animations; captive portal + mDNS |

**One-line synthesis:** OmniBot's *topology* + Pathfinder's *guts* + StackChan's *body* +
sesame's *device contract*.

---

## 2. Reference audit details (the facts we build on)

### 2.1 StackChan — the robot (read-only audit)

**AI path today = cloud only.** Vendored xiaozhi-esp32: mic → Opus 16 kHz → WebSocket → cloud
ASR/LLM/TTS → Opus 24 kHz → speaker.

- **Choke point #1 — AI server URL comes only from OTA.** `CONFIG_OTA_URL`
  (`firmware/main/Kconfig.projbuild:16-18`, default `https://api.tenclass.net/xiaozhi/ota/`) →
  OTA response writes NVS `Settings("websocket")/url` → xiaozhi connects. Nothing else supplies it.
  **Fix: serve a local OTA JSON that returns `ws://<hub>:<port>`** — no firmware code change.
- **Everything else is already LAN.** One knob: `CONFIG_STACKCHAN_SERVER_URL`
  (`Kconfig.projbuild:3-5`) drives avatar WS, app center, account. Override seam:
  `sdkconfig.defaults.local` (`firmware/CMakeLists.txt:11-20`). Weak symbols in
  `hal/utils/secret_logic/secret_logic.cpp:11-28` are overridable by design.
- **Go server (`:12800`)**: REST + avatar relay (`/stackChan/ws`, first-byte message types) — no
  inference anywhere. Cloud bits: `server/internal/xiaozhi/xiaozhi.go:37` proxies
  `https://xiaozhi.me/`; hardcoded IP `47.113.125.164` at `dance_v2_get_list.go:15`.
- **Flutter app**: `app/lib/network/urls.dart:24` is a configurable const; agent-config UI is
  hardcoded to `https://XiaoZhi.me/` (`XiaoZhi_util.dart:38`) — affects UX only, not the voice path.
- **Device strengths to keep**: on-device wake word/AFE/VAD, Opus codec, MCP tools executed locally
  (`hal_mcp.cpp` — head/servo), speaker, face, servos.
- **Cloud leftovers**: NTP needed only for TLS (`hal_network.cpp:37-39`), ezdata is opt-in, OTA
  update path follows `CONFIG_OTA_URL` (solved by the same local OTA).
- MQTT fallback fires if the WS handshake times out (10 s server-hello deadline) — our local server
  must answer `hello` fast or the device silently switches protocol.

**xiaozhi protocol spec (what the local hub must speak):**

```
Connect headers: Authorization, Protocol-Version, Device-Id, Client-Id
Client hello: {"type":"hello","version":1,"features":{"mcp":true},"transport":"websocket",
               "audio_params":{"format":"opus","sample_rate":16000,"channels":1,"frame_duration":60}}
Server hello: {"type":"hello","transport":"websocket","session_id":"...",
               "audio_params":{"sample_rate":24000,"frame_duration":60}}   (≤10 s)
Control: {"type":"listen","state":"start","mode":"auto"} / "stop" / {"type":"abort",...}
Audio:   binary Opus frames, 60 ms; 16 k up, 24 k down
Framing: v1 = raw Opus | v2 = 16-byte header | v3 = 4-byte header (NVS-selected, default v1)
Server→client types: tts, stt, llm, asr, mcp, listening, goodbye
Local OTA JSON: {"websocket":{"url":"ws://<hub>:<port>...","token":"","version":1}}
```

### 2.2 OmniBot — the hub pattern

- **One process owns everything** (`app/backend/app.py`, FastAPI `:8000`): device socket
  `/ws/stream`, dashboard `/ws/monitor`, optional `/ws/voice-bridge`.
- **Thin device protocol**: binary `[1 type byte][payload]` in (`0x10` = PCM s16le 16 kHz/2048 B,
  `0x02` = JPEG ≤62 KB @10 fps), JSON text control out. Reconnect every 5 s.
- **Session model**: `active_streams[ws] = {device_id, wake_processor, video ring, coordinator}`
  (`app.py:2712`), per-device asyncio turn lock, device_id inference/binding.
- **Runtime config push on connect** (`app.py:2743-2771`): vision, wake, timezone, sleep → device
  persists to NVS. No device-side REST needed. **Adopt this.**
- **Persona = markdown files + tools** (`persona.py:297-355`): AGENTS/SOUL/IDENTITY/USER/TOOLS/
  MEMORY/HEARTBEAT + daily logs, composed into system instruction; model edits its own memory via
  `soul_replace` / `memory_replace` / `daily_log_append`. **Model-agnostic — survives the LLM swap. Adopt.**
- **Wake word + VAD run on the hub** (openWakeWord + webrtcvad, `wake_listen.py`), 1.2 s pre-roll.
  Our xiaozhi path is better: wake word already on-device, hub only receives bounded utterances.
- **LLM layer is Gemini, 100% cloud, no provider abstraction** — seam list documented
  (`hub_config.py:157-170` client factory, `app.py:2322-2513` REST turn, `live.py:111-887` Live
  session, TTS out PCM 24 kHz). We copy the *shape* of those seams with Leafcutter behind them.
- **Flaws we do not copy**: no auth on `/ws/stream`, always-on mic streaming, Gemini types baked in.
- Already-local pieces worth mirroring: face matching, BLE Wi-Fi provisioning, fastembed cache is a
  **stale artifact** (its router was deleted — do not assume embeddings ship with it).

### 2.3 pi/ (Pathfinder Eye) — the local AI guts

Everything runs on the Pi 5 today, split into localhost services:

| Service | Port | Stack | Role |
|---|---|---|---|
| `go_brain` | 8080 | Go | voice, 3-tier intent router, 8 hardware tools, REST/WS, Dendrite memory |
| `LeafcutterLLM` | 8081 | Rust (Axum) | LLM inference, **OpenAI-compatible `/v1/chat/completions`** |
| Needle 2 | 8082 | — | sub-50 ms fast intent classifier |
| Pocket-TTS | 8020 | — | TTS (piper binary also at `pi/piper/`), espeak-ng fallback |

- **The seam we reuse:** `go_brain/ai.go:106` → `http://localhost:8081/v1/chat/completions`.
  Swap `localhost` for the hub's own address and the LLM layer is done.
- **3-tier intent cascade** (README: `Needle ≥0.70 → rule parser → LLM`): hardware commands like
  *stop* must never wait on a 9B model. **This is the latency backbone — adopt for cynpase-bot.**
- **LeafcutterLLM** (`pi/LeafcutterLLM/README.md`): Rust engine, native SIMD/quantized kernels with
  auto-fallback to llama.cpp FFI, layer-streaming `madvise` (bounded RAM), built-in OpenAI HTTP API,
  no Python/CUDA. Status: production-ready for 2B–9B-class models on modest hardware.
- **Cynapse is already defined** (`pi/LeafcutterLLM/CYNAPSE_INTEGRATION.md`): *Cynapse = orchestration
  layer* (model hub, TUI/API, Dendrite memory), *Leafcutter = inference engine*. **cynpase-bot is the
  robot arm of that ecosystem** — we do not invent a new AI stack, we wire StackChan into it.
- **whisper.cpp in-process** via Go bindings (`voice.go:18`) — no Python ASR daemon needed.
- **Security pattern to adapt:** `http_auth.go:60` refuses non-localhost (403). Ours must accept the
  LAN **but only with a token** — same spirit, wider boundary, never wide open.
- Ops lessons: AI as a separate systemd unit started on demand ("Attention"/"Sleep"), fallback AP when
  WiFi fails (`setup-fallback-ap.sh`), voice/audio policy documented (`AUDIO_POLICY.md`).

### 2.4 sesame-robot — the device contract

- **Proof of the topology:** PC does recognition/sentiment → `POST /api/command
  {"command":...,"face":...}` → ESP32 acts. Fully offline. (`software/README.md:101-110`)
- **Two-channel command schema:** action and expression are separate fields; face-only updates valid
  (`firmware/sesame-firmware-main.ino` `handleApiCommand`). **Adopt as the non-audio device channel.**
- **`talk_*` face variants** (`face-bitmaps.h:34-48`): the robot "speaks" with its face while audio
  streams. StackChan has a face *and* a speaker → show `talk_*` during TTS, clear on `tts end`.
- **Interruptible animations:** `pressingCheck()` polls the network inside every animation frame and
  aborts when the command changes — stop always lands mid-motion. **Rule: never block the socket on
  an animation.**
- **Local-first networking:** `WIFI_AP_STA` dual mode + captive DNS + mDNS (`sesame-robot.local`) →
  findable without cloud or known IP. Runtime tunables via `/setSettings` (no reflash).
- Hardware gotchas: staggered servo writes (`motorCurrentDelay` 20 ms) prevent brownout;
  `ESP32Servo` pinned v3.0.9 (upstream multi-servo bug); ESP32PWM timer exhaustion can kill the
  network stack — documented ceiling.
- **Do not copy:** no auth / no HTTPS (we token-auth), sync `WebServer` single-core loop, the
  example's `recognize_google` cloud call.

---

## 3. Target architecture

```
┌──────────────────────────── PC / Pi — the HUB (cynpase-bot) ────────────────────────────┐
│                                                                                        │
│  [Device layer]   xiaozhi WS server (:8xxx)  ← StackChan speaks this unmodified          │
│                   + local OTA endpoint       → returns ws://<hub>                       │
│                   + JSON action/face channel → sesame-style commands                    │
│  [Session layer]  per-connection session: wake state, utterance buffer, turn lock        │
│  [Intent layer]   3-tier cascade: fast-intent → rules → LLM (Pathfinder pattern)         │
│  [Voice layer]    whisper.cpp STT (in-process) │ Piper/Pocket-TTS → Opus encode (24 kHz) │
│  [Brain layer]    Leafcutter (:8081, OpenAI-compatible) + Dendrite memory + persona/*.md  │
│  [Ops layer]      token auth │ runtime config push │ monitor WS (dashboard) │ logs        │
└──────────────────────────────────────▲──────────────────────────────────────────────────┘
                                       │ LAN: WebSocket (Opus + JSON) — no internet
                        ┌──────────────┴──────────────┐
                        │  StackChan ESP32-S3         │
                        │  wake word (on-device)      │
                        │  Opus codec, speaker, face,  │
                        │  servos, MCP local tools     │
                        │  config-only change: OTA URL │
                        └─────────────────────────────┘
```

**Turn flow (one voice interaction):**

1. On-device wake → device sends `listen/start` + Opus frames.
2. Hub buffers the utterance until `listen/stop` (VAD or device-side endpointing).
3. whisper → text. Tier 1/2 (fast intent/rules) handles commands instantly; else Leafcutter via
   OpenAI API with persona + Dendrite context; tools → device actions (`{"action","expression"}`).
4. Reply text → Piper TTS → PCM 24 kHz → Opus → hub sends `tts` frames; device plays, face switches
   to `talk_*`; dashboard gets transcript + text stream on the monitor WS.
5. `tts end` → face clears → device returns to idle animation.

**Why batch ASR (not streaming):** xiaozhi already gives utterance boundaries
(`listen start/stop`), so we transcribe whole utterances — simpler and lower latency than
streaming. `# ponytail: batch whisper per utterance, upgrade path = streaming whisper if >800 ms`

---

## 4. Reuse vs build

**Reuse as-is (yours already):**

1. **LeafcutterLLM** — LLM server, OpenAI-compatible (drop-in behind the OpenAI client seam).
2. **whisper.cpp** (Go bindings or CLI) — STT. **Piper/Pocket-TTS/espeak** — TTS ladder.
3. **StackChan firmware** — unchanged; only `sdkconfig.defaults.local` + local OTA URL.
4. **OmniBot patterns** — session objects, runtime config push, monitor WS, persona file system.
5. **Pathfinder patterns** — 3-tier cascade, localhost-service split, auth-at-boundary, systemd units.

**Build here (the only genuinely new code):**

1. **xiaozhi-protocol hub server** — WS hello/listen/abort + Opus framing v1 (the missing piece
   neither reference has; StackChan waits for it, OmniBot has no audio downlink).
2. **Local OTA endpoint** — tiny JSON GET so the device discovers the hub.
3. **Pipeline glue** — utterance → whisper → cascade → Leafcutter → TTS → Opus, with tool dispatch.
4. **Action/face channel** — sesame-style `{"action","expression"}` for servos/face/movement +
   interruptible animation handling.
5. **Hub shell** — token auth, runtime config, monitor WS, config file. (Dashboard UI: later, YAGNI
   until the voice loop works.)

**Deliberately NOT built (YAGNI):** mobile app, Go server port, app-store/social features, BLE
provisioning (StackChan already provisions WiFi), streaming ASR, multi-robot orchestration.

---

## 5. Roadmap

Each phase has an observable acceptance outcome. `GATES.md` gets written (CHECK/EXPECT) before
phase 1 implementation starts.

- **Phase 0 — Decisions** (~10 min): hub host, hub language, first model. (Section 6, open items.)
  *Outcome: all three chosen and written into this file.*
- **Phase 1 — The handshake spike** (~half a day): local OTA JSON + xiaozhi WS server that answers
  `hello` and echoes frames; point StackChan's `CONFIG_OTA_URL` at it.
  *Outcome: StackChan connects to the hub on LAN and the session stays up (no MQTT fallback).*
- **Phase 2 — Voice loop** (~1–2 days): utterance → whisper → Leafcutter → Piper → Opus → speaker.
  Text-first, then audio.
  *Outcome: ask a question offline, hear a local-model answer on the robot.*
- **Phase 3 — Personality + tools** (~2–3 days): persona files, Dendrite memory, 3-tier cascade,
  `{"action","expression"}` channel (servos/face), `talk_*` face sync.
  *Outcome: "stop"/"wave" execute in <100 ms without touching the LLM; memory persists across runs.*
- **Phase 4 — Hardening** (~1–2 days): token auth on the device socket, runtime config push,
  monitor WS for a minimal dashboard, reconnect/backoff, logs.
  *Outcome: unauthenticated client refused; hub restart recovers the robot without reflashing.*
- **Phase 5 — Offline proof** (half a day): unplug router, run from AP/fallback, full conversation.
  *Outcome: complete voice interaction with zero internet.*

---

## 6. Open decisions (blocking Phase 1)

1. **Hub host** — this PC (GPU possible, always on?) vs the Pi 5 (known-good, but CPU-bound).
   *Recommendation: PC if it stays on; Leafcutter runs anywhere.*
2. **Hub language** — **Rust** (axum; matches your stack, Leafcutter is Rust) vs **Go** (fastest —
   go_brain already has whisper + cascade + auth we could lift) vs Python (fastest draft, weakest fit).
   *Recommendation: Go for phases 1–2 speed, Rust if you want cynpase-bot to be the Rust flagship.
   Both defensible; pick on feel, not features.*
3. **Model** — Leafcutter GGUF for the hub (Qwen3.5-2B/9B class, per `CYNAPSE_INTEGRATION.md`).
4. **Protocol stance** — *confirmed by default:* implement the xiaozhi server, keep StackChan
   firmware unmodified. Alternative (new OmniBot-style firmware) rejected: more code, loses
   on-device wake + Opus downlink.

**Known risks:** xiaozhi field names for OTA JSON need verification against the vendored tree at
first build (`fetch_repos.py` pulls it; upstream v2.2.4 was our reference); local LLM latency must
stay under the cascade threshold for Tier 3 (tools keep simple commands fast); ESP32PWM/timer
discipline only matters if we drive extra peripherals; NTP only matters if we choose TLS (prefer
plain `ws://` on a trusted LAN + token).

---

## 7. Non-negotiables

- Reference repos stay untouched. Ever.
- Token auth at the device socket (trust boundary — never simplified away).
- Input validation + error handling at every network edge; no secrets in the repo.
- One runnable check for every non-trivial logic piece; `GATES.md` evidence before any "done".
- Local-first means *no internet at runtime* — NTP/OTA/telemetry either local or disabled.
