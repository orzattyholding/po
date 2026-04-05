<#
.SYNOPSIS
    Installs Protocol Orzatty (PO) CLI to the system PATH.
.DESCRIPTION
    Copies po-cli.exe as 'po.exe' to %USERPROFILE%\.po\bin\ and adds
    the directory to the user's PATH environment variable.
    After installation, you can use 'po' from any terminal.
.EXAMPLE
    .\install.ps1
    po listen --port 4433
    po connect 192.168.1.5:4433
    po chat 4433
    po identity
#>

$ErrorActionPreference = "Stop"

# ── Configuration ────────────────────────────────────────────
$BinaryName   = "po.exe"
$SourceBinary = Join-Path $PSScriptRoot "target\release\po-cli.exe"
$InstallDir   = Join-Path $env:USERPROFILE ".po\bin"

# ── Preflight checks ────────────────────────────────────────
Write-Host ""
Write-Host "╔══════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║  Protocol Orzatty (PO) — Installer           ║" -ForegroundColor Cyan
Write-Host "║  orzatty.com                                 ║" -ForegroundColor Cyan
Write-Host "╚══════════════════════════════════════════════╝" -ForegroundColor Cyan
Write-Host ""

if (-not (Test-Path $SourceBinary)) {
    Write-Host "❌ No se encontró el binario compilado." -ForegroundColor Red
    Write-Host "   Esperado en: $SourceBinary" -ForegroundColor Yellow
    Write-Host "   Primero compila con: cargo build --release -p po-cli" -ForegroundColor Yellow
    exit 1
}

# ── Get version ──────────────────────────────────────────────
$Version = & $SourceBinary --version 2>$null
if ($Version) {
    Write-Host "   Versión: $Version" -ForegroundColor Gray
}

$FileSize = [math]::Round((Get-Item $SourceBinary).Length / 1MB, 2)
Write-Host "   Binario: $FileSize MB (release + LTO)" -ForegroundColor Gray
Write-Host ""

# ── Create install directory ─────────────────────────────────
if (-not (Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    Write-Host "📁 Directorio creado: $InstallDir" -ForegroundColor Green
}

# ── Copy binary ─────────────────────────────────────────────
$Destination = Join-Path $InstallDir $BinaryName
Copy-Item -Path $SourceBinary -Destination $Destination -Force
Write-Host "📦 Binario instalado: $Destination" -ForegroundColor Green

# ── Add to PATH if not already present ───────────────────────
$CurrentPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($CurrentPath -notlike "*$InstallDir*") {
    $NewPath = "$CurrentPath;$InstallDir"
    [Environment]::SetEnvironmentVariable("Path", $NewPath, "User")
    Write-Host "🔧 Agregado al PATH del usuario" -ForegroundColor Green

    # Also update current session so it works immediately
    $env:Path = "$env:Path;$InstallDir"
} else {
    Write-Host "✅ Ya está en el PATH" -ForegroundColor Gray
}

# ── Verify installation ─────────────────────────────────────
Write-Host ""
try {
    $InstalledVersion = & $Destination --version 2>$null
    Write-Host "══════════════════════════════════════════════" -ForegroundColor Green
    Write-Host "✅ ¡Instalación exitosa!" -ForegroundColor Green
    Write-Host "══════════════════════════════════════════════" -ForegroundColor Green
    Write-Host ""
    Write-Host "   Ahora puedes usar 'po' desde cualquier terminal:" -ForegroundColor White
    Write-Host ""
    Write-Host "     po identity              — Ver tu identidad de nodo" -ForegroundColor Cyan
    Write-Host "     po listen --port 4433    — Escuchar conexiones" -ForegroundColor Cyan
    Write-Host "     po connect <ip>:4433     — Conectar a un peer" -ForegroundColor Cyan
    Write-Host "     po chat <port|ip:port>   — Chat cifrado P2P" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "   ⚠️  Abre una NUEVA terminal para que el PATH surta efecto." -ForegroundColor Yellow
    Write-Host ""
} catch {
    Write-Host "❌ Error verificando la instalación: $_" -ForegroundColor Red
    exit 1
}
