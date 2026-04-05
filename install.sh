#!/usr/bin/env bash
# ──────────────────────────────────────────────────────────────
# Protocol Orzatty (PO) — Linux/macOS Installer
# Copies the CLI binary to ~/.local/bin/po and ensures it's on PATH.
# Usage: chmod +x install.sh && ./install.sh
# ──────────────────────────────────────────────────────────────

set -euo pipefail

BINARY_NAME="po"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SOURCE_BINARY="$SCRIPT_DIR/target/release/po-cli"
INSTALL_DIR="$HOME/.local/bin"

echo ""
echo "╔══════════════════════════════════════════════╗"
echo "║  Protocol Orzatty (PO) — Installer           ║"
echo "║  orzatty.com                                 ║"
echo "╚══════════════════════════════════════════════╝"
echo ""

# ── Preflight checks ────────────────────────────────────────
if [ ! -f "$SOURCE_BINARY" ]; then
    echo "❌ No se encontró el binario compilado."
    echo "   Esperado en: $SOURCE_BINARY"
    echo "   Primero compila con: cargo build --release -p po-cli"
    exit 1
fi

FILE_SIZE=$(du -h "$SOURCE_BINARY" | cut -f1)
echo "   Binario: $FILE_SIZE (release + LTO)"
echo ""

# ── Create install directory ─────────────────────────────────
mkdir -p "$INSTALL_DIR"

# ── Copy binary ─────────────────────────────────────────────
DESTINATION="$INSTALL_DIR/$BINARY_NAME"
cp "$SOURCE_BINARY" "$DESTINATION"
chmod +x "$DESTINATION"
echo "📦 Binario instalado: $DESTINATION"

# ── Add to PATH if not already present ───────────────────────
add_to_path() {
    local shell_rc="$1"
    local path_line="export PATH=\"\$HOME/.local/bin:\$PATH\""

    if [ -f "$shell_rc" ]; then
        if ! grep -q '.local/bin' "$shell_rc" 2>/dev/null; then
            echo "" >> "$shell_rc"
            echo "# Protocol Orzatty (PO)" >> "$shell_rc"
            echo "$path_line" >> "$shell_rc"
            echo "🔧 Agregado al PATH en $(basename "$shell_rc")"
        fi
    fi
}

# Detect shell and add to appropriate rc file
if echo "$PATH" | grep -q "$INSTALL_DIR"; then
    echo "✅ Ya está en el PATH"
else
    add_to_path "$HOME/.bashrc"
    add_to_path "$HOME/.zshrc"
    export PATH="$INSTALL_DIR:$PATH"
fi

# ── Verify installation ─────────────────────────────────────
echo ""
if "$DESTINATION" --version >/dev/null 2>&1; then
    VERSION=$("$DESTINATION" --version 2>/dev/null || true)
    echo "══════════════════════════════════════════════"
    echo "✅ ¡Instalación exitosa! $VERSION"
    echo "══════════════════════════════════════════════"
    echo ""
    echo "   Ahora puedes usar 'po' desde cualquier terminal:"
    echo ""
    echo "     po identity              — Ver tu identidad de nodo"
    echo "     po listen --port 4433    — Escuchar conexiones"
    echo "     po connect <ip>:4433     — Conectar a un peer"
    echo "     po chat <port|ip:port>   — Chat cifrado P2P"
    echo ""
    echo "   ⚠️  Ejecuta 'source ~/.bashrc' o abre una nueva terminal."
    echo ""
else
    echo "❌ Error verificando la instalación."
    exit 1
fi
