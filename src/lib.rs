pub mod cli;
pub mod config;
pub mod core;
pub mod tmux;
pub mod ui;

use anyhow::{bail, Context, Result};
use clap::Parser;
use cli::{Cli, Commands, SessionAction};
use config::Config;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

/// Main CLI entry point and dispatcher
pub fn run() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::load().unwrap_or_default();

    let dir = cli
        .dir
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let project = cli
        .project
        .or_else(|| std::env::var("AGYMUX_PROJECT").ok())
        .unwrap_or_else(|| {
            dir.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "default".to_string())
        });

    let session_name = format!("agy-{}", project);
    let remote_control = cli.remote_control || config.general.remote_control;

    // Direct execution without tmux
    if cli.direct {
        let requested_rc = remote_control;
        let mut effective_remote_control = requested_rc;
        if requested_rc
            && config.general.single_remote_control_instance
            && is_remote_control_active()
        {
            effective_remote_control = false;
            eprintln!("hx: Active remote-control instance already running on this machine (skipping --remote-control)");
        }
        let mut cmd = Command::new("agy");
        cmd.args(["--project", &project]);
        if effective_remote_control {
            cmd.arg("--remote-control");
        }
        let status = cmd.status()?;
        std::process::exit(status.code().unwrap_or(0));
    }

    match cli.command {
        None => {
            handle_attach(
                &project,
                &dir,
                &session_name,
                &config,
                &[],
                remote_control,
                cli.harness.as_deref(),
                cli.fresh,
                cli.restore,
            )?;
        }
        Some(Commands::Attach { args }) => {
            let rc = remote_control || args.contains(&"--remote-control".to_string());
            handle_attach(
                &project,
                &dir,
                &session_name,
                &config,
                &args,
                rc,
                cli.harness.as_deref(),
                cli.fresh,
                cli.restore,
            )?;
        }

        Some(Commands::Switch { scope }) => {
            ui::run_picker(scope, project, dir)?;
        }

        Some(Commands::Project) => {
            let base_dir = config.resolved_projects_dir();
            ui::run_project_picker(&base_dir)?;
        }

        Some(Commands::Palette { action }) => {
            if let Some(act_str) = action {
                let act = match act_str.to_lowercase().as_str() {
                    "rename" => ui::palette::PaletteAction::RenameTab,
                    "new" => ui::palette::PaletteAction::NewTab,
                    "fork" => ui::palette::PaletteAction::ForkConversation,
                    "worktree" => ui::palette::PaletteAction::NewWorktreeTab,
                    "flags" => ui::palette::PaletteAction::LaunchCustomFlags,
                    "close" => ui::palette::PaletteAction::CloseTab,
                    "switch" => ui::palette::PaletteAction::SwitchTab,
                    "project" => ui::palette::PaletteAction::SwitchProject,
                    "helper" => ui::palette::PaletteAction::Shortcuts,
                    "detach" => ui::palette::PaletteAction::Detach,
                    "quit" => ui::palette::PaletteAction::QuitKeepTabs,
                    "kill" | "kill-session" | "terminate" => {
                        ui::palette::PaletteAction::KillSession
                    }
                    "bind" => ui::palette::PaletteAction::BindConversation,
                    "term" | "terminal" => ui::palette::PaletteAction::NewTerminalTab,
                    "main" => ui::palette::PaletteAction::SwitchToMain,
                    "reload" | "sync" | "refresh" => {
                        ui::palette::PaletteAction::ReloadActiveSession
                    }
                    "split-h" | "splith" | "split-horizontal" => {
                        ui::palette::PaletteAction::SplitHorizontal
                    }
                    "split-v" | "splitv" | "split-vertical" => {
                        ui::palette::PaletteAction::SplitVertical
                    }
                    "zoom" => ui::palette::PaletteAction::TogglePaneZoom,
                    "break" | "break-pane" => ui::palette::PaletteAction::BreakPane,
                    "copy" | "copy-mode" | "scroll" => ui::palette::PaletteAction::CopyMode,
                    "close-pane" | "kill-pane" => ui::palette::PaletteAction::ClosePane,
                    "claude" | "new-claude" => ui::palette::PaletteAction::NewClaudeSession,
                    "agy" | "antigravity" | "new-agy" => {
                        ui::palette::PaletteAction::NewAntigravitySession
                    }
                    "move-left" | "tab-left" => ui::palette::PaletteAction::MoveTabLeft,
                    "move-right" | "tab-right" => ui::palette::PaletteAction::MoveTabRight,
                    "reset-title" | "reset-name" | "auto-title" => {
                        ui::palette::PaletteAction::ResetTabTitle
                    }
                    "join" | "join-tab" | "split-with-tab" => ui::palette::PaletteAction::JoinTab,
                    "unsplit" | "unsplit-pane" | "break-tab" => {
                        ui::palette::PaletteAction::UnsplitPane
                    }
                    "restore" | "restore-session" | "restore-layout" => {
                        ui::palette::PaletteAction::RestoreSession
                    }
                    "history" | "session-history" | "snapshots" => {
                        ui::palette::PaletteAction::SessionHistory
                    }
                    other => bail!("Unknown palette action: {}", other),
                };
                ui::palette::execute_action(act, Some(&project))?;
            } else {
                ui::run_palette(Some(project))?;
            }
        }

        Some(Commands::Helper) => {
            ui::run_helper()?;
        }

        Some(Commands::Bind { conversation_id }) => {
            run_bind(&project, &dir, conversation_id)?;
        }

        Some(Commands::New {
            name,
            harness,
            args,
        }) => {
            let agent_harness = crate::core::AgentHarness::from_str_loose(&harness);
            let tab_name = name.unwrap_or_else(|| match agent_harness {
                crate::core::AgentHarness::PlainTerminal => "term".to_string(),
                _ => "+ new".to_string(),
            });
            let is_agy = agent_harness == crate::core::AgentHarness::Antigravity;
            let requested_rc =
                is_agy && (remote_control || args.contains(&"--remote-control".to_string()));
            let mut effective_remote_control = requested_rc;
            if requested_rc
                && config.general.single_remote_control_instance
                && is_remote_control_active()
            {
                effective_remote_control = false;
                if std::env::var("TMUX").is_ok() {
                    let _ = Command::new("tmux")
                        .args([
                            "display-message",
                            "hx: Active remote-control instance already running on this machine (skipping --remote-control)",
                        ])
                        .status();
                } else {
                    eprintln!("hx: Active remote-control instance already running on this machine (skipping --remote-control)");
                }
            }
            let mut run_args = format!(
                "hx run -H {} --project '{}'",
                agent_harness.as_str(),
                project
            );
            if effective_remote_control {
                run_args.push_str(" --remote-control");
            }
            let filtered_args: Vec<String> = if is_agy && !effective_remote_control {
                args.into_iter()
                    .filter(|a| a != "--remote-control")
                    .collect()
            } else {
                args
            };
            if !filtered_args.is_empty() {
                run_args.push(' ');
                run_args.push_str(&filtered_args.join(" "));
            }

            if std::env::var("TMUX").is_ok() {
                tmux::TmuxDriver::new_window(Some(&tab_name), Some(&run_args))?;
                let _ = crate::core::save_session_layout(&project);
            } else {
                if !tmux::TmuxDriver::session_exists(&session_name) {
                    tmux::TmuxDriver::ensure_session(
                        &session_name,
                        &dir,
                        &config,
                        Some(&run_args),
                        Some(&tab_name),
                    )?;
                } else {
                    let _ = Command::new("tmux")
                        .args([
                            "new-window",
                            "-t",
                            &format!("{}:", session_name),
                            "-c",
                            &dir.display().to_string(),
                            "-n",
                            &tab_name,
                            &run_args,
                        ])
                        .status();
                }
                let _ = crate::core::save_session_layout(&project);
                tmux::TmuxDriver::attach_session(&session_name)?;
            }
        }

        Some(Commands::Run {
            name,
            harness,
            args,
        }) => {
            let rc = remote_control || args.contains(&"--remote-control".to_string());
            run_runner(&project, name, args, rc, &harness, &config)?;
        }

        Some(Commands::Preview { conversation_id }) => {
            run_preview(&conversation_id)?;
        }

        Some(Commands::Ls) => {
            run_ls()?;
        }

        Some(Commands::CloseWindow { target }) => {
            run_close_window(target.as_deref())?;
        }

        Some(Commands::Join { target, vertical }) => {
            run_join(target, vertical)?;
        }

        Some(Commands::Unsplit) => {
            run_unsplit()?;
        }

        Some(Commands::Rename { title, target }) => {
            run_rename(title, target, &project)?;
        }

        Some(Commands::Restore {
            project: restore_proj,
        }) => {
            let target_project = restore_proj.unwrap_or_else(|| project.clone());
            execute_restore(&target_project)?;
        }

        Some(Commands::Session { action }) => match action {
            None => {
                execute_restore(&project)?;
            }
            Some(SessionAction::Restore {
                project: restore_proj,
                snapshot,
            }) => {
                let target_project = restore_proj.unwrap_or_else(|| project.clone());
                if let Some(snap_id) = snapshot {
                    match crate::core::restore_session_from_history(&target_project, &snap_id) {
                        Ok(true) => {
                            println!(
                                "Restored session snapshot '{}' for '{}'",
                                snap_id, target_project
                            );
                            let target_session = if target_project.starts_with("agy-") {
                                target_project
                            } else {
                                format!("agy-{}", target_project)
                            };
                            if std::env::var("TMUX").is_ok() {
                                tmux::TmuxDriver::switch_client(&target_session)?;
                            } else {
                                tmux::TmuxDriver::attach_session(&target_session)?;
                            }
                        }
                        Ok(false) => {
                            println!(
                                "No snapshot matching '{}' found for project '{}'",
                                snap_id, target_project
                            );
                        }
                        Err(e) => {
                            eprintln!("Failed to restore session snapshot: {}", e);
                        }
                    }
                } else {
                    execute_restore(&target_project)?;
                }
            }
            Some(SessionAction::Save {
                project: save_proj,
                name,
            }) => {
                let target_project = save_proj.unwrap_or_else(|| project.clone());
                match crate::core::save_named_session_layout(&target_project, name.as_deref()) {
                    Ok(path) => {
                        if let Some(ref n) = name {
                            println!("Saved session snapshot '{}' to {}", n, path.display());
                        } else {
                            println!("Saved session snapshot to {}", path.display());
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to save session snapshot: {}", e);
                    }
                }
            }
            Some(SessionAction::History { project: hist_proj }) => {
                let target_project = hist_proj.unwrap_or_else(|| project.clone());
                run_session_history(&target_project)?;
            }
            Some(SessionAction::Ls { project: ls_proj }) => {
                run_session_ls(ls_proj.as_deref())?;
            }
        },

        Some(Commands::Reload) => {
            run_reload(&session_name, &config)?;
        }

        Some(Commands::Restart { target }) => {
            run_restart(target)?;
        }

        Some(Commands::Hook { running, start }) => {
            run_hook(running.as_deref(), start)?;
        }

        Some(Commands::GitStatus { path }) => {
            run_git_status(path)?;
        }

        Some(Commands::Install) => {
            run_install()?;
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn handle_attach(
    project: &str,
    dir: &Path,
    session_name: &str,
    config: &Config,
    args: &[String],
    remote_control: bool,
    harness: Option<&str>,
    fresh_flag: bool,
    restore_flag: bool,
) -> Result<()> {
    use std::io::IsTerminal;

    let _ = ensure_hook_installed();
    let _ = ensure_claude_hooks_installed();
    let _ = heal_unbound_windows(project);

    let fallback_harness = if !config.general.default_harness.is_empty() {
        config.general.default_harness.as_str()
    } else {
        "agy"
    };
    let effective_harness = harness.unwrap_or(fallback_harness);
    let agent_harness = crate::core::AgentHarness::from_str_loose(effective_harness);
    let is_agy = agent_harness == crate::core::AgentHarness::Antigravity;

    let requested_rc =
        is_agy && (remote_control || args.contains(&"--remote-control".to_string()));
    let mut effective_remote_control = requested_rc;
    if requested_rc && config.general.single_remote_control_instance && is_remote_control_active() {
        effective_remote_control = false;
        if std::env::var("TMUX").is_ok() {
            let _ = Command::new("tmux")
                .args([
                    "display-message",
                    "hx: Active remote-control instance already running on this machine (skipping --remote-control)",
                ])
                .status();
        } else {
            eprintln!("hx: Active remote-control instance already running on this machine (skipping --remote-control)");
        }
    }

    // Resolve active session: check explicit session_name first, then find by directory
    let active_session = if tmux::TmuxDriver::session_exists(session_name) {
        Some(session_name.to_string())
    } else {
        tmux::TmuxDriver::find_session_by_dir(dir)
    };

    // If session already exists (by name or by directory), attach or switch directly
    if let Some(ref target_session) = active_session {
        let clean_project = if !project.is_empty() && !crate::core::session::is_uuid(project) {
            project.to_string()
        } else if let Some(dir_name) = dir.file_name().and_then(|n| n.to_str()) {
            dir_name.to_string()
        } else {
            "main".to_string()
        };
        let _ = Command::new("tmux")
            .args([
                "set-option",
                "-t",
                target_session,
                "@hx_project",
                &clean_project,
            ])
            .status();
        let _ = Command::new("tmux")
            .args([
                "set-option",
                "-t",
                target_session,
                "@ax_project",
                &clean_project,
            ])
            .status();
        let _ = Command::new("tmux")
            .args([
                "set-option",
                "-t",
                target_session,
                "@agymux_project",
                &clean_project,
            ])
            .status();

        let status_val = if config.theme.tabs.spacer_lines == 0 {
            "on"
        } else {
            "2"
        };
        let effective_pos = if config.theme.tabs.position != "bottom" {
            &config.theme.tabs.position
        } else {
            &config.theme.status_position
        };
        let _ = Command::new("tmux")
            .args(["set", "-g", "status", status_val])
            .status();
        let _ = Command::new("tmux")
            .args(["set", "-g", "status-position", effective_pos])
            .status();
        let _ = Command::new("tmux")
            .args(["set", "-t", target_session, "status", status_val])
            .status();
        let _ = Command::new("tmux")
            .args([
                "set",
                "-t",
                target_session,
                "status-position",
                effective_pos,
            ])
            .status();

        if let Some(req_harness) = harness {
            // Explicit harness was specified (e.g. cli.harness.is_some())
            let mut found_win_id = None;
            if args.is_empty() {
                if let Ok(output) = Command::new("tmux")
                    .args([
                        "list-windows",
                        "-t",
                        target_session,
                        "-F",
                        "#{window_id}:#{@harness}",
                    ])
                    .output()
                {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    for line in stdout.lines() {
                        let parts: Vec<&str> = line.trim().splitn(2, ':').collect();
                        if parts.len() == 2 {
                            let win_id = parts[0];
                            let win_harness = parts[1];
                            if win_harness == req_harness
                                || crate::core::AgentHarness::from_str_loose(win_harness).as_str()
                                    == crate::core::AgentHarness::from_str_loose(req_harness)
                                        .as_str()
                            {
                                found_win_id = Some(win_id.to_string());
                                break;
                            }
                        }
                    }
                }
            }

            if let Some(ref win_id) = found_win_id {
                let _ = Command::new("tmux")
                    .args(["select-window", "-t", win_id])
                    .status();
            } else {
                let req_agent_harness = crate::core::AgentHarness::from_str_loose(req_harness);
                let req_is_agy = req_agent_harness == crate::core::AgentHarness::Antigravity;
                let mut run_cmd = format!("hx run -H {} --project '{}'", req_harness, project);
                if req_is_agy && effective_remote_control {
                    run_cmd.push_str(" --remote-control");
                }
                let filtered_args: Vec<&String> = if req_is_agy && !effective_remote_control {
                    args.iter().filter(|a| *a != "--remote-control").collect()
                } else {
                    args.iter().collect()
                };
                if !filtered_args.is_empty() {
                    run_cmd.push(' ');
                    run_cmd.push_str(
                        &filtered_args
                            .iter()
                            .map(|s| {
                                if s.contains(' ') || s.contains('"') || s.contains('\'') {
                                    format!("\"{}\"", s.replace('"', "\\\""))
                                } else {
                                    s.to_string()
                                }
                            })
                            .collect::<Vec<_>>()
                            .join(" "),
                    );
                }
                let tab_name = match crate::core::AgentHarness::from_str_loose(req_harness) {
                    crate::core::AgentHarness::PlainTerminal => "term",
                    _ => "+ new",
                };
                let _ = Command::new("tmux")
                    .args(["new-window", "-t", target_session, "-n", tab_name, &run_cmd])
                    .status();
            }

            if std::env::var("TMUX").is_ok() {
                if let Some(curr_session) = tmux::TmuxDriver::current_session() {
                    if &curr_session == target_session {
                        return Ok(());
                    }
                }
                return tmux::TmuxDriver::switch_client(target_session);
            } else if std::io::stdin().is_terminal() || std::io::stdout().is_terminal() {
                return tmux::TmuxDriver::attach_session(target_session);
            } else {
                eprintln!(
                    "hx: Session '{}' is running in the background. (attach with: tmux attach -t {})",
                    target_session, target_session
                );
                return Ok(());
            }
        }

        // No explicit harness passed: attach/switch directly as before
        if !args.is_empty() {
            if std::env::var("TMUX").is_ok() {
                if let Some(curr_session) = tmux::TmuxDriver::current_session() {
                    if &curr_session == target_session {
                        return run_runner(
                            project,
                            None,
                            args.to_vec(),
                            effective_remote_control,
                            effective_harness,
                            config,
                        );
                    }
                }
                return tmux::TmuxDriver::switch_client(target_session);
            }
            if std::io::stdin().is_terminal() || std::io::stdout().is_terminal() {
                return tmux::TmuxDriver::attach_session(target_session);
            } else {
                eprintln!(
                    "hx: Session '{}' is running in the background. (attach with: tmux attach -t {})",
                    target_session, target_session
                );
                return Ok(());
            }
        }

        if std::env::var("TMUX").is_ok() {
            if let Some(curr_session) = tmux::TmuxDriver::current_session() {
                if &curr_session == target_session {
                    // Already inside: run agy -c in this window
                    return run_runner(
                        project,
                        None,
                        vec!["-c".to_string()],
                        effective_remote_control,
                        effective_harness,
                        config,
                    );
                }
            }
            return tmux::TmuxDriver::switch_client(target_session);
        } else if std::io::stdin().is_terminal() || std::io::stdout().is_terminal() {
            return tmux::TmuxDriver::attach_session(target_session);
        } else {
            eprintln!(
                "hx: Session '{}' is running in the background. (attach with: tmux attach -t {})",
                target_session, target_session
            );
            return Ok(());
        }
    }

    // Session does not exist yet
    if !args.is_empty() {
        let mut run_cmd = format!("hx run -H {} --project '{}'", effective_harness, project);
        if is_agy && effective_remote_control {
            run_cmd.push_str(" --remote-control");
        }
        let filtered_args: Vec<&String> = if is_agy && !effective_remote_control {
            args.iter().filter(|a| *a != "--remote-control").collect()
        } else {
            args.iter().collect()
        };
        if !filtered_args.is_empty() {
            run_cmd.push(' ');
            run_cmd.push_str(
                &filtered_args
                    .iter()
                    .map(|s| {
                        if s.contains(' ') || s.contains('"') || s.contains('\'') {
                            format!("\"{}\"", s.replace('"', "\\\""))
                        } else {
                            s.to_string()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" "),
            );
        }
        let tab_name = match crate::core::AgentHarness::from_str_loose(effective_harness) {
            crate::core::AgentHarness::PlainTerminal => "term",
            _ => "+ new",
        };
        tmux::TmuxDriver::ensure_session(
            session_name,
            dir,
            config,
            Some(&run_cmd),
            Some(tab_name),
        )?;

        if std::env::var("TMUX").is_ok() {
            return tmux::TmuxDriver::switch_client(session_name);
        } else if std::io::stdin().is_terminal() || std::io::stdout().is_terminal() {
            return tmux::TmuxDriver::attach_session(session_name);
        } else {
            eprintln!(
                "hx: Session '{}' is running in the background. (attach with: tmux attach -t {})",
                session_name, session_name
            );
            return Ok(());
        }
    }

    // Session does not exist yet: check restore flow according to flags & configuration
    let mut should_restore = false;
    let mut chosen_snapshot: Option<crate::core::SessionSnapshot> = None;

    if restore_flag {
        should_restore = true;
    } else if fresh_flag {
        should_restore = false;
    } else {
        let snapshot_file = crate::core::session_snapshot_path(project);
        if snapshot_file.exists() {
            let existing_snap: Option<crate::core::SessionSnapshot> =
                fs::read_to_string(&snapshot_file)
                    .ok()
                    .and_then(|content| serde_json::from_str(&content).ok());

            if let Some(snap) = existing_snap {
                match config.session.restore_mode {
                    crate::config::SessionRestoreMode::Restore => {
                        should_restore = true;
                        chosen_snapshot = Some(snap);
                    }
                    crate::config::SessionRestoreMode::Fresh => {
                        should_restore = false;
                    }
                    crate::config::SessionRestoreMode::Ask => {
                        if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
                            match ui::run_restore_selector(project.to_string(), &snap)? {
                                Some((ui::RestoreChoice::RestorePrevious, remember)) => {
                                    if remember {
                                        let mut updated = config.clone();
                                        updated.session.restore_mode =
                                            crate::config::SessionRestoreMode::Restore;
                                        let _ = updated.save();
                                    }
                                    should_restore = true;
                                    chosen_snapshot = Some(snap);
                                }
                                Some((ui::RestoreChoice::StartFresh, remember)) => {
                                    if remember {
                                        let mut updated = config.clone();
                                        updated.session.restore_mode =
                                            crate::config::SessionRestoreMode::Fresh;
                                        let _ = updated.save();
                                    }
                                    should_restore = false;
                                }
                                Some((ui::RestoreChoice::BrowseHistory, _)) => {
                                    if let Some(hist_snap) =
                                        ui::run_session_history_picker(project)?
                                    {
                                        should_restore = true;
                                        chosen_snapshot = Some(hist_snap);
                                    } else {
                                        return Ok(());
                                    }
                                }
                                None => {
                                    // User cancelled
                                    return Ok(());
                                }
                            }
                        } else {
                            should_restore = true;
                            chosen_snapshot = Some(snap);
                        }
                    }
                }
            }
        }
    }

    if should_restore {
        let restore_res = if let Some(ref snap) = chosen_snapshot {
            crate::core::restore_session_snapshot(project, snap)
        } else {
            crate::core::restore_session_layout(project)
        };

        if let Ok(true) = restore_res {
            if let Some(req_harness) = harness {
                let out = Command::new("tmux")
                    .args([
                        "list-windows",
                        "-t",
                        session_name,
                        "-F",
                        "#{window_id}:#{@harness}",
                    ])
                    .output();
                if let Ok(o) = out {
                    let s = String::from_utf8_lossy(&o.stdout);
                    for line in s.lines() {
                        let parts: Vec<&str> = line.split(':').collect();
                        if parts.len() >= 2 && parts[1].trim() == req_harness {
                            let win_id = parts[0].trim();
                            let _ = Command::new("tmux")
                                .args(["select-window", "-t", win_id])
                                .status();
                            break;
                        }
                    }
                }
            }
            if std::env::var("TMUX").is_ok() {
                return tmux::TmuxDriver::switch_client(session_name);
            } else if std::io::stdin().is_terminal() || std::io::stdout().is_terminal() {
                return tmux::TmuxDriver::attach_session(session_name);
            } else {
                return Ok(());
            }
        }
    }

    // Session does not exist yet and no args:
    // If an explicit harness was requested, skip Ask and immediately create session
    if harness.is_some() {
        let mut run_cmd = format!("hx run -H {} --project '{}'", effective_harness, project);
        if is_agy && effective_remote_control {
            run_cmd.push_str(" --remote-control");
        }
        let tab_name = match crate::core::AgentHarness::from_str_loose(effective_harness) {
            crate::core::AgentHarness::PlainTerminal => "term",
            _ => "+ new",
        };
        tmux::TmuxDriver::ensure_session(
            session_name,
            dir,
            config,
            Some(&run_cmd),
            Some(tab_name),
        )?;
        if std::env::var("TMUX").is_ok() {
            return tmux::TmuxDriver::switch_client(session_name);
        } else if std::io::stdin().is_terminal() || std::io::stdout().is_terminal() {
            return tmux::TmuxDriver::attach_session(session_name);
        } else {
            return Ok(());
        }
    }

    // Dispatch according to startup_action
    let mut startup_action = config.general.startup_action;

    if startup_action == crate::config::StartupAction::Ask {
        if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
            let latest_summary = {
                let db = db_path();
                if db.exists() {
                    rusqlite::Connection::open(&db).ok().and_then(|conn| {
                        let hygiene = crate::core::session::session_hygiene_conditions(&conn, "conversation_summaries");
                        let hygiene_and = if hygiene.is_empty() {
                            String::new()
                        } else {
                            format!(" AND {}", hygiene.join(" AND "))
                        };
                        let pattern = format!("%{}%", project);
                        let sql = format!(
                            "SELECT title, strftime('%s', last_modified_time) FROM conversation_summaries WHERE (workspace_uris LIKE ?1 OR project_id = ?2){} ORDER BY last_modified_time DESC LIMIT 1",
                            hygiene_and
                        );
                        conn.query_row(
                            &sql,
                            rusqlite::params![pattern, project],
                            |r| {
                                let title: String = r.get(0)?;
                                let epoch_str: Option<String> = r.get(1).ok();
                                let rel = if let Some(Ok(epoch)) = epoch_str.map(|s| s.parse::<i64>()) {
                                    crate::core::session::format_relative_time(epoch)
                                } else {
                                    "recently".to_string()
                                };
                                Ok((sanitize_title(&title), rel))
                            },
                        ).ok()
                    })
                } else {
                    None
                }
            };

            match ui::run_startup_selector(project.to_string(), latest_summary)? {
                Some((action, remember)) => {
                    if remember {
                        let mut updated_config = config.clone();
                        updated_config.general.startup_action = action;
                        let _ = updated_config.save();
                    }
                    startup_action = action;
                }
                None => {
                    // User cancelled (Esc / q)
                    return Ok(());
                }
            }
        } else {
            startup_action = crate::config::StartupAction::ResumeLatest;
        }
    }

    match startup_action {
        crate::config::StartupAction::New => {
            let mut run_cmd = format!("hx run -H {} --project '{}'", effective_harness, project);
            if is_agy && effective_remote_control {
                run_cmd.push_str(" --remote-control");
            }
            let tab_name = match crate::core::AgentHarness::from_str_loose(effective_harness) {
                crate::core::AgentHarness::PlainTerminal => "term",
                _ => "+ new",
            };
            tmux::TmuxDriver::ensure_session(
                session_name,
                dir,
                config,
                Some(&run_cmd),
                Some(tab_name),
            )?;
            if std::env::var("TMUX").is_ok() {
                tmux::TmuxDriver::switch_client(session_name)?;
            } else if std::io::stdin().is_terminal() || std::io::stdout().is_terminal() {
                tmux::TmuxDriver::attach_session(session_name)?;
            }
        }
        crate::config::StartupAction::ResumeLatest => {
            let latest_id = {
                let db = db_path();
                if db.exists() {
                    rusqlite::Connection::open(&db).ok().and_then(|conn| {
                        crate::core::session::latest_conversation_id(&conn, Some(project))
                    })
                } else {
                    None
                }
            };

            let mut run_cmd = if let Some(ref lid) = latest_id {
                format!(
                    "hx run -H {} --project '{}' --conversation '{}'",
                    effective_harness, project, lid
                )
            } else {
                format!("hx run -H {} --project '{}'", effective_harness, project)
            };
            if is_agy && effective_remote_control {
                run_cmd.push_str(" --remote-control");
            }
            let tab_name = match crate::core::AgentHarness::from_str_loose(effective_harness) {
                crate::core::AgentHarness::PlainTerminal => "term",
                _ => "+ new",
            };
            tmux::TmuxDriver::ensure_session(
                session_name,
                dir,
                config,
                Some(&run_cmd),
                Some(tab_name),
            )?;
            if std::env::var("TMUX").is_ok() {
                tmux::TmuxDriver::switch_client(session_name)?;
            } else if std::io::stdin().is_terminal() || std::io::stdout().is_terminal() {
                tmux::TmuxDriver::attach_session(session_name)?;
            }
        }
        crate::config::StartupAction::Picker => {
            ui::run_picker(cli::ScopeArg::Local, project.to_string(), dir.to_path_buf())?;
        }
        crate::config::StartupAction::Terminal => {
            let _ = Command::new("tmux")
                .args(["new-session", "-d", "-s", "main"])
                .status();
            if std::env::var("TMUX").is_ok() {
                tmux::TmuxDriver::switch_client("main")?;
            } else if std::io::stdin().is_terminal() || std::io::stdout().is_terminal() {
                tmux::TmuxDriver::attach_session("main")?;
            }
        }
        crate::config::StartupAction::Ask => unreachable!(),
    }

    Ok(())
}

/// Collect all non-empty @conversation_id values currently bound to tmux windows.
/// If `session_name` is Some, queries only windows in that session; otherwise queries all windows across all sessions.
pub fn get_bound_conversation_ids(session_name: Option<&str>) -> std::collections::HashSet<String> {
    let mut bound = std::collections::HashSet::new();
    let mut cmd = Command::new("tmux");
    cmd.arg("list-windows");
    if let Some(sess) = session_name {
        cmd.args(["-t", sess]);
    } else {
        cmd.arg("-a");
    }
    cmd.args(["-F", "#{@conversation_id}"]);

    if let Ok(output) = cmd.output() {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    bound.insert(trimmed.to_string());
                }
            }
        }
    }
    bound
}

