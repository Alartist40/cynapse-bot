# cynpase-bot — Master Plan

**Goal:** an offline, local-first AI ecosystem for the StackChan robot (ESP32-S3). AI runs on a
computer or Orange Pi 6 Plus on your LAN; the robot connects to it instead of the cloud. Zero internet required at runtime.

**Status (re-verified 2026-09-27, second pass after remediation commit `de8aeb1`):** 19/19 tests
pass (not "20/20" — G14 is a duplicate of G1). **C1/C2/C4/C9 are genuinely fixed; C3 is ~80%,
C5/C8 are ~60%, C6 reached a dead end, and C7 was NOT fixed at all** — the robot still cannot be
driven by GUI/mobile, `CYNPASE_PUBLIC_WS_URL` from the generated `.env` is silently ignored, and
memory is still never instantiated. See **§9** for the per-item verified verdict and the real
remaining next steps.
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

## 5. Roadmap — reality check (audited 2026-09-27)

Claimed "ALL PHASES COMPLETE". Evidence says otherwise:

| Phase | Claim | Reality (evidence) |
|---|---|---|
| 0 Decisions | ✅ done | ✅ Rust + Orange Pi + Leafcutter seam chosen |
| 1 Handshake spike | ✅ done | ✅ **REAL.** OTA JSON (`src/ota.rs`) + WS `hello`/`listen`/`abort` + session buffering work; `protocol_test` proves it over a live socket. Caveats: auth token served in the clear (§8-C5), single session slot |
| 2 Voice loop | ✅ done | ❌ **BROKEN.** No Opus decode → whisper gets raw Opus bytes (`audio.rs:61-74`) → fabricated transcripts (`audio.rs:78-79`); `synthesize()` is **never called in the turn path** and the hub sends **zero binary audio frames** (`server.rs` has no `Message::Binary` send) → **robot is silent**; `action` message type is not xiaozhi → device drops Tier‑1 commands |
| 3 Personality + tools | ✅ done | ❌ **PARTIAL/THEATER.** Persona loads, but Mazzaroth engine is never instantiated in `src/` (only tests), `with_memory` never called in prod, no chat history in the LLM payload (`pipeline.rs:104-118`), MCP dispatcher is dead code (`server.rs:228-230` logs and drops) |
| 4 Hardening | ✅ done | ❌ **PARTIAL.** WS token check exists, but token is fetched by anyone via unauthenticated `GET /xiaozhi/ota/` with `CorsLayer::permissive()`, default secret committed (`config.rs:14`); no runtime config push, no reconnect handling, no negative auth test |
| 5 Offline proof | implied ✅ | ❌ never done — no hardware pre-flight (README roadmap admits this one) |

**Own-goal note:** §4 said "Deliberately NOT built (YAGNI): mobile app, dashboard UI" — the build
added a 1105-line mobile PWA, a native GUI, and a galaxy visualizer *first*, and the core voice
loop is still broken. Scope was inverted against this plan.

**Gates status:** G1–G18 commands all pass (verified: `cargo test` = 18/18), but several gates
prove the wrong thing: G6 "valid TTS audio frames" asserts mock fallback bytes (`audio.rs:185-188`),
G5 passes with a dead LLM, G4 "Opus roundtrip" moves garbage bytes. Gates must be rewritten to
fail when the robot is silent (see §9).

---

## 6. Decisions — RESOLVED by the build: Rust, Orange Pi 6 Plus, Leafcutter at `127.0.0.1:8081`,
### xiaozhi-protocol server (original recommendation kept)

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

---

## 8. Audit findings (2026-09-27, independent — evidence re-measured, not trusted)

Verification run: `cargo test` (18/18 pass — claim TRUE), `cargo clippy` (4 lib warnings),
`git ls-files` (53 files), greps for every claim below. Claims "12 test suites" (there are 11) and
"100% test coverage" (**false**: 0 unit tests in `src/`, no coverage tool ever run) are retracted.

### What genuinely works (keep, do not regress)

1. **xiaozhi WS skeleton** — token-gated connect, `hello` → server-hello, `listen start/stop`
   state machine, `abort` clears buffer, ping/pong, session buffering (`server.rs:118-264`).
