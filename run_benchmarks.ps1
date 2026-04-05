$ErrorActionPreference = "Continue"

Write-Host "=============================================" -ForegroundColor Cyan
Write-Host "🚀 CONSTRUYENDO BINAROS DE ORZATTY PROTOCOL (PO) " -ForegroundColor Cyan
Write-Host "=============================================" -ForegroundColor Cyan

# Cerrar rust-analyzer si está molestando con los locks
Stop-Process -Name rust-analyzer -ErrorAction SilentlyContinue

Write-Host "`n[1/3] Compilando CLI (.exe) de producción..." -ForegroundColor Yellow
cargo build --release -p po-cli
if ($LASTEXITCODE -eq 0) {
    Write-Host "¡CLI Compilado con éxito! -> target/release/po-cli.exe" -ForegroundColor Green
} else {
    Write-Host "Error compilando CLI." -ForegroundColor Red
}

Write-Host "`n[2/3] Corriendo Benchmarks Reales (PO vs WebSocket)..." -ForegroundColor Yellow
# Un loop por si el rust-analyzer vuelve a molestar con el os error 32
$retryCount = 0
while ($retryCount -lt 5) {
    cargo run --release -p po-bench
    if ($LASTEXITCODE -eq 0) {
        break
    }
    $retryCount++
    Start-Sleep -Seconds 2
}

Write-Host "`n[3/3] ¿Cómo compilar para Linux desde Windows?" -ForegroundColor Yellow
Write-Host "Para los servidores en Ubuntu/Debian, lo más pro y limpio es usar WSL2."
Write-Host "En tu WSL (Ubuntu), ve a la ruta de Mnt y corre:"
Write-Host "  $ cargo build --release -p po-cli"
Write-Host "Eso generará el binario ELF nativo de Linux en la misma carpeta target/release."
Write-Host "`n¡Todo listo, jefe!" -ForegroundColor Green