/// Heal an unbound window only when there is unambiguously exactly 1 window in the session
/// and 1 recent conversation within the last 5 minutes, avoiding cross-window guessing.
pub fn heal_unbound_windows(project: &str) -> Result<()> {
    if std::env::var("TMUX").is_err() {
        return Ok(());
    }

    let session_name = format!("agy-{}", project);
    let output = Command::new("tmux")
        .args([
            "list-windows",
            "-t",
            &session_name,
            "-F",
            "#{window_id}\t#{window_name}\t#{@conversation_id}",
        ])
        .output();

    let out = match output {
        Ok(o) if o.status.success() => o,
        _ => return Ok(()),
    };

    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut total_windows = 0;
    let mut unbound_windows = Vec::new();
    let mut bound_ids = std::collections::HashSet::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 3 {
            total_windows += 1;
            let win_id = parts[0];
            let win_name = parts[1];
            let conv_id = parts[2];
            if !conv_id.is_empty() {
                bound_ids.insert(conv_id.to_string());
            } else if win_name == "+ new" || win_name == "agy" || win_name.starts_with("+ new") {
                unbound_windows.push(win_id.to_string());
            }
        }
    }

    // Only heal if there is unambiguously exactly 1 window in the session and it is unbound.
    // Avoid guessing or popping across multiple windows which causes title swapping.
    if total_windows != 1 || unbound_windows.len() != 1 {
        return Ok(());
    }

    let win_id = &unbound_windows[0];

    let db = db_path();
    if !db.exists() {
        return Ok(());
    }

    if let Ok(conn) = rusqlite::Connection::open(&db) {
        let hygiene =
            crate::core::session::session_hygiene_conditions(&conn, "conversation_summaries");
        let hygiene_and = if hygiene.is_empty() {
            String::new()
        } else {
            format!(" AND {}", hygiene.join(" AND "))
        };
        let pattern = format!("%{}%", project);
        // Only consider conversation updated within the last 5 minutes (300 seconds)
        let sql = format!(
            "SELECT conversation_id, title FROM conversation_summaries 
             WHERE (workspace_uris LIKE ?1 OR project_id = ?2) 
               AND title IS NOT NULL AND title != ''
               AND (strftime('%s', 'now') - strftime('%s', last_modified_time)) < 300{} 
             ORDER BY last_modified_time DESC LIMIT 1",
            hygiene_and
        );

        if let Ok(mut stmt) = conn.prepare(&sql) {
            if let Ok((cid, title)) = stmt.query_row(rusqlite::params![pattern, project], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            }) {
                if !bound_ids.contains(&cid) {
                    let _ = tmux::TmuxDriver::set_window_option(win_id, "@conversation_id", &cid);
                    let clean = sanitize_title(&title);
                    let _ = tmux::TmuxDriver::rename_window(win_id, &clean);
                    let _ = crate::core::TabRegistry::bind_static(win_id, &cid);
                }
            }
        }
    }

    Ok(())
}

