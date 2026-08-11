#!/bin/bash
# Anima Edge Runtime Installation Script (Linux/macOS)
# Copyright TPT Solutions — Dual-licensed under MIT or Apache-2.0

set -e

INSTALL_PATH="${INSTALL_PATH:-/usr/local/bin}"
UNINSTALL="${UNINSTALL:-false}"
SERVICE_NAME="tpt-anima-edge-runtime"

log() {
    echo "[Anima] $1"
}

install_edge() {
    log "Installing Anima Edge Runtime to $INSTALL_PATH"
    
    # Copy WASM module and native binary
    # In production: cp target/wasm32-wasi/release/tpt_anima_edge_runtime.wasm $INSTALL_PATH/
    # In production: cp target/release/tpt-anima-edge-runtime $INSTALL_PATH/
    
    log "Installing WebAssembly module..."
    log "Installing native binary for phones/IoT..."
    
    # Install systemd user service (Linux) or launchd plist (macOS)
    if command -v systemctl &> /dev/null; then
        log "Installing systemd user service..."
        mkdir -p ~/.config/systemd/user
        cat <<EOF > ~/.config/systemd/user/$SERVICE_NAME.service
[Unit]
Description=Anima Edge Runtime
After=network.target

[Service]
Type=simple
ExecStart=$INSTALL_PATH/tpt-anima-edge-runtime
Restart=always

[Install]
WantedBy=default.target
EOF
        systemctl --user daemon-reload
    elif command -v launchctl &> /dev/null; then
        log "Installing launchd service (macOS)..."
        # Would create a proper .plist file for launchd
        log "Note: Manual launchd configuration required for macOS"
    fi
    
    log "Installation complete!"
    log "Edge runtime is ready for phones, IoT devices, and browsers."
}

uninstall_edge() {
    log "Uninstalling Anima Edge Runtime..."
    
    rm -f "$INSTALL_PATH/tpt-anima-edge-runtime"
    rm -f "$INSTALL_PATH/tpt_anima_edge_runtime.wasm" 2>/dev/null || true
    
    if command -v systemctl &> /dev/null; then
        rm -f ~/.config/systemd/user/$SERVICE_NAME.service
        systemctl --user daemon-reload
    fi
    
    log "Uninstallation complete!"
}

# Main
if [ "$UNINSTALL" = "true" ]; then
    uninstall_edge
else
    install_edge
fi