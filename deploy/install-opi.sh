#!/usr/bin/env bash
set -e

# Installer for cynpase-bot systemd service on Orange Pi
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "Installing cynpase-bot service..."
sudo cp "$SCRIPT_DIR/cynpase-bot.service" /etc/systemd/system/cynpase-bot.service
sudo systemctl daemon-reload
sudo systemctl enable cynpase-bot.service

echo "cynpase-bot service installed and enabled."
echo "Commands:"
echo "  Start:   sudo systemctl start cynpase-bot"
echo "  Status:  sudo systemctl status cynpase-bot"
echo "  Logs:    journalctl -u cynpase-bot -f"
