#!/usr/bin/env bash
# Project Local Mind — Unified Offline AI Stack Runner
# Starts Ollama, Pocket-TTS, Rust Core Orchestrator, and Local Mind Voice Hub / MQTT Bridge.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
cd "$ROOT_DIR"

echo "=========================================================="
echo "  Project Local Mind — StackChan Offline AI System"
echo "=========================================================="

PIDS=()

cleanup() {
    echo ""
    echo "Shutting down all Local Mind services..."
    for pid in "${PIDS[@]}"; do
        if kill -0 "$pid" 2>/dev/null; then
            kill "$pid" 2>/dev/null || true
        fi
    done
    wait 2>/dev/null || true
    echo "All services stopped."
    exit 0
}

trap cleanup SIGINT SIGTERM EXIT

# 1. Start Ollama if not already running
if ! curl -s http://127.0.0.1:11434/api/tags >/dev/null 2>&1; then
    echo "[1/4] Starting Ollama daemon..."
    ollama serve &
    PIDS+=($!)
    sleep 2
else
    echo "[1/4] Ollama daemon is already running."
fi

# 2. Start Pocket-TTS server on port 8000
if ! curl -s http://127.0.0.1:8000/health >/dev/null 2>&1; then
    echo "[2/4] Starting Pocket-TTS on :8000..."
    uvx pocket-tts serve --port 8000 &
    PIDS+=($!)
    sleep 2
else
    echo "[2/4] Pocket-TTS is already running on :8000."
fi

# 3. Start Python Voice Hub & MQTT Bridge on :8100 and :1883
echo "[3/4] Starting Local Mind Voice Hub & MQTT Bridge (mode=full)..."
uv run python -m hub.mqtt_bridge --mode full &
PIDS+=($!)
sleep 2

# 4. Start Rust Core Orchestrator on :8088
echo "[4/4] Starting Rust Core Orchestrator (:8088)..."
if [ -f "./target/release/localmind-hub" ]; then
    ./target/release/localmind-hub &
else
    ./target/debug/localmind-hub &
fi
PIDS+=($!)

HOST_IP=$(ip route get 1.1.1.1 2>/dev/null | awk '{print $7}' || hostname -I | awk '{print $1}' || echo "127.0.0.1")

echo "=========================================================="
echo "  All Local Mind services are LIVE & READY!"
echo "  - Current Host LAN IP:       $HOST_IP"
echo "  - MQTT Bus & Voice Bridge:   $HOST_IP:1883"
echo "  - WebSocket Server:          ws://$HOST_IP:8100/xiaozhi/v1/"
echo "  - Rust Vision & Gaze API:    http://$HOST_IP:8088"
echo "  - Ollama LLM Engine:         http://127.0.0.1:11434 (qwen2.5:3b-instruct)"
echo "  - Pocket-TTS Engine:         http://127.0.0.1:8000"
echo "=========================================================="
echo "Press Ctrl+C to stop all services."

# Wait for all background processes
wait
