# ax — Terminal Cockpit & Multi-Harness Multiplexor for AI Coding Agents

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**`ax`** is a high-performance terminal cockpit and multi-harness multiplexer for AI coding agents inside `tmux`.

It combines OpenCode's visual delight and spatial workflow ergonomics (top tab strip, two-pane switcher, command palette, live transcript previews, git worktree isolation, protected mouse tab closing, and vertical IDE sidebar) with first-class multi-harness execution across the premier AI coding agents — built in native Rust for sub-millisecond cold starts and zero runtime dependencies.

---

## Supported Harnesses

`ax` provides unified window lifecycle management, prompt-driven dynamic naming, and dedicated shortcuts for:

- **Google Antigravity (`agy`)** (`󰚩`): `Ctrl+Space n` / `т`
- **Claude Code (`claude`)** (`󰊠`): `Ctrl+Space c` / `с`
- **OpenCode (`opencode`)** (`󰘐`): `Ctrl+Space o` / `щ`
- **Codex (`codex`)** (`󱚣`): `ax new -H codex` / Command Palette
- **Plain Terminal (`term`)** (``): `Ctrl+Space t` / `е`
- **Custom Pluggable Harnesses**: Configured in `~/.config/ax/config.toml` (`[harnesses.<name>]`) or modular `~/.config/ax/harnesses/*.toml` files with custom binary, icon, hotkey, cyrillic hotkey, args, and environment variables.

---

## Highlights

- **Sub-Millisecond Cold-Start (< 1 ms):** Written in Rust with Ratatui and bundled C-SQLite. Modals launch in ~700 µs inside `tmux display-popup`.
- **Zero External CLI Dependencies:** Bundled SQLite (`rusqlite`), streaming NDJSON parser (`serde_json`), and built-in fuzzy matching (`nucleo-matcher`). Does not require `fzf`, `jq`, or external `sqlite3`.
- **First-Class Multi-Harness Ergonomics:** Launch and switch between Antigravity, Claude Code, OpenCode, Codex, and terminal shells inside a single unified session with dedicated icons in the tab strip.
- **Pluggable Harness Architecture:** Define custom AI harnesses with custom hotkeys and environment overrides in `~/.config/ax/harnesses/*.toml`.
- **Top Tab Strip with Dynamic Harness Icons:** Visual status strip positioned at the top of the terminal (`status-position top`) styled in `#282a3a` warm dark grey with crisp rectangular highlighted active tabs, breathing spacer, live harness icons (`󰚩`, `󰊠`, `󰘐`, `󱚣`, ``), and status badges (`󰢹`, `󰛌`, `󰂞`, `󰅖`).
- **Protected Mouse Tab Ergonomics:** Left-click tabs to switch, drag tabs with mouse to silently reorder, click `󰅖` or middle-click to close (with an automatic confirmation guard protecting the last tab from closing the session), and right-click for a native context menu.
- **Vertical IDE Sidebar Rail (`Ctrl+Space B` / `И`):** Toggle an interactive 42-column sidebar rail (`ax layout`) showing git branches, conversation turn counts, token metrics, and instant hotkey selection (`1`..`9`).
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
| **`●`** | **Active Turn Running** | Golden amber dot indicates an agent turn is actively processing in that tab. |
| **`󰢹`** | **Remote Control Session** | Displayed when a session runs with `--remote-control` active. |
| **`󰛌`** | **External / Worktree Sync** | Changes synced from another workspace. Clears automatically on pane focus. |
| **`󰂞`** | **Alert / Activity Flag** | Signals completed background turn, test failure, or terminal bell. |
| **`󰅖`** | **Interactive Close Button** | Mouse-clickable close button bound to safe tab termination. |

### Mouse Controls

- **Left-Click Tab**: Instantly switch to clicked tab.
- **Drag Tab**: Drag and drop tabs across the status line to reorder them silently without focus disruption.
- **Left-Click `󰅖`**: Close tab. If closing the final tab, prompts `Close final tab <title>? (y/n)` to prevent accidentally dropping the session.
- **Middle-Click Tab**: Close tab with last-tab confirmation protection.
- **Right-Click Tab**: Open context menu with quick actions (`Close Tab`, `Rename Tab`, `Fork Tab`, `Join Tab as Split...`, `Unsplit Pane to Tab`, `Reload Session`).

---

## Layout Modes: Horizontal Tabs & Vertical IDE Rail

`ax` supports both horizontal tab strips and a vertical OpenCode-style sidebar rail:

- **Horizontal Tabs (Default):** Rendered via tmux status bar at top (or bottom) with breathing spacer.
- **Vertical IDE Rail:** Toggle with `Ctrl+Space B` / `И`, the command palette, or `ax layout`. Hides horizontal status bar and opens a 42-column interactive TUI sidebar pane on the left showing active windows, git branches, turn metrics, and hotkeys (`1`..`9`, `x` to close, `Enter` to switch, `q` to quit). Automatically adapts to a compact 10-column icon bar on narrow terminals (< 106 cols).

---

## Inside the Two-Pane Switcher (`Ctrl+Space s`)

- `Tab` or `Ctrl+A`: Toggle search scope between **Local Project** (current active tabs + repo history) and **Global Machine** (all repositories across `~/projects/`).
- `j` / `k` or `Up` / `Down`: Navigate list.
- `Enter`: Switch to selected tab or resume historical conversation.
- `Esc` / `q`: Cancel and return to terminal.

---

## Configuration

Detailed documentation for all configuration options, multi-harness schemas, visual layout diagrams, and preset recipes is available in:
👉 [**ax Configuration Reference**](docs/configuration.md)

Default configuration path: `~/.config/ax/config.toml`

---

## Installation

### From Source (Cargo)
```bash
git clone https://github.com/rockad/ax.git
cd ax
cargo build --release

# Install binary
cp target/release/ax ~/.local/bin/
```

---

## CLI Usage

```text
Usage: ax [OPTIONS] [COMMAND]

Commands:
  attach   Attach to active session or start session in current directory (default)
  switch   Open two-pane conversation & tab picker modal
  project  Open project switcher modal
  palette  Open quick command palette modal
  helper   Display keyboard shortcuts cheatsheet modal
  new      Open a new conversation tab in current session
  bind     Bind a conversation to the current tab
  join     Join another tab into current window as a split pane (-v for vertical)
  unsplit  Break current split pane into its own standalone tab
  run      Internal process runner with instant prompt-based window naming
  preview  Render rich preview for a conversation ID
  layout   Toggle OpenCode-style vertical IDE sidebar layout or switch mode
  rail     Run interactive vertical sidebar rail TUI
  ls       List active ax sessions
  help     Print help

Options:
  -p, --project <PROJECT>  Override project session name
  -C, --dir <DIR>          Working directory (default: current directory)
  -d, --direct             Direct execution without tmux
  -V, --version            Print version
```

---

## Session Layout Persistence

`ax` automatically snapshots and restores your workspace structure across reboots and relaunches:

- **Automatic Snapshots**: Layout state (window indices, names, harnesses, conversation IDs, user-rename locks, and pane layouts) is saved to `~/.config/ax/sessions/<project>.json` on every reload and agent turn completion.
- **Seamless Recovery**: When launching or attaching (`ax` / `ax attach`), `ax` detects if the session was restarted and automatically recreates tabs, harnesses, and layout splits matching your previous session snapshot.

---

## License

MIT © Aleksandr Korolev
