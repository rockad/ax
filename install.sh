#!/usr/bin/env bash
# install.sh: Idempotent installer for hx

set -euo pipefail

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="${HOME}/.local/bin"
TMUX_CONF_DIR="${HOME}/.config/tmux"
TMUX_CONF="${TMUX_CONF_DIR}/tmux.conf"

echo "==> Installing hx from ${REPO_DIR}..."

# 1. Install hx binary into ~/.local/bin
mkdir -p "$BIN_DIR"
# Clean up any legacy binaries or symlinks
for legacy in ax agymux agemux agymux-tab-picker agymux-tab-runner agymux-palette agymux-project-picker agymux-helper agymux-preview; do
    rm -f "${BIN_DIR}/$legacy"
done

if [[ -f "${REPO_DIR}/hx" ]]; then
    echo "  -> Installing pre-built binary to $BIN_DIR"
    cp -f "${REPO_DIR}/hx" "${BIN_DIR}/hx"
    chmod +x "${BIN_DIR}/hx"
elif [[ -f "${REPO_DIR}/target/release/hx" ]]; then
    echo "  -> Linking release binary to $BIN_DIR"
    ln -sf "${REPO_DIR}/target/release/hx" "${BIN_DIR}/hx"
elif command -v cargo >/dev/null 2>&1; then
    echo "  -> Building release binary with cargo"
    (cd "${REPO_DIR}" && cargo build --release)
    ln -sf "${REPO_DIR}/target/release/hx" "${BIN_DIR}/hx"
elif [[ -x "${HOME}/.cargo/bin/cargo" ]]; then
    echo "  -> Building release binary with ~/.cargo/bin/cargo"
    (cd "${REPO_DIR}" && "${HOME}/.cargo/bin/cargo" build --release)
    ln -sf "${REPO_DIR}/target/release/hx" "${BIN_DIR}/hx"
else
    echo "Error: No binaries found and cargo is not available." >&2
    exit 1
fi

# 2. Wire tmux configuration
mkdir -p "$TMUX_CONF_DIR"
# Clean up legacy tmux configs
rm -f "${TMUX_CONF_DIR}/agymux.conf" "${TMUX_CONF_DIR}/agymux-tabs.conf"

if [[ -d "${REPO_DIR}/.git" ]]; then
    ln -sf "${REPO_DIR}/config/tmux/hx.conf" "${TMUX_CONF_DIR}/hx.conf"
elif [[ -f "${REPO_DIR}/config/tmux/hx.conf" ]]; then
    cp -f "${REPO_DIR}/config/tmux/hx.conf" "${TMUX_CONF_DIR}/hx.conf"
fi

if [[ -f "$TMUX_CONF" ]]; then
    if ! grep -q "hx.conf" "$TMUX_CONF"; then
        echo "  -> Adding hx.conf source directive to $TMUX_CONF"
        cat << 'EOF' >> "$TMUX_CONF"

# Source hx configuration & keybindings
if-shell '[ -f ~/.config/tmux/hx.conf ]' 'source-file ~/.config/tmux/hx.conf'
EOF
    else
        echo "  -> hx.conf directive already present in $TMUX_CONF"
    fi
fi

# 3. Reload tmux configuration if server is active
if command -v tmux >/dev/null 2>&1 && tmux has-session 2>/dev/null; then
    echo "  -> Reloading active tmux configuration"
    tmux source-file "$TMUX_CONF" 2>/dev/null || true
fi

echo "==> hx installation complete!"
echo "    Run 'hx' to start or 'hx switch' for the tab switcher."
