# hx Configuration Reference

`hx` is configured through a single TOML configuration file. It manages window ergonomics, multi-harness execution, tab strip placement, visual styling, mouse click handlers, and bilingual keyboard shortcuts.

---

## Configuration File Locations

`hx` checks configuration files in the following resolution order:

- **Explicit flag / environment**: Config passed via CLI flags or `$HX_CONFIG`.
- **Primary XDG Path**: `~/.config/hx/config.toml` (or `$XDG_CONFIG_HOME/hx/config.toml`).
- **Modular Harness Configs**: Supplementary `*.toml` files in `~/.config/hx/harnesses/*.toml`.
- **Legacy Migration**: `~/.config/ax/` and `~/.config/agymux/` (automatically migrated to `~/.config/hx/` on first launch).
- **Internal Defaults**: If no file is present, `hx` automatically falls back to built-in defaults.

Generated tmux profiles and snippets are synchronized to:
- `~/.config/tmux/hx.conf` (or custom profile target).

Workspace layout snapshots are stored in:
- `~/.config/hx/sessions/<project>.json`.

---

## Configuration Schema

```toml
[general]
projects_dir = "~/projects"
default_model = "Gemini 3.8 Flash (Medium)"
default_effort = "medium"
startup_action = "ask"
remote_control = false
single_remote_control_instance = true

[theme]
preset = "default"
rounded_pills = false
status_position = "top"

[theme.tabs]
position = "top"
layout = "horizontal"
sidebar_position = "left"
spacer_lines = 1
alignment = "left"
show_close_button = true
min_width = 8
standard_width = 22
max_width = 30

[keybindings]
prefix = "C-Space"
bilingual_cyrillic = true

[tmux]
isolated_socket = false

# Optional Custom Pluggable Harnesses
[harnesses.aider]
command = "aider"
display_name = "Aider Chat"
icon = "󰚩"
hotkey = "a"
cyrillic_hotkey = "ф"
args = ["--no-auto-commits"]

[harnesses.aider.env]
ANTHROPIC_API_KEY = "your-api-key"
```

---

## Exhaustive Settings Reference

### `[general]` — General & Workflow Settings

| Key | Type | Default | Choices / Format | Description |
| :--- | :--- | :--- | :--- | :--- |
| `projects_dir` | `string` | `"~/projects"` | Filesystem path | Root directory recursively scanned by Project Switcher (`Ctrl+Space P` / `З`) for git repositories and workspaces. |
| `default_model` | `string` | `"Gemini 3.8 Flash (Medium)"` | Any agent model name | Default model preselected when creating new conversation tabs. |
| `default_effort` | `string` | `"medium"` | `"none"`, `"low"`, `"medium"`, `"high"` | Default thinking / reasoning effort level passed to sessions. |
| `startup_action` | `string` | `"ask"` | `"new"`, `"resume_latest"`, `"picker"`, `"terminal"`, `"ask"` | Cockpit launch behavior when invoked without command arguments. |
| `remote_control` | `boolean` | `false` | `true`, `false` | Enable cross-device remote control mode (`--remote-control`). Windows display the `󰢹` indicator. |
| `single_remote_control_instance` | `boolean` | `true` | `true`, `false` | Restrict remote-control mode to a single active instance per machine to prevent conversation forking across competing remote sessions. |

#### Startup Action Choices:
- `"ask"` (default): Interactive prompt offering New Tab, Resume Latest, or Switcher.
- `"new"`: Immediately spawn a new conversation tab.
- `"resume_latest"`: Automatically attach to the most recently active conversation in the project.
- `"picker"`: Instantly pop up the two-pane conversation switcher (`Ctrl+Space s`).
- `"terminal"`: Open a plain shell tab without starting an AI session.

---

### `[theme]` — Status Bar & Styling

| Key | Type | Default | Choices | Description |
| :--- | :--- | :--- | :--- | :--- |
| `preset` | `string` | `"default"` | `"default"`, `"octo"` | Color preset palette (Monokai Pro Octagon). |
| `rounded_pills` | `boolean` | `false` | `true`, `false` | Use Powerline rounded glyphs (`` / ``) instead of crisp rectangular flat highlighted tabs. |
| `status_position` | `string` | `"top"` | `"top"`, `"bottom"` | Global tmux status line position. |
| `status_style` | `string` (optional) | `None` | Tmux style string | Custom override for base status bar (`bg=#282a3a,fg=#eaf2f1`). |
| `status_left` | `string` | `None` | Tmux format string | Custom override for left status section (Project pill or Prefix indicator). |
| `status_right` | `string` | `""` | Tmux format string | Custom override for right status section. Defaults to empty (`""`) to dedicate 100% of the top status line to the tab bar. |
| `window_status_format` | `string` | `None` | Tmux format string | Custom template for inactive tabs. |
| `window_status_current_format` | `string` | `None` | Tmux format string | Custom template for active tab. |
| `pane_border_status` | `string` (optional) | `"bottom"` | `"bottom"`, `"top"`, `"off"` | Pane border statusline display. Defaults to `"bottom"` to display active harness, project, git branch, directory, and right-aligned time & date. Set to `"off"` to hide. |

