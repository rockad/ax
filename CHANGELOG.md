## [0.8.7] - 2026-11-12

### 🚀 Features

- Harness-agnostic tab alerts: remove `@running`, `@agy_running`, and `@claude_running` window options in favor of tmux native `window_activity_flag` with alert bell icon (`󰂞`)
- Streamline hooks: prune `hx hook` from Claude Code `settings.json` and remove `PreInvocation` from Antigravity `hooks.json`, keeping only `Stop: hx hook` for title synchronization

### 🐛 Bug Fixes

- *(tmux)* Enable `monitor-activity on`, `visual-activity off`, `monitor-bell on`, and `activity-action other` in tmux profile and `hx.conf` so background tab output triggers activity alerts reliably

## [0.8.6] - 2026-11-11

### 🚀 Features

- Rebrand binary and CLI from `ax` to `hx` (Harness Multiplexor) across all entry points, configurations, and docs

### 🐛 Bug Fixes

- *(remote-control)* Gate remote-control instance checks and flags strictly to the Antigravity (`agy`) harness
- *(remote-control)* Prevent non-agy tabs (terminal, claude, opencode) from evaluating single-instance remote-control gating or printing false-positive warnings
- *(remote-control)* Auto-heal stale `@remote_control` window options on non-agy windows in tmux

## [0.8.5] - 2026-11-10

### 🚀 Features

- In-place harness restart command across ax, tmux, claude, and opencode (`ax restart`, Ctrl+Space S, right-click tab menu)
- Interactive session history navigator modal and configurable launch restore mode
- Resilient session restoration, snapshot clobber protection, and Claude Code `--resume` session parsing
- Claude Code running & completion indicator parity via `UserPromptSubmit` and `Stop` hooks
- Safe hook installation merging hooks into `~/.claude/settings.json` and `~/.gemini/config/hooks.json`

### 🐛 Bug Fixes

- *(tmux)* Omit leading separator dot when pane directory is not a git repository

### 🎨 Styling

- *(ci)* Resolve clippy warnings and format codebase with rustfmt

## [0.8.4] - 2026-11-05

### 🚀 Features

- Branch ahead/behind telemetry in pane border statusline (`ax git-status`)

## [0.8.3] - 2026-11-05

### 🚀 Features

- Bidirectional tab renaming synchronization and multi-harness session switcher (`ax rename`, multi-harness discovery)

## [0.8.2] - 2026-11-05

### 🚀 Features

- Session restoration precedence, tab title preservation, per-pane harness telemetry, and `ax restore` command

## [0.8.1] - 2026-11-05

### 🚀 Features

- Global harness flag (`--harness`), dynamic attach tab selection, and consolidation of `dga`/`dgc`/`dgo` into `ax`

## [0.8.0] - 2026-11-04

### 🚀 Features

- Tab joining & unsplitting (`ax join`, `ax unsplit`), layout persistence, and slot 2 indicator refinement

## [0.7.1] - 2026-11-04

### 🚀 Features

- Dynamic harness telemetry, per-tab running indicators, and full-width tab formatting

## [0.7.0] - 2026-11-03

### 🚀 Features

- Evolve into pluggable multi-harness multiplexor and rebrand to ax
## [0.6.2] - 2026-10-29

### 🐛 Bug Fixes

- *(profile)* Explicitly set status-format[0] and unset status-format[1] on reload

### ⚙️ Miscellaneous Tasks

- *(release)* Bump version to 0.6.2
## [0.6.1] - 2026-10-29

### 🚀 Features

- *(reload)* Add ax reload subcommand, Ctrl+Space R binding, and fix CI clippy checks

### ⚙️ Miscellaneous Tasks

- *(release)* Bump version to 0.6.1
## [0.6.0] - 2026-10-29

### 🚀 Features

- *(theme)* Set status-position default to bottom and remove top hardcode
- Multi-harness engine (claude/agy), top tabs layout, silent drag reorder, rebrand to ax

### 🐛 Bug Fixes

- *(build)* Ensure make install cleanly overwrites existing binaries or symlinks

### ⚙️ Miscellaneous Tasks

- *(release)* Bump version to 0.6.0
## [0.5.0] - 2026-10-28

### 🚀 Features

- *(core)* Unify tab registry, race-free mouse close, and consolidate tmux driver

### ⚙️ Miscellaneous Tasks