fn run_bind(project: &str, dir: &Path, conversation_id: Option<String>) -> Result<()> {
    let _ = heal_unbound_windows(project);

    let target_cid = match conversation_id {
        Some(cid) => cid,
        None => {
            // Open interactive conversation picker in bind mode
            let picked = ui::run_picker_for_bind(
                cli::ScopeArg::Local,
                project.to_string(),
                dir.to_path_buf(),
            )?;
            match picked {
                Some(cid) => cid,
                None => return Ok(()),
            }
        }
    };

    let db = db_path();
    let title = if db.exists() {
        rusqlite::Connection::open(&db).ok().and_then(|conn| {
            conn.query_row(
                "SELECT title FROM conversation_summaries WHERE conversation_id = ?1 LIMIT 1",
                [&target_cid],
                |r| r.get::<_, String>(0),
            )
            .ok()
        })
    } else {
        None
    };

    let title_str = title.as_deref().unwrap_or("agy");
    let clean = sanitize_title(title_str);

    if std::env::var("TMUX").is_ok() {
        let current_win_id = if let Ok(pane_id) = std::env::var("TMUX_PANE") {
            Command::new("tmux")
                .args(["display-message", "-p", "-t", &pane_id, "#{window_id}"])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        } else {
            Command::new("tmux")
                .args(["display-message", "-p", "#{window_id}"])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        };

        let target_win = current_win_id.as_deref().unwrap_or("");
        let _ = tmux::TmuxDriver::set_window_option(target_win, "@conversation_id", &target_cid);
        let _ = tmux::TmuxDriver::rename_window(target_win, &clean);
        if let Some(ref win_id) = current_win_id {
            let _ = crate::core::TabRegistry::bind_static(win_id, &target_cid);
        }
        let toast = format!("Bound tab to: {} ({})", clean, target_cid);
        let _ = Command::new("tmux")
            .args(["display-message", &toast])
            .status();
    } else {
        println!("Bound tab to: {} ({})", clean, target_cid);
    }

    Ok(())
}

pub fn db_path() -> PathBuf {
    if let Ok(custom) = std::env::var("AGYMUX_DB_PATH") {
        PathBuf::from(custom)
    } else {
        dirs::home_dir()
            .unwrap_or_default()
            .join(".gemini/antigravity-cli/conversation_summaries.db")
    }
}

pub fn sanitize_title(raw: &str) -> String {
    let clean: String = raw
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '_' || *c == '-' || *c == '.')
        .collect();
    let trimmed = clean.trim();
    if trimmed.is_empty() {
        "agy".to_string()
    } else {
        let truncated: String = trimmed.chars().take(30).collect();
        let res = truncated.trim_end().to_string();
        if res.is_empty() {
            "agy".to_string()
        } else {
            res
        }
    }
}

/// Check if an active remote-control instance is running elsewhere on this machine
pub fn is_remote_control_active() -> bool {
    let my_pid = std::process::id();
    if is_agy_process_remote_control_active(my_pid) {
        return true;
    }
    if is_tmux_remote_control_active(my_pid) {
        return true;
    }
    false
}

/// Helper function to resolve effective remote control state based on request, config, and liveness
pub fn resolve_effective_remote_control(
    requested: bool,
    single_instance: bool,
    is_active: bool,
) -> bool {
    if requested && single_instance && is_active {
        false
    } else {
        requested
    }
}

#[cfg(target_os = "linux")]
fn is_agy_process_remote_control_active(my_pid: u32) -> bool {
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return false;
    };
    for entry in entries.flatten() {
        let Ok(file_name) = entry.file_name().into_string() else {
            continue;
        };
        let Ok(pid) = file_name.parse::<u32>() else {
            continue;
        };
        if pid == my_pid {
            continue;
        }
        let proc_path = entry.path();
        let cmdline_path = proc_path.join("cmdline");
        let Ok(cmdline_bytes) = std::fs::read(&cmdline_path) else {
            continue;
        };
        if cmdline_bytes.is_empty() {
            continue;
        }
        let args: Vec<&str> = cmdline_bytes
            .split(|&b| b == 0)
            .filter_map(|slice| std::str::from_utf8(slice).ok())
            .filter(|s| !s.is_empty())
            .collect();
        if args.is_empty() {
            continue;
        }
        let is_agy = if let Ok(exe) = std::fs::read_link(proc_path.join("exe")) {
            exe.file_name()
                .is_some_and(|name| name == "agy" || name.to_string_lossy().starts_with("agy"))
        } else {
            std::path::Path::new(args[0])
                .file_name()
                .is_some_and(|name| name == "agy" || name.to_string_lossy().starts_with("agy"))
        };
        if is_agy
            && args
                .iter()
                .any(|arg| *arg == "--remote-control" || arg.starts_with("--remote-control="))
        {
            return true;
        }
    }
    false
}

