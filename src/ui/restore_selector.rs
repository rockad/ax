use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use std::io;
use std::time::Duration;

use crate::core::SessionSnapshot;
use crate::ui::app::TerminalGuard;
use crate::ui::theme::{
    base_style, highlight_style, muted_style, styled_block, COLOR_CYAN, COLOR_FG, COLOR_GREEN,
    COLOR_PURPLE, COLOR_YELLOW,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestoreChoice {
    RestorePrevious,
    StartFresh,
    BrowseHistory,
}

pub struct RestoreOption {
    pub choice: RestoreChoice,
    pub key_label: &'static str,
    pub title: String,
    pub subtitle: String,
}

pub struct RestoreSelectorState {
    pub options: Vec<RestoreOption>,
    pub selected: usize,
    pub remember: bool,
    pub project: String,
}

impl RestoreSelectorState {
    pub fn new(project: String, snapshot: &SessionSnapshot) -> Self {
        let rel = crate::core::session::format_relative_time(snapshot.timestamp);
        let win_names: Vec<String> = snapshot
            .windows
            .iter()
            .take(4)
            .map(|w| format!("\"{}\"", w.name))
            .collect();
        let tabs_preview = if win_names.is_empty() {
            String::new()
        } else {
            format!(" ({})", win_names.join(", "))
        };

        let options = vec![
            RestoreOption {
                choice: RestoreChoice::RestorePrevious,
                key_label: "1",
                title: format!("🔄 Restore previous session ({} tabs{})", snapshot.windows.len(), tabs_preview),
                subtitle: format!("Saved {}", rel),
            },
            RestoreOption {
                choice: RestoreChoice::StartFresh,
                key_label: "2",
                title: "➕ Start fresh session".to_string(),
                subtitle: "Launch fresh tabs without restoring previous layout".to_string(),
            },
            RestoreOption {
                choice: RestoreChoice::BrowseHistory,
                key_label: "3",
                title: "🗂️ Browse session history...".to_string(),
                subtitle: "Pick from earlier snapshots saved on disk".to_string(),
            },
        ];

        Self {
            options,
            selected: 0,
            remember: false,
            project,
        }
    }

    pub fn select_next(&mut self) {
        if !self.options.is_empty() {
            if self.selected + 1 >= self.options.len() {
                self.selected = 0;
            } else {
                self.selected += 1;
            }
        }
    }

    pub fn select_prev(&mut self) {
        if !self.options.is_empty() {
            if self.selected == 0 {
                self.selected = self.options.len() - 1;
            } else {
                self.selected -= 1;
            }
        }
    }

    pub fn toggle_remember(&mut self) {
        self.remember = !self.remember;
    }

    pub fn selected_choice(&self) -> RestoreChoice {
        self.options
            .get(self.selected)
            .map(|o| o.choice)
            .unwrap_or(RestoreChoice::RestorePrevious)
    }
}

pub enum RestoreSelectorResult {
    Continue,
    Confirm(RestoreChoice, bool),
    Cancel,
}

pub fn handle_key(key: KeyEvent, state: &mut RestoreSelectorState) -> RestoreSelectorResult {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('й') => RestoreSelectorResult::Cancel,
        KeyCode::Enter => RestoreSelectorResult::Confirm(state.selected_choice(), state.remember),
        KeyCode::Char('1') => {
            state.selected = 0;
            RestoreSelectorResult::Confirm(RestoreChoice::RestorePrevious, state.remember)
        }
        KeyCode::Char('2') => {
            state.selected = 1;
            RestoreSelectorResult::Confirm(RestoreChoice::StartFresh, state.remember)
        }
        KeyCode::Char('3') => {
            state.selected = 2;
            RestoreSelectorResult::Confirm(RestoreChoice::BrowseHistory, state.remember)
        }
        KeyCode::Char(' ')
        | KeyCode::Char('r')
        | KeyCode::Char('R')
        | KeyCode::Char('к')
        | KeyCode::Char('К') => {
            state.toggle_remember();
            RestoreSelectorResult::Continue
        }
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('л') => {
            state.select_prev();
            RestoreSelectorResult::Continue
        }
        KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('о') => {
            state.select_next();
            RestoreSelectorResult::Continue
        }
        _ => RestoreSelectorResult::Continue,
    }
}