---

### `[theme.tabs]` — Tab Strip Ergonomics & Rail

| Key | Type | Default | Choices | Description |
| :--- | :--- | :--- | :--- | :--- |
| `position` | `string` | `"top"` | `"top"`, `"bottom"` | Position of the tab strip relative to the terminal viewport. |
| `layout` | `string` | `"horizontal"` | `"horizontal"`, `"vertical"` | Tab strip mode. `"horizontal"` renders the tmux status bar. `"vertical"` toggles the interactive IDE sidebar rail. |
| `sidebar_position` | `string` | `"left"` | `"left"`, `"right"` | Position of the interactive IDE sidebar when in vertical layout mode. |
| `spacer_lines` | `integer` | `1` | `0`, `1` | Number of empty breathing rows separating tabs from the active pane (`status 2` vs `status on`). |
| `alignment` | `string` | `"left"` | `"left"`, `"centre"`, `"center"`, `"right"` | Tab bar alignment along the horizontal status row. |
| `show_close_button` | `boolean` | `true` | `true`, `false` | Render the interactive close glyph `󰅖` on active and inactive tabs. |
| `min_width` | `integer` | `8` | Positive integer | Minimum tab width constraint for truncated window names. |
| `standard_width` | `integer` | `22` | Positive integer | Standard optimal tab width for window names. |
| `max_width` | `integer` | `30` | Positive integer | Maximum width limit before window names are truncated. |

---

### `[keybindings]` — Prefix & Keyboard Resilience

| Key | Type | Default | Choices | Description |
| :--- | :--- | :--- | :--- | :--- |
| `prefix` | `string` | `"C-Space"` | Valid tmux key | Primary prefix key for all hx shortcuts. |
| `bilingual_cyrillic` | `boolean` | `true` | `true`, `false` | Automatically bind paired RussianWin Cyrillic keys (e.g. `n` / `т`, `c` / `с`, `o` / `щ`, `x` / `ч`, `t` / `е`, `s` / `ы`, `p` / `з`) so layout switching never breaks shortcuts. |

---

### `[tmux]` — Tmux Server Isolation

| Key | Type | Default | Choices | Description |
| :--- | :--- | :--- | :--- | :--- |
| `isolated_socket` | `boolean` | `false` | `true`, `false` | Run hx sessions under a dedicated socket (`tmux -L hx`) isolated from default user sessions. |

---

### `[harnesses.<name>]` — Pluggable Custom Harnesses

Define custom agent harnesses directly in `~/.config/hx/config.toml` or as separate files in `~/.config/hx/harnesses/<name>.toml`:

| Key | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `command` | `string` (optional) | `<name>` | Executable command or path to run. |
| `display_name` | `string` (optional) | `<name>` | Human-readable harness name displayed in menus and status bars. |
| `icon` | `string` (optional) | `"󰚩"` | Nerd font glyph shown in the tab strip. |
| `hotkey` | `char` (optional) | `None` | Direct single-character keybinding under prefix (e.g. `'a'` binds `Ctrl+Space a` / `A`). |
| `cyrillic_hotkey` | `char` (optional) | `None` | Paired Cyrillic hotkey (e.g. `'ф'` binds `Ctrl+Space ф` / `Ф`). |
| `args` | `array of strings` | `[]` | Static arguments passed to the command. |
| `env` | `table` | `{}` | Environment variables injected when launching the harness. |

---

## Status Indicator Legend

`hx` renders real-time visual indicators directly inside each tab's title:

| Glyph | Unicode | Meaning |
| :---: | :---: | :--- |
| **`󰚩`** | `\u{F06A9}` | **Google Antigravity Harness**: Running `agy`. |
| **`󰊠`** | `\u{F02A0}` | **Claude Code Harness**: Running `claude`. |
| **`󰘐`** | `\u{F0610}` | **OpenCode Harness**: Running `opencode`. |
| **`󱚣`** | `\u{F16A3}` | **Codex Harness**: Running `codex`. |
| **``** | `\u{E795}` | **Plain Terminal**: Running `$SHELL`. |
| **`󰢹`** | `\u{F08B9}` | **Remote Control Session**: Tab was launched with `--remote-control` active. |
| **`󰛌`** | `\u{F06CC}` | **External Sync / Worktree Sync**: Changes were synced externally. Clears on pane focus (`pane-focus-in`). |
| **`󰂞`** | `\u{F009E}` | **Activity / Bell Alert**: Window activity flag or terminal bell triggered by a finished run. |
| **`󰅖`** | `\u{F0156}` | **Interactive Close Button**: Mouse-clickable zone bound via `#[range=user\|close]`. |

