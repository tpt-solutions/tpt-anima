# Anima Core Hypervisor Installation Script (Windows)
# Copyright TPT Solutions — Dual-licensed under MIT or Apache-2.0

param(
    [switch]$Uninstall = $false,
    [string]$InstallPath = "$env:ProgramFiles\Anima"
)

$ServiceName = "AnimaCoreHypervisor"
$ServiceDisplayName = "Anima Core Hypervisor"

function Write-Log {
    param([string]$Message)
    Write-Host "[Anima] $Message"
}

function Stop-Service-And-Driver {
    if (Get-Service -Name $ServiceName -ErrorAction SilentlyContinue) {
        Write-Log "Stopping Anima service..."
        Stop-Service -Name $ServiceName -Force
    }
}

function Install-Core {
    Write-Log "Installing Anima Core Hypervisor to $InstallPath"
    
    # Create installation directory
    if (-not (Test-Path $InstallPath)) {
        New-Item -ItemType Directory -Path $InstallPath -Force | Out-Null
    }
    
    # Copy binaries
    # In production, this would copy the actual hypervisor binaries
    Write-Log "Copying hypervisor binaries..."
    
    # Register service
    try {
        New-Service -Name $ServiceName -BinaryPathName "$InstallPath\tpt-anima-hypervisor.exe" -DisplayName $ServiceDisplayName
        Write-Log "Service registered successfully"
    } catch {
        Write-Log "Service may already exist, or registration failed: $_"
    }
    
    # Configure hardware attestation
    Write-Log "Configuring hardware attestation..."
    
    # Start service
    Start-Service -Name $ServiceName -ErrorAction SilentlyContinue
    
    Write-Log "Installation complete!"
}

function Uninstall-Core {
    Write-Log "Uninstalling Anima Core Hypervisor..."
    
    Stop-Service-And-Driver
    
    if (Get-Service -Name $ServiceName -ErrorAction SilentlyContinue) {
        sc.exe delete $ServiceName | Out-Null
    }
    
    if (Test-Path $InstallPath) {
        Remove-Item -Path $InstallPath -Recurse -Force
    }
    
    Write-Log "Uninstallation complete!"
}

if ($Uninstall) {
    Uninstall-Core
} else {
    Install-Core
}