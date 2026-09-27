#!/usr/bin/env bash
set -e

# cynpase-bot Hub Launcher for Orange Pi 6 Plus
echo "================================================="
echo "   🤖 Starting cynpase-bot Local Hub on OPi 6+   "
echo "================================================="

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT_DIR"

export RUST_LOG=info,cynpase_bot=debug,tower_http=info

# Build release binary if not present
if [ ! -f "$ROOT_DIR/target/release/cynpase-bot" ]; then
    echo "Building release binary..."
    cargo build --release
fi

exec "$ROOT_DIR/target/release/cynpase-bot" --bind-addr 0.0.0.0:8000 "$@"