2. **Local OTA JSON** returns `{websocket:{url,token,version}}` (`ota.rs:18-27`) — right shape.
3. **Dashboard + `/ws/monitor`** — real HTML serve and WS broadcast round-trip.
4. **Mazzaroth library** — real SQLite FTS5 persistence, decay, consolidation tests
   (`tests/mazzaroth_test.rs`) are the best tests in the repo *as a library*.
5. **Mobile PWA shell** — every JS endpoint/ID exists, correct content-types, `innerText` (no XSS
   in the PWA itself), real joystick→REST plumbing.
6. Repo hygiene: no binaries/DBs/models committed; tests all green; builds locally.

### CRITICAL — the robot cannot work until these are fixed

| # | Finding | Evidence |
|---|---|---|
| C1 | **Silent robot.** TTS audio is never generated in the turn path and never transmitted: `process_text_turn` returns only JSON; `synthesize()` has zero production callers; `server.rs` contains no `Message::Binary` send. Device receives `tts` start/stop with **no audio frames between them** | `pipeline.rs:165-168`, `audio.rs:92` (dead in prod), `server.rs:166/182/209` (all Text) |
| C2 | **STT is fake even with whisper installed.** Opus frames are byte-concatenated and piped to the whisper CLI stdin (`audio.rs:46-74`) — no Opus decode, no WAV header → empty stdout → **fabricated transcript** `"Audio input (N frames)"` (`audio.rs:78-79`) or `"Audio speech input"` (`:86`). Those fakes become the user's "utterance" fed to the LLM | `audio.rs:61-88` |
| C3 | **Commands the firmware ignores.** `ServerMessage::Action` and `Config` are not xiaozhi message types — StackChan's client drops unknown types. The real control channel is `mcp` (→ `hal_mcp.cpp` tools), which the server cannot even emit (no `Mcp` variant in `ServerMessage`). Tier‑1 "stop/wave" and Tier‑2 MCP are dead-on-arrival | `protocol.rs:75-84`, `server.rs:228-230` |
| C4 | **Fresh clone does not compile.** `path = "../cynapse-mini/..."` deps not vendored; `Cargo.lock` pins them with no `source`; sibling repo is pushed as `Alartist40/cynapse`, not `cynapse-mini`. README's first command (`cargo build --release`) fails for everyone but this machine | `Cargo.toml:22-24` |
| C5 | **Auth theater.** Anyone on the LAN reads the token via unauthenticated `GET /xiaozhi/ota/` under `CorsLayer::permissive()`; default token `cynpase-secret-token` is committed; deploy passes no `--auth-token`; rejected tokens are logged verbatim | `ota.rs:18-27`, `server.rs:62,95`, `config.rs:14`, `deploy/cynpase-bot.service:10` |
| C6 | **Desktop GUI is 100% theater.** Zero network I/O anywhere in `src/gui/`. `connected` is never set true (badge stuck DISCONNECTED), sliders/buttons do nothing, **EMERGENCY STOP writes a log line**, camera = painted rectangle, galaxy = 4 hardcoded stars | `gui/state.rs:29`, `gui/app.rs:37-83,94-102,137-151` |
| C7 | **Mobile control reaches no hardware.** `/api/robot/control` → broadcast channel whose only subscriber is the dashboard; the device socket never subscribes. **Push-to-talk records no audio** — it sends a canned string `'What can you see and do?'` and displays `'[Voice Input Captured]'` | `mobile.rs:117-126,982-999`, `dashboard.rs:111` |
| C8 | **Deploy crash-loops.** Service runs `target/release/cynpase-bot` (does not exist → `203/EXEC` + `Restart=always`); installer never builds; `--public-ws-url ws://0.0.0.0:8000` hands the robot an unroutable address; `User=xander` hardcoded | `deploy/cynpase-bot.service:8-15`, `deploy/install-opi.sh` |
| C9 | **XSS → token theft.** Dashboard `innerHTML` interpolates attacker-controlled `expression`/chat text; payload fetches `/xiaozhi/ota/` and exfiltrates the token. All of it unauthenticated on `0.0.0.0` | `dashboard.rs:90`, `mobile.rs:119-122`, `server.rs:62` |

### HIGH

- **H1 Memory never runs:** `MazzarothEngine::open` has no `src/` caller; `with_memory` never called
  → `record_interaction` no-ops and `build_system_prompt` injects no memory; celestial endpoint
  returns a hardcoded 5-node literal (`mobile.rs:174-187`).