---

## Mouse Ergonomics & Protection

`hx` features full OpenCode-style mouse handling with safety protections:

### 1. Left Click (`MouseDown1Status`)
- **Clicking the tab body:** Switches to the clicked tab instantly (`switch-client -t =`).
- **Clicking the close icon `󰅖`:**
  - If **2 or more tabs** are open: Immediately closes the tab (`kill-window -t =`).
  - If **only 1 tab** remains: Triggers a tmux confirmation prompt:
    ```text
    Close final tab <title>? (y/n)
    ```
    This prevents accidental window closure from dropping the entire session unexpectedly.

### 2. Middle Click (`MouseDown2Status`)
- Clicking anywhere on a tab with the middle mouse button executes a direct close.
- Protected by the same final-tab confirmation guard as Left Click on `󰅖`.

### 3. Right Click (`MouseDown3Status`)
- Opens an interactive context menu directly at the mouse position:
  - `Close Tab` (`x`)
  - `Rename Tab` (`r`)
  - `Fork Tab` (`f`)
  - `Reload Session` (`R`)

### 4. Tab Drag & Drop (`MouseDrag1Status`)
- Left-click and drag any tab horizontally across the status bar to silently swap and reorder tabs in real-time.

---

## Tab Bar Visual Layouts & ASCII Diagrams

### 1. Top Spaced (`position = "top"`, `spacer_lines = 1`) — Default OpenCode Parity
```text
┌────────────────────────────────────────────────────────────────────────┐
│ #S  󰘳  1 󰚩 main 󰅖   2 󰊠 auth-fix 󰢹 󰅖   3 󰘐 test 󰅖      ● RUNNING 12:45 hx │ <- Row 0: Tabs
├────────────────────────────────────────────────────────────────────────┤
│                                                                        │ <- Row 1: Spacer
├────────────────────────────────────────────────────────────────────────┤
│ $ hx run -H claude                                                     │ <- Row 2+: Terminal
│ Claude Code session ready.                                             │
│                                                                        │
└────────────────────────────────────────────────────────────────────────┘
```

### 2. Top Compact (`position = "top"`, `spacer_lines = 0`)
```text
┌────────────────────────────────────────────────────────────────────────┐
│ #S  󰘳  1 󰚩 main 󰅖   2 󰊠 auth-fix 󰢹 󰅖   3 󰘐 test 󰅖      ● RUNNING 12:45 hx │ <- Row 0: Tabs
├────────────────────────────────────────────────────────────────────────┤
│ $ hx run -H agy                                                        │ <- Row 1+: Terminal
│ Pair programming session ready.                                        │
└────────────────────────────────────────────────────────────────────────┘
```

### 3. Vertical IDE Rail (`hx layout` or `Ctrl+Space B` / `И`)
When activated, horizontal status tabs hide (`status off`) and a vertical 42-column sidebar opens on the left (or right):
```text
┌──────────────────────────────────────────┬─────────────────────────────┐
│ 󰘳 hx Sessions                       [1] │ $ hx run -H claude          │
│                                          │                             │
│ > 1  󰊠  ● fix-search                     │ Running agent loop...       │
│      git: feature/search-revamp          │ Executed search_web: 4 hits │
│      turns: 12  tokens: 42.1k           │                             │
│                                          │                             │
│   2  󰚩  󰢹 remote-control-tab             │                             │
│      git: main                           │                             │
│      turns: 3   tokens: 11.2k           │                             │
│                                          │                             │
│   3      plain-terminal                 │                             │
│      git: main                           │                             │
├──────────────────────────────────────────┤                             │
│ [Enter] Select  [x] Close  [q] Quit      │                             │
└──────────────────────────────────────────┴─────────────────────────────┘
```

---

## Pre-Configured Profile Recipes

### 1. OpenCode Parity (Default)
Top tabs, 1-line breathing spacer, flat highlighted Octagon theme, and protected mouse close:
```toml
# ~/.config/hx/config.toml

[theme]
preset = "default"
rounded_pills = false
status_position = "top"

[theme.tabs]
position = "top"
layout = "horizontal"
spacer_lines = 1
alignment = "left"
show_close_button = true
standard_width = 22
```

### 2. Multi-Harness Developer Setup
Configured with multiple custom tools alongside Antigravity, Claude, and OpenCode:
```toml
# ~/.config/hx/config.toml

[general]
startup_action = "ask"

[harnesses.aider]
command = "aider"
display_name = "Aider"
icon = "󰚩"
hotkey = "a"
cyrillic_hotkey = "ф"
args = ["--no-auto-commits"]
```
