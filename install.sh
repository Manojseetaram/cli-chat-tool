#!/usr/bin/env bash
# ╔══════════════════════════════════════════════════════════════════════════╗
# ║                        VIVA  —  installer                               ║
# ║                                                                          ║
# ║  curl -fsSL https://raw.githubusercontent.com/YOUR/viva/main/install.sh ║
# ║       | bash                                                             ║
# ║                                                                          ║
# ║  No Rust. No Docker. No build step. Just download and run.              ║
# ╚══════════════════════════════════════════════════════════════════════════╝

set -euo pipefail

# ── CONFIGURE THIS before publishing ─────────────────────────────────────────
GITHUB_USER="Manojseetaram"
GITHUB_REPO="cli-chat-tool"
# ─────────────────────────────────────────────────────────────────────────────

REPO="https://github.com/${GITHUB_USER}/${GITHUB_REPO}"
API="https://api.github.com/repos/${GITHUB_USER}/${GITHUB_REPO}/releases/latest"
INSTALL_DIR="${VIVA_INSTALL_DIR:-$HOME/.local/bin}"

# ── Banner ─────────────────────────────────────────────────────────────────────
echo ""
echo "  ██╗   ██╗██╗██╗   ██╗  █████╗ "
echo "  ██║   ██║██║██║   ██║ ██╔══██╗"
echo "  ██║   ██║██║██║   ██║ ███████║"
echo "  ╚██╗ ██╔╝██║╚██╗ ██╔╝ ██╔══██║"
echo "   ╚████╔╝ ██║ ╚████╔╝  ██║  ██║"
echo "    ╚═══╝  ╚═╝  ╚═══╝   ╚═╝  ╚═╝"
echo "  terminal chat  ·  no install needed"
echo ""

# ── Detect OS + arch ─────────────────────────────────────────────────────────
OS="$(uname -s 2>/dev/null || echo "unknown")"
ARCH="$(uname -m 2>/dev/null || echo "unknown")"

case "$OS" in
  Linux*)
    case "$ARCH" in
      x86_64|amd64)   ARTIFACT="viva-linux-x86_64"   ;;
      aarch64|arm64)  ARTIFACT="viva-linux-aarch64"  ;;
      *)
        echo "  ✗  Unsupported Linux arch: $ARCH"
        echo "     Please open an issue at $REPO"
        exit 1 ;;
    esac ;;
  Darwin*)
    case "$ARCH" in
      x86_64)         ARTIFACT="viva-macos-x86_64"   ;;
      arm64)          ARTIFACT="viva-macos-aarch64"  ;;
      *)
        echo "  ✗  Unsupported macOS arch: $ARCH"
        exit 1 ;;
    esac ;;
  MINGW*|MSYS*|CYGWIN*)
    # Git Bash / WSL on Windows
    ARTIFACT="viva-windows-x86_64.exe"
    INSTALL_DIR="${USERPROFILE:-$HOME}/bin"
    mkdir -p "$INSTALL_DIR" ;;
  *)
    echo "  ✗  Unsupported OS: $OS"
    echo ""
    echo "  Windows users: download viva-windows-x86_64.exe from:"
    echo "  $REPO/releases/latest"
    echo "  Then rename it to viva.exe and run it directly."
    exit 1 ;;
esac

echo "  Detected: $OS / $ARCH"
echo "  Package:  $ARTIFACT"
echo ""

# ── Get latest release download URL ──────────────────────────────────────────
echo "  Fetching latest release info..."

# Try GitHub API first (gives exact asset URL)
DOWNLOAD_URL=""
if command -v curl >/dev/null 2>&1; then
  DOWNLOAD_URL=$(curl -fsSL "$API" 2>/dev/null \
    | grep "browser_download_url" \
    | grep "$ARTIFACT" \
    | head -1 \
    | sed 's/.*"browser_download_url": *"\([^"]*\)".*/\1/')
