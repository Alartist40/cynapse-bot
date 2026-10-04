# Phase 0 Reconnaissance Findings: StackChan CoreS3

Document findings from initial physical device inspection, firmware backups, and network captures.

---

## 1. Device & Network Baseline
- **Robot Model**: M5Stack StackChan (CoreS3 SKU K151)
- **Robot MAC Address**: `TBD`
- **Robot Assigned IP**: `TBD`
- **Host Static IP**: `192.168.1.50` (or `192.168.50.1` on localmind hotspot)

---

## 2. Firmware Safety Net & Backups
- [ ] **M5Burner Baseline Image Downloaded**: `[Yes/No] (Path: ...)`
- [ ] **esptool Full Flash Dump (16MB)**:
  ```bash
  esptool.py --chip esp32s3 --port /dev/ttyUSB0 read_flash 0x0 0x1000000 docs/flash_backup.bin
  ```
  - Backup Status: `[Pending/Verified]`
- [ ] **Boot Serial Log Captured (`docs/phase0-boot.log`)**:
  ```bash
  # Watch boot sequence and WebSocket connection logs @ 115200 baud
  picocom -b 115200 /dev/ttyUSB0 | tee docs/phase0-boot.log
  ```
  - Serial Log Status: `[Pending/Captured]`

---

## 3. WebSocket Endpoint Configurability (The Key Question)
- **Is the XiaoZhi WebSocket URL configurable without a full reflash?**
  - [ ] **Path A**: Configurable via mobile app settings or NVS key `websocket.url`.
  - [ ] **Path B**: Fixed in stock build, but reflashable with community firmware ([eid390](https://github.com/eid390/stackchan-xiaozhi-firmware) / Kconfig override).
  - [ ] **Path C**: Fixed in stock build, requires official `m5stack/StackChan` ESP-IDF source patch.
- **Chosen Connection Path**: `[Path A / Path B / Path C]`

---

## 4. Handshake & Protocol Observations
- **Stock WebSocket URI Dialed by Device (from serial log)**: `[e.g. wss://api.xiaozhi.me/v1/]`
- **Target Local Hub URI**: `ws://<host-ip>:8100/xiaozhi/v1/`
- **Handshake Version & Fields**:
  - `hello` client payload captured to `tests/fixtures/hello_client.json`: `[Pending/Captured]`
  - `listen` start/stop payload captured: `[Pending/Captured]`
  - OTA check payload captured: `[Pending/Captured]`

---

## 5. Hardware Servo Kinematics Check
- **Pan Servo (X-axis)**:
  - Neutral pulse width: `1500 µs`
  - Operating Mode: `[Standard Positional 0-180° / Continuous Rotation Velocity]`
- **Tilt Servo (Y-axis)**:
  - Positional standard servo verified with hard software clamp: `5.0° to 85.0°`
