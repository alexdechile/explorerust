#!/usr/bin/env bash
# ExploreRust — Script de instalación
# Uso: ./install.sh [--prefix /usr/local]

set -euo pipefail

PREFIX="${1:-/usr/local}"
BIN_DIR="$PREFIX/bin"
SHARE_DIR="$PREFIX/share"
DESKTOP_DIR="$SHARE_DIR/applications"
ICON_DIR="$SHARE_DIR/icons/hicolor/32x32/apps"

BINARY="./target/release/explorerust"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# ── Colores ─────────────────────────────────────────────────────────────────
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
CYAN='\033[0;36m'; BOLD='\033[1m'; NC='\033[0m'

info()    { echo -e "${CYAN}[*]${NC} $*"; }
ok()      { echo -e "${GREEN}[✓]${NC} $*"; }
warn()    { echo -e "${YELLOW}[!]${NC} $*"; }
error()   { echo -e "${RED}[✗]${NC} $*" >&2; exit 1; }

echo -e "${BOLD}"
echo "  ╔═══════════════════════════════════╗"
echo "  ║       ExploreRust Installer       ║"
echo "  ╚═══════════════════════════════════╝"
echo -e "${NC}"

cd "$SCRIPT_DIR"

# ── Verificar si hay binario pre-compilado ──────────────────────────────────
if [[ ! -f "$BINARY" ]]; then
    warn "No se encontró binario pre-compilado. Compilando desde fuente…"
    if ! command -v cargo &>/dev/null; then
        error "Cargo no encontrado. Instala Rust desde: https://rustup.rs"
    fi
    info "Compilando en modo release (puede tardar ~4 minutos en la primera vez)…"
    cargo build --release
    ok "Compilación exitosa."
fi

# ── Detectar si necesita sudo ───────────────────────────────────────────────
NEED_SUDO=""
if [[ ! -w "$BIN_DIR" ]] 2>/dev/null || [[ ! -d "$BIN_DIR" ]]; then
    NEED_SUDO="sudo"
    warn "Se requieren permisos de administrador para instalar en $PREFIX"
fi

# ── Instalar binario ────────────────────────────────────────────────────────
info "Instalando binario en $BIN_DIR/explorerust …"
$NEED_SUDO install -Dm755 "$BINARY" "$BIN_DIR/explorerust"
ok "Binario instalado."

# ── Instalar icono ──────────────────────────────────────────────────────────
if [[ -f "assets/icon.png" ]]; then
    info "Instalando icono…"
    $NEED_SUDO install -Dm644 "assets/icon.png" "$ICON_DIR/explorerust.png"
    ok "Icono instalado."
fi

# ── Crear archivo .desktop ──────────────────────────────────────────────────
DESKTOP_FILE="/tmp/explorerust.desktop"
cat > "$DESKTOP_FILE" <<EOF
[Desktop Entry]
Version=1.0
Type=Application
Name=ExploreRust
GenericName=File Manager
Comment=A modern dual-interface file explorer written in Rust
Exec=explorerust %F
Icon=explorerust
Categories=System;FileManager;
Keywords=file;manager;explorer;rust;
MimeType=inode/directory;
StartupNotify=true
StartupWMClass=explorerust
EOF

info "Instalando entrada de escritorio…"
$NEED_SUDO install -Dm644 "$DESKTOP_FILE" "$DESKTOP_DIR/explorerust.desktop"
rm -f "$DESKTOP_FILE"
ok "Entrada de escritorio instalada."

# ── Actualizar caché de iconos ──────────────────────────────────────────────
if command -v update-desktop-database &>/dev/null; then
    $NEED_SUDO update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true
fi
if command -v gtk-update-icon-cache &>/dev/null; then
    $NEED_SUDO gtk-update-icon-cache -f -t "$SHARE_DIR/icons/hicolor" 2>/dev/null || true
fi

echo ""
echo -e "${GREEN}${BOLD}✅ ExploreRust instalado correctamente.${NC}"
echo ""
echo -e "  Comandos disponibles:"
echo -e "    ${CYAN}explorerust${NC}        → GUI de escritorio"
echo -e "    ${CYAN}explorerust --tui${NC}  → Interfaz de terminal"
echo -e "    ${CYAN}explorerust --help${NC} → Ayuda"
echo ""
echo -e "  Configuración: ${YELLOW}~/.config/explorerust/config.toml${NC}"
echo -e "  (se crea automáticamente al primer inicio)"
echo ""
