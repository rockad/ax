#!/usr/bin/env bash
# uninstall.sh: Clean uninstaller for ax

set -euo pipefail

BIN_DIR="${HOME}/.local/bin"
TMUX_CONF="${HOME}/.config/tmux/tmux.conf"
TMUX_AX_CONF="${HOME}/.config/tmux/ax.conf"

echo "==> Uninstalling ax..."

# 1. Remove binary from ~/.local/bin
if [[ -L "$BIN_DIR/ax" || -f "$BIN_DIR/ax" ]]; then
    echo "  -> Removing $BIN_DIR/ax"
    rm -f "$BIN_DIR/ax"
fi

# Clean up any legacy binaries or symlinks
for legacy in agymux agemux agymux-tab-picker agymux-tab-runner agymux-palette agymux-project-picker agymux-helper agymux-preview; do
    rm -f "${BIN_DIR}/$legacy"
done

# 2. Remove tmux config
if [[ -L "$TMUX_AX_CONF" || -f "$TMUX_AX_CONF" ]]; then
    echo "  -> Removing $TMUX_AX_CONF"
    rm -f "$TMUX_AX_CONF"
fi
rm -f "${HOME}/.config/tmux/agymux.conf" "${HOME}/.config/tmux/agymux-tabs.conf"

# 3. Strip directive from tmux.conf
if [[ -f "$TMUX_CONF" ]]; then
    if grep -q "ax.conf" "$TMUX_CONF" || grep -q "agymux" "$TMUX_CONF"; then
        echo "  -> Removing source directive from $TMUX_CONF"
        tmp=$(mktemp)
        grep -v "ax.conf" "$TMUX_CONF" | grep -v "agymux.conf" | grep -v "agymux-tabs.conf" | grep -v "# Source ax" | grep -v "# Source agymux" > "$tmp" || true
        mv "$tmp" "$TMUX_CONF"
    fi
fi

# 4. Reload tmux configuration if server is active
if command -v tmux >/dev/null 2>&1 && tmux has-session 2>/dev/null; then
    echo "  -> Reloading active tmux configuration"
    tmux source-file "$TMUX_CONF" 2>/dev/null || true
fi

echo "==> ax uninstalled cleanly."