pub fn render(f: &mut Frame, area: Rect, state: &RestoreSelectorState) {
    let width = 72.min(area.width.saturating_sub(4));
    let height = 16.min(area.height.saturating_sub(2));

    let horiz_pad = (area.width.saturating_sub(width)) / 2;
    let vert_pad = (area.height.saturating_sub(height)) / 2;

    let modal_area = Rect {
        x: horiz_pad,
        y: vert_pad,
        width,
        height,
    };

    f.render_widget(Clear, modal_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header title
            Constraint::Min(7),    // Options list
            Constraint::Length(3), // Remember checkbox & hint
        ])
        .split(modal_area);

    // 1. Header
    let block = styled_block("ax Session Restore", true);
    let title_p = Paragraph::new(Line::from(vec![
        Span::styled(" Project: ", Style::default().fg(COLOR_CYAN)),
        Span::styled(
            &state.project,
            Style::default().fg(COLOR_FG).add_modifier(Modifier::BOLD),
        ),
        Span::styled("  —  Previous session layout detected", muted_style()),
    ]))
    .block(block)
    .style(base_style());
    f.render_widget(title_p, chunks[0]);

    // 2. Options list
    let items: Vec<ListItem> = state
        .options
        .iter()
        .enumerate()
        .map(|(i, opt)| {
            let is_selected = i == state.selected;
            let line1 = Line::from(vec![
                Span::styled(
                    format!(" [{}] ", opt.key_label),
                    if is_selected {
                        Style::default()
                            .fg(COLOR_YELLOW)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        muted_style()
                    },
                ),
                Span::styled(
                    &opt.title,
                    if is_selected {
                        Style::default()
                            .fg(COLOR_GREEN)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(COLOR_FG)
                    },
                ),
            ]);
            let line2 = Line::from(vec![
                Span::raw("     "),
                Span::styled(
                    &opt.subtitle,
                    if is_selected {
                        Style::default().fg(COLOR_PURPLE)
                    } else {
                        muted_style()
                    },
                ),
            ]);

            if is_selected {
                ListItem::new(vec![line1, line2]).style(highlight_style())
            } else {
                ListItem::new(vec![line1, line2]).style(base_style())
            }
        })
        .collect();

    let list_block = styled_block("Choose Restore Option (1-3)", false);
    let list = List::new(items)
        .block(list_block)
        .highlight_style(highlight_style());

    let mut list_state = ListState::default();
    list_state.select(Some(state.selected));
    f.render_stateful_widget(list, chunks[1], &mut list_state);

    // 3. Remember checkbox & footer
    let checkbox = if state.remember { "[x]" } else { "[ ]" };
    let footer_p = Paragraph::new(vec![
        Line::from(vec![Span::styled(
            format!(
                "  {} Remember choice in ~/.config/ax/config.toml (Space/r)",
                checkbox
            ),
            if state.remember {
                Style::default()
                    .fg(COLOR_GREEN)
                    .add_modifier(Modifier::BOLD)
            } else {
                muted_style()
            },
        )]),
        Line::from(vec![Span::styled(
            "  Enter: confirm • 1-3: direct pick • Esc: cancel",
            muted_style(),
        )]),
    ])
    .style(base_style());
    f.render_widget(footer_p, chunks[2]);
}

/// Run interactive restore selector modal
pub fn run_restore_selector(
    project: String,
    snapshot: &SessionSnapshot,
) -> Result<Option<(RestoreChoice, bool)>> {
    let mut guard = TerminalGuard::enter()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut state = RestoreSelectorState::new(project, snapshot);

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
                    RestoreSelectorResult::Continue => {}
                    RestoreSelectorResult::Confirm(choice, remember) => {
                        guard.leave();
                        return Ok(Some((choice, remember)));
                    }
                    RestoreSelectorResult::Cancel => {
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
    fn test_restore_selector_state_and_navigation() {
        let snap = SessionSnapshot {
            project: "my-proj".to_string(),
            session_name: "agy-my-proj".to_string(),
            timestamp: 1234567890,
            name: None,
            windows: vec![],
        };

        let mut state = RestoreSelectorState::new("my-proj".to_string(), &snap);
        assert_eq!(state.options.len(), 3);
        assert_eq!(state.selected, 0);
        assert_eq!(state.selected_choice(), RestoreChoice::RestorePrevious);

        state.select_next();
        assert_eq!(state.selected, 1);
        assert_eq!(state.selected_choice(), RestoreChoice::StartFresh);

        state.select_next();
        assert_eq!(state.selected, 2);
        assert_eq!(state.selected_choice(), RestoreChoice::BrowseHistory);

        state.select_next();
        assert_eq!(state.selected, 0);

        state.toggle_remember();
        assert!(state.remember);
    }

    #[test]
    fn test_restore_selector_handle_key() {
        let snap = SessionSnapshot {
            project: "my-proj".to_string(),
            session_name: "agy-my-proj".to_string(),
            timestamp: 1234567890,
            name: None,
            windows: vec![],
        };

        let mut state = RestoreSelectorState::new("my-proj".to_string(), &snap);

        // '1' confirms RestorePrevious
        match handle_key(KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE), &mut state) {
            RestoreSelectorResult::Confirm(choice, rem) => {
                assert_eq!(choice, RestoreChoice::RestorePrevious);
                assert!(!rem);
            }
            _ => panic!("Expected Confirm"),
        }

        // '2' confirms StartFresh
        state.remember = true;
        match handle_key(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE), &mut state) {
            RestoreSelectorResult::Confirm(choice, rem) => {
                assert_eq!(choice, RestoreChoice::StartFresh);
                assert!(rem);
            }
            _ => panic!("Expected Confirm"),
        }

        // '3' confirms BrowseHistory
        match handle_key(KeyEvent::new(KeyCode::Char('3'), KeyModifiers::NONE), &mut state) {
            RestoreSelectorResult::Confirm(choice, _) => {
                assert_eq!(choice, RestoreChoice::BrowseHistory);
            }
            _ => panic!("Expected Confirm"),
        }

        // Esc cancels
        match handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), &mut state) {
            RestoreSelectorResult::Cancel => {}
            _ => panic!("Expected Cancel"),
        }
    }
}
