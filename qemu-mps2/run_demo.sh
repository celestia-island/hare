#!/usr/bin/env bash
# Launch the hare QEMU demo end-to-end:
#   1. Build the kei-qemu-mps2 firmware for thumbv7em-none-eabi
#   2. Build the host_gateway example for the host
#   3. Start QEMU with the firmware, piping UART to the gateway
#
# Usage: ./run_demo.sh
#
# Requires: qemu-system-arm, rustup target thumbv7em-none-eabi
set -euo pipefail

cd "$(dirname "$0")"

# The workspace target dir lives at the repository root, not next to this member.
TARGET_DIR="$(cd .. && pwd)/target"

echo "=== Building firmware (thumbv7em-none-eabi) ==="
cargo build --release -p kei-qemu-mps2 --target thumbv7em-none-eabi 2>&1 | tail -3

ELF="$TARGET_DIR/thumbv7em-none-eabi/release/kei-qemu-mps2"

if [ ! -f "$ELF" ]; then
    echo "ERROR: firmware not found at $ELF"
    exit 1
fi

echo "=== Building host gateway ==="
cargo build -p kei-qemu-mps2 --example host_gateway --target x86_64-unknown-linux-gnu 2>&1 | tail -3

GW="$TARGET_DIR/x86_64-unknown-linux-gnu/debug/examples/host_gateway"

if [ ! -f "$GW" ]; then
    echo "ERROR: host gateway not found at $GW"
    exit 1
fi

echo "=== Starting QEMU + gateway ==="
echo "Firmware: $ELF"
echo "Gateway:  $GW"
echo ""

# QEMU UART0 → stdout, gateway reads stdin.
# The firmware sends both human-readable debug lines AND binary wire frames.
# The gateway's FrameDecoder will skip non-frame bytes (no 0xCE magic).
qemu-system-arm \
    -M mps2-an386 \
    -cpu cortex-m4 \
    -m 4M \
    -nographic \
    -semihosting \
    -serial stdio \
    -kernel "$ELF" 2>&1 | "$GW"
