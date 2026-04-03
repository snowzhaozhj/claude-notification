#!/usr/bin/env bash
# hook-wrapper.sh — Claude Notification Plugin hook dispatcher
# Detects OS/ARCH, ensures binary exists, then execs it.

set -euo pipefail

# ---------------------------------------------------------------------------
# Resolve CLAUDE_PLUGIN_ROOT (directory containing this script)
# ---------------------------------------------------------------------------
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export CLAUDE_PLUGIN_ROOT="${CLAUDE_PLUGIN_ROOT:-"$(dirname "$SCRIPT_DIR")"}"

# ---------------------------------------------------------------------------
# Detect OS and ARCH
# ---------------------------------------------------------------------------
detect_os() {
  case "$(uname -s)" in
    Darwin)  echo "macos" ;;
    Linux)   echo "linux" ;;
    MINGW*|MSYS*|CYGWIN*) echo "windows" ;;
    *)       echo "unknown" ;;
  esac
}

detect_arch() {
  case "$(uname -m)" in
    x86_64|amd64)   echo "x86_64" ;;
    aarch64|arm64)  echo "aarch64" ;;
    armv7l)         echo "armv7" ;;
    *)              echo "unknown" ;;
  esac
}

OS="$(detect_os)"
ARCH="$(detect_arch)"

BINARY_NAME="claude-notify-${OS}-${ARCH}"
BINARY_PATH="${CLAUDE_PLUGIN_ROOT}/bin/${BINARY_NAME}"

# ---------------------------------------------------------------------------
# Ensure binary exists
# ---------------------------------------------------------------------------
if [[ ! -x "$BINARY_PATH" ]]; then
  echo "[claude-notification] Binary not found at: $BINARY_PATH" >&2
  echo "[claude-notification] Attempting to build from source..." >&2

  CRATES_DIR="${CLAUDE_PLUGIN_ROOT}/crates"

  if [[ -d "$CRATES_DIR" ]] && command -v cargo &>/dev/null; then
    if cargo build --release --manifest-path "${CRATES_DIR}/Cargo.toml" 2>&1; then
      # Locate the built binary (name may vary by package)
      BUILT_BINARY="$(find "${CRATES_DIR}/target/release" -maxdepth 1 -type f -name "claude-notify" | head -1)"
      if [[ -z "$BUILT_BINARY" ]]; then
        # Fallback: find any executable in release dir
        BUILT_BINARY="$(find "${CRATES_DIR}/target/release" -maxdepth 1 -type f -perm +111 | grep -v '\.' | head -1)"
      fi

      if [[ -n "$BUILT_BINARY" && -f "$BUILT_BINARY" ]]; then
        mkdir -p "${CLAUDE_PLUGIN_ROOT}/bin"
        cp "$BUILT_BINARY" "$BINARY_PATH"
        chmod +x "$BINARY_PATH"
        echo "[claude-notification] Binary built and copied to: $BINARY_PATH" >&2
      else
        echo "[claude-notification] Build succeeded but binary not found in release dir." >&2
        BINARY_PATH=""
      fi
    else
      echo "[claude-notification] cargo build failed." >&2
      BINARY_PATH=""
    fi
  else
    echo "[claude-notification] cargo not available or crates/ dir missing, skipping build." >&2
    BINARY_PATH=""
  fi

  # If build failed, try downloading from GitHub Releases
  if [[ -z "$BINARY_PATH" || ! -x "$BINARY_PATH" ]]; then
    echo "[claude-notification] Attempting to download binary from GitHub Releases..." >&2

    REPO="snowzhaozhj/claude-notification"
    # Read version from plugin.json if available
    PLUGIN_JSON="${CLAUDE_PLUGIN_ROOT}/.claude-plugin/plugin.json"
    if [[ -f "$PLUGIN_JSON" ]] && command -v python3 &>/dev/null; then
      VERSION="v$(python3 -c "import json,sys; d=json.load(open('$PLUGIN_JSON')); print(d['version'])" 2>/dev/null || echo "")"
    fi
    VERSION="${VERSION:-latest}"

    if [[ "$VERSION" == "latest" ]]; then
      DOWNLOAD_URL="https://github.com/${REPO}/releases/latest/download/${BINARY_NAME}"
    else
      DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${VERSION}/${BINARY_NAME}"
    fi

    DEST_PATH="${CLAUDE_PLUGIN_ROOT}/bin/${BINARY_NAME}"
    mkdir -p "${CLAUDE_PLUGIN_ROOT}/bin"

    if command -v curl &>/dev/null; then
      if curl -fsSL "$DOWNLOAD_URL" -o "$DEST_PATH"; then
        chmod +x "$DEST_PATH"
        BINARY_PATH="$DEST_PATH"
        echo "[claude-notification] Downloaded binary to: $BINARY_PATH" >&2
      else
        echo "[claude-notification] Download failed from: $DOWNLOAD_URL" >&2
        BINARY_PATH=""
      fi
    elif command -v wget &>/dev/null; then
      if wget -qO "$DEST_PATH" "$DOWNLOAD_URL"; then
        chmod +x "$DEST_PATH"
        BINARY_PATH="$DEST_PATH"
        echo "[claude-notification] Downloaded binary to: $BINARY_PATH" >&2
      else
        echo "[claude-notification] Download failed from: $DOWNLOAD_URL" >&2
        BINARY_PATH=""
      fi
    else
      echo "[claude-notification] Neither curl nor wget available. Cannot download binary." >&2
      BINARY_PATH=""
    fi
  fi
fi

# ---------------------------------------------------------------------------
# Final check
# ---------------------------------------------------------------------------
if [[ -z "$BINARY_PATH" || ! -x "$BINARY_PATH" ]]; then
  echo "[claude-notification] ERROR: No usable binary found. Exiting without notification." >&2
  exit 0
fi

# ---------------------------------------------------------------------------
# Exec binary, forwarding all args and stdin
# ---------------------------------------------------------------------------
exec "$BINARY_PATH" "$@"
