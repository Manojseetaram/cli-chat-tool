# ╔══════════════════════════════════════════════════════════════════════════╗
# ║                    VIVA  —  Windows PowerShell Installer                 ║
# ║                                                                          ║
# ║  Run in PowerShell:                                                      ║
# ║  irm https://raw.githubusercontent.com/Manojseetaram/cli-chat-tool/main/install.ps1 | iex
# ╚══════════════════════════════════════════════════════════════════════════╝

$ErrorActionPreference = "Stop"

$GITHUB_USER = "Manojseetaram"
$GITHUB_REPO = "cli-chat-tool"
$ARTIFACT    = "viva-windows-x86_64.exe"
$INSTALL_DIR = "$env:USERPROFILE\bin"
$BIN_PATH    = "$INSTALL_DIR\viva.exe"

# ── Banner ────────────────────────────────────────────────────────────────────
Write-Host ""
Write-Host "  ██╗   ██╗██╗██╗   ██╗  █████╗ " -ForegroundColor Cyan
Write-Host "  ██║   ██║██║██║   ██║ ██╔══██╗" -ForegroundColor Cyan
Write-Host "  ██║   ██║██║██║   ██║ ███████║" -ForegroundColor Cyan
Write-Host "  ╚██╗ ██╔╝██║╚██╗ ██╔╝ ██╔══██║" -ForegroundColor Cyan
Write-Host "   ╚████╔╝ ██║ ╚████╔╝  ██║  ██║" -ForegroundColor Cyan
Write-Host "    ╚═══╝  ╚═╝  ╚═══╝   ╚═╝  ╚═╝" -ForegroundColor Cyan
Write-Host "  terminal chat  ·  no build step needed" -ForegroundColor DarkGray
Write-Host ""

# ── Create install directory ──────────────────────────────────────────────────
if (-not (Test-Path $INSTALL_DIR)) {
    New-Item -ItemType Directory -Path $INSTALL_DIR -Force | Out-Null
}

# ── Get download URL from GitHub API ─────────────────────────────────────────
Write-Host "  Fetching latest release..." -ForegroundColor DarkGray

$API_URL = "https://api.github.com/repos/$GITHUB_USER/$GITHUB_REPO/releases/latest"
$DOWNLOAD_URL = $null

try {
    $release = Invoke-RestMethod -Uri $API_URL -Headers @{ "User-Agent" = "viva-installer" }
    $asset = $release.assets | Where-Object { $_.name -eq $ARTIFACT } | Select-Object -First 1
    if ($asset) {
        $DOWNLOAD_URL = $asset.browser_download_url
    }
} catch {
    Write-Host "  (GitHub API unavailable, using direct URL)" -ForegroundColor DarkGray
}

if (-not $DOWNLOAD_URL) {
    $DOWNLOAD_URL = "https://github.com/$GITHUB_USER/$GITHUB_REPO/releases/latest/download/$ARTIFACT"
}

Write-Host "  Downloading: $DOWNLOAD_URL" -ForegroundColor DarkGray
Write-Host ""

# ── Download ──────────────────────────────────────────────────────────────────
try {
    $ProgressPreference = "SilentlyContinue"   # makes Invoke-WebRequest much faster
    Invoke-WebRequest -Uri $DOWNLOAD_URL -OutFile $BIN_PATH -UseBasicParsing
} catch {
    Write-Host "  [ERROR] Download failed: $_" -ForegroundColor Red
    Write-Host "  Download manually from: https://github.com/$GITHUB_USER/$GITHUB_REPO/releases/latest" -ForegroundColor DarkGray
    exit 1
}

# ── Verify size ───────────────────────────────────────────────────────────────
$size = (Get-Item $BIN_PATH).Length
if ($size -lt 100000) {
    Write-Host "  [ERROR] Downloaded file too small ($size bytes). Something went wrong." -ForegroundColor Red
    Remove-Item $BIN_PATH -Force
    exit 1
}

Write-Host "  [OK] Installed: $BIN_PATH" -ForegroundColor Green

# ── Add to PATH (current user, permanent) ────────────────────────────────────
$currentPath = [System.Environment]::GetEnvironmentVariable("PATH", "User")
if ($currentPath -notlike "*$INSTALL_DIR*") {
    $newPath = "$currentPath;$INSTALL_DIR"
    [System.Environment]::SetEnvironmentVariable("PATH", $newPath, "User")
    # Also update current session
    $env:PATH = "$env:PATH;$INSTALL_DIR"
    Write-Host "  [OK] Added $INSTALL_DIR to your PATH" -ForegroundColor Green
    Write-Host "       Open a new terminal to use 'viva' by name." -ForegroundColor DarkGray
} else {
    Write-Host "  [OK] $INSTALL_DIR already in PATH" -ForegroundColor Green
}

# ── Done ──────────────────────────────────────────────────────────────────────
Write-Host ""
Write-Host "  ╔══════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "  ║  viva is ready!                                  ║" -ForegroundColor Cyan
Write-Host "  ╚══════════════════════════════════════════════════╝" -ForegroundColor Cyan
Write-Host ""
Write-Host "  Start chatting:" -ForegroundColor White
Write-Host '    $env:RELAY="cli-chat-tool-g1n0.onrender.com"; viva' -ForegroundColor Yellow
Write-Host ""
Write-Host "  Or in CMD:" -ForegroundColor White
Write-Host "    set RELAY=cli-chat-tool-g1n0.onrender.com && viva.exe" -ForegroundColor Yellow
Write-Host ""

# ── Offer to launch now ───────────────────────────────────────────────────────
$ans = Read-Host "  Start viva now? [y/N]"
if ($ans -eq "y" -or $ans -eq "Y") {
    & $BIN_PATH
}