#[cfg(not(target_os = "linux"))]
fn is_agy_process_remote_control_active(my_pid: u32) -> bool {
    if let Ok(output) = Command::new("pgrep")
        .args(["-f", "agy.*--remote-control"])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if let Ok(pid) = line.trim().parse::<u32>() {
                    if pid != my_pid {
                        return true;
                    }
                }
            }
        }
    } else if let Ok(output) = Command::new("ps").args(["-eo", "pid,args"]).output() {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let mut parts = line.split_whitespace();
                if let Some(pid_str) = parts.next() {
                    if let Ok(pid) = pid_str.parse::<u32>() {
                        if pid == my_pid {
                            continue;
                        }
                        let rest: Vec<&str> = parts.collect();
                        if let Some(cmd) = rest.first() {
                            let is_agy = std::path::Path::new(cmd).file_name().is_some_and(|n| {
                                n == "agy" || n.to_string_lossy().starts_with("agy")
                            });
                            if is_agy
                                && rest.iter().any(|a| {
                                    *a == "--remote-control" || a.starts_with("--remote-control=")
                                })
                            {
                                return true;
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

fn is_tmux_remote_control_active(my_pid: u32) -> bool {
    let current_pane_pid = if std::env::var("TMUX").is_ok() {
        Command::new("tmux")
            .args(["display-message", "-p", "#{pane_pid}"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .and_then(|s| s.trim().parse::<u32>().ok())
    } else {
        None
    };

    let Ok(output) = Command::new("tmux")
        .args([
            "list-windows",
            "-a",
            "-F",
            "#{window_id}:#{pane_pid}:#{@remote_control}:#{@harness}:#{pane_current_command}",
        ])
        .output()
    else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let parts: Vec<&str> = trimmed.split(':').collect();
        if parts.len() >= 3 {
            let win_id = parts[0].trim();
            let pane_pid_str = parts[1].trim();
            let rc_val = parts[2].trim();
            let harness = if parts.len() >= 4 { parts[3].trim() } else { "" };
            let current_cmd = if parts.len() >= 5 { parts[4].trim() } else { "" };

            if rc_val != "1" {
                continue;
            }

            let is_non_agy_harness =
                !harness.is_empty() && harness != "agy" && harness != "antigravity";
            let is_non_agy_cmd =
                !current_cmd.is_empty() && !current_cmd.starts_with("agy") && current_cmd != "hx";

            if is_non_agy_harness || is_non_agy_cmd {
                // Auto-heal stale or mistakenly assigned remote_control flag on non-agy windows
                let _ = Command::new("tmux")
                    .args(["set-window-option", "-t", win_id, "@remote_control", "0"])
                    .status();
                continue;
            }

            if let Ok(pid) = pane_pid_str.parse::<u32>() {
                if pid != my_pid && Some(pid) != current_pane_pid && is_process_alive(pid) {
                    return true;
                }
            }
        }
    }
    false
}

fn is_process_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        let res = unsafe { libc::kill(pid as libc::pid_t, 0) };
        if res == 0 {
            true
        } else {
            std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
        }
    }
    #[cfg(not(unix))]
    {
        false
    }
}

/// Build arguments forwarded to agy.
/// Ensures project is not duplicated, -c vs --conversation semantics are respected
/// (agy takes --conversation <id> for specific conversations, and boolean -c for latest),
/// and --remote-control is forwarded appropriately.
pub fn build_agy_args(
    project: &str,
    args: &[String],
    conv_id: Option<&str>,
    has_continue_flag: bool,
    remote_control: bool,
) -> Vec<String> {
    let mut forward_args = vec!["--project".to_string(), project.to_string()];
    let mut i = 0;
    let mut has_remote_control = false;

    while i < args.len() {
        let a = &args[i];
        if a == "--project" || a == "-p" {
            i += 1;
            if i < args.len() && !args[i].starts_with('-') {
                i += 1;
            }
            continue;
        }
        if a == "--remote-control" || a.starts_with("--remote-control=") {
            if remote_control {
                has_remote_control = true;
                forward_args.push(a.clone());
            }
            i += 1;
            continue;
        }
        if a == "-c" || a == "--conversation" || a == "--continue" || a == "--resume" || a == "-r" {
            if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                i += 2;
            } else {
                i += 1;
            }
            continue;
        }
        forward_args.push(a.clone());
        i += 1;
    }

    if let Some(cid) = conv_id {
        forward_args.push("--conversation".to_string());
        forward_args.push(cid.to_string());
    } else if has_continue_flag {
        forward_args.push("-c".to_string());
    }

    if remote_control && !has_remote_control {
        forward_args.push("--remote-control".to_string());
    }

    forward_args
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunnerParsedArgs {
    pub conv_id: Option<String>,
    pub initial_prompt: Option<String>,
    pub has_continue_flag: bool,
    pub filtered_args: Vec<String>,
}

/// Parse runner arguments for conversation id, resume flags, initial prompt, and filtered arguments
pub fn parse_runner_args(args: &[String]) -> RunnerParsedArgs {
    let mut conv_id = None;
    let mut initial_prompt = None;
    let mut has_continue_flag = false;
    let mut filtered_args = Vec::new();

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if arg == "--project" || arg == "-p" {
            filtered_args.push(arg.clone());
            i += 1;
            if i < args.len() && !args[i].starts_with('-') {
                filtered_args.push(args[i].clone());
                i += 1;
            }
        } else if arg == "--conversation" || arg == "-c" || arg == "--continue" {
            has_continue_flag = true;
            filtered_args.push(arg.clone());
            i += 1;
            if i < args.len() && !args[i].starts_with('-') {
                if conv_id.is_none() {
                    conv_id = Some(args[i].clone());
                }
                filtered_args.push(args[i].clone());
                i += 1;
            }
        } else if let Some(val) = arg.strip_prefix("--conversation=") {
            has_continue_flag = true;
            if conv_id.is_none() && !val.is_empty() {
                conv_id = Some(val.to_string());
            }
            filtered_args.push(arg.clone());
            i += 1;
        } else if arg == "--resume" || arg == "-r" {
            has_continue_flag = true;
            i += 1;
            if i < args.len() && !args[i].starts_with('-') {
                if conv_id.is_none() {
                    conv_id = Some(args[i].clone());
                }
                // Strip --resume / -r <id> from filtered_args so it does not get passed redundantly
                // or mistakenly as an initial prompt
                i += 1;
            }
        } else if let Some(val) = arg.strip_prefix("--resume=") {
            has_continue_flag = true;
            if conv_id.is_none() && !val.is_empty() {
                conv_id = Some(val.to_string());
            }
            i += 1;
        } else {
            if !arg.starts_with('-') && initial_prompt.is_none() {
                initial_prompt = Some(arg.clone());
            }
            filtered_args.push(arg.clone());
            i += 1;
        }
    }

    RunnerParsedArgs {
        conv_id,
        initial_prompt,
        has_continue_flag,
        filtered_args,
    }
}

/// Internal process runner with instant prompt-based window naming and title watcher
fn run_runner(
    project: &str,
    name: Option<String>,
    args: Vec<String>,
    remote_control: bool,
    harness: &str,
    config: &Config,
) -> Result<()> {
    let agent_harness = crate::core::AgentHarness::from_str_loose(harness);
    let in_tmux = std::env::var("TMUX").is_ok();

    let parsed = parse_runner_args(&args);
    let mut conv_id = parsed.conv_id;
    let initial_prompt = parsed.initial_prompt;
    let has_continue_flag = parsed.has_continue_flag;
    let filtered_args = parsed.filtered_args;

    let latest_existing_id = if agent_harness == crate::core::AgentHarness::Antigravity {
        let db = db_path();
        if db.exists() {
            rusqlite::Connection::open(&db)
                .ok()
                .and_then(|conn| crate::core::session::latest_conversation_id(&conn, Some(project)))
        } else {
            None
        }
    } else {
        None
    };

    // If -c was supplied without an argument, resolve to latest_existing_id
    if has_continue_flag && conv_id.is_none() {
        conv_id = latest_existing_id.clone();
    }

    let is_agy = agent_harness == crate::core::AgentHarness::Antigravity;
    let requested_rc =
        is_agy && (remote_control || args.contains(&"--remote-control".to_string()));
    let mut effective_remote_control = requested_rc;

    if requested_rc && config.general.single_remote_control_instance && is_remote_control_active() {
        effective_remote_control = false;
        if in_tmux {
            let _ = Command::new("tmux")
                .args([
                    "display-message",
                    "hx: Active remote-control instance already running on this machine (skipping --remote-control)",
                ])
                .status();
        } else {
            eprintln!("hx: Active remote-control instance already running on this machine (skipping --remote-control)");
        }
    }

    // 1. Instant window naming and capture window id
    let target_window = if in_tmux {
        if let Ok(pane_id) = std::env::var("TMUX_PANE") {
            Command::new("tmux")
                .args(["display-message", "-p", "-t", &pane_id, "#{window_id}"])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        } else {
            Command::new("tmux")
                .args(["display-message", "-p", "#{window_id}"])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        }
    } else {
        None
    };

    if in_tmux {
        let rc_val = if effective_remote_control { "1" } else { "0" };
        let win_target = target_window.as_deref().unwrap_or("");
        let _ = tmux::TmuxDriver::set_window_option(win_target, "@remote_control", rc_val);
        let _ = tmux::TmuxDriver::set_window_option(win_target, "@harness", agent_harness.as_str());

        // Set @harness at the pane level so split panes in the same window show their own harness
        if let Ok(pane_id) = std::env::var("TMUX_PANE") {
            let _ = Command::new("tmux")
                .args([
                    "set-option",
                    "-p",
                    "-t",
                    &pane_id,
                    "@harness",
                    agent_harness.as_str(),
                ])
                .status();
        } else {
            let _ = Command::new("tmux")
                .args(["set-option", "-p", "@harness", agent_harness.as_str()])
                .status();
        }

        // Check if user has explicitly named/renamed the tab
        let is_user_renamed = if let Some(ref custom_name) = name {
            let _ = tmux::TmuxDriver::rename_window(win_target, custom_name);
            let _ = tmux::TmuxDriver::set_window_option(win_target, "@user_renamed", "1");
            true
        } else {
            let out = Command::new("tmux")
                .args([
                    "display-message",
                    "-p",
                    "-t",
                    win_target,
                    "#{@user_renamed}",
                ])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
                .unwrap_or_default();
            out == "1"
        };

        if let Some(ref cid) = conv_id {
            let _ = tmux::TmuxDriver::set_window_option(win_target, "@conversation_id", cid);

            // Set @conversation_id on pane as well
            if let Ok(pane_id) = std::env::var("TMUX_PANE") {
                let _ = Command::new("tmux")
                    .args(["set-option", "-p", "-t", &pane_id, "@conversation_id", cid])
                    .status();
            } else {
                let _ = Command::new("tmux")
                    .args(["set-option", "-p", "@conversation_id", cid])
                    .status();
            }

            let win_to_bind = target_window.as_deref().unwrap_or(win_target);
            if !win_to_bind.is_empty() {
                let _ = crate::core::TabRegistry::bind_with_harness_static(
                    win_to_bind,
                    cid,
                    agent_harness.clone(),
                );
            }

            if !is_user_renamed {
                let db = db_path();
                if db.exists() {
                    if let Ok(conn) = rusqlite::Connection::open(&db) {
                        let stmt = conn
                            .prepare("SELECT title FROM conversation_summaries WHERE conversation_id = ?1 LIMIT 1")
                            .ok();
                        if let Some(mut s) = stmt {
                            if let Ok(title) = s.query_row([cid], |r| r.get::<_, String>(0)) {
                                let clean_title = sanitize_title(&title);
                                let _ = tmux::TmuxDriver::rename_window(win_target, &clean_title);
                            }
                        }
                    }
                }
            }
        } else if let Some(ref prompt) = initial_prompt {
            if !is_user_renamed {
                let clean_prompt = sanitize_title(prompt);
                let _ = tmux::TmuxDriver::rename_window(win_target, &clean_prompt);
            }
            if let Some(ref win) = target_window {
                let _ = crate::core::TabRegistry::set_harness_static(win, agent_harness.clone());
            }
        } else {
            if !is_user_renamed {
                let initial_name = match agent_harness {
                    crate::core::AgentHarness::PlainTerminal => "term",
                    _ => "+ new",
                };
                let _ = tmux::TmuxDriver::rename_window(win_target, initial_name);
            }
            if let Some(ref win) = target_window {
                let _ = crate::core::TabRegistry::set_harness_static(win, agent_harness.clone());
            }
        }
    }

    // 2. Background detached watcher retired: turn-driven tab titles are updated via agymux hook on Stop

    // 3. Assemble and execute harness command
    let registry = crate::core::harness::HarnessRegistry::new_with_config(config);
    let ctx = crate::core::harness::HarnessLaunchContext {
        project: project.to_string(),
        conversation_id: conv_id,
        remote_control: effective_remote_control,
        continue_flag: has_continue_flag,
        args: filtered_args,
        workdir: None,
        env: std::collections::HashMap::new(),
    };

    let mut cmd = if let Some(driver) = registry.get_for_harness(&agent_harness) {
        driver.build_command(&ctx)
    } else {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "sh".to_string());
        let mut c = Command::new(shell);
        if !ctx.args.is_empty() {
            c.args(&ctx.args);
        }
        c
    };

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let err = cmd.exec();
        eprintln!("Failed to execute {}: {}", agent_harness.as_str(), err);
        std::process::exit(1);
    }

    #[cfg(not(unix))]
    {
        let status = cmd.status();
        let code = match status {
            Ok(s) => s.code().unwrap_or(0),
            Err(e) => {
                eprintln!("Failed to execute agy: {}", e);
                1
            }
        };
        std::process::exit(code);
    }
}

/// Print formatted transcript preview to stdout
fn run_preview(conv_id: &str) -> Result<()> {
    let summary = match core::transcript::TranscriptParser::parse(conv_id) {
        Ok(s) => s,
        Err(e) => {
            println!(
                "\x1b[1;31m[!] Transcript not found for conversation:\x1b[0m {} ({})",
                conv_id, e
            );
            return Ok(());
        }
    };

    let total = summary.total_input_tokens + summary.total_output_tokens;
    println!("\x1b[1;36m━━━ Conversation Details ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\x1b[0m");
    println!("\x1b[1;33mID:\x1b[0m     {}", conv_id);
    println!(
        "\x1b[1;33mTokens:\x1b[0m Input: \x1b[1;32m{}\x1b[0m | Output: \x1b[1;32m{}\x1b[0m (Total: {})",
        format_number_cli(summary.total_input_tokens),
        format_number_cli(summary.total_output_tokens),
        format_number_cli(total)
    );
    println!(
        "\x1b[1;33mTools:\x1b[0m  {} executed\n",
        summary.tools_used.len()
    );

    println!(
        "\x1b[1;36m━━━ Initial User Prompt ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\x1b[0m"
    );
    if !summary.first_prompt.is_empty() {
        println!("{}", summary.first_prompt);
    } else {
        println!("\x1b[0;90m(No initial prompt recorded)\x1b[0m");
    }
    println!();

    println!(
        "\x1b[1;36m━━━ Latest Assistant Turn ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\x1b[0m"
    );
    if !summary.last_response.is_empty() {
        println!("{}", summary.last_response);
    } else {
        println!("\x1b[0;90m(No assistant responses yet)\x1b[0m");
    }
    println!();

    println!(
        "\x1b[1;36m━━━ Executed Tools Breakdown ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\x1b[0m"
    );
    if !summary.tools_used.is_empty() {
        for t in &summary.tools_used {
            println!("  \x1b[0;32m●\x1b[0m \x1b[1m{}\x1b[0m", t);
        }
    } else {
        println!("  \x1b[0;90mNone\x1b[0m");
    }

    Ok(())
}

fn format_number_cli(num: u64) -> String {
    if num >= 1_000_000 {
        format!("{:.1}M", num as f64 / 1_000_000.0)
    } else if num >= 1_000 {
        format!("{:.1}k", num as f64 / 1_000.0)
    } else {
        num.to_string()
    }
}

/// List active hx sessions
fn run_ls() -> Result<()> {
    let output = Command::new("tmux")
        .args([
            "list-sessions",
            "-F",
            "#{session_name} (#{?session_attached,attached,detached})",
        ])
        .output();

    let mut active = Vec::new();
    if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines() {
            if line.starts_with("agy-") {
                active.push(line.to_string());
            }
        }
    }

    if active.is_empty() {
        println!("No active hx sessions found.");
    } else {
        println!("Active hx sessions:");
        for s in active {
            println!("  {}", s);
        }
    }

    Ok(())
}

/// Restore session tabs and split layouts from saved snapshot for target project
pub fn execute_restore(target_project: &str) -> Result<()> {
    match crate::core::restore_session_layout(target_project) {
        Ok(true) => {
            println!("Restored session layout for '{}'", target_project);
            let target_session = if target_project.starts_with("agy-") {
                target_project.to_string()
            } else {
                format!("agy-{}", target_project)
            };
            if std::env::var("TMUX").is_ok() {
                tmux::TmuxDriver::switch_client(&target_session)?;
            } else {
                tmux::TmuxDriver::attach_session(&target_session)?;
            }
        }
        Ok(false) => {
            println!(
                "No saved session snapshot found for '{}' (or session already active)",
                target_project
            );
        }
        Err(e) => {
            eprintln!("Failed to restore session snapshot: {}", e);
        }
    }
    Ok(())
}

/// Browse and restore session history
pub fn run_session_history(target_project: &str) -> Result<()> {
    use std::io::IsTerminal;
    if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
        if let Some(selected_snap) = ui::run_session_history_picker(target_project)? {
            match crate::core::restore_session_snapshot(target_project, &selected_snap) {
                Ok(true) => {
                    println!("Restored session snapshot for '{}'", target_project);
                    let target_session = if target_project.starts_with("agy-") {
                        target_project.to_string()
                    } else {
                        format!("agy-{}", target_project)
                    };
                    if std::env::var("TMUX").is_ok() {
                        tmux::TmuxDriver::switch_client(&target_session)?;
                    } else {
                        tmux::TmuxDriver::attach_session(&target_session)?;
                    }
                }
                Ok(false) => {
                    println!("Failed to restore session snapshot");
                }
                Err(e) => {
                    eprintln!("Failed to restore session snapshot: {}", e);
                }
            }
        }
    } else {
        let history = crate::core::list_session_history(target_project)?;
        if history.is_empty() {
            println!("No session history found for '{}'", target_project);
        } else {
            println!(
                "Session history for '{}' ({} entries):",
                target_project,
                history.len()
            );
            for snap in history {
                let rel = crate::core::session::format_relative_time(snap.timestamp);
                let label = snap
                    .name
                    .as_deref()
                    .map(|n| format!(" [{}]", n))
                    .unwrap_or_default();
                println!(
                    "  {} ({} tabs, saved {}){}",
                    snap.timestamp,
                    snap.windows.len(),
                    rel,
                    label
                );
            }
        }
    }
    Ok(())
}

/// List available session layout snapshots saved on disk
pub fn run_session_ls(project_filter: Option<&str>) -> Result<()> {
    if let Some(proj) = project_filter {
        let history = crate::core::list_session_history(proj)?;
        if history.is_empty() {
            println!("No saved session snapshots found for '{}'", proj);
        } else {
            println!(
                "Saved session snapshots for '{}' ({}):",
                proj,
                history.len()
            );
            for snap in history {
                let rel = crate::core::session::format_relative_time(snap.timestamp);
                let label = snap
                    .name
                    .as_deref()
                    .map(|n| format!(" [{}]", n))
                    .unwrap_or_default();
                println!(
                    "  {} ({} windows, saved {}){}",
                    snap.timestamp,
                    snap.windows.len(),
                    rel,
                    label
                );
            }
        }
        return Ok(());
    }

    let dir = crate::core::sessions_dir();
    if !dir.exists() {
        println!("No saved session snapshots found.");
        return Ok(());
    }

    let mut snapshots = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(snap) = serde_json::from_str::<crate::core::SessionSnapshot>(&content)
                    {
                        snapshots.push(snap);
                    }
                }
            }
        }
    }

    if snapshots.is_empty() {
        println!("No saved session snapshots found in {}", dir.display());
    } else {
        snapshots.sort_by_key(|a| std::cmp::Reverse(a.timestamp));
        println!("Saved session snapshots ({}):", dir.display());
        for snap in snapshots {
            let rel = crate::core::session::format_relative_time(snap.timestamp);
            let label = snap
                .name
                .as_deref()
                .map(|n| format!(" [{}]", n))
                .unwrap_or_default();
            println!(
                "  {} ({} windows, saved {}){}",
                snap.project,
                snap.windows.len(),
                rel,
                label
            );
        }
    }
    Ok(())
}

