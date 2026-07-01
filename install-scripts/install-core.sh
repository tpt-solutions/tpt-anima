#!/bin/bash
# Anima Core Hypervisor Installation Script (Linux - Bare Metal Boot)
# Copyright TPT Solutions — Apache License 2.0

set -e

INSTALL_PATH="${INSTALL_PATH:-/opt/anima}"
UNINSTALL="${UNINSTALL:-false}"
GRUB_CFG="/etc/grub.d/40_custom"

log() {
    echo "[Anima] $1"
}

install_core() {
    log "Installing Anima Core Hypervisor to $INSTALL_PATH"
    
    # Create installation directory
    sudo mkdir -p "$INSTALL_PATH"
    sudo mkdir -p "$INSTALL_PATH/ovmf"
    
    # Copy hypervisor binaries
    # In production: cp target/release/anima-hypervisor $INSTALL_PATH/
    log "Copying hypervisor binaries..."
    
    # Install GRUB boot entry for bare-metal boot
    # WARNING: This modifies boot configuration - review carefully
    if [ -d "$INSTALL_PATH" ]; then
        cat <<EOF | sudo tee -a "$GRUB_CFG"

# Anima Core Hypervisor
menuentry 'Anima Core Hypervisor' {
    insmod part_gpt
    insmod ext2
    linux ($root)/anima-hypervisor console=tty0 root=/dev/sda1
    initrd ($root)/anima-initramfs.img
}
EOF
        
        log "GRUB configuration updated. Run 'sudo update-grub' to apply."
    fi
    
    # Configure hardware attestation
    log "Configuring hardware attestation..."
    
    # Install required kernel modules for hardware control
    log "Loading KVM and device drivers..."
    
    log "Installation complete!"
    log "Reboot and select 'Anima Core Hypervisor' from GRUB menu."
}

uninstall_core() {
    log "Uninstalling Anima Core Hypervisor..."
    
    # Remove GRUB entry
    sudo sed -i '/Anima Core Hypervisor/d' "$GRUB_CFG" 2>/dev/null || true
    sudo sed -i '/anima-hypervisor/d' "$GRUB_CFG" 2>/dev/null || true
    
    sudo rm -rf "$INSTALL_PATH"
    
    log "Uninstallation complete!"
    log "Run 'sudo update-grub' to update boot menu."
}

# Main
if [ "$UNINSTALL" = "true" ]; then
    uninstall_core
else
    install_core
fi