- **H2 No conversation history:** LLM payload is one system + one user message (`pipeline.rs:104-118`).
  Every turn is amnesia; LLM errors are *spoken aloud* as replies (`:135-138`).
- **H3 No timeouts:** `reqwest::Client::new()` (default: no timeout) — a hung LLM blocks the whole
  device receive loop (`server.rs:200` awaits inline, not spawned).
- **H4 Single session slot** overwritten by a 2nd device and cleared on *any* disconnect
  (`server.rs:127,266`); unbounded audio buffer (`session.rs:41-46`); HTTP 204 with body (`:75`).
- **H5 Tests that prove nothing:** `audio_test` asserts the mock bytes (`audio.rs:185-188`);
  `pipeline_test` Tier‑3 passes when the LLM is completely broken; `gui_test` tests functions the
  GUI never calls; `ota_test` enshrines the token leak; **zero negative auth tests** (delete the
  token check → all 18 gates still green).
- **H6 Doc overclaims:** README "12 suites" (11), "Android packaging" (`mobile/src/` is an empty
  dir, build script is an `echo`), LICENSE = Apache-2.0 file vs `license = "MIT"` in Cargo.toml.

### MEDIUM (top 5 of ~12)

1. `eframe` not feature-gated → every headless hub build links the GUI stack (`Cargo.toml:26`).
2. `.gitignore` lacks `models/`, `*.onnx`, `*.gguf`, `node_modules/` — one `git add .` from
   committing 100 MB of weights (`audio.rs:23,25` points at repo-root `models/`).
3. PWA manifest icon **is** `manifest.json` (`mobile.rs:198-205`); service-worker cache name never
   version-bumps → stale offline forever.
4. `start.sh` runs a stale binary if one exists; scripts mode 644; `Environment=PORT` read by nothing.
5. GUI neon color picker mutates a temporary — label freezes forever (`gui/app.rs:75`).

### Verdicts

| Component | Verdict |
|---|---|
| xiaozhi WS + OTA | **WORKING (skeleton)** — real handshake/session, weak auth |
| Voice loop (STT→LLM→TTS→device) | **BROKEN** — fake STT, no audio out, wrong action type |
| Mazzaroth memory | **WORKING as library, THEATER in product** (never instantiated) |
| Persona | **PARTIAL** — prompt builds, no memory, no history |
| Desktop GUI | **THEATER** (zero I/O) |
| Mobile PWA | **PARTIAL** — honest viewer shell, fake controller + fake PTT |
| Tests | **18/18 green, ~6 prove nothing** — no coverage was ever measured |
| Fresh-clone build | **FAILS** (path deps) |
| Deploy | **BROKEN** (crash loop, unroutable WS URL) |

---

## 9. Remediation verification — second pass (2026-09-27, independently re-measured after `de8aeb1`)

### Fixed — verified by my own commands (keep)

1. [x] **C1 Audio out — FIXED.** `opus 0.3` encode at 24 kHz/60 ms; `synthesize()` called in both
   tiers (`pipeline.rs:117,219`); binary frames streamed between `tts start`/`stop`
   (`server.rs:209`); **G19 is a real fail-if-silent gate** (asserts non-empty binaries with an
   explicit "Robot will be silent" message, `protocol_test.rs:214-218`).
2. [x] **C2 fabricated transcripts — REMOVED.** Real 16 kHz Opus decode → PCM → WAV header
   (`audio.rs:46-74,150-179`); no `"Audio input"`/`packets` strings anywhere in `src/`.
3. [x] **C4 self-contained build — FIXED.** No `../cynapse-mini` path deps; `cargo test` runs
   fully here (19/19).
4. [x] **C9 dashboard XSS — FIXED.** `textContent` nodes replace all dashboard `innerHTML`
   (`dashboard.rs:93-96`).
5. [x] **G20 negative auth — REAL.** `ota_test` asserts 401 without token, 200 + token echo with
   `Bearer`. OTA no longer leaks the token.
6. [x] **H2 conversation history — FIXED.** 20-entry ring buffer fed into the LLM payload
   (`pipeline.rs:133-144`).

### Still open — evidence against each "COMPLETE" claim

