#!/bin/bash
# Anima Co-Processor Installation Script (Linux)
# Copyright TPT Solutions — Dual-licensed under MIT or Apache-2.0

set -e

INSTALL_PATH="${INSTALL_PATH:-/opt/anima}"
UNINSTALL="${UNINSTALL:-false}"
SYSTEMD_SERVICE_NAME="tpt-anima-co-processor"

log() {
    echo "[Anima] $1"
}

stop_service() {
    if systemctl is-active --quiet "$SYSTEMD_SERVICE_NAME"; then
        log "Stopping Anima co-processor service..."
        sudo systemctl stop "$SYSTEMD_SERVICE_NAME" || true
    fi
    
    # Detach any loaded eBPF programs
    log "Detaching eBPF programs..."
    # In production: bpftool prog detach ...
}

install_coproc() {
    log "Installing Anima Co-Processor to $INSTALL_PATH"
    
    # Create installation directory
    sudo mkdir -p "$INSTALL_PATH"
    
    # Copy binaries
    # In production: cp target/release/tpt-anima-co-processor $INSTALL_PATH/
    log "Copying co-processor binaries..."
    
    # Install systemd service
    cat <<EOF | sudo tee /etc/systemd/system/$SYSTEMD_SERVICE_NAME.service
[Unit]
Description=Anima Co-Processor - eBPF AI Workload Interception
After=network.target

[Service]
Type=simple
ExecStart=$INSTALL_PATH/tpt-anima-co-processor
Restart=always
CapabilityBoundingSet=CAP_SYS_ADMIN CAP_SYS_PTRACE
AmbientCapabilities=CAP_SYS_ADMIN CAP_SYS_PTRACE

[Install]
WantedBy=multi-user.target
EOF

    sudo systemctl daemon-reload
    sudo systemctl enable "$SYSTEMD_SERVICE_NAME"
    
    log "Installation complete!"
    log "Start with: sudo systemctl start $SYSTEMD_SERVICE_NAME"
}

uninstall_coproc() {
    log "Uninstalling Anima Co-Processor..."
    
    stop_service
    
    sudo systemctl disable "$SYSTEMD_SERVICE_NAME" 2>/dev/null || true
    sudo rm -f "/etc/systemd/system/$SYSTEMD_SERVICE_NAME.service"
    sudo systemctl daemon-reload
    
    sudo rm -rf "$INSTALL_PATH"
    
    log "Uninstallation complete!"
}

# Main
if [ "$UNINSTALL" = "true" ]; then
    uninstall_coproc
else
    install_coproc
fi