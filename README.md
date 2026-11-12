# hx — Terminal Cockpit & Multi-Harness Multiplexor for AI Coding Agents

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**`hx`** is a high-performance terminal cockpit and multi-harness multiplexer for AI coding agents inside `tmux`.

It combines high-ergonomic spatial workflow tooling (top tab strip, two-pane switcher, command palette, live transcript previews, git worktree isolation, protected mouse tab closing, and vertical IDE sidebar) with first-class multi-harness execution across the premier AI coding agents (Claude Code, Google Antigravity, OpenCode, Codex, and terminal shells) — built in native Rust for sub-millisecond cold starts and zero runtime dependencies.

---

## Supported Harnesses

`hx` provides unified window lifecycle management, prompt-driven dynamic naming, and dedicated shortcuts for:

- **Google Antigravity (`agy`)** (`󰚩`): `Ctrl+Space n` / `т`
- **Claude Code (`claude`)** (`󰊠`): `Ctrl+Space c` / `с`
- **OpenCode (`opencode`)** (`󰘐`): `Ctrl+Space o` / `щ`
- **Codex (`codex`)** (`󱚣`): `hx new -H codex` / Command Palette
- **Plain Terminal (`term`)** (``): `Ctrl+Space t` / `е`
- **Custom Pluggable Harnesses**: Configured in `~/.config/hx/config.toml` (`[harnesses.<name>]`) or modular `~/.config/hx/harnesses/*.toml` files with custom binary, icon, hotkey, cyrillic hotkey, args, and environment variables.

---

## Highlights

- **Sub-Millisecond Cold-Start (< 1 ms):** Written in Rust with Ratatui and bundled C-SQLite. Modals launch in ~700 µs inside `tmux display-popup`.
- **Zero External CLI Dependencies:** Bundled SQLite (`rusqlite`), streaming NDJSON parser (`serde_json`), and built-in fuzzy matching (`nucleo-matcher`). Does not require `fzf`, `jq`, or external `sqlite3`.
- **First-Class Multi-Harness Ergonomics:** Launch and switch between Antigravity, Claude Code, OpenCode, Codex, and terminal shells inside a single unified session with dedicated icons in the tab strip.
- **Pluggable Harness Architecture:** Define custom AI harnesses with custom hotkeys and environment overrides in `~/.config/hx/harnesses/*.toml`.
- **Top Tab Strip with Dynamic Harness Icons:** Visual status strip positioned at the top of the terminal (`status-position top`) styled in `#282a3a` warm dark grey with crisp rectangular highlighted active tabs, breathing spacer, live harness icons (`󰚩`, `󰊠`, `󰘐`, `󱚣`, ``), and status badges (`󰢹`, `󰛌`, `󰂞`, `󰅖`).
- **Protected Mouse Tab Ergonomics:** Left-click tabs to switch, drag tabs with mouse to silently reorder, click `󰅖` or middle-click to close (with an automatic confirmation guard protecting the last tab from closing the session), and right-click for a native context menu.
- **Vertical IDE Sidebar Rail (`Ctrl+Space B` / `И`):** Toggle an interactive 42-column sidebar rail (`hx layout`) showing git branches, conversation turn counts, token metrics, and instant hotkey selection (`1`..`9`).
- **Two-Pane Conversation Switcher (`Ctrl+Space s` / `ы`):**
  - **Left Pane (45%):** Active open tabs and historical project conversations with fuzzy search. Supports `Tab` or `Ctrl+A` to toggle between **Local Project** and **Global Machine** search scope.
  - **Right Pane (55%):** Live rich transcript preview showing token counts, user prompt, latest turn excerpt, and executed tools.
- **Global Project Hopper (`Ctrl+Space P` / `З`):** Search and jump between repositories across `~/projects/` and active `agy-*` sessions.
- **Quick Command Palette (`Ctrl+Space p` / `з`):** Interactive action palette (tab rename, bind, fork, new worktree, launch flags, quit cockpit, cheatsheet).
- **Git Worktree Isolation (`Ctrl+Space w` / `ц`):** Create isolated git worktrees on demand under `.worktrees/<branch>`.
- **Bilingual Keymap Resilience:** Every shortcut features paired English and Cyrillic (RussianWin) bindings so CapsLock layout switching never produces dead keys.
- **Instant Prompt-Based Naming:** Windows are named immediately from the first prompt argument, updated dynamically in the background with automatic title reconciliation.

---

## Shortcuts Cheatsheet