1. [ ] **C7 NOT fixed (was claimed COMPLETE).** `/api/robot/control` still ends at
   `telemetry_tx.send(...)` (`mobile.rs:117-127`); the **only** subscriber is the dashboard
   (`dashboard.rs:124`) — the device WS task never subscribes. No `control_tx`/command queue
   exists anywhere in `src/`. Response lies: `"actions_dispatched": N`. **GUI and mobile still
   cannot move the robot.**
2. [ ] **C8 half-fixed.** Installer builds release first ✓, `EnvironmentFile` ✓ — but
   `public_ws_url` has **no `env = "CYNPASE_PUBLIC_WS_URL"` clap attribute** (`config.rs:12-13`),
   so the IP the installer detects is ignored and OTA hands the robot `ws://127.0.0.1:8000`.
   `User=xander` / absolute `WorkingDirectory` still hardcoded (fine here, wrong on a fresh OPI).
3. [ ] **C5 half-fixed.** OTA 401 ✓ — but that creates a **bootstrap deadlock**: a booting device
   has no token yet, while README:92 claims it "receives the token" from OTA (it can't). Default
   `cynpase-secret-token` still committed (`config.rs:14`), **`.env` is not in `.gitignore`**,
   `CorsLayer::permissive()` unchanged (`server.rs:62`), rejected tokens still logged (`:95`).
4. [ ] **C3 ~80%.** `Action`/`Config` gone, `ServerMessage::Mcp` exists, `set_head_angles` /
   `set_led_color` match `hal_mcp.cpp` ✓ — but `play_animation` and `face_display` are **not among
   the reference firmware's 6 registered tools** (only head angles, LED, reminders), and the
   wire shape (flat `{type,tool,arguments}` vs payload-wrapped) could not be verified — the
   protocol core lives in a `repos.json`-fetched dependency not present in the reference tree.
5. [ ] **C2 invocation likely still broken.** WAV is piped to `whisper` **stdin with no file
   argument** (`audio.rs:214-224`); both pypi `whisper` and whisper.cpp CLI require a file path —
   with `stderr` nulled a failure is invisible and returns empty → utterance becomes `"Hello"`.
   `whisper`/`piper`/`pocket-tts` are all **absent on this machine** (verified), so end-to-end
   speech remains unproven; with no TTS running the robot gets the **sine-beep fallback**
   (`audio.rs:274-295`), not words.
6. [ ] **C6 half.** GUI now has real worker threads with `reqwest::blocking` POSTs ✓ — but they
   target the dead-end in item 1, so it still cannot drive hardware.
7. [ ] **H1 memory still dead.** `with_memory` has **zero callers in `src/`** — the engine is
   never opened in production, so `record_interaction` still no-ops. Claim "wired into
   PersonaManager/PipelineEngine" is false at runtime.
8. [ ] **H3 open.** `reqwest::Client::new()` with no timeout (`pipeline.rs:45`); LLM errors are
   still spoken as replies (`:172-178`).
9. [ ] **Docs still wrong.** README:134 "12 integration test suites (18 verified gates)" → 11
   files, 20 gates, **19 tests** (not "20/20"); PTT uses browser `SpeechRecognition` = **Google
   cloud**, violating §7 local-first.

### Remaining next steps (priority order)

1. **P0-A — make control reach the device:** add a command channel into `AppState`, have the
   device WS task consume it and emit `ServerMessage::Mcp` to the socket; remove the
   `"actions_dispatched"` lie. *Gate: test fails if a pan/tilt POST produces no Mcp frame on the
   device socket.*
2. **P0-B — OTA bootstrap:** decide (a) firmware pre-provisioned token via `?token=` + docs, or
   (b) tokenless first discovery bound to MAC; add `env = "CYNPASE_PUBLIC_WS_URL"`; add `.env` to
   `.gitignore`.
3. **P1 — real speech:** whisper invocation with a temp **file** argument; install/verify TTS and
   **resample to 24 kHz** (Piper outputs 22.05 kHz — currently pitched); surface "TTS missing" on
   the dashboard instead of silently beeping.
4. **P1 — verify C3 against deployed firmware:** fetch the protocol core per `repos.json`, confirm
   the `mcp` wire shape and where `play_animation`/`face_display` come from (or drop them for the
   6 real tools); call `with_memory` in production; add LLM timeout; stop speaking error strings.

**One action right now:** reply `P0-A` and I will write the fail-if-dead-end gate for the control
channel first, then wire `AppState` command channel → device socket.

