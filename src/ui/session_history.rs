use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;
use std::time::Duration;

use crate::core::SessionSnapshot;
use crate::ui::app::TerminalGuard;
use crate::ui::theme::{
    base_style, highlight_style, muted_style, styled_block, COLOR_CYAN, COLOR_FG, COLOR_GREEN,
    COLOR_PINK, COLOR_PURPLE, COLOR_YELLOW,
};

pub struct SessionHistoryState {
    pub project: String,
    pub snapshots: Vec<SessionSnapshot>,
    pub selected: usize,
    pub status_message: Option<String>,
}

impl SessionHistoryState {
    pub fn new(project: String, snapshots: Vec<SessionSnapshot>) -> Self {
        Self {
            project,
            snapshots,
            selected: 0,
            status_message: None,
        }
    }

    pub fn select_next(&mut self) {
        if !self.snapshots.is_empty() {
            if self.selected + 1 >= self.snapshots.len() {
                self.selected = 0;
            } else {
                self.selected += 1;
            }
        }
    }

    pub fn select_prev(&mut self) {
        if !self.snapshots.is_empty() {
            if self.selected == 0 {
                self.selected = self.snapshots.len() - 1;
            } else {
                self.selected -= 1;
            }
        }
    }

    pub fn selected_snapshot(&self) -> Option<SessionSnapshot> {
        self.snapshots.get(self.selected).cloned()
    }

    pub fn delete_selected(&mut self) -> Option<SessionSnapshot> {
        if self.snapshots.is_empty() {
            return None;
        }
        let removed = self.snapshots.remove(self.selected);
        let _ = crate::core::delete_session_history_entry(&self.project, removed.timestamp);
        if self.selected >= self.snapshots.len() && !self.snapshots.is_empty() {
            self.selected = self.snapshots.len() - 1;
        }
        self.status_message = Some(format!(
            "Deleted snapshot from {}",
            crate::core::session::format_relative_time(removed.timestamp)
        ));
        Some(removed)
    }
}

pub enum SessionHistoryResult {
    Continue,
    Confirm(SessionSnapshot),
    Cancel,
}

pub fn handle_key(key: KeyEvent, state: &mut SessionHistoryState) -> SessionHistoryResult {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('й') => SessionHistoryResult::Cancel,
        KeyCode::Enter => {
            if let Some(snap) = state.selected_snapshot() {
                SessionHistoryResult::Confirm(snap)
            } else {
                SessionHistoryResult::Cancel
            }
        }
        KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('о') => {
            state.select_next();
            SessionHistoryResult::Continue
        }
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('л') => {
            state.select_prev();
            SessionHistoryResult::Continue
        }
        KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Char('в') | KeyCode::Delete => {
            state.delete_selected();
            SessionHistoryResult::Continue
        }
        _ => SessionHistoryResult::Continue,
    }
}