| Action | Direct Shortcut | Command Palette Name |
| :--- | :--- | :--- |
| **Switch Tab / Session** | `Ctrl+Space s` / `S` / `ы` / `Ы` | `🗂️ Switch Tab / Session` |
| **Switch Project** | `Ctrl+Space P` / `З` | `🌐 Open Project` |
| **Command Palette** | `Ctrl+Space p` / `з` | `⚙️ Open Command Palette` |
| **Keyboard Cheatsheet** | `Ctrl+Space ?` / `h` / `H` / `р` / `Р` | `❓ Keyboard Shortcuts & Help` |
| **New Antigravity Tab** | `Ctrl+Space n` / `N` / `т` / `Т` | `➕ New Tab (Antigravity)` |
| **New Claude Code Tab** | `Ctrl+Space c` / `C` / `с` / `С` | `➕ New Tab (Claude Code)` |
| **New OpenCode Tab** | `Ctrl+Space o` / `O` / `щ` / `Щ` | `➕ New Tab (OpenCode)` |
| **New Plain Terminal Tab** | `Ctrl+Space t` / `T` / `е` / `Е` | `💻 New Plain Terminal Tab` |
| **Switch to Main Terminal** | `Ctrl+Space m` / `M` / `ь` / `Ь` | `🖥️ Switch to Main Terminal (main)` |
| **New Worktree Tab** | `Ctrl+Space w` / `W` / `ц` / `Ц` | `📁 New Worktree Tab (Branch)` |
| **Rename Active Tab** | `Ctrl+Space r` / `R` / `к` / `К` | `📝 Rename Active Tab` |
| **Join Tab as Split** | `Ctrl+Space j` / `J` / `о` / `О` | `🪟 Join Tab as Split (Horizontal/Vertical)` |
| **Unsplit Pane to Tab** | `Ctrl+Space u` / `U` / `г` / `Г` | `📤 Unsplit Pane to Tab` |
| **Fork Conversation** | `Ctrl+Space f` / `F` / `а` / `А` | `🌿 Fork Conversation (-c)` |
| **Launch with Custom Flags** | `Ctrl+Space l` / `L` / `д` / `Д` | `⚙️ Launch with Custom Flags` |
| **Close Tab** | `Ctrl+Space x` / `X` / `ч` / `Ч` (also `q` / `Q` / `й` / `Й` / `&`) | `🧹 Close Tab` |
| **Detach Session** | `Ctrl+Space d` / `D` / `в` / `В` | `🚪 Detach Session` |
| **Kill Session** | `Ctrl+Space k` / `K` / `л` / `Л` | `🛑 Kill Session (Exit Completely)` |
| **Direct Tab Jump** | `Alt+1` .. `9` / `Ctrl+Space 1` .. `9` | N/A |
| **Cycle Adjacent Tabs** | `Alt+Left` / `Right` / `Ctrl+Space Left` / `Right` | N/A |

---

## Tab Bar Icons & Mouse Ergonomics

### Status Glyphs Legend

| Glyph | Meaning | Lifecycle |
| :---: | :--- | :--- |
| **`󰂞`** | **Unread Response / Activity Alert** | Native tmux window activity alert triggered when a background tab produces terminal output, finishes an agent turn, or rings the terminal bell. Displays against a high-contrast yellow background (`#I 󰂞 <name>`). Clears automatically upon switching to the tab. |
| **`󰢹`** | **Remote Control Session** | Displayed when a session runs with `--remote-control` active (Antigravity web surface sync). |
| **`󰛌`** | **External / Worktree Sync** | Changes synced from another workspace. Clears automatically on pane focus. |
| **`󰅖`** | **Interactive Close Button** | Mouse-clickable close button bound to safe tab termination with last-tab guard. |

### Mouse Controls

- **Left-Click Tab**: Instantly switch to clicked tab.
- **Drag Tab**: Drag and drop tabs across the status line to reorder them silently without focus disruption.
- **Left-Click `󰅖`**: Close tab. If closing the final tab, prompts `Close final tab <title>? (y/n)` to prevent accidentally dropping the session.
- **Middle-Click Tab**: Close tab with last-tab confirmation protection.
- **Right-Click Tab**: Open context menu with quick actions (`Close Tab`, `Rename Tab`, `Fork Tab`, `Join Tab as Split...`, `Unsplit Pane to Tab`, `Reload Session`).

---

## Layout Modes: Horizontal Tabs & Vertical IDE Rail

`hx` supports both horizontal tab strips and a vertical OpenCode-style sidebar rail:

- **Horizontal Tabs (Default):** Rendered via tmux status bar at top (or bottom) with breathing spacer.
- **Vertical IDE Rail:** Toggle with `Ctrl+Space B` / `И`, the command palette, or `hx layout`. Hides horizontal status bar and opens a 42-column interactive TUI sidebar pane on the left showing active windows, git branches, turn metrics, and hotkeys (`1`..`9`, `x` to close, `Enter` to switch, `q` to quit). Automatically adapts to a compact 10-column icon bar on narrow terminals (< 106 cols).

