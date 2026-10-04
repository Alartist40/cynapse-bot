# Phase 0 Reconnaissance Findings: StackChan CoreS3

Document findings from physical device inspection, firmware backups, NVS parsing, and network captures.

---

## 1. Device & Network Baseline
- **Robot Model**: M5Stack StackChan (CoreS3 SKU K151)
- **Chip Type**: ESP32-S3 (QFN56) revision v0.2, 240MHz, 8MB PSRAM, 16MB Flash
- **Robot MAC Address**: `7c:4f:ad:af:82:64`
- **Robot UUID**: `4a4abe6f-88de-435d-961a-34c05f0de569`
- **Firmware Version**: Official `stack-chan v1.5.1` (ESP-IDF v5.5.4, Mooncake v2.3.3, build Jul 31 2026)
- **Host LAN IP**: `192.168.10.129` (`wlan0`)

---

## 2. Firmware Safety Net & Backups
- [x] **M5Burner Baseline Image Downloaded**: Official stack-chan v1.5.1
- [x] **esptool Full Flash Dump (16MB)**:
  - Saved to: `docs/flash_backup.bin` (16,777,216 bytes verified)
- [x] **Boot Serial Log Captured (`docs/phase0-boot.log`)**:
  - Verified hardware: SCS Serial Bus Servos (ID 1 zero: 469, ID 2 zero: 579), GC0308 Camera (PID 0x9b), ILI9342C LCD (LVGL 2MB PSRAM cache), Si12T Head Touch, BMI270 IMU, PCF8563 RTC, AXP2101 PMIC, AW9523 IO expander.

---

## 3. WebSocket Endpoint Configurability (The Key Question)
- **Is the XiaoZhi WebSocket URL configurable without a full reflash?**
  - [x] **Path A (CONFIRMED)**: Stored in NVS partition (offset `0x9000`, 16KB) under namespace `websocket`, key `url`.
  - [ ] Path B: Not needed.
  - [ ] Path C: Not needed.
- **Chosen Connection Path**: **Path A (NVS URL update to `ws://192.168.10.129:8100/xiaozhi/v1/`)**

---

## 4. Handshake & Protocol Observations
- **Stock WebSocket URI Dialed by Device**: `wss://api.tenclass.net/xiaozhi/v1/` (token: `test-token`)
- **Target Local Hub URI**: `ws://192.168.10.129:8100/xiaozhi/v1/`
- **Stock MQTT Endpoint**: `api.tenclass.net:1883`

---

## 5. Hardware Servo Kinematics Check
- **Servo Type**: **SCS Serial Bus Servos** (Feetech / UART servo bus on GPIO with ID 1 & ID 2), zero calibration offsets loaded from NVS (`zero_pos_1: 469`, `zero_pos_2: 579`).
- **Tilt Servo (ID 2)**: Hard clamped in Local Mind tracker to $[5.0^\circ, 85.0^\circ]$.
- **Pan Servo (ID 1)**: Serial bus position/angle controllable.
