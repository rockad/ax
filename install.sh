#!/usr/bin/env bash
# install.sh: Idempotent installer for ax

set -euo pipefail

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="${HOME}/.local/bin"
TMUX_CONF_DIR="${HOME}/.config/tmux"
TMUX_CONF="${TMUX_CONF_DIR}/tmux.conf"

echo "==> Installing ax from ${REPO_DIR}..."

# 1. Install ax binary into ~/.local/bin
mkdir -p "$BIN_DIR"
# Clean up any legacy binaries or symlinks
for legacy in agymux agemux agymux-tab-picker agymux-tab-runner agymux-palette agymux-project-picker agymux-helper agymux-preview; do
    rm -f "${BIN_DIR}/$legacy"
done

if [[ -f "${REPO_DIR}/ax" ]]; then
    echo "  -> Installing pre-built binary to $BIN_DIR"
    cp -f "${REPO_DIR}/ax" "${BIN_DIR}/ax"
    chmod +x "${BIN_DIR}/ax"
elif [[ -f "${REPO_DIR}/target/release/ax" ]]; then
    echo "  -> Linking release binary to $BIN_DIR"
    ln -sf "${REPO_DIR}/target/release/ax" "${BIN_DIR}/ax"
elif command -v cargo >/dev/null 2>&1; then
    echo "  -> Building release binary with cargo"
    (cd "${REPO_DIR}" && cargo build --release)
    ln -sf "${REPO_DIR}/target/release/ax" "${BIN_DIR}/ax"
elif [[ -x "${HOME}/.cargo/bin/cargo" ]]; then
    echo "  -> Building release binary with ~/.cargo/bin/cargo"
    (cd "${REPO_DIR}" && "${HOME}/.cargo/bin/cargo" build --release)
    ln -sf "${REPO_DIR}/target/release/ax" "${BIN_DIR}/ax"
else
    echo "Error: No binaries found and cargo is not available." >&2
    exit 1
fi

# 2. Wire tmux configuration
mkdir -p "$TMUX_CONF_DIR"
# Clean up legacy tmux configs
rm -f "${TMUX_CONF_DIR}/agymux.conf" "${TMUX_CONF_DIR}/agymux-tabs.conf"

if [[ -d "${REPO_DIR}/.git" ]]; then
    ln -sf "${REPO_DIR}/config/tmux/ax.conf" "${TMUX_CONF_DIR}/ax.conf"
elif [[ -f "${REPO_DIR}/config/tmux/ax.conf" ]]; then
    cp -f "${REPO_DIR}/config/tmux/ax.conf" "${TMUX_CONF_DIR}/ax.conf"
fi

if [[ -f "$TMUX_CONF" ]]; then
    if ! grep -q "ax.conf" "$TMUX_CONF"; then
        echo "  -> Adding ax.conf source directive to $TMUX_CONF"
        cat << 'EOF' >> "$TMUX_CONF"

# Source ax configuration & keybindings
if-shell '[ -f ~/.config/tmux/ax.conf ]' 'source-file ~/.config/tmux/ax.conf'
EOF
    else
        echo "  -> ax.conf directive already present in $TMUX_CONF"
    fi
fi

# 3. Reload tmux configuration if server is active
if command -v tmux >/dev/null 2>&1 && tmux has-session 2>/dev/null; then
    echo "  -> Reloading active tmux configuration"
    tmux source-file "$TMUX_CONF" 2>/dev/null || true
fi

echo "==> ax installation complete!"
echo "    Run 'ax' to start or 'ax switch' for the tab switcher."