elif command -v wget >/dev/null 2>&1; then
  DOWNLOAD_URL=$(wget -qO- "$API" 2>/dev/null \
    | grep "browser_download_url" \
    | grep "$ARTIFACT" \
    | head -1 \
    | sed 's/.*"browser_download_url": *"\([^"]*\)".*/\1/')
fi

# Fallback: construct URL from latest tag
if [ -z "$DOWNLOAD_URL" ]; then
  echo "  (Could not reach GitHub API, trying direct URL...)"
  DOWNLOAD_URL="$REPO/releases/latest/download/$ARTIFACT"
fi

echo "  Downloading from: $DOWNLOAD_URL"
echo ""

# ── Create install dir ────────────────────────────────────────────────────────
mkdir -p "$INSTALL_DIR"

BIN_NAME="viva"
[ "$OS" = "MINGW64_NT" ] && BIN_NAME="viva.exe"
BIN_PATH="$INSTALL_DIR/$BIN_NAME"

# ── Download ──────────────────────────────────────────────────────────────────
if command -v curl >/dev/null 2>&1; then
  curl -fL --progress-bar "$DOWNLOAD_URL" -o "$BIN_PATH"
elif command -v wget >/dev/null 2>&1; then
  wget -q --show-progress "$DOWNLOAD_URL" -O "$BIN_PATH"
else
  echo "  ✗  Neither curl nor wget found."
  echo "     Install one of them and retry."
  exit 1
fi

chmod +x "$BIN_PATH"

# ── Verify it actually downloaded something real ──────────────────────────────
FILE_SIZE=$(wc -c < "$BIN_PATH" 2>/dev/null || echo 0)
if [ "$FILE_SIZE" -lt 100000 ]; then
  echo ""
  echo "  ✗  Download looks too small (${FILE_SIZE} bytes). Something went wrong."
  echo "     Check that a release exists at: $REPO/releases"
  rm -f "$BIN_PATH"
  exit 1
fi

echo ""
echo "  ✓  Installed to: $BIN_PATH"

# ── PATH setup ────────────────────────────────────────────────────────────────
PATH_ADDED=false
if ! command -v viva >/dev/null 2>&1; then
  SHELL_RC=""
  case "${SHELL:-}" in
    */zsh)  SHELL_RC="$HOME/.zshrc" ;;
    */bash) SHELL_RC="$HOME/.bashrc" ;;
    *)      SHELL_RC="$HOME/.profile" ;;
  esac

  PATH_LINE='export PATH="$HOME/.local/bin:$PATH"'

  if [ -n "$SHELL_RC" ] && ! grep -qF '.local/bin' "$SHELL_RC" 2>/dev/null; then
    echo "" >> "$SHELL_RC"
    echo "# Added by viva installer" >> "$SHELL_RC"
    echo "$PATH_LINE" >> "$SHELL_RC"
    PATH_ADDED=true
  fi

  # Also export for this session
  export PATH="$HOME/.local/bin:$PATH"
fi

# ── Done ──────────────────────────────────────────────────────────────────────
echo ""
echo "  ╔══════════════════════════════════════════════════╗"
echo "  ║  ✓  viva is ready!                               ║"
echo "  ╚══════════════════════════════════════════════════╝"
echo ""
echo "  Start chatting:"
echo "    viva"
echo ""
echo "  Connect to a shared server:"
echo "    RELAY=your-server.com:3000 viva"
echo ""

if $PATH_ADDED; then
  echo "  ℹ  PATH updated in $SHELL_RC"
  echo "     Run this to apply now:  source $SHELL_RC"
  echo "     Or just open a new terminal."
  echo ""
fi

# ── Offer to run immediately ──────────────────────────────────────────────────
# Only if we're in an interactive terminal (not piped from curl)
if [ -t 0 ] && [ -t 1 ]; then
  echo "  Start viva now? [y/N]  "
  read -r ans
  if [ "${ans}" = "y" ] || [ "${ans}" = "Y" ]; then
    exec "$BIN_PATH"
  fi
fi