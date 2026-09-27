#!/usr/bin/env bash
set -e

# Installer for cynpase-bot systemd service on Orange Pi 6 Plus
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "==> Building cynpase-bot release binary..."
cd "$ROOT_DIR"
cargo build --release --bin cynpase-bot

if [ ! -f "$ROOT_DIR/target/release/cynpase-bot" ]; then
    echo "Error: Release binary build failed!"
    exit 1
fi

echo "==> Detecting host network IP..."
PRIMARY_IP=$(hostname -I 2>/dev/null | awk '{print $1}')
if [ -z "$PRIMARY_IP" ]; then
    PRIMARY_IP="127.0.0.1"
fi
echo "Host IP detected: $PRIMARY_IP"

ENV_FILE="$ROOT_DIR/.env"
if [ ! -f "$ENV_FILE" ]; then
    echo "==> Generating initial .env with secure token..."
    TOKEN=$(head -c 16 /dev/urandom | od -An -tx1 | tr -d ' \n')
    cat <<EOF > "$ENV_FILE"
CYNPASE_AUTH_TOKEN=$TOKEN
CYNPASE_PUBLIC_WS_URL=ws://${PRIMARY_IP}:8000/xiaozhi/ws
CYNPASE_LEAFCUTTER_URL=http://127.0.0.1:8081/v1/chat/completions
CYNPASE_MODEL_NAME=qwen2.5-7b
EOF
    echo "Generated $ENV_FILE with token."
fi

echo "==> Installing systemd service..."
sudo cp "$SCRIPT_DIR/cynpase-bot.service" /etc/systemd/system/cynpase-bot.service
sudo systemctl daemon-reload
sudo systemctl enable cynpase-bot.service

echo "==> cynpase-bot service successfully installed and enabled."
echo "Commands:"
echo "  Start:   sudo systemctl start cynpase-bot"
echo "  Status:  sudo systemctl status cynpase-bot"
echo "  Logs:    journalctl -u cynpase-bot -f"