pub fn render(f: &mut Frame, area: Rect, state: &SessionHistoryState) {
    let width = 96.min(area.width.saturating_sub(4));
    let height = 24.min(area.height.saturating_sub(2));

    let horiz_pad = (area.width.saturating_sub(width)) / 2;
    let vert_pad = (area.height.saturating_sub(height)) / 2;

    let modal_area = Rect {
        x: horiz_pad,
        y: vert_pad,
        width,
        height,
    };

    f.render_widget(Clear, modal_area);

    let vchunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header title
            Constraint::Min(10),   // Main columns
            Constraint::Length(2), // Footer help
        ])
        .split(modal_area);

    // 1. Header
    let block = styled_block("Session History & Snapshots", true);
    let title_p = Paragraph::new(Line::from(vec![
        Span::styled(" Project: ", Style::default().fg(COLOR_CYAN)),
        Span::styled(
            &state.project,
            Style::default().fg(COLOR_FG).add_modifier(Modifier::BOLD),
        ),
        Span::styled("  —  Select a snapshot to restore", muted_style()),
    ]))
    .block(block)
    .style(base_style());
    f.render_widget(title_p, vchunks[0]);

    // 2. Main columns (Left: snapshot list, Right: preview)
    let hchunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
        .split(vchunks[1]);

    // Left: list of snapshots
    let list_title = format!("Snapshots ({})", state.snapshots.len());
    let list_block = styled_block(&list_title, false);

    let items: Vec<ListItem> = if state.snapshots.is_empty() {
        vec![ListItem::new(vec![Line::from(vec![Span::styled(
            "  No snapshots found",
            muted_style(),
        )])])]
    } else {
        state
            .snapshots
            .iter()
            .enumerate()
            .map(|(i, snap)| {
                let is_selected = i == state.selected;
                let rel = crate::core::session::format_relative_time(snap.timestamp);
                let label = snap.name.as_deref().unwrap_or("");

                let mut line1_spans = vec![
                    Span::styled(
                        if is_selected { "▶ " } else { "  " },
                        if is_selected {
                            Style::default()
                                .fg(COLOR_YELLOW)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            muted_style()
                        },
                    ),
                    Span::styled(
                        rel,
                        if is_selected {
                            Style::default()
                                .fg(COLOR_GREEN)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(COLOR_FG)
                        },
                    ),
                ];

                if !label.is_empty() {
                    line1_spans.push(Span::raw(" "));
                    line1_spans.push(Span::styled(
                        format!("[{}]", label),
                        Style::default().fg(COLOR_PINK).add_modifier(Modifier::BOLD),
                    ));
                }

                let line2 = Line::from(vec![
                    Span::raw("    "),
                    Span::styled(
                        format!("{} tabs", snap.windows.len()),
                        if is_selected {
                            Style::default().fg(COLOR_PURPLE)
                        } else {
                            muted_style()
                        },
                    ),
                ]);

                if is_selected {
                    ListItem::new(vec![Line::from(line1_spans), line2]).style(highlight_style())
                } else {
                    ListItem::new(vec![Line::from(line1_spans), line2]).style(base_style())
                }
            })
            .collect()
    };

    let list = List::new(items)
        .block(list_block)
        .highlight_style(highlight_style());

    let mut list_state = ListState::default();
    if !state.snapshots.is_empty() {
        list_state.select(Some(state.selected));
    }
    f.render_stateful_widget(list, hchunks[0], &mut list_state);

    // Right: snapshot preview
    let preview_block = styled_block("Snapshot Preview", false);
    let preview_p = if let Some(snap) = state.snapshots.get(state.selected) {
        let mut lines = Vec::new();

        lines.push(Line::from(vec![
            Span::styled("Session: ", Style::default().fg(COLOR_CYAN)),
            Span::styled(
                &snap.session_name,
                Style::default().fg(COLOR_FG).add_modifier(Modifier::BOLD),
            ),
        ]));

        if let Some(ref name) = snap.name {
            lines.push(Line::from(vec![
                Span::styled("Label:   ", Style::default().fg(COLOR_CYAN)),
                Span::styled(
                    name,
                    Style::default().fg(COLOR_PINK).add_modifier(Modifier::BOLD),
                ),
            ]));
        }

        let rel = crate::core::session::format_relative_time(snap.timestamp);
        lines.push(Line::from(vec![
            Span::styled("Saved:   ", Style::default().fg(COLOR_CYAN)),
            Span::styled(
                format!("{} (epoch: {})", rel, snap.timestamp),
                muted_style(),
            ),
        ]));

        lines.push(Line::from(vec![Span::styled(
            format!("Windows ({}):", snap.windows.len()),
            Style::default()
                .fg(COLOR_YELLOW)
                .add_modifier(Modifier::BOLD),
        )]));

        for win in &snap.windows {
            let h_icon = match win.harness.as_deref() {
                Some("claude") => "󰊠 claude",
                Some("opencode") => "󰘐 opencode",
                Some("term") => " term",
                _ => "󰚩 agy",
            };
            let mut win_spans = vec![
                Span::styled(
                    format!("  • Tab {}: ", win.index),
                    Style::default().fg(COLOR_GREEN),
                ),
                Span::styled(
                    &win.name,
                    Style::default().fg(COLOR_FG).add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
                Span::styled(format!("({})", h_icon), Style::default().fg(COLOR_PURPLE)),
            ];
            if let Some(ref cid) = win.conversation_id {
                win_spans.push(Span::raw(" "));
                win_spans.push(Span::styled(format!("cid: {:.8}...", cid), muted_style()));
            }
            lines.push(Line::from(win_spans));

            for pane in &win.panes {
                let cmd_str = pane.command.as_deref().unwrap_or("");
                let cmd_part = if !cmd_str.is_empty() {
                    format!(" [cmd: {}]", cmd_str)
                } else {
                    String::new()
                };
                lines.push(Line::from(vec![
                    Span::raw("      "),
                    Span::styled(
                        format!("pane {}: {}{}", pane.index, pane.path, cmd_part),
                        muted_style(),
                    ),
                ]));
            }
        }

        Paragraph::new(lines)
            .block(preview_block)
            .wrap(Wrap { trim: false })
    } else {
        Paragraph::new("  (No snapshot selected)")
            .block(preview_block)
            .style(muted_style())
    };
    f.render_widget(preview_p, hchunks[1]);

    // 3. Footer
    let status_text = state
        .status_message
        .as_deref()
        .unwrap_or(" ↑/k: up • ↓/j: down • Enter: restore • d: delete • Esc: exit");
    let footer_p = Paragraph::new(Line::from(vec![Span::styled(
        format!("  {}", status_text),
        if state.status_message.is_some() {
            Style::default()
                .fg(COLOR_YELLOW)
                .add_modifier(Modifier::BOLD)
        } else {
            muted_style()
        },
    )]))
    .style(base_style());
    f.render_widget(footer_p, vchunks[2]);
}