/// Check if a workspace URI list matches the current tmux pane path
fn check_workspace_matches(raw_ws: &str, pane_path: &str) -> bool {
    let clean_pane = pane_path.trim().trim_end_matches('/');
    if clean_pane.is_empty() {
        return true;
    }

    if let Ok(serde_json::Value::Array(arr)) = serde_json::from_str::<serde_json::Value>(raw_ws) {
        if arr.is_empty() {
            return true;
        }
        for item in arr {
            if let Some(s) = item.as_str() {
                let p = crate::core::session::url_decode(s.strip_prefix("file://").unwrap_or(s));
                let clean_p = p.trim().trim_end_matches('/');
                if !clean_p.is_empty()
                    && (clean_pane == clean_p
                        || clean_pane.starts_with(&format!("{}/", clean_p))
                        || clean_p.starts_with(&format!("{}/", clean_pane)))
                {
                    return true;
                }
            }
        }
        return false;
    }

    let p = crate::core::session::url_decode(raw_ws.strip_prefix("file://").unwrap_or(raw_ws));
    let clean_p = p.trim().trim_end_matches('/');
    if clean_p.is_empty() {
        return true;
    }

    clean_pane == clean_p
        || clean_pane.starts_with(&format!("{}/", clean_p))
        || clean_p.starts_with(&format!("{}/", clean_pane))
}

/// Lifecycle hook invoked by Antigravity or harnesses on turn events to synchronize tab titles and running status
pub fn run_hook(running: Option<&str>, start: bool) -> Result<()> {
    use std::io::{IsTerminal, Read};

    let is_running = start
        || matches!(
            running,
            Some("1") | Some("true") | Some("on") | Some("start")
        );
    let running_val = if is_running { "1" } else { "0" };

    let mut input = String::new();
    if !std::io::stdin().is_terminal() {
        let _ = std::io::stdin().read_to_string(&mut input);
    }

    let mut resolved_win: Option<String> = None;
    let mut resolved_clean_title: Option<String> = None;

    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&input) {
        let conv_id = val
            .get("conversationId")
            .or_else(|| val.get("conversation_id"))
            .or_else(|| val.get("session_id"))
            .or_else(|| val.get("sessionId"))
            .and_then(|v| v.as_str());

        if let Some(cid) = conv_id {
            // Strategy 1: Check list-windows for matching @conversation_id (Live Tmux Options)
            if let Ok(out) = Command::new("tmux")
                .args(["list-windows", "-F", "#{window_id}:#{@conversation_id}"])
                .output()
            {
                if out.status.success() {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    for line in stdout.lines() {
                        let parts: Vec<&str> = line.splitn(2, ':').collect();
                        if parts.len() == 2 {
                            let win_id = parts[0].trim();
                            let win_cid = parts[1].trim();
                            if win_cid == cid {
                                resolved_win = Some(win_id.to_string());
                                break;
                            }
                        }
                    }
                }
            }

            // Strategy 2: Check TabRegistry cross-reference map
            if resolved_win.is_none() {
                if let Some(reg_win) = crate::core::TabRegistry::get_window_for_conv_static(cid) {
                    // Verify this window actually exists in tmux
                    let exists = Command::new("tmux")
                        .args(["display-message", "-p", "-t", &reg_win, "#{window_id}"])
                        .output()
                        .ok()
                        .and_then(|o| {
                            if o.status.success() {
                                String::from_utf8(o.stdout).ok()
                            } else {
                                None
                            }
                        })
                        .map(|s| s.trim() == reg_win)
                        .unwrap_or(false);

                    if exists {
                        // Check if that window's current @conversation_id is set to something else
                        let current_cid = Command::new("tmux")
                            .args([
                                "display-message",
                                "-p",
                                "-t",
                                &reg_win,
                                "#{@conversation_id}",
                            ])
                            .output()
                            .ok()
                            .and_then(|o| {
                                if o.status.success() {
                                    String::from_utf8(o.stdout).ok()
                                } else {
                                    None
                                }
                            })
                            .map(|s| s.trim().to_string())
                            .unwrap_or_default();

                        if !current_cid.is_empty() && current_cid != cid {
                            let _ = crate::core::TabRegistry::unbind_window_static(&reg_win);
                        } else {
                            resolved_win = Some(reg_win);
                        }
                    } else {
                        let _ = crate::core::TabRegistry::unbind_window_static(&reg_win);
                    }
                }
            }

            let db = db_path();
            if db.exists() {
                if let Ok(conn) = rusqlite::Connection::open(&db) {
                    type HookSummaryRow =
                        (Option<String>, Option<String>, Option<i64>, Option<String>);
                    let summary_res: rusqlite::Result<HookSummaryRow> = conn.query_row(
                        "SELECT title, parent_conversation_id, nesting_depth, workspace_uris FROM conversation_summaries WHERE conversation_id = ?1 LIMIT 1",
                        [cid],
                        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
                    );

                    let summary = match summary_res {
                        Ok(s) => Some(s),
                        Err(_) => conn
                            .query_row(
                                "SELECT title FROM conversation_summaries WHERE conversation_id = ?1 LIMIT 1",
                                [cid],
                                |r| Ok((r.get(0)?, None::<String>, None::<i64>, None::<String>)),
                            )
                            .ok(),
                    };

                    if let Some((title_opt, parent_id_opt, depth_opt, ws_uris_opt)) = summary {
                        // Filter out subagents: child/nested conversations must never hijack parent tab titles
                        let is_subagent = parent_id_opt
                            .as_deref()
                            .map(|s| !s.trim().is_empty())
                            .unwrap_or(false)
                            || depth_opt.unwrap_or(0) > 0;

                        if is_subagent {
                            println!("{{}}");
                            return Ok(());
                        }

                        if let Some(t) = title_opt {
                            if !t.trim().is_empty() {
                                resolved_clean_title = Some(sanitize_title(&t));
                            }
                        }

                        // Strategy 3: Safe TMUX_PANE fallback
                        if resolved_win.is_none() {
                            if let Ok(pane_id) = std::env::var("TMUX_PANE") {
                                if !pane_id.trim().is_empty() {
                                    let w_id_opt = Command::new("tmux")
                                        .args([
                                            "display-message",
                                            "-p",
                                            "-t",
                                            &pane_id,
                                            "#{window_id}",
                                        ])
                                        .output()
                                        .ok()
                                        .and_then(|o| {
                                            if o.status.success() {
                                                String::from_utf8(o.stdout).ok()
                                            } else {
                                                None
                                            }
                                        })
                                        .map(|s| s.trim().to_string())
                                        .filter(|s| !s.is_empty());

                                    if let Some(w_id) = w_id_opt {
                                        let win_cid = Command::new("tmux")
                                            .args([
                                                "display-message",
                                                "-p",
                                                "-t",
                                                &w_id,
                                                "#{@conversation_id}",
                                            ])
                                            .output()
                                            .ok()
                                            .and_then(|o| {
                                                if o.status.success() {
                                                    String::from_utf8(o.stdout).ok()
                                                } else {
                                                    None
                                                }
                                            })
                                            .map(|s| s.trim().to_string())
                                            .unwrap_or_default();

                                        let cid_conflicts = !win_cid.is_empty() && win_cid != cid;
                                        let reg_cid =
                                            crate::core::TabRegistry::get_conv_for_window_static(
                                                &w_id,
                                            )
                                            .unwrap_or_default();
                                        let reg_conflicts = !reg_cid.is_empty() && reg_cid != cid;

                                        if !cid_conflicts && !reg_conflicts {
                                            let mut workspace_matches = true;
                                            if let Some(ref ws_str) = ws_uris_opt {
                                                let trimmed = ws_str.trim();
                                                if !trimmed.is_empty()
                                                    && trimmed != "[]"
                                                    && trimmed != "null"
                                                {
                                                    let pane_path = Command::new("tmux")
                                                        .args([
                                                            "display-message",
                                                            "-p",
                                                            "-t",
                                                            &pane_id,
                                                            "#{pane_current_path}",
                                                        ])
                                                        .output()
                                                        .ok()
                                                        .and_then(|o| {
                                                            if o.status.success() {
                                                                String::from_utf8(o.stdout).ok()
                                                            } else {
                                                                None
                                                            }
                                                        })
                                                        .map(|s| s.trim().to_string())
                                                        .unwrap_or_default();

                                                    if !pane_path.is_empty() {
                                                        workspace_matches = check_workspace_matches(
                                                            trimmed, &pane_path,
                                                        );
                                                    }
                                                }
                                            }

                                            if workspace_matches {
                                                resolved_win = Some(w_id);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if let Some(ref target_win) = resolved_win {
                let _ = crate::core::TabRegistry::bind_static(target_win, cid);
                let _ = tmux::TmuxDriver::set_window_option(target_win, "@conversation_id", cid);
            }
        }
    }

    // Determine effective target window
    let target_win_opt = resolved_win.or_else(|| {
        std::env::var("TMUX_PANE")
            .ok()
            .filter(|p| !p.trim().is_empty())
            .and_then(|pane| {
                Command::new("tmux")
                    .args(["display-message", "-p", "-t", &pane, "#{window_id}"])
                    .output()
                    .ok()
                    .and_then(|o| {
                        if o.status.success() {
                            String::from_utf8(o.stdout).ok()
                        } else {
                            None
                        }
                    })
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
            })
            .or_else(|| {
                Command::new("tmux")
                    .args(["display-message", "-p", "#{window_id}"])
                    .output()
                    .ok()
                    .and_then(|o| {
                        if o.status.success() {
                            String::from_utf8(o.stdout).ok()
                        } else {
                            None
                        }
                    })
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
            })
    });

    if let Some(target_win) = target_win_opt {
        let _ = tmux::TmuxDriver::set_window_option(&target_win, "@running", running_val);
        let _ = tmux::TmuxDriver::set_window_option(&target_win, "@claude_running", running_val);
        if running_val == "0" {
            let _ = tmux::TmuxDriver::set_window_option(&target_win, "@agy_running", "0");
        } else {
            let _ = tmux::TmuxDriver::set_window_option(&target_win, "@agy_running", "1");
        }

        if let Some(clean) = resolved_clean_title {
            let user_renamed = Command::new("tmux")
                .args([
                    "display-message",
                    "-p",
                    "-t",
                    &target_win,
                    "#{@user_renamed}",
                ])
                .output()
                .ok()
                .and_then(|o| {
                    if o.status.success() {
                        String::from_utf8(o.stdout).ok()
                    } else {
                        None
                    }
                })
                .map(|s| s.trim().to_string())
                .unwrap_or_default();

            if user_renamed != "1" {
                let _ = tmux::TmuxDriver::rename_window(&target_win, &clean);
            }
        }
    }

    // Auto-save session layout snapshot
    let active_proj = Command::new("tmux")
        .args([
            "display-message",
            "-p",
            "#{?#{@hx_project},#{@hx_project},#{@ax_project}}",
        ])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout).ok()
            } else {
                None
            }
        })
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            Command::new("tmux")
                .args(["display-message", "-p", "#{session_name}"])
                .output()
                .ok()
                .and_then(|o| {
                    if o.status.success() {
                        String::from_utf8(o.stdout).ok()
                    } else {
                        None
                    }
                })
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .map(|s| s.strip_prefix("agy-").unwrap_or(&s).to_string())
        });

    if let Some(ref p) = active_proj {
        let _ = crate::core::save_session_layout(p);
    }

    println!("{{}}");
    Ok(())
}

/// Safely close a window/tab by id or status range, unbinding from tab registry
pub fn run_close_window(target: Option<&str>) -> Result<()> {
    let win_id = if let Some(t) = target {
        let stripped = t.strip_prefix("close:").unwrap_or(t).trim();
        if stripped.is_empty() || stripped == "close" {
            Command::new("tmux")
                .args(["display-message", "-p", "#{window_id}"])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
        } else {
            Some(stripped.to_string())
        }
    } else {
        Command::new("tmux")
            .args(["display-message", "-p", "#{window_id}"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
    };

    let Some(target_win) = win_id else {
        return Ok(());
    };

    if target_win.is_empty() {
        return Ok(());
    }

    // Determine total windows in the session containing target_win
    let total_windows = Command::new("tmux")
        .args([
            "display-message",
            "-p",
            "-t",
            &target_win,
            "#{session_windows}",
        ])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse::<usize>().ok())
        .unwrap_or(1);

    if total_windows > 1 {
        let _ = tmux::TmuxDriver::kill_window(&target_win);
        let _ = crate::core::TabRegistry::unbind_window_static(&target_win);
    } else {
        let _ = Command::new("tmux")
            .args([
                "confirm-before",
                "-p",
                "Close final tab? (y/n)",
                &format!("kill-window -t {}", target_win),
            ])
            .status();
        let _ = crate::core::TabRegistry::unbind_window_static(&target_win);
    }

    Ok(())
}

/// Rename active tab and synchronize title to backing harness conversation
pub fn run_rename(title: Option<String>, target: Option<String>, project: &str) -> Result<()> {
    use std::io::IsTerminal;

    let effective_title = match title {
        Some(t) if !t.trim().is_empty() => t.trim().to_string(),
        _ => {
            if std::io::stdin().is_terminal() {
                eprint!("Enter new tab title: ");
                let mut line = String::new();
                std::io::stdin().read_line(&mut line)?;
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    bail!("Title cannot be empty");
                }
                trimmed.to_string()
            } else {
                bail!("Title argument required when not in an interactive terminal");
            }
        }
    };

    let target_opt = target.as_deref();

    // 1. Rename window in tmux and set @user_renamed 1
    let mut rename_cmd = Command::new("tmux");
    rename_cmd.arg("rename-window");
    if let Some(t) = target_opt {
        rename_cmd.args(["-t", t]);
    }
    rename_cmd.arg(&effective_title);
    let status = rename_cmd.status()?;
    if !status.success() {
        bail!("Failed to rename tmux window");
    }

    let mut set_opt_cmd = Command::new("tmux");
    set_opt_cmd.args(["set-option", "-w"]);
    if let Some(t) = target_opt {
        set_opt_cmd.args(["-t", t]);
    }
    set_opt_cmd.args(["@user_renamed", "1"]);
    let _ = set_opt_cmd.status();

    // 2. Query target window/pane info: window_id, pane_id, @conversation_id, @harness
    let mut disp_cmd = Command::new("tmux");
    disp_cmd.args(["display-message", "-p"]);
    if let Some(t) = target_opt {
        disp_cmd.args(["-t", t]);
    }
    disp_cmd.arg("#{window_id}|#{pane_id}|#{@conversation_id}|#{?#{@harness},#{@harness},#{pane_current_command}}");
    let disp_out = disp_cmd
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_default();
    let parts: Vec<&str> = disp_out.trim().split('|').collect();
    let window_id = parts.first().copied().unwrap_or("").trim().to_string();
    let _pane_id = parts.get(1).copied().unwrap_or("").trim().to_string();
    let conv_id = parts.get(2).copied().unwrap_or("").trim().to_string();
    let harness_str = parts.get(3).copied().unwrap_or("").trim().to_string();

    let harness = crate::core::AgentHarness::from_str_loose(&harness_str);

    // 3. Sync to backing harness conversation store if conversation_id is present
    if !conv_id.is_empty() {
        if let Err(e) =
            crate::core::harness::sync_conversation_title(&harness, &conv_id, &effective_title)
        {
            eprintln!(
                "Warning: failed to synchronize title to backing harness: {}",
                e
            );
        }
    }

    // 4. Update TabRegistry if window_id is known
    if !window_id.is_empty() {
        if let Ok(mut reg) = crate::core::TabRegistry::open() {
            let _ = reg.bind_full(
                &window_id,
                &conv_id,
                Some(project),
                Some(&effective_title),
                Some(harness),
            );
        }
    }

    // 5. Persist layout snapshot
    let _ = crate::core::persistence::save_session_layout(project);

    Ok(())
}

/// Join another tab into current window as a split pane
pub fn run_join(target: Option<String>, vertical: bool) -> Result<()> {
    // Ensure destination pane explicitly retains its harness at pane level
    let cur_harness = Command::new("tmux")
        .args([
            "display-message",
            "-p",
            "#{?#{@harness},#{@harness},#{pane_current_command}}",
        ])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    if !cur_harness.is_empty() {
        let _ = Command::new("tmux")
            .args(["set-option", "-p", "@harness", &cur_harness])
            .status();
    }

    if let Some(t) = target {
        let split_flag = if vertical { "-v" } else { "-h" };
        let target_arg = if t.starts_with(':') || t.starts_with('@') {
            t
        } else {
            format!(":{}", t)
        };
        let src_info = Command::new("tmux")
            .args([
                "display-message",
                "-p",
                "-t",
                &target_arg,
                "#{pane_id}|#{?#{@harness},#{@harness},#{pane_current_command}}",
            ])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        let src_parts: Vec<&str> = src_info.split('|').collect();
        let src_pane_id = src_parts.first().copied().unwrap_or("").to_string();
        let src_harness = src_parts.get(1).copied().unwrap_or("").to_string();

        let status = Command::new("tmux")
            .args(["join-pane", split_flag, "-s", &target_arg])
            .status()?;
        if !status.success() {
            eprintln!("Failed to join tab {}", target_arg);
        } else if !src_pane_id.is_empty() && !src_harness.is_empty() {
            let _ = Command::new("tmux")
                .args([
                    "set-option",
                    "-p",
                    "-t",
                    &src_pane_id,
                    "@harness",
                    &src_harness,
                ])
                .status();
        }
        return Ok(());
    }

    // Query session windows excluding current window
    let current_win = Command::new("tmux")
        .args(["display-message", "-p", "#{window_index}"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout).ok()
            } else {
                None
            }
        })
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    let out = Command::new("tmux")
        .args([
            "list-windows",
            "-F",
            "#{window_index}|#{window_name}|#{window_active}|#{@harness}",
        ])
        .output()?;

    let out_str = String::from_utf8_lossy(&out.stdout);
    let mut candidates = Vec::new();
    for line in out_str.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 4 {
            let idx = parts[0].trim();
            let name = parts[1].trim();
            let active = parts[2].trim() == "1" || idx == current_win;
            let harness = parts[3].trim();
            if !active {
                candidates.push((idx.to_string(), name.to_string(), harness.to_string()));
            }
        }
    }

    if candidates.is_empty() {
        if std::env::var("TMUX").is_ok() {
            let _ = Command::new("tmux")
                .args(["display-message", "No other tabs in this session to join"])
                .status();
        } else {
            println!("No other tabs in this session to join");
        }
        return Ok(());
    }

    if let Some(selected_index) = ui::run_join_picker(&candidates, vertical)? {
        let split_flag = if vertical { "-v" } else { "-h" };
        let target_arg = format!(":{}", selected_index);
        let src_info = Command::new("tmux")
            .args([
                "display-message",
                "-p",
                "-t",
                &target_arg,
                "#{pane_id}|#{?#{@harness},#{@harness},#{pane_current_command}}",
            ])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        let src_parts: Vec<&str> = src_info.split('|').collect();
        let src_pane_id = src_parts.first().copied().unwrap_or("").to_string();
        let src_harness = src_parts.get(1).copied().unwrap_or("").to_string();

        let status = Command::new("tmux")
            .args(["join-pane", split_flag, "-s", &target_arg])
            .status();

        if status.is_ok() && !src_pane_id.is_empty() && !src_harness.is_empty() {
            let _ = Command::new("tmux")
                .args([
                    "set-option",
                    "-p",
                    "-t",
                    &src_pane_id,
                    "@harness",
                    &src_harness,
                ])
                .status();
        }
    }

    Ok(())
}

