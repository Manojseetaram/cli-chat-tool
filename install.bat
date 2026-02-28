#!/usr/bin/env bash
# ╔══════════════════════════════════════════════════════════════════════════╗
# ║                        VIVA  —  installer (Unix)                         ║
# ║                                                                          ║
# ║  curl -fsSL https://raw.githubusercontent.com/YOUR/viva/main/install.sh ║
# ║       | bash                                                             ║
# ║                                                                          ║
# ║  Supports: Linux x86_64 / aarch64, macOS x86_64 / arm64,                ║
# ║            Windows (Git Bash / WSL)                                      ║
# ╚══════════════════════════════════════════════════════════════════════════╝

set -euo pipefail

# ── CONFIGURE BEFORE PUBLISHING ──────────────────────────────────────────────
GITHUB_USER="Manojseetaram"
GITHUB_REPO="cli-chat-tool"
# ─────────────────────────────────────────────────────────────────────────────

REPO="https://github.com/${GITHUB_USER}/${GITHUB_REPO}"
API="https://api.github.com/repos/${GITHUB_USER}/${GITHUB_REPO}/releases/latest"
INSTALL_DIR="${VIVA_INSTALL_DIR:-$HOME/.local/bin}"

# ── Banner ────────────────────────────────────────────────────────────────────
echo ""
echo "  ██╗   ██╗██╗██╗   ██╗  █████╗ "
echo "  ██║   ██║██║██║   ██║ ██╔══██╗"
echo "  ██║   ██║██║██║   ██║ ███████║"
echo "  ╚██╗ ██╔╝██║╚██╗ ██╔╝ ██╔══██║"
echo "   ╚████╔╝ ██║ ╚████╔╝  ██║  ██║"
echo "    ╚═══╝  ╚═╝  ╚═══╝   ╚═╝  ╚═╝"
echo "  terminal chat  ·  no build step needed"
echo ""

# ── Detect OS + arch ─────────────────────────────────────────────────────────
OS="$(uname -s 2>/dev/null || echo unknown)"
ARCH="$(uname -m 2>/dev/null || echo unknown)"

case "$OS" in
  Linux*)
    case "$ARCH" in
      x86_64|amd64)  ARTIFACT="viva-linux-x86_64"  ;;
      aarch64|arm64) ARTIFACT="viva-linux-aarch64" ;;
      *)
        echo "  ✗  Unsupported Linux arch: $ARCH"
        echo "     Open an issue at $REPO"
        exit 1 ;;
    esac
    BIN_NAME="viva"
    ;;

  Darwin*)
    case "$ARCH" in
      x86_64) ARTIFACT="viva-macos-x86_64"  ;;
      arm64)  ARTIFACT="viva-macos-aarch64" ;;
      *)
        echo "  ✗  Unsupported macOS arch: $ARCH"
        exit 1 ;;
    esac
    BIN_NAME="viva"
    ;;

  MINGW*|MSYS*|CYGWIN*)
    # Git Bash on Windows
    ARTIFACT="viva-windows-x86_64.exe"
    BIN_NAME="viva.exe"
    INSTALL_DIR="${USERPROFILE:-$HOME}/bin"
    mkdir -p "$INSTALL_DIR"
    ;;

  *)
    # Could be WSL — treat as Linux
    if grep -qi microsoft /proc/version 2>/dev/null; then
      ARTIFACT="viva-linux-x86_64"
      BIN_NAME="viva"
    else
      echo "  ✗  Unsupported OS: $OS"
      echo ""
      echo "  Windows users: download the .bat installer instead:"
      echo "  $REPO/releases/latest  →  install.bat"
      exit 1
    fi
    ;;
esac

echo "  Detected: $OS / $ARCH"
echo "  Package:  $ARTIFACT"
echo ""

# ── Resolve download URL ──────────────────────────────────────────────────────
echo "  Fetching latest release info..."

DOWNLOAD_URL=""

_fetch() {
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$1" 2>/dev/null
  elif command -v wget >/dev/null 2>&1; then
    wget -qO- "$1" 2>/dev/null
  fi
}

DOWNLOAD_URL=$(
  _fetch "$API" \
  | grep "browser_download_url" \
  | grep "$ARTIFACT" \
  | head -1 \
  | sed 's/.*"browser_download_url": *"\([^"]*\)".*/\1/'
)

if [ -z "$DOWNLOAD_URL" ]; then
  echo "  (GitHub API unavailable, using direct URL fallback)"
  DOWNLOAD_URL="$REPO/releases/latest/download/$ARTIFACT"
fi

echo "  Downloading: $DOWNLOAD_URL"
echo ""

# ── Download ──────────────────────────────────────────────────────────────────
mkdir -p "$INSTALL_DIR"
BIN_PATH="$INSTALL_DIR/$BIN_NAME"

if command -v curl >/dev/null 2>&1; then
  curl -fL --progress-bar "$DOWNLOAD_URL" -o "$BIN_PATH"
elif command -v wget >/dev/null 2>&1; then
  wget -q --show-progress "$DOWNLOAD_URL" -O "$BIN_PATH"
else
  echo "  ✗  Neither curl nor wget found. Install one and retry."
  exit 1
fi

chmod +x "$BIN_PATH"

# ── Sanity check ──────────────────────────────────────────────────────────────
FILE_SIZE=$(wc -c < "$BIN_PATH" 2>/dev/null || echo 0)
if [ "$FILE_SIZE" -lt 100000 ]; then
  echo ""
  echo "  ✗  Download too small (${FILE_SIZE} bytes) — something went wrong."
  echo "     Check releases exist at: $REPO/releases"
  rm -f "$BIN_PATH"
  exit 1
fi

echo ""
echo "  ✓  Installed: $BIN_PATH"

# ── PATH setup ────────────────────────────────────────────────────────────────
PATH_ADDED=false

if ! command -v viva >/dev/null 2>&1; then
  SHELL_RC=""
  case "${SHELL:-}" in
    */zsh)  SHELL_RC="$HOME/.zshrc"  ;;
    */bash) SHELL_RC="$HOME/.bashrc" ;;
    *)      SHELL_RC="$HOME/.profile" ;;
  esac

  PATH_LINE='export PATH="$HOME/.local/bin:$PATH"'

  if [ -n "$SHELL_RC" ] && ! grep -qF '.local/bin' "$SHELL_RC" 2>/dev/null; then
    {
      echo ""
      echo "# Added by viva installer"
      echo "$PATH_LINE"
    } >> "$SHELL_RC"
    PATH_ADDED=true
  fi

  export PATH="$HOME/.local/bin:$PATH"
fi

# ── Done ──────────────────────────────────────────────────────────────────────
echo ""
echo "  ╔══════════════════════════════════════════════════╗"
echo "  ║  ✓  viva is ready!                               ║"
echo "  ╚══════════════════════════════════════════════════╝"
echo ""
echo "  Start chatting (local relay):"
echo "    viva"
echo ""
echo "  Connect to your Render server:"
echo "    RELAY=your-app.onrender.com viva"
echo ""

if $PATH_ADDED; then
  echo "  ℹ  PATH updated in $SHELL_RC"
  echo "     Apply now:  source $SHELL_RC"
  echo "     Or open a new terminal."
  echo ""
fi

# ── Offer immediate launch (only if interactive terminal) ─────────────────────
if [ -t 0 ] && [ -t 1 ]; then
  printf "  Start viva now? [y/N]  "
  read -r ans
  if [ "${ans}" = "y" ] || [ "${ans}" = "Y" ]; then
    exec "$BIN_PATH"
  fi
fi