/// Run interactive session history picker modal
pub fn run_session_history_picker(project: &str) -> Result<Option<SessionSnapshot>> {
    let mut snapshots = crate::core::list_session_history(project)?;
    // If empty history, check canonical file
    if snapshots.is_empty() {
        let canonical_path = crate::core::session_snapshot_path(project);
        if canonical_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&canonical_path) {
                if let Ok(snap) = serde_json::from_str::<SessionSnapshot>(&content) {
                    snapshots.push(snap);
                }
            }
        }
    }

    let mut guard = TerminalGuard::enter()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut state = SessionHistoryState::new(project.to_string(), snapshots);

    loop {
        terminal.draw(|f| {
            let area = f.area();
            render(f, area, &state);
        })?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match handle_key(key, &mut state) {
                    SessionHistoryResult::Continue => {}
                    SessionHistoryResult::Confirm(snap) => {
                        guard.leave();
                        return Ok(Some(snap));
                    }
                    SessionHistoryResult::Cancel => {
                        guard.leave();
                        return Ok(None);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    #[test]
    fn test_session_history_state_navigation() {
        let snap1 = SessionSnapshot {
            project: "test".to_string(),
            session_name: "agy-test".to_string(),
            timestamp: 1000,
            name: Some("first".to_string()),
            windows: vec![],
        };
        let snap2 = SessionSnapshot {
            project: "test".to_string(),
            session_name: "agy-test".to_string(),
            timestamp: 2000,
            name: Some("second".to_string()),
            windows: vec![],
        };

        let mut state =
            SessionHistoryState::new("test".to_string(), vec![snap1.clone(), snap2.clone()]);
        assert_eq!(state.selected, 0);
        assert_eq!(state.selected_snapshot().unwrap().timestamp, 1000);

        state.select_next();
        assert_eq!(state.selected, 1);
        assert_eq!(state.selected_snapshot().unwrap().timestamp, 2000);

        state.select_next();
        assert_eq!(state.selected, 0);

        state.select_prev();
        assert_eq!(state.selected, 1);

        state.select_prev();
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn test_session_history_key_handling() {
        let snap = SessionSnapshot {
            project: "test".to_string(),
            session_name: "agy-test".to_string(),
            timestamp: 1000,
            name: None,
            windows: vec![],
        };

        let mut state = SessionHistoryState::new("test".to_string(), vec![snap]);

        // Enter confirms
        match handle_key(
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
            &mut state,
        ) {
            SessionHistoryResult::Confirm(s) => assert_eq!(s.timestamp, 1000),
            _ => panic!("Expected Confirm"),
        }

        // Esc cancels
        match handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), &mut state) {
            SessionHistoryResult::Cancel => {}
            _ => panic!("Expected Cancel"),
        }
    }
}