/// Break current split pane into its own standalone tab
pub fn run_unsplit() -> Result<()> {
    let status = Command::new("tmux").args(["break-pane", "-d"]).status()?;
    if !status.success() {
        eprintln!("Failed to unsplit pane (pane might already be sole pane in window)");
    }
    Ok(())
}

pub fn run_reload(default_session: &str, config: &Config) -> Result<()> {
    let active_session = Command::new("tmux")
        .args(["display-message", "-p", "#{session_name}"])
        .output()
        .ok()
        .and_then(|out| {
            if out.status.success() {
                let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !s.is_empty() {
                    Some(s)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .unwrap_or_else(|| default_session.to_string());

    let conf_path = crate::tmux::TmuxProfile::generate(config, &active_session)?;
    let status = Command::new("tmux")
        .args(["source-file", conf_path.to_str().unwrap()])
        .status()?;

    // Clear any lingering session-level overrides on status-position or status
    let _ = Command::new("tmux")
        .args(["set-option", "-u", "-t", &active_session, "status-position"])
        .status();
    let _ = Command::new("tmux")
        .args(["set-option", "-u", "-t", &active_session, "status"])
        .status();

    if status.success() {
        let proj = active_session
            .strip_prefix("agy-")
            .unwrap_or(&active_session);
        let _ = crate::core::save_session_layout(proj);

        let _ = Command::new("tmux")
            .args(["display-message", "hx: configuration reloaded"])
            .status();
        println!(
            "Successfully reloaded hx configuration for session '{}'",
            active_session
        );
        Ok(())
    } else {
        bail!("Failed to reload tmux configuration from {:?}", conf_path);
    }
}

/// Find active Claude Code session ID from ~/.claude/sessions/ by PID or tmux window/pane identifier
fn find_claude_session_id(pane_pid: &str, window_id: &str, pane_id: &str) -> Option<String> {
    let home = dirs::home_dir()?;
    let sessions_dir = home.join(".claude").join("sessions");
    if !sessions_dir.exists() {
        return None;
    }

    // 1. Direct match by pane_pid (e.g. ~/.claude/sessions/{pane_pid}.json)
    if !pane_pid.is_empty() {
        let direct_path = sessions_dir.join(format!("{}.json", pane_pid));
        if direct_path.exists() {
            if let Ok(content) = fs::read_to_string(&direct_path) {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(sid) = v.get("sessionId").and_then(|s| s.as_str()) {
                        if !sid.is_empty() {
                            return Some(sid.to_string());
                        }
                    }
                }
            }
        }
    }

    // 2. Scan sessions directory for matching tmux window_id or pane_id in metadata
    if let Ok(entries) = fs::read_dir(&sessions_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(tmux_val) = v.get("tmux").and_then(|t| t.as_str()) {
                            if (!window_id.is_empty() && tmux_val.contains(window_id))
                                || (!pane_id.is_empty() && tmux_val.contains(pane_id))
                            {
                                if let Some(sid) = v.get("sessionId").and_then(|s| s.as_str()) {
                                    if !sid.is_empty() {
                                        return Some(sid.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

/// Restart the harness running in the active (or target) pane in-place
pub fn run_restart(target: Option<String>) -> Result<()> {
    // 1. Resolve target pane: target, $TMUX_PANE, or default
    let target_str = match target {
        Some(ref t) if !t.trim().is_empty() => Some(t.trim().to_string()),
        _ => std::env::var("TMUX_PANE")
            .ok()
            .filter(|p| !p.trim().is_empty()),
    };

    // 2. Query pane details via tmux in a single call including #{pane_pid}
    let mut disp_cmd = Command::new("tmux");
    disp_cmd.args(["display-message", "-p"]);
    if let Some(ref t) = target_str {
        disp_cmd.args(["-t", t]);
    }
    disp_cmd.arg("#{pane_id}|#{window_id}|#{window_name}|#{pane_current_path}|#{@harness}|#{@conversation_id}|#{?#{@hx_project},#{@hx_project},#{@ax_project}}|#{@remote_control}|#{@user_renamed}|#{pane_current_command}|#{session_name}|#{pane_pid}");

    let disp_out = disp_cmd
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to run tmux display-message: {}", e))?;
    if !disp_out.status.success() {
        let err_msg = String::from_utf8_lossy(&disp_out.stderr);
        bail!("tmux display-message failed: {}", err_msg.trim());
    }

    let out_str = String::from_utf8_lossy(&disp_out.stdout);
    let parts: Vec<&str> = out_str.trim().split('|').collect();
    if parts.len() < 10 {
        bail!(
            "Invalid response from tmux display-message: '{}'",
            out_str.trim()
        );
    }

    let pane_id = parts[0].trim().to_string();
    let window_id = parts[1].trim().to_string();
    let window_name = parts[2].trim().to_string();
    let pane_current_path = parts[3].trim().to_string();
    let raw_harness = parts[4].trim();
    let raw_conv_id = parts[5].trim();
    let raw_ax_project = parts[6].trim();
    let raw_remote_control = parts[7].trim();
    let raw_user_renamed = parts[8].trim();
    let pane_current_cmd = parts[9].trim();
    let session_name = parts.get(10).copied().unwrap_or("").trim();
    let pane_pid = parts.get(11).copied().unwrap_or("").trim().to_string();

    // 3. Infer harness if empty from pane_current_command, window_name, or TabRegistry
    let mut harness = if !raw_harness.is_empty() {
        Some(crate::core::AgentHarness::from_str_loose(raw_harness))
    } else {
        None
    };

    if harness.is_none() {
        if let Some(h) = crate::core::TabRegistry::get_harness_static(&window_id) {
            harness = Some(h);
        }
    }

    if harness.is_none() {
        let cmd_lower = pane_current_cmd.to_lowercase();
        let name_lower = window_name.to_lowercase();
        harness = Some(match () {
            _ if cmd_lower.contains("claude") || name_lower.contains("claude") => {
                crate::core::AgentHarness::ClaudeCode
            }
            _ if cmd_lower.contains("agy")
                || cmd_lower.contains("antigravity")
                || name_lower.contains("agy")
                || name_lower.contains("antigravity") =>
            {
                crate::core::AgentHarness::Antigravity
            }
            _ if cmd_lower.contains("opencode") || name_lower.contains("opencode") => {
                crate::core::AgentHarness::OpenCode
            }
            _ if cmd_lower.contains("codex") || name_lower.contains("codex") => {
                crate::core::AgentHarness::Codex
            }
            _ => crate::core::AgentHarness::PlainTerminal,
        });
    }
    let harness = harness.unwrap_or(crate::core::AgentHarness::PlainTerminal);

    if raw_harness.is_empty() {
        let _ = Command::new("tmux")
            .args([
                "set-option",
                "-w",
                "-t",
                &window_id,
                "@harness",
                harness.as_str(),
            ])
            .status();
        let _ = Command::new("tmux")
            .args([
                "set-option",
                "-p",
                "-t",
                &pane_id,
                "@harness",
                harness.as_str(),
            ])
            .status();
    }

    // 4. Infer project from @ax_project or session name stripped of agy-
    let project = if !raw_ax_project.is_empty() {
        raw_ax_project.to_string()
    } else if !session_name.is_empty() {
        session_name
            .strip_prefix("agy-")
            .unwrap_or(session_name)
            .to_string()
    } else {
        std::env::current_dir()
            .ok()
            .and_then(|p| p.file_name().map(|f| f.to_string_lossy().to_string()))
            .unwrap_or_else(|| "default".to_string())
    };

    // 5. Infer conversation ID if empty
    let mut conv_id = if !raw_conv_id.is_empty() && raw_conv_id != "-" {
        Some(raw_conv_id.to_string())
    } else {
        crate::core::TabRegistry::get_conv_for_window_static(&window_id)
    };

    if conv_id.is_none() {
        if harness == crate::core::AgentHarness::ClaudeCode {
            if let Some(sid) = find_claude_session_id(&pane_pid, &window_id, &pane_id) {
                let _ = Command::new("tmux")
                    .args([
                        "set-option",
                        "-w",
                        "-t",
                        &window_id,
                        "@conversation_id",
                        &sid,
                    ])
                    .status();
                let _ = Command::new("tmux")
                    .args(["set-option", "-p", "-t", &pane_id, "@conversation_id", &sid])
                    .status();
                let _ = crate::core::TabRegistry::bind_with_harness_static(
                    &window_id,
                    &sid,
                    crate::core::AgentHarness::ClaudeCode,
                );
                conv_id = Some(sid);
            }
        } else if harness == crate::core::AgentHarness::Antigravity {
            let db = crate::db_path();
            if db.exists() {
                if let Ok(conn) = rusqlite::Connection::open(&db) {
                    conv_id = crate::core::session::latest_conversation_id(&conn, Some(&project));
                }
            }
        }
    }

    // 6. Build respawn command
    let cmd = if matches!(harness, crate::core::AgentHarness::PlainTerminal) {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string())
    } else {
        let mut c = format!("hx run -H {} --project '{}'", harness.as_str(), project);
        if let Some(ref cid) = conv_id {
            if !cid.is_empty() {
                c.push_str(&format!(" --conversation '{}'", cid));
            }
        }
        if raw_user_renamed == "1" && !window_name.is_empty() {
            c.push_str(&format!(" -n '{}'", window_name));
        }
        if raw_remote_control == "1" {
            c.push_str(" --remote-control");
        }
        c
    };

    // 7. Execute tmux respawn-pane -k -t <pane_id> -c <current_path> "<cmd>"
    let mut respawn = Command::new("tmux");
    respawn.args(["respawn-pane", "-k", "-t", &pane_id]);
    if !pane_current_path.is_empty() {
        respawn.args(["-c", &pane_current_path]);
    }
    respawn.arg(&cmd);

    let status = respawn
        .status()
        .map_err(|e| anyhow::anyhow!("Failed to execute tmux respawn-pane: {}", e))?;
    if !status.success() {
        bail!("tmux respawn-pane failed with exit status: {:?}", status);
    }

    // 8. Toast in tmux and stdout
    let toast = format!(
        "hx: Restarted {} in tab '{}'",
        harness.as_str(),
        window_name
    );
    let _ = Command::new("tmux")
        .args(["display-message", &toast])
        .status();

    println!(
        "Restarted {} in tab '{}' (pane {})",
        harness.as_str(),
        window_name,
        pane_id
    );
    Ok(())
}

/// Install or update ax tab sync lifecycle hook at a specific path
pub fn ensure_hook_installed_at(hook_path: &Path) -> Result<PathBuf> {
    let (canonical_path, target_dir) = if let Ok(meta) = fs::symlink_metadata(hook_path) {
        if meta.file_type().is_symlink() {
            let canon = fs::canonicalize(hook_path).unwrap_or_else(|_| hook_path.to_path_buf());
            let parent = canon.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| {
                hook_path
                    .parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| PathBuf::from("."))
            });
            (canon, parent)
        } else {
            let parent = hook_path
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| PathBuf::from("."));
            (hook_path.to_path_buf(), parent)
        }
    } else {
        let parent = hook_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        (hook_path.to_path_buf(), parent)
    };

    fs::create_dir_all(&target_dir)?;

    let (mut root, exists) = if canonical_path.exists() {
        let content = fs::read_to_string(&canonical_path)?;
        let parsed = match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(val) => val,
            Err(e) => {
                let ts = chrono::Utc::now().format("%Y%m%d%H%M%S");
                let quarantine_path = target_dir.join(format!("hooks.json.corrupt.{}.bak", ts));
                let _ = fs::copy(&canonical_path, &quarantine_path);
                bail!(
                    "hooks.json is invalid JSON ({}). Corrupted file preserved at {:?}. Refusing to overwrite.",
                    e,
                    quarantine_path
                );
            }
        };
        (parsed, true)
    } else {
        (serde_json::json!({}), false)
    };

    if exists {
        let mut os_str = canonical_path.as_os_str().to_os_string();
        os_str.push(".bak");
        let bak_path = PathBuf::from(os_str);
        let _ = fs::copy(&canonical_path, &bak_path);
    }

    if !root.is_object() {
        root = serde_json::json!({});
    }

    let hook_entry = serde_json::json!({
        "enabled": true,
        "PreInvocation": [
            {
                "type": "command",
                "command": "hx hook --running 1",
                "timeout": 5
            }
        ],
        "Stop": [
            {
                "type": "command",
                "command": "hx hook",
                "timeout": 5
            }
        ]
    });

    if let Some(obj) = root.as_object_mut() {
        obj.remove("agymux-tab-sync");
        obj.remove("ax-tab-sync");
        obj.insert("hx-tab-sync".to_string(), hook_entry);
    }

    let formatted = serde_json::to_string_pretty(&root)?;
    let tmp_path = target_dir.join(format!(".hooks.json.tmp.{}", std::process::id()));

    {
        use std::io::Write;
        let mut file = fs::File::create(&tmp_path)?;
        file.write_all(formatted.as_bytes())?;
        file.flush()?;
        file.sync_all()?;
    }

    fs::rename(&tmp_path, &canonical_path)?;
    Ok(canonical_path)
}

/// Install or update ax tab sync lifecycle hook in ~/.gemini/config/hooks.json
pub fn ensure_hook_installed() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Cannot determine home directory")?;
    let config_dir = home.join(".gemini/config");
    let hook_path = config_dir.join("hooks.json");
    ensure_hook_installed_at(&hook_path)
}

/// Install or update Claude Code lifecycle hooks at a specific settings.json path
pub fn ensure_claude_hooks_installed_at(settings_path: &Path) -> Result<PathBuf> {
    let (canonical_path, target_dir) = if let Ok(meta) = fs::symlink_metadata(settings_path) {
        if meta.file_type().is_symlink() {
            let canon =
                fs::canonicalize(settings_path).unwrap_or_else(|_| settings_path.to_path_buf());
            let parent = canon.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| {
                settings_path
                    .parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| PathBuf::from("."))
            });
            (canon, parent)
        } else {
            let parent = settings_path
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| PathBuf::from("."));
            (settings_path.to_path_buf(), parent)
        }
    } else {
        let parent = settings_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        (settings_path.to_path_buf(), parent)
    };

    fs::create_dir_all(&target_dir)?;

    let (mut root, exists) = if canonical_path.exists() {
        let content = fs::read_to_string(&canonical_path)?;
        let parsed = match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(val) => val,
            Err(e) => {
                let ts = chrono::Utc::now().format("%Y%m%d%H%M%S");
                let quarantine_path = target_dir.join(format!("settings.json.corrupt.{}.bak", ts));
                let _ = fs::copy(&canonical_path, &quarantine_path);
                bail!(
                    "settings.json is invalid JSON ({}). Corrupted file preserved at {:?}. Refusing to overwrite.",
                    e,
                    quarantine_path
                );
            }
        };
        (parsed, true)
    } else {
        (serde_json::json!({}), false)
    };

    if exists {
        let mut os_str = canonical_path.as_os_str().to_os_string();
        os_str.push(".bak");
        let bak_path = PathBuf::from(os_str);
        let _ = fs::copy(&canonical_path, &bak_path);
    }

    if !root.is_object() {
        root = serde_json::json!({});
    }

    if let Some(root_obj) = root.as_object_mut() {
        let hooks_val = root_obj
            .entry("hooks")
            .or_insert_with(|| serde_json::json!({}));
        if !hooks_val.is_object() {
            *hooks_val = serde_json::json!({});
        }
        if let Some(hooks_obj) = hooks_val.as_object_mut() {
            let merge_hook_event = |event_arr: &mut Vec<serde_json::Value>, target_cmd: &str| {
                let already_present = event_arr.iter().any(|item| {
                    if let Some(cmd) = item.get("command").and_then(|c| c.as_str()) {
                        if cmd == target_cmd {
                            return true;
                        }
                    }
                    if let Some(sub_hooks) = item.get("hooks").and_then(|h| h.as_array()) {
                        if sub_hooks.iter().any(|sub| {
                            sub.get("command").and_then(|c| c.as_str()) == Some(target_cmd)
                        }) {
                            return true;
                        }
                    }
                    false
                });

                if !already_present {
                    let hook_entry = serde_json::json!({
                        "hooks": [
                            {
                                "type": "command",
                                "command": target_cmd
                            }
                        ]
                    });
                    event_arr.push(hook_entry);
                }
            };

            let submit_val = hooks_obj
                .entry("UserPromptSubmit")
                .or_insert_with(|| serde_json::json!([]));
            if !submit_val.is_array() {
                *submit_val = serde_json::json!([]);
            }
            if let Some(submit_arr) = submit_val.as_array_mut() {
                merge_hook_event(submit_arr, "hx hook --running 1");
            }

            let stop_val = hooks_obj
                .entry("Stop")
                .or_insert_with(|| serde_json::json!([]));
            if !stop_val.is_array() {
                *stop_val = serde_json::json!([]);
            }
            if let Some(stop_arr) = stop_val.as_array_mut() {
                merge_hook_event(stop_arr, "hx hook --running 0");
            }
        }
    }

    let formatted = serde_json::to_string_pretty(&root)?;
    let tmp_path = target_dir.join(format!(".settings.json.tmp.{}", std::process::id()));

    {
        use std::io::Write;
        let mut file = fs::File::create(&tmp_path)?;
        file.write_all(formatted.as_bytes())?;
        file.flush()?;
        file.sync_all()?;
    }

    fs::rename(&tmp_path, &canonical_path)?;
    Ok(canonical_path)
}