- *(release)* Bump version to 0.5.0
## [0.4.0] - 2026-10-28

### 🚀 Features

- *(palette)* Add window splits, tmux controls, and clean aligned layout

### ⚙️ Miscellaneous Tasks

- *(release)* Bump version to 0.4.0
## [0.3.0] - 2026-10-21

### 🚀 Features

- *(remote-control)* Add remote-control flag forwarding, config option, and palette session reload action
- *(palette)* Add Kill Session option terminating pane processes and tmux session
- *(tmux)* Add turn-driven tab sync hook, width truncation, mouse close, and ide rail
- *(ui)* Disambiguate keybindings, add Alt tab navigation, and fix title swapping
- *(remote-control)* Enforce single active instance to prevent conversation forking
- *(session)* Auto-attach by directory over SSH and display clean project names
- *(ui)* Remap keybindings to mnemonic defaults and underline accelerators in palette
- *(cli)* Add -v/--version flags and semver release automation

### 🐛 Bug Fixes

- *(session)* Filter subagent and zero-step abandoned sessions from picker and discovery
- *(watcher)* Prevent duplicate tab naming by excluding all bound conversation IDs and expand title limit
- *(picker)* Ensure outside-tmux selections attach to agymux tmux session cleanly
- *(runner)* Pass --conversation instead of -c <id> when resuming session
- *(palette)* Fix clippy ascii comparison lint and rustfmt formatting

### 🎨 Styling

- Apply rustfmt formatting across codebase

### 🧪 Testing

- *(session)* Add integration test suite for subagent and abandoned session filtering
- *(sync)* Add cross surface sync integration tests

### ⚙️ Miscellaneous Tasks

- *(release)* Bump version to 0.3.0
## [0.2.1] - 2026-10-13

### ⚙️ Miscellaneous Tasks

- *(release)* Bump version to 0.2.1 and fix rustfmt in tmux profile
## [0.2.0] - 2026-10-13

### 🚀 Features

- Initial release of agymux tabbed terminal multiplexer
- Add --help / help flag and usage documentation
- Agymux v0.2 release with OpenCode ergonomics, two-pane switcher, command palette, and shortcut helper
- Rewrite in native Rust with sub-millisecond cold start and ax dual entrypoint
- *(keys)* Move direct tab navigation from root Alt to prefix chords (Ctrl+Space 1..9, Left/Right)
- *(test)* Implement comprehensive test suite, headless tui tests, benchmarks, and ci
- Release v0.2.0 with configurable startup lifecycle, palette expansions, and terminal color inheritance
- *(tmux)* Use ANSI terminal palette colors for tab row and status indicators
- *(tmux)* Add pipe separator and 3-state tab coloring with distinct elevated inactive shade
- *(tmux)* Place project name on the right with dot-separated time and date
- *(tmux)* Add 1-line bottom margin spacer beneath top tabs

### 🐛 Bug Fixes

- *(tmux)* Eliminate comma parsing artifact in status-left style tags
- *(tabs)* Spawn detached watcher process to sync window titles across exec()
- *(tabs)* Reduce title watcher interval to 500ms and exclude previous conversation
- *(attach)* Launch agymux run in initial tmux window to reach tui on first attach
- *(tmux)* Attach interactively with inherited tty and prefer existing session
- *(tmux)* Remove double braces in status-left format string and allow theme overrides
- *(tmux)* Split reverse and bold style tags to avoid comma in status-left ternary
- *(tmux)* Keep session name persistent and prepend prefix indicator on #{client_prefix}

### ⚡ Performance

- Eliminate input latency by removing forced --remote-control and using exec()
- *(tmux)* Enable synchronized output (mode 2026) and relax status interval
- *(tmux)* Set status-interval to 1s for real-time statusline updates

### 🚜 Refactor

- Consolidate into single agymux entrypoint binary
- Retire legacy daemon subcommand and cleanup references
- *(config)* Generalize agymux-tabs.conf to agymux.conf with compatibility symlink

### 🎨 Styling

- Format attach and driver code with cargo fmt
- *(tmux)* Use distinct white text on black for inactive tabs and white clock details
- *(tmux)* Keep brightblack background with white text for inactive tabs
- *(tmux)* Adopt 256-color palette pairing (colour73 teal active, colour237 dark slate inactive)

### ⚙️ Miscellaneous Tasks

- Update install.sh to build and link target/release binaries
