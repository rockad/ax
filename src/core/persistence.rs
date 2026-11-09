use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PaneSnapshot {
    pub index: usize,
    pub path: String,
    pub command: Option<String>,
    #[serde(default)]
    pub harness: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WindowSnapshot {
    pub index: usize,
    pub name: String,
    pub harness: Option<String>,
    pub conversation_id: Option<String>,
    pub user_renamed: bool,
    pub layout: Option<String>,
    pub panes: Vec<PaneSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionSnapshot {
    pub project: String,
    pub session_name: String,
    pub timestamp: i64,
    #[serde(default)]
    pub name: Option<String>,
    pub windows: Vec<WindowSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveWindow {
    pub index: usize,
    pub name: String,
    pub conversation_id: Option<String>,
    pub harness: Option<String>,
}

/// Directory where session layout snapshots are stored (~/.config/ax/sessions)
pub fn sessions_dir() -> PathBuf {
    dirs::config_dir()
        .map(|p| p.join("ax").join("sessions"))
        .unwrap_or_else(|| PathBuf::from(".config/ax/sessions"))
}

/// Directory where session layout history snapshots are stored (~/.config/ax/sessions/history/<project>)
pub fn session_history_dir(project: &str) -> PathBuf {
    sessions_dir().join("history").join(project)
}

/// Filepath for a given project's session snapshot (~/.config/ax/sessions/<project>.json)
pub fn session_snapshot_path(project: &str) -> PathBuf {
    sessions_dir().join(format!("{}.json", project))
}

fn sanitize_snapshot_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Save a snapshot of the active tmux session layout for `project`
pub fn save_session_layout(project: &str) -> Result<PathBuf> {
    save_named_session_layout(project, None)
}

/// Save a snapshot of the active tmux session layout for `project` with an optional custom name/label
pub fn save_named_session_layout(project: &str, name: Option<&str>) -> Result<PathBuf> {
    if project.is_empty() {
        anyhow::bail!("Cannot save session snapshot for empty project");
    }

    let s_name = if project.starts_with("agy-") {
        project.to_string()
    } else {
        format!("agy-{}", project)
    };

    let exists = Command::new("tmux")
        .args(["has-session", "-t", &s_name])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !exists {
        anyhow::bail!("Session '{}' is not active in tmux", s_name);
    }

    // List windows with metadata: index, name, harness, conversation_id, user_renamed, layout
    let win_out = Command::new("tmux")
        .args([
            "list-windows",
            "-t",
            &s_name,
            "-F",
            "#{window_index}|#{window_name}|#{@harness}|#{@conversation_id}|#{@user_renamed}|#{window_layout}",
        ])
        .output()
        .context("Failed to list windows for session snapshot")?;

    if !win_out.status.success() {
        anyhow::bail!("Failed to query tmux windows for session '{}'", s_name);
    }

    let win_str = String::from_utf8_lossy(&win_out.stdout);
    let mut windows = Vec::new();

    for line in win_str.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 6 {
            let index: usize = parts[0].trim().parse().unwrap_or(0);
            let name = parts[1].trim().to_string();
            let harness = if parts[2].trim().is_empty() {
                None
            } else {
                Some(parts[2].trim().to_string())
            };
            let conversation_id = if parts[3].trim().is_empty() {
                None
            } else {
                Some(parts[3].trim().to_string())
            };
            let user_renamed = parts[4].trim() == "1";
            let layout = if parts[5].trim().is_empty() {
                None
            } else {
                Some(parts[5].trim().to_string())
            };

            // Query panes for this window: index, current path, current command, pane-level harness
            let pane_target = format!("{}:{}", s_name, index);
            let pane_out = Command::new("tmux")
                .args([
                    "list-panes",
                    "-t",
                    &pane_target,
                    "-F",
                    "#{pane_index}|#{pane_current_path}|#{pane_current_command}|#{@harness}",
                ])
                .output();

            let mut panes = Vec::new();
            if let Ok(po) = pane_out {
                if po.status.success() {
                    let po_str = String::from_utf8_lossy(&po.stdout);
                    for pline in po_str.lines() {
                        let pparts: Vec<&str> = pline.split('|').collect();
                        if pparts.len() >= 3 {
                            let pidx: usize = pparts[0].trim().parse().unwrap_or(0);
                            let path = pparts[1].trim().to_string();
                            let cmd = if pparts[2].trim().is_empty() {
                                None
                            } else {
                                Some(pparts[2].trim().to_string())
                            };
                            let pharness = pparts
                                .get(3)
                                .map(|s| s.trim())
                                .filter(|s| !s.is_empty())
                                .map(|s| s.to_string());
                            panes.push(PaneSnapshot {
                                index: pidx,
                                path,
                                command: cmd,
                                harness: pharness,
                            });
                        }
                    }
                }
            }

            windows.push(WindowSnapshot {
                index,
                name,
                harness,
                conversation_id,
                user_renamed,
                layout,
                panes,
            });
        }
    }

    if windows.is_empty() {
        anyhow::bail!("No windows found to snapshot for session '{}'", s_name);
    }

    let timestamp = chrono::Utc::now().timestamp();
    let snapshot = SessionSnapshot {
        project: project.to_string(),
        session_name: s_name,
        timestamp,
        name: name.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
        windows,
    };

    let file_path = session_snapshot_path(project);
    let wrote_canonical = write_session_snapshot(&file_path, &snapshot)?;

    if wrote_canonical {
        let hist_dir = session_history_dir(project);
        let _ = fs::create_dir_all(&hist_dir);
        let filename = match snapshot.name.as_deref() {
            Some(n) if !n.is_empty() => format!("{}_{}.json", timestamp, sanitize_snapshot_name(n)),
            _ => format!("{}.json", timestamp),
        };
        let hist_file = hist_dir.join(filename);
        if let Ok(json) = serde_json::to_string_pretty(&snapshot) {
            let _ = fs::write(&hist_file, json);
        }

        let config = crate::config::Config::load().unwrap_or_default();
        let _ = prune_session_history(project, config.session.max_history);
    }

    Ok(file_path)
}

/// Prune history entries for `project` keeping the most recent `max_history` entries
pub fn prune_session_history(project: &str, max_history: usize) -> Result<()> {
    if max_history == 0 {
        return Ok(());
    }

    let hist_dir = session_history_dir(project);
    if !hist_dir.exists() {
        return Ok(());
    }

    let mut entries = Vec::new();
    if let Ok(rd) = fs::read_dir(&hist_dir) {
        for entry in rd.flatten() {
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) == Some("json") {
                let ts = p
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .and_then(|s| s.split('_').next())
                    .and_then(|s| s.parse::<i64>().ok())
                    .unwrap_or(0);
                entries.push((ts, p));
            }
        }
    }

    entries.sort_by_key(|a| std::cmp::Reverse(a.0));

    if entries.len() > max_history {
        for (_, path) in &entries[max_history..] {
            let _ = fs::remove_file(path);
        }
    }

    Ok(())
}

/// List all historical snapshots for `project` sorted newest first
pub fn list_session_history(project: &str) -> Result<Vec<SessionSnapshot>> {
    let hist_dir = session_history_dir(project);
    let mut snapshots = Vec::new();

    if hist_dir.exists() {
        if let Ok(rd) = fs::read_dir(&hist_dir) {
            for entry in rd.flatten() {
                let p = entry.path();
                if p.extension().and_then(|e| e.to_str()) == Some("json") {
                    if let Ok(content) = fs::read_to_string(&p) {
                        if let Ok(snap) = serde_json::from_str::<SessionSnapshot>(&content) {
                            snapshots.push(snap);
                        }
                    }
                }
            }
        }
    }

    // Include canonical snapshot if not already in history
    let canonical_path = session_snapshot_path(project);
    if canonical_path.exists() {
        if let Ok(content) = fs::read_to_string(&canonical_path) {
            if let Ok(snap) = serde_json::from_str::<SessionSnapshot>(&content) {
                if !snapshots.iter().any(|s| s.timestamp == snap.timestamp) {
                    snapshots.push(snap);
                }
            }
        }
    }

    snapshots.sort_by_key(|a| std::cmp::Reverse(a.timestamp));
    Ok(snapshots)
}

/// Delete a history entry matching `timestamp` for `project`
pub fn delete_session_history_entry(project: &str, timestamp: i64) -> Result<()> {
    let hist_dir = session_history_dir(project);
    if !hist_dir.exists() {
        return Ok(());
    }

    if let Ok(rd) = fs::read_dir(&hist_dir) {
        for entry in rd.flatten() {
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) == Some("json") {
                let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                let file_ts = stem.split('_').next().and_then(|s| s.parse::<i64>().ok());
                if file_ts == Some(timestamp) {
                    let _ = fs::remove_file(&p);
                } else if let Ok(content) = fs::read_to_string(&p) {
                    if let Ok(snap) = serde_json::from_str::<SessionSnapshot>(&content) {
                        if snap.timestamp == timestamp {
                            let _ = fs::remove_file(&p);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Guarded session snapshot writer with backup (.json.bak) and empty/default session overwrite protection.
/// Returns Ok(true) if the file was written, Ok(false) if overwrite was skipped to preserve multi-window layout.
pub fn write_session_snapshot(file_path: &Path, snapshot: &SessionSnapshot) -> Result<bool> {
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)?;
    }

    if file_path.exists() {
        if let Ok(existing_content) = fs::read_to_string(file_path) {
            if let Ok(existing) = serde_json::from_str::<SessionSnapshot>(&existing_content) {
                let _ = fs::copy(file_path, file_path.with_extension("json.bak"));
                if existing.windows.len() > 1 && snapshot.windows.len() <= 1 {
                    let is_unconfigured = snapshot.windows.is_empty()
                        || snapshot.windows.iter().all(|w| {
                            (w.name == "+ new" || w.name == "term") && w.conversation_id.is_none()
                        });
                    if is_unconfigured {
                        eprintln!(
                            "ax: Preserving existing multi-window snapshot for '{}' (skipping overwrite with empty/default window)",
                            snapshot.project
                        );
                        return Ok(false);
                    }
                }
            }
        }
    }

    let json = serde_json::to_string_pretty(snapshot)?;
    fs::write(file_path, json)?;
    Ok(true)
}

/// Identifies snapshot windows that are not present in `live_windows` (matching either by conversation_id or name).
pub fn find_missing_windows<'a>(
    snapshot_windows: &'a [WindowSnapshot],
    live_windows: &[LiveWindow],
) -> Vec<&'a WindowSnapshot> {
    snapshot_windows
        .iter()
        .filter(|win| {
            !live_windows.iter().any(|lw| {
                if let (Some(scid), Some(lcid)) = (&win.conversation_id, &lw.conversation_id) {
                    if scid == lcid {
                        return true;
                    }
                }
                if !win.name.is_empty() && win.name == lw.name {
                    return true;
                }
                false
            })
        })
        .collect()
}

/// Restore a previously saved session snapshot for `project`.
/// Returns Ok(true) if a layout was found and successfully restored.
pub fn restore_session_layout(project: &str) -> Result<bool> {
    let file_path = session_snapshot_path(project);
    if !file_path.exists() {
        return Ok(false);
    }

    let json = match fs::read_to_string(&file_path) {
        Ok(s) => s,
        Err(_) => return Ok(false),
    };

    let snapshot: SessionSnapshot = match serde_json::from_str(&json) {
        Ok(s) => s,
        Err(_) => return Ok(false),
    };

    restore_session_snapshot(project, &snapshot)
}

/// Restore a session snapshot from history matching a timestamp or custom name.
/// Returns Ok(true) if a matching snapshot was found and restored.
pub fn restore_session_from_history(project: &str, identifier: &str) -> Result<bool> {
    let snapshots = list_session_history(project)?;
    let trimmed = identifier.trim();

    let found = snapshots.into_iter().find(|s| {
        s.timestamp.to_string() == trimmed
            || s.name
                .as_deref()
                .map(|n| n.eq_ignore_ascii_case(trimmed))
                .unwrap_or(false)
            || trimmed.is_empty()
    });

    if let Some(snap) = found {
        let file_path = session_snapshot_path(project);
        let _ = write_session_snapshot(&file_path, &snap);
        restore_session_snapshot(project, &snap)
    } else {
        Ok(false)
    }
}

/// Restore windows and layouts from an explicit SessionSnapshot into tmux for `project`.
/// Returns Ok(true) on successful restoration.
pub fn restore_session_snapshot(project: &str, snapshot: &SessionSnapshot) -> Result<bool> {
    if snapshot.windows.is_empty() {
        return Ok(false);
    }

    let s_name = if project.starts_with("agy-") {
        project.to_string()
    } else {
        format!("agy-{}", project)
    };

    let first_win = &snapshot.windows[0];
    let first_pane_path = first_win
        .panes
        .first()
        .map(|p| p.path.clone())
        .unwrap_or_else(|| ".".to_string());

    // If session already exists in tmux, perform resilient restore:
    // Never overwrite, close, or restart active working windows.
    if crate::tmux::TmuxDriver::session_exists(&s_name) {
        let win_out = Command::new("tmux")
            .args([
                "list-windows",
                "-t",
                &s_name,
                "-F",
                "#{window_index}|#{window_name}|#{@conversation_id}|#{@harness}",
            ])
            .output();

        let mut live_windows = Vec::new();
        if let Ok(ref out) = win_out {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout);
                for line in stdout.lines() {
                    let parts: Vec<&str> = line.split('|').collect();
                    if parts.len() >= 2 {
                        let idx: usize = parts[0].trim().parse().unwrap_or(0);
                        let name = parts[1].trim().to_string();
                        let cid = parts
                            .get(2)
                            .map(|s| s.trim())
                            .filter(|s| !s.is_empty())
                            .map(|s| s.to_string());
                        let harness = parts
                            .get(3)
                            .map(|s| s.trim())
                            .filter(|s| !s.is_empty())
                            .map(|s| s.to_string());
                        live_windows.push(LiveWindow {
                            index: idx,
                            name,
                            conversation_id: cid,
                            harness,
                        });
                    }
                }
            }
        }

        let mut existing_indices: std::collections::HashSet<usize> =
            live_windows.iter().map(|w| w.index).collect();

        let is_empty_single_window = if live_windows.len() == 1 {
            let w0 = &live_windows[0];
            w0.conversation_id.is_none() && (w0.name == "+ new" || w0.name == "term")
        } else {
            false
        };

        if is_empty_single_window {
            let w0 = &live_windows[0];
            let win_target = format!("{}:{}", s_name, w0.index);

            // Check if window 1 has an active non-shell process
            let pane_cmd = Command::new("tmux")
                .args([
                    "display-message",
                    "-p",
                    "-t",
                    &win_target,
                    "#{pane_current_command}",
                ])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
                .unwrap_or_default();

            let is_idle_shell =
                pane_cmd.is_empty() || matches!(pane_cmd.as_str(), "zsh" | "bash" | "sh" | "fish");

            if is_idle_shell {
                let cmd = build_window_command(project, first_win);
                if cmd != "$SHELL" {
                    let _ = Command::new("tmux")
                        .args(["respawn-pane", "-k", "-t", &win_target, &cmd])
                        .status();
                }
            }

            apply_window_snapshot_options_target(&win_target, first_win);
            restore_panes_and_layout_target(&win_target, first_win);

            for win in &snapshot.windows[1..] {
                create_and_restore_window(
                    &s_name,
                    project,
                    win,
                    &first_pane_path,
                    &mut existing_indices,
                );
            }

            let _ = Command::new("tmux")
                .args(["select-window", "-t", &win_target])
                .status();

            return Ok(true);
        } else {
            // Multiple windows or non-empty window: recreate missing windows from snapshot
            let missing = find_missing_windows(&snapshot.windows, &live_windows);
            for win in missing {
                create_and_restore_window(
                    &s_name,
                    project,
                    win,
                    &first_pane_path,
                    &mut existing_indices,
                );
            }

            return Ok(true);
        }
    }

    // Session does not exist yet: create session with first window and restore all
    let config = crate::config::Config::load().unwrap_or_default();
    let first_cmd = build_window_command(project, first_win);

    crate::tmux::TmuxDriver::ensure_session(
        &s_name,
        Path::new(&first_pane_path),
        &config,
        Some(&first_cmd),
        Some(&first_win.name),
    )?;

    apply_window_snapshot_options(&s_name, first_win);
    restore_panes_and_layout(&s_name, first_win);

    let mut existing_indices: std::collections::HashSet<usize> = std::collections::HashSet::new();
    existing_indices.insert(first_win.index);

    for win in &snapshot.windows[1..] {
        create_and_restore_window(
            &s_name,
            project,
            win,
            &first_pane_path,
            &mut existing_indices,
        );
    }

    let _ = Command::new("tmux")
        .args([
            "select-window",
            "-t",
            &format!("{}:{}", s_name, first_win.index),
        ])
        .status();

    Ok(true)
}

fn create_and_restore_window(
    session: &str,
    project: &str,
    win: &WindowSnapshot,
    default_path: &str,
    existing_indices: &mut std::collections::HashSet<usize>,
) {
    let pane_path = win
        .panes
        .first()
        .map(|p| p.path.clone())
        .unwrap_or_else(|| default_path.to_string());

    let cmd = build_window_command(project, win);

    let (target_arg, explicit_index) = if !existing_indices.contains(&win.index) {
        (format!("{}:{}", session, win.index), Some(win.index))
    } else {
        (format!("{}:", session), None)
    };

    let mut tmux_cmd = Command::new("tmux");
    tmux_cmd.args([
        "new-window",
        "-d",
        "-t",
        &target_arg,
        "-n",
        &win.name,
        "-c",
        &pane_path,
        "-P",
        "-F",
        "#{window_index}",
    ]);
    if !cmd.is_empty() {
        tmux_cmd.arg(&cmd);
    }

    let created_idx = if let Ok(out) = tmux_cmd.output() {
        if out.status.success() {
            String::from_utf8_lossy(&out.stdout)
                .trim()
                .parse::<usize>()
                .ok()
                .or(explicit_index)
                .unwrap_or(win.index)
        } else {
            explicit_index.unwrap_or(win.index)
        }
    } else {
        explicit_index.unwrap_or(win.index)
    };

    existing_indices.insert(created_idx);
    let target = format!("{}:{}", session, created_idx);
    apply_window_snapshot_options_target(&target, win);
    restore_panes_and_layout_target(&target, win);
}

fn build_window_command(project: &str, win: &WindowSnapshot) -> String {
    let harness = win.harness.as_deref().unwrap_or("agy");
    if harness == "term" {
        return "$SHELL".to_string();
    }
    let name_arg = if win.name != "+ new" && !win.name.is_empty() {
        format!(" -n \"{}\"", win.name.replace('"', "\\\""))
    } else {
        String::new()
    };
    if let Some(ref cid) = win.conversation_id {
        format!(
            "ax run -H {} --project '{}'{name_arg} --conversation '{}'",
            harness, project, cid
        )
    } else {
        format!("ax run -H {} --project '{}'{name_arg}", harness, project)
    }
}

pub fn apply_window_snapshot_options_target(target: &str, win: &WindowSnapshot) {
    if !win.name.is_empty() {
        let _ = crate::tmux::TmuxDriver::rename_window(target, &win.name);
    }
    if let Some(ref h) = win.harness {
        let _ = crate::tmux::TmuxDriver::set_window_option(target, "@harness", h);
        let _ = Command::new("tmux")
            .args(["set-option", "-p", "-t", target, "@harness", h])
            .status();
    }
    if let Some(ref cid) = win.conversation_id {
        let _ = crate::tmux::TmuxDriver::set_window_option(target, "@conversation_id", cid);
        let _ = crate::core::TabRegistry::bind_static(target, cid);
    }
    if win.user_renamed {
        let _ = crate::tmux::TmuxDriver::set_window_option(target, "@user_renamed", "1");
    }
}

pub fn apply_window_snapshot_options(session: &str, win: &WindowSnapshot) {
    let target = format!("{}:{}", session, win.index);
    apply_window_snapshot_options_target(&target, win);
}

pub fn restore_panes_and_layout_target(target: &str, win: &WindowSnapshot) {
    if win.panes.len() > 1 {
        for pane in &win.panes[1..] {
            let _ = Command::new("tmux")
                .args(["split-window", "-t", target, "-c", &pane.path])
                .status();
            if let Some(ref h) = pane.harness {
                let pane_target = format!("{}.{}", target, pane.index);
                let _ = Command::new("tmux")
                    .args(["set-option", "-p", "-t", &pane_target, "@harness", h])
                    .status();
            }
        }
    }
    if let Some(first_pane) = win.panes.first() {
        if let Some(ref h) = first_pane.harness {
            let pane_target = format!("{}.{}", target, first_pane.index);
            let _ = Command::new("tmux")
                .args(["set-option", "-p", "-t", &pane_target, "@harness", h])
                .status();
        }
    }
    if let Some(ref l) = win.layout {
        let _ = Command::new("tmux")
            .args(["select-layout", "-t", target, l])
            .status();
    }
}

pub fn restore_panes_and_layout(session: &str, win: &WindowSnapshot) {
    let target = format!("{}:{}", session, win.index);
    restore_panes_and_layout_target(&target, win);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_serialization_roundtrip() {
        let snapshot = SessionSnapshot {
            project: "test-proj".to_string(),
            session_name: "agy-test-proj".to_string(),
            timestamp: 1234567890,
            name: None,
            windows: vec![
                WindowSnapshot {
                    index: 1,
                    name: "main tab".to_string(),
                    harness: Some("agy".to_string()),
                    conversation_id: Some("conv-123".to_string()),
                    user_renamed: true,
                    layout: Some("b1b2,160x40,0,0".to_string()),
                    panes: vec![
                        PaneSnapshot {
                            index: 0,
                            path: "/home/user/project".to_string(),
                            command: Some("zsh".to_string()),
                            harness: Some("agy".to_string()),
                        },
                        PaneSnapshot {
                            index: 1,
                            path: "/home/user/project/src".to_string(),
                            command: None,
                            harness: None,
                        },
                    ],
                },
                WindowSnapshot {
                    index: 2,
                    name: "claude tab".to_string(),
                    harness: Some("claude".to_string()),
                    conversation_id: None,
                    user_renamed: false,
                    layout: None,
                    panes: vec![PaneSnapshot {
                        index: 0,
                        path: "/home/user/project".to_string(),
                        command: Some("claude".to_string()),
                        harness: Some("claude".to_string()),
                    }],
                },
            ],
        };

        let json = serde_json::to_string_pretty(&snapshot).unwrap();
        let deserialized: SessionSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(snapshot, deserialized);
        assert_eq!(deserialized.windows.len(), 2);
        assert_eq!(deserialized.windows[0].panes.len(), 2);
    }

    #[test]
    fn test_session_snapshot_paths() {
        let path = session_snapshot_path("my-awesome-project");
        assert!(path.to_string_lossy().contains("my-awesome-project.json"));
        assert!(path.to_string_lossy().contains("sessions"));
    }

    #[test]
    fn test_restore_nonexistent_returns_false() {
        let res = restore_session_layout("nonexistent-project-xyz-123").unwrap();
        assert!(!res);
    }

    #[test]
    fn test_write_session_snapshot_backup_and_guard() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("my-proj.json");

        let multi_snapshot = SessionSnapshot {
            project: "my-proj".to_string(),
            session_name: "agy-my-proj".to_string(),
            timestamp: 1000,
            name: None,
            windows: vec![
                WindowSnapshot {
                    index: 1,
                    name: "+ new".to_string(),
                    harness: Some("agy".to_string()),
                    conversation_id: Some("conv-1".to_string()),
                    user_renamed: false,
                    layout: None,
                    panes: vec![],
                },
                WindowSnapshot {
                    index: 2,
                    name: "ce-research".to_string(),
                    harness: Some("claude".to_string()),
                    conversation_id: Some("conv-2".to_string()),
                    user_renamed: true,
                    layout: None,
                    panes: vec![],
                },
            ],
        };

        // 1. Initial write of multi-window layout
        let res = write_session_snapshot(&file_path, &multi_snapshot).unwrap();
        assert!(res);
        assert!(file_path.exists());

        // 2. Attempt to overwrite with single unconfigured window ("+ new" with no cid)
        let unconf_snapshot = SessionSnapshot {
            project: "my-proj".to_string(),
            session_name: "agy-my-proj".to_string(),
            timestamp: 2000,
            name: None,
            windows: vec![WindowSnapshot {
                index: 1,
                name: "+ new".to_string(),
                harness: Some("agy".to_string()),
                conversation_id: None,
                user_renamed: false,
                layout: None,
                panes: vec![],
            }],
        };

        let res_guard = write_session_snapshot(&file_path, &unconf_snapshot).unwrap();
        // Guarded: returns false and does NOT overwrite file_path
        assert!(!res_guard);
        // Backup file exists
        let bak_path = file_path.with_extension("json.bak");
        assert!(bak_path.exists());

        // Verify file_path still has 2 windows
        let content = fs::read_to_string(&file_path).unwrap();
        let current: SessionSnapshot = serde_json::from_str(&content).unwrap();
        assert_eq!(current.windows.len(), 2);

        // 3. Overwrite with configured single window ("ce-research" with cid)
        let conf_single_snapshot = SessionSnapshot {
            project: "my-proj".to_string(),
            session_name: "agy-my-proj".to_string(),
            timestamp: 3000,
            name: None,
            windows: vec![WindowSnapshot {
                index: 1,
                name: "ce-research".to_string(),
                harness: Some("claude".to_string()),
                conversation_id: Some("conv-2".to_string()),
                user_renamed: true,
                layout: None,
                panes: vec![],
            }],
        };

        let res_conf = write_session_snapshot(&file_path, &conf_single_snapshot).unwrap();
        assert!(res_conf);
        let content2 = fs::read_to_string(&file_path).unwrap();
        let current2: SessionSnapshot = serde_json::from_str(&content2).unwrap();
        assert_eq!(current2.windows.len(), 1);
        assert_eq!(current2.windows[0].name, "ce-research");
    }

    #[test]
    fn test_find_missing_windows() {
        let snapshot_windows = vec![
            WindowSnapshot {
                index: 1,
                name: "+ new".to_string(),
                harness: Some("agy".to_string()),
                conversation_id: Some("cid-1".to_string()),
                user_renamed: false,
                layout: None,
                panes: vec![],
            },
            WindowSnapshot {
                index: 2,
                name: "ce-research".to_string(),
                harness: Some("claude".to_string()),
                conversation_id: Some("cid-2".to_string()),
                user_renamed: true,
                layout: None,
                panes: vec![],
            },
            WindowSnapshot {
                index: 3,
                name: "sessionN".to_string(),
                harness: Some("term".to_string()),
                conversation_id: None,
                user_renamed: false,
                layout: None,
                panes: vec![],
            },
        ];

        // Case 1: Tab 2 (ce-research) is live, but tab 3 is missing, tab 1 is live
        let live_windows = vec![
            LiveWindow {
                index: 1,
                name: "different-name-1".to_string(),
                conversation_id: Some("cid-1".to_string()), // matches by cid!
                harness: Some("agy".to_string()),
            },
            LiveWindow {
                index: 2,
                name: "ce-research".to_string(), // matches by name and cid
                conversation_id: Some("cid-2".to_string()),
                harness: Some("claude".to_string()),
            },
        ];

        let missing = find_missing_windows(&snapshot_windows, &live_windows);
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].name, "sessionN");

        // Case 2: All windows present
        let live_all = vec![
            LiveWindow {
                index: 1,
                name: "+ new".to_string(),
                conversation_id: Some("cid-1".to_string()),
                harness: Some("agy".to_string()),
            },
            LiveWindow {
                index: 2,
                name: "ce-research".to_string(),
                conversation_id: Some("cid-2".to_string()),
                harness: Some("claude".to_string()),
            },
            LiveWindow {
                index: 3,
                name: "sessionN".to_string(),
                conversation_id: None,
                harness: Some("term".to_string()),
            },
        ];

        let missing_none = find_missing_windows(&snapshot_windows, &live_all);
        assert!(missing_none.is_empty());
    }

    #[test]
    fn test_session_history_save_and_prune() {
        let temp_dir = tempfile::tempdir().unwrap();
        let proj = "test-hist-proj";
        let hist_dir = temp_dir.path().join("history").join(proj);
        fs::create_dir_all(&hist_dir).unwrap();

        // Write 5 fake history entries
        for i in 1..=5 {
            let snap = SessionSnapshot {
                project: proj.to_string(),
                session_name: format!("agy-{}", proj),
                timestamp: 1000 + i,
                name: if i == 5 {
                    Some("named-snap".to_string())
                } else {
                    None
                },
                windows: vec![],
            };
            let fname = format!("{}.json", 1000 + i);
            fs::write(hist_dir.join(fname), serde_json::to_string(&snap).unwrap()).unwrap();
        }

        // Test manual prune directly using logic
        let mut entries = Vec::new();
        for entry in fs::read_dir(&hist_dir).unwrap().flatten() {
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) == Some("json") {
                let ts = p
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .and_then(|s| s.split('_').next())
                    .and_then(|s| s.parse::<i64>().ok())
                    .unwrap_or(0);
                entries.push((ts, p));
            }
        }
        entries.sort_by_key(|a| std::cmp::Reverse(a.0));
        assert_eq!(entries.len(), 5);

        // Keep 3
        let max_history = 3;
        for (_, p) in &entries[max_history..] {
            fs::remove_file(p).unwrap();
        }

        let remaining: Vec<_> = fs::read_dir(&hist_dir).unwrap().flatten().collect();
        assert_eq!(remaining.len(), 3);
    }
}