/// Install or update Claude Code lifecycle hooks in ~/.claude/settings.json
pub fn ensure_claude_hooks_installed() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Cannot determine home directory")?;
    let claude_dir = home.join(".claude");
    let settings_path = claude_dir.join("settings.json");
    ensure_claude_hooks_installed_at(&settings_path)
}

/// CLI command to install hx lifecycle hook to ~/.gemini/config/hooks.json and ~/.claude/settings.json
pub fn run_install() -> Result<()> {
    let hook_path = ensure_hook_installed()?;
    println!("Installed hx hook to {}", hook_path.display());
    match ensure_claude_hooks_installed() {
        Ok(claude_path) => println!("Installed hx Claude hooks to {}", claude_path.display()),
        Err(e) => eprintln!("Warning: failed to install Claude hooks: {}", e),
    }
    Ok(())
}

/// Parse git rev-list left-right count output into (ahead, behind)
pub fn parse_rev_list_count(text: &str) -> (u64, u64) {
    let parts: Vec<&str> = text.split_whitespace().collect();
    if parts.len() >= 2 {
        let behind = parts[0].parse::<u64>().unwrap_or(0);
        let ahead = parts[1].parse::<u64>().unwrap_or(0);
        (ahead, behind)
    } else {
        (0, 0)
    }
}

/// Query ahead/behind commit counts relative to upstream
fn get_git_ahead_behind(dir: &Path) -> (u64, u64) {
    let output = Command::new("git")
        .args(["-C"])
        .arg(dir)
        .args(["rev-list", "--left-right", "--count", "@{upstream}...HEAD"])
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout);
            return parse_rev_list_count(&text);
        }
    }
    (0, 0)
}

/// Resolve git status (branch and ahead/behind) for a directory and format it for tmux pane border
pub fn format_git_status(dir: &Path) -> Option<String> {
    if !dir.exists() {
        return None;
    }

    let branch_output = Command::new("git")
        .args(["-C"])
        .arg(dir)
        .args(["branch", "--show-current"])
        .output()
        .ok()?;

    let mut branch = String::from_utf8_lossy(&branch_output.stdout)
        .trim()
        .to_string();
    if branch.is_empty() {
        let rev_output = Command::new("git")
            .args(["-C"])
            .arg(dir)
            .args(["rev-parse", "--short", "HEAD"])
            .output()
            .ok()?;
        if !rev_output.status.success() {
            return None;
        }
        branch = String::from_utf8_lossy(&rev_output.stdout)
            .trim()
            .to_string();
        if branch.is_empty() {
            return None;
        }
    }

    let (ahead, behind) = get_git_ahead_behind(dir);

    let mut formatted = format!("#[fg=colour180]{branch}#[default]");
    if ahead > 0 {
        formatted.push_str(&format!(" #[fg=colour80]↑{ahead}#[default]"));
    }
    if behind > 0 {
        formatted.push_str(&format!(" #[fg=colour140]↓{behind}#[default]"));
    }

    Some(formatted)
}

