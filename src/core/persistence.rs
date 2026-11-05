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
    pub windows: Vec<WindowSnapshot>,
}

/// Directory where session layout snapshots are stored (~/.config/ax/sessions)
pub fn sessions_dir() -> PathBuf {
    dirs::config_dir()
        .map(|p| p.join("ax").join("sessions"))
        .unwrap_or_else(|| PathBuf::from(".config/ax/sessions"))
}

/// Filepath for a given project's session snapshot (~/.config/ax/sessions/<project>.json)
pub fn session_snapshot_path(project: &str) -> PathBuf {
    sessions_dir().join(format!("{}.json", project))
}

/// Save a snapshot of the active tmux session layout for `project`
pub fn save_session_layout(project: &str) -> Result<PathBuf> {
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

    let snapshot = SessionSnapshot {
        project: project.to_string(),
        session_name: s_name,
        timestamp: chrono::Utc::now().timestamp(),
        windows,
    };

    let dir = sessions_dir();
    fs::create_dir_all(&dir)?;
    let file_path = session_snapshot_path(project);
    let json = serde_json::to_string_pretty(&snapshot)?;
    fs::write(&file_path, json)?;

    Ok(file_path)
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

    if snapshot.windows.is_empty() {
        return Ok(false);
    }

    let s_name = if project.starts_with("agy-") {
        project.to_string()
    } else {
        format!("agy-{}", project)
    };

    // If session already exists in tmux, do not overwrite running session
    if crate::tmux::TmuxDriver::session_exists(&s_name) {
        return Ok(false);
    }

    let config = crate::config::Config::load().unwrap_or_default();

    // Create session with the first window
    let first_win = &snapshot.windows[0];
    let first_pane_path = first_win
        .panes
        .first()
        .map(|p| p.path.clone())
        .unwrap_or_else(|| ".".to_string());

    let first_cmd = build_window_command(project, first_win);

    crate::tmux::TmuxDriver::ensure_session(
        &s_name,
        Path::new(&first_pane_path),
        &config,
        Some(&first_cmd),
        Some(&first_win.name),
    )?;

    // Apply window options for first window
    apply_window_snapshot_options(&s_name, first_win);

    // Restore splits for first window if any
    restore_panes_and_layout(&s_name, first_win);

    // Create and restore remaining windows
    for win in &snapshot.windows[1..] {
        let pane_path = win
            .panes
            .first()
            .map(|p| p.path.clone())
            .unwrap_or_else(|| first_pane_path.clone());

        let cmd = build_window_command(project, win);
        let win_target = format!("{}:{}", s_name, win.index);

        let _ = Command::new("tmux")
            .args([
                "new-window",
                "-t",
                &win_target,
                "-n",
                &win.name,
                "-c",
                &pane_path,
                &cmd,
            ])
            .status();

        apply_window_snapshot_options(&s_name, win);
        restore_panes_and_layout(&s_name, win);
    }

    // Select the first window
    let _ = Command::new("tmux")
        .args([
            "select-window",
            "-t",
            &format!("{}:{}", s_name, first_win.index),
        ])
        .status();

    Ok(true)
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

fn apply_window_snapshot_options(session: &str, win: &WindowSnapshot) {
    let target = format!("{}:{}", session, win.index);
    if !win.name.is_empty() {
        let _ = crate::tmux::TmuxDriver::rename_window(&target, &win.name);
    }
    if let Some(ref h) = win.harness {
        let _ = crate::tmux::TmuxDriver::set_window_option(&target, "@harness", h);
        let _ = Command::new("tmux")
            .args(["set-option", "-p", "-t", &target, "@harness", h])
            .status();
    }
    if let Some(ref cid) = win.conversation_id {
        let _ = crate::tmux::TmuxDriver::set_window_option(&target, "@conversation_id", cid);
        let _ = crate::core::TabRegistry::bind_static(&target, cid);
    }
    if win.user_renamed {
        let _ = crate::tmux::TmuxDriver::set_window_option(&target, "@user_renamed", "1");
    }
}

fn restore_panes_and_layout(session: &str, win: &WindowSnapshot) {
    let target = format!("{}:{}", session, win.index);
    if win.panes.len() > 1 {
        for pane in &win.panes[1..] {
            let _ = Command::new("tmux")
                .args(["split-window", "-t", &target, "-c", &pane.path])
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
            .args(["select-layout", "-t", &target, l])
            .status();
    }
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
}