---

## Inside the Two-Pane Switcher (`Ctrl+Space s`)

- `Tab` or `Ctrl+A`: Toggle search scope between **Local Project** (current active tabs + repo history) and **Global Machine** (all repositories across `~/projects/`).
- `j` / `k` or `Up` / `Down`: Navigate list.
- `Enter`: Switch to selected tab or resume historical conversation.
- `Esc` / `q`: Cancel and return to terminal.

---

## Configuration

Detailed documentation for all configuration options, multi-harness schemas, visual layout diagrams, and preset recipes is available in:
👉 [**hx Configuration Reference**](docs/configuration.md)

Default configuration path: `~/.config/hx/config.toml`

---

## Installation

### From Source (Cargo)
```bash
git clone https://github.com/rockad/hx.git
cd hx
cargo build --release

# Install binary
cp target/release/hx ~/.local/bin/
```

---

## CLI Usage

```text
Usage: hx [OPTIONS] [COMMAND]

Commands:
  attach        Attach to active session or start session in current directory (default)
  switch        Open two-pane conversation & tab picker modal
  project       Open project switcher modal
  palette       Open quick command palette modal
  helper        Display keyboard shortcuts cheatsheet modal
  new           Open a new conversation tab in current session
  run           Internal process runner with instant prompt-based window naming
  preview       Render rich preview for a conversation ID
  ls            List active hx sessions
  bind          Bind a conversation ID to the active tmux tab
  close-window  Safely close a window/tab by id or status range, unbinding from tab registry
  reload        Reload configuration, theme, and keybindings in active session
  join          Join another tab into current window as a split pane
  unsplit       Break current split pane into its own standalone tab
  restore       Restore session tabs and split layouts from saved snapshot
  session       Manage and restore session snapshots (e.g. hx session restore)
  restart       Restart the harness running in the active (or target) pane in-place
  rename        Rename active tab and synchronize title to backing harness conversation
  git-status    Output git branch and ahead/behind status formatted for tmux pane border
  install       Install or update hx lifecycle hooks in Antigravity and Claude Code configuration
  help          Print help

Options:
  -p, --project <PROJECT>  Override project session name
  -C, --dir <DIR>          Working directory (default: current directory)
  -d, --direct             Direct execution without tmux
      --remote-control     Enable remote control mode (Google Antigravity web surface sync)
  -H, --harness <HARNESS>  Override agent harness (e.g. agy, claude, opencode, codex, term)
  -f, --fresh              Start a fresh session without restoring previous layout
  -r, --restore            Restore previous session layout
  -v, --version            Print version
```

---

## Native Activity Alerts & Hook Integration

`hx` uses native tmux activity monitoring rather than polling or synthetic status loops:

- **Built-in Activity & Bell Alerts**: When background tabs produce terminal output, finish a turn, or ring the terminal bell (`\a`), tmux automatically raises `window_activity_flag`. `hx` highlights the tab in amber/yellow (`#I 󰂞 <name>`). Visiting the tab clears the alert immediately.
- **Hook Lifecycle (`hx install`)**:
  - **Google Antigravity (`~/.gemini/config/hooks.json`)**: Configures a lightweight `Stop` hook (`hx hook`) to dynamically update tmux tab titles from SQLite conversation summaries when turns conclude.
  - **Claude Code (`~/.claude/settings.json`)**: Clean zero-hook operation. Claude Code runs without process spawning overhead, relying entirely on tmux native activity detection.
  - Run `hx install` at any time to verify and configure hooks.

---

## Session Layout Persistence

`hx` automatically snapshots and restores your workspace structure across reboots and relaunches:

- **Automatic Snapshots**: Layout state (window indices, names, harnesses, conversation IDs, user-rename locks, and pane layouts) is saved to `~/.config/hx/sessions/<project>.json` on every reload, client detach, window unlinking, and agent turn completion.
- **Seamless Recovery**: When launching or attaching (`hx` / `hx attach`), `hx` detects if the session was restarted and automatically recreates tabs, harnesses, and layout splits matching your previous session snapshot.
- **Manual Restoration**: Run `hx restore [project]` or trigger "Restore Session Layout" from the command palette (`Ctrl+Space p`) at any time.
- **Tab Name & Pane Telemetry Preservation**: Renamed tabs retain their titles across restorations, and joined or split panes preserve individual harness status telemetry on their respective bottom pane borders.

---

## License

MIT © Aleksandr Korolev