/// Output git status to stdout for tmux pane border format
pub fn run_git_status(path: Option<PathBuf>) -> Result<()> {
    let dir =
        path.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    if let Some(status) = format_git_status(&dir) {
        print!(" · {}", status);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_agy_args_with_conv_id() {
        let args = vec![
            "--project".to_string(),
            "doppelganger".to_string(),
            "-c".to_string(),
            "conv-uuid-1".to_string(),
        ];
        let res = build_agy_args("doppelganger", &args, Some("conv-uuid-1"), true, false);
        assert_eq!(
            res,
            vec!["--project", "doppelganger", "--conversation", "conv-uuid-1"]
        );
    }

    #[test]
    fn test_build_agy_args_bare_continue_resolved() {
        let args = vec!["-c".to_string()];
        let res = build_agy_args("doppelganger", &args, Some("conv-uuid-latest"), true, false);
        assert_eq!(
            res,
            vec![
                "--project",
                "doppelganger",
                "--conversation",
                "conv-uuid-latest"
            ]
        );
    }

    #[test]
    fn test_build_agy_args_bare_continue_unresolved() {
        let args = vec!["-c".to_string()];
        let res = build_agy_args("doppelganger", &args, None, true, false);
        assert_eq!(res, vec!["--project", "doppelganger", "-c"]);
    }

    #[test]
    fn test_build_agy_args_new_session_no_flags() {
        let args: Vec<String> = vec![];
        let res = build_agy_args("doppelganger", &args, None, false, false);
        assert_eq!(res, vec!["--project", "doppelganger"]);
    }

    #[test]
    fn test_build_agy_args_preserves_custom_flags_and_remote_control() {
        let args = vec![
            "--model".to_string(),
            "gemini-3.8-flash".to_string(),
            "--conversation".to_string(),
            "conv-uuid-2".to_string(),
        ];
        let res = build_agy_args("doppelganger", &args, Some("conv-uuid-2"), true, true);
        assert_eq!(
            res,
            vec![
                "--project",
                "doppelganger",
                "--model",
                "gemini-3.8-flash",
                "--conversation",
                "conv-uuid-2",
                "--remote-control",
            ]
        );
    }

    #[test]
    fn test_build_agy_args_strips_resume_and_short_flags() {
        let args = vec!["--resume".to_string(), "conv-uuid-resume".to_string()];
        let res = build_agy_args("doppelganger", &args, Some("conv-uuid-resume"), true, false);
        assert_eq!(
            res,
            vec![
                "--project",
                "doppelganger",
                "--conversation",
                "conv-uuid-resume"
            ]
        );

        let args_short = vec!["-r".to_string(), "conv-uuid-short".to_string()];
        let res_short = build_agy_args(
            "doppelganger",
            &args_short,
            Some("conv-uuid-short"),
            true,
            false,
        );
        assert_eq!(
            res_short,
            vec![
                "--project",
                "doppelganger",
                "--conversation",
                "conv-uuid-short"
            ]
        );
    }

    #[test]
    fn test_parse_runner_args_resume_long_flag() {
        let args = vec![
            "--resume".to_string(),
            "conv-12345".to_string(),
            "hello world".to_string(),
        ];
        let parsed = parse_runner_args(&args);
        assert_eq!(parsed.conv_id.as_deref(), Some("conv-12345"));
        assert!(parsed.has_continue_flag);
        assert_eq!(parsed.initial_prompt.as_deref(), Some("hello world"));
        assert_eq!(parsed.filtered_args, vec!["hello world"]);
    }

    #[test]
    fn test_parse_runner_args_resume_short_flag() {
        let args = vec!["-r".to_string(), "conv-67890".to_string()];
        let parsed = parse_runner_args(&args);
        assert_eq!(parsed.conv_id.as_deref(), Some("conv-67890"));
        assert!(parsed.has_continue_flag);
        assert_eq!(parsed.initial_prompt, None);
        assert!(parsed.filtered_args.is_empty());
    }

    #[test]
    fn test_parse_runner_args_resume_equals() {
        let args = vec![
            "--resume=conv-eq-999".to_string(),
            "explain code".to_string(),
        ];
        let parsed = parse_runner_args(&args);
        assert_eq!(parsed.conv_id.as_deref(), Some("conv-eq-999"));
        assert!(parsed.has_continue_flag);
        assert_eq!(parsed.initial_prompt.as_deref(), Some("explain code"));
        assert_eq!(parsed.filtered_args, vec!["explain code"]);
    }

    #[test]
    fn test_parse_runner_args_bare_resume() {
        let args = vec!["--resume".to_string()];
        let parsed = parse_runner_args(&args);
        assert_eq!(parsed.conv_id, None);
        assert!(parsed.has_continue_flag);
        assert_eq!(parsed.initial_prompt, None);
        assert!(parsed.filtered_args.is_empty());
    }

    #[test]
    fn test_parse_runner_args_conversation_flag() {
        let args = vec![
            "--conversation".to_string(),
            "conv-uuid-abc".to_string(),
            "some prompt".to_string(),
        ];
        let parsed = parse_runner_args(&args);
        assert_eq!(parsed.conv_id.as_deref(), Some("conv-uuid-abc"));
        assert!(parsed.has_continue_flag);
        assert_eq!(parsed.initial_prompt.as_deref(), Some("some prompt"));
        assert_eq!(
            parsed.filtered_args,
            vec!["--conversation", "conv-uuid-abc", "some prompt"]
        );
    }

    #[test]
    fn test_ensure_hook_installed_and_json_format() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_dir = temp_dir.path().join(".gemini/config");
        std::fs::create_dir_all(&config_dir).unwrap();
        let hook_path = config_dir.join("hooks.json");

        // Write existing dummy hook
        let initial_json = r#"{ "other-hook": { "enabled": true } }"#;
        std::fs::write(&hook_path, initial_json).unwrap();

        // Run real function directly
        let res_path = ensure_hook_installed_at(&hook_path).unwrap();
        assert_eq!(res_path, hook_path);

        let read_back: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&hook_path).unwrap()).unwrap();
        assert!(read_back.get("other-hook").is_some());
        assert!(read_back.get("hx-tab-sync").is_some());
        assert_eq!(read_back["hx-tab-sync"]["enabled"], true);
        assert_eq!(read_back["hx-tab-sync"]["Stop"][0]["command"], "hx hook");
    }

    #[test]
    fn test_ensure_hook_installed_invalid_json_quarantine() {
        let temp_dir = tempfile::tempdir().unwrap();
        let hook_path = temp_dir.path().join("hooks.json");

        let corrupt_content = r#"{ "incomplete_json": true, "#;
        std::fs::write(&hook_path, corrupt_content).unwrap();

        let res = ensure_hook_installed_at(&hook_path);
        assert!(res.is_err());
        let err_str = res.unwrap_err().to_string();
        assert!(err_str.contains("hooks.json is invalid JSON"));
        assert!(err_str.contains("Refusing to overwrite"));

        // Verify original file is intact and not truncated
        let orig = std::fs::read_to_string(&hook_path).unwrap();
        assert_eq!(orig, corrupt_content);

        // Verify quarantine file exists
        let mut found_quarantine = false;
        for entry in std::fs::read_dir(temp_dir.path()).unwrap().flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("hooks.json.corrupt.") && name.ends_with(".bak") {
                found_quarantine = true;
                let quaran_content = std::fs::read_to_string(entry.path()).unwrap();
                assert_eq!(quaran_content, corrupt_content);
            }
        }
        assert!(found_quarantine, "Quarantine backup file was not created");
    }

    #[test]
    fn test_ensure_hook_installed_success_and_backup() {
        let temp_dir = tempfile::tempdir().unwrap();
        let hook_path = temp_dir.path().join("hooks.json");

        let initial = r#"{ "my-hook": { "enabled": true } }"#;
        std::fs::write(&hook_path, initial).unwrap();

        let res = ensure_hook_installed_at(&hook_path);
        assert!(res.is_ok());

        // Verify backup copy
        let bak_path = temp_dir.path().join("hooks.json.bak");
        assert!(bak_path.exists());
        assert_eq!(std::fs::read_to_string(&bak_path).unwrap(), initial);

        // Verify updated file
        let updated: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&hook_path).unwrap()).unwrap();
        assert!(updated.get("my-hook").is_some());
        assert!(updated.get("hx-tab-sync").is_some());
    }

    #[test]
    fn test_ensure_hook_installed_preserves_key_order() {
        let temp_dir = tempfile::tempdir().unwrap();
        let hook_path = temp_dir.path().join("hooks.json");

        // Write keys in non-alphabetical order: zebra, alpha, middle
        let initial = "{\n  \"zebra\": 1,\n  \"alpha\": 2,\n  \"middle\": 3\n}";
        std::fs::write(&hook_path, initial).unwrap();

        let res = ensure_hook_installed_at(&hook_path);
        assert!(res.is_ok());

        let raw_str = std::fs::read_to_string(&hook_path).unwrap();
        let idx_z = raw_str.find("\"zebra\"").unwrap();
        let idx_a = raw_str.find("\"alpha\"").unwrap();
        let idx_m = raw_str.find("\"middle\"").unwrap();
        let idx_ax = raw_str.find("\"hx-tab-sync\"").unwrap();

        assert!(idx_z < idx_a);
        assert!(idx_a < idx_m);
        assert!(idx_m < idx_ax);

        let parsed: serde_json::Value = serde_json::from_str(&raw_str).unwrap();
        let keys: Vec<&str> = parsed
            .as_object()
            .unwrap()
            .keys()
            .map(|k| k.as_str())
            .collect();
        assert_eq!(keys, vec!["zebra", "alpha", "middle", "hx-tab-sync"]);
    }

    #[test]
    fn test_ensure_hook_installed_symlink_handling() {
        use std::os::unix::fs::symlink;

        let target_dir = tempfile::tempdir().unwrap();
        let link_dir = tempfile::tempdir().unwrap();

        let real_path = target_dir.path().join("canonical_hooks.json");
        let initial = r#"{ "real-hook": true }"#;
        std::fs::write(&real_path, initial).unwrap();

        let symlink_path = link_dir.path().join("hooks.json");
        symlink(&real_path, &symlink_path).unwrap();

        assert!(std::fs::symlink_metadata(&symlink_path)
            .unwrap()
            .file_type()
            .is_symlink());

        let res = ensure_hook_installed_at(&symlink_path);
        assert!(res.is_ok());

        // Symlink must remain a symlink
        assert!(std::fs::symlink_metadata(&symlink_path)
            .unwrap()
            .file_type()
            .is_symlink());

        // Canonical target must be updated
        let real_str = std::fs::read_to_string(&real_path).unwrap();
        let val: serde_json::Value = serde_json::from_str(&real_str).unwrap();
        assert!(val.get("real-hook").is_some());
        assert!(val.get("hx-tab-sync").is_some());

        // Backup must exist alongside the canonical target
        let real_bak = target_dir.path().join("canonical_hooks.json.bak");
        assert!(real_bak.exists());
        assert_eq!(std::fs::read_to_string(&real_bak).unwrap(), initial);
    }

    #[test]
    fn test_ensure_claude_hooks_installed_and_json_format() {
        let temp_dir = tempfile::tempdir().unwrap();
        let settings_path = temp_dir.path().join("settings.json");

        // Write existing dummy settings with permissions and custom fields
        let initial_json = r#"{
  "permissions": { "allow": ["Bash(git status)"] },
  "outputStyle": "Concise"
}"#;
        std::fs::write(&settings_path, initial_json).unwrap();

        let res_path = ensure_claude_hooks_installed_at(&settings_path).unwrap();
        assert_eq!(res_path, settings_path);

        let read_back: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&settings_path).unwrap()).unwrap();
        assert_eq!(read_back["outputStyle"], "Concise");
        assert_eq!(read_back["permissions"]["allow"][0], "Bash(git status)");

        // Verify hooks
        let hooks = &read_back["hooks"];
        assert!(hooks.is_object());
        let submit_cmd = &hooks["UserPromptSubmit"][0]["hooks"][0]["command"];
        assert_eq!(submit_cmd, "hx hook --running 1");
        let stop_cmd = &hooks["Stop"][0]["hooks"][0]["command"];
        assert_eq!(stop_cmd, "hx hook --running 0");
    }

    #[test]
    fn test_ensure_claude_hooks_idempotent() {
        let temp_dir = tempfile::tempdir().unwrap();
        let settings_path = temp_dir.path().join("settings.json");

        std::fs::write(&settings_path, r#"{ "model": "sonnet" }"#).unwrap();

        // Run twice
        ensure_claude_hooks_installed_at(&settings_path).unwrap();
        ensure_claude_hooks_installed_at(&settings_path).unwrap();

        let read_back: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&settings_path).unwrap()).unwrap();
        assert_eq!(
            read_back["hooks"]["UserPromptSubmit"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(read_back["hooks"]["Stop"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_ensure_claude_hooks_invalid_json_quarantine() {
        let temp_dir = tempfile::tempdir().unwrap();
        let settings_path = temp_dir.path().join("settings.json");

        let corrupt_content = r#"{ "incomplete_settings": true, "#;
        std::fs::write(&settings_path, corrupt_content).unwrap();

        let res = ensure_claude_hooks_installed_at(&settings_path);
        assert!(res.is_err());
        let err_str = res.unwrap_err().to_string();
        assert!(err_str.contains("settings.json is invalid JSON"));
        assert!(err_str.contains("Refusing to overwrite"));

        let orig = std::fs::read_to_string(&settings_path).unwrap();
        assert_eq!(orig, corrupt_content);

        let mut found_quarantine = false;
        for entry in std::fs::read_dir(temp_dir.path()).unwrap().flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("settings.json.corrupt.") && name.ends_with(".bak") {
                found_quarantine = true;
                assert_eq!(
                    std::fs::read_to_string(entry.path()).unwrap(),
                    corrupt_content
                );
            }
        }
        assert!(found_quarantine, "Quarantine file was not created");
    }

    #[test]
    fn test_check_workspace_matches() {
        // Exact match
        assert!(check_workspace_matches(
            r#"["file:///home/user/projects/repo"]"#,
            "/home/user/projects/repo"
        ));
        // Subpath match
        assert!(check_workspace_matches(
            r#"["file:///home/user/projects/repo"]"#,
            "/home/user/projects/repo/src/core"
        ));
        // URL encoded
        assert!(check_workspace_matches(
            r#"["file:///home/user/my%20project"]"#,
            "/home/user/my project"
        ));
        // Single plain string
        assert!(check_workspace_matches(
            "file:///home/user/projects/repo",
            "/home/user/projects/repo"
        ));
        // Empty workspace string allows any path
        assert!(check_workspace_matches("", "/some/random/path"));
        assert!(check_workspace_matches("[]", "/some/random/path"));
        // Non-matching path
        assert!(!check_workspace_matches(
            r#"["file:///home/user/projects/repo-a"]"#,
            "/home/user/projects/repo-b"
        ));
    }

    #[test]
    fn test_parse_rev_list_count() {
        assert_eq!(parse_rev_list_count("0\t0"), (0, 0));
        assert_eq!(parse_rev_list_count("0\t0\n"), (0, 0));
        assert_eq!(parse_rev_list_count("2\t5"), (5, 2)); // ahead: 5, behind: 2
        assert_eq!(parse_rev_list_count("0\t3"), (3, 0)); // ahead: 3, behind: 0
        assert_eq!(parse_rev_list_count("4\t0"), (0, 4)); // ahead: 0, behind: 4
        assert_eq!(parse_rev_list_count(""), (0, 0));
        assert_eq!(parse_rev_list_count("invalid"), (0, 0));
    }

    #[test]
    fn test_format_git_status_repo() {
        let repo_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let status = format_git_status(&repo_dir);
        assert!(status.is_some());
        let s = status.unwrap();
        assert!(s.starts_with("#[fg=colour180]"));
        assert!(s.contains("#[default]"));
    }

    #[test]
    fn test_format_git_status_nonexistent() {
        let nonexistent = PathBuf::from("/nonexistent/directory/path/12345");
        assert!(format_git_status(&nonexistent).is_none());
    }

    #[test]
    fn test_find_claude_session_id_parsing() {
        let temp_dir = std::env::temp_dir().join(format!("hx-test-claude-{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);
        let sample_json =
            r#"{"pid":99999,"sessionId":"test-session-uuid-1234","tmux":"agy-test:@10.%1"}"#;
        let file_path = temp_dir.join("99999.json");
        fs::write(&file_path, sample_json).unwrap();

        // Verify JSON parse directly
        let content = fs::read_to_string(&file_path).unwrap();
        let val: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(
            val.get("sessionId").and_then(|s| s.as_str()),
            Some("test-session-uuid-1234")
        );
        assert_eq!(
            val.get("tmux").and_then(|s| s.as_str()),
            Some("agy-test:@10.%1")
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
