use crossterm::event::{KeyCode, KeyEvent};
use nucleo_matcher::{Config, Matcher, Utf32Str};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::ui::theme::{
    base_style, highlight_style, styled_block, COLOR_CYAN, COLOR_FG, COLOR_GREEN, COLOR_PINK,
    COLOR_YELLOW,
};

#[derive(Debug, Clone)]
pub struct JoinCandidate {
    pub index: String,
    pub name: String,
    pub harness: String,
}

pub struct JoinPickerState {
    pub candidates: Vec<JoinCandidate>,
    pub filtered_indices: Vec<usize>,
    pub selected: usize,
    pub query: String,
    pub vertical: bool,
}

impl JoinPickerState {
    pub fn new(items: &[(String, String, String)], vertical: bool) -> Self {
        let candidates = items
            .iter()
            .map(|(idx, name, harness)| JoinCandidate {
                index: idx.clone(),
                name: name.clone(),
                harness: harness.clone(),
            })
            .collect::<Vec<_>>();
        let filtered_indices = (0..candidates.len()).collect();
        Self {
            candidates,
            filtered_indices,
            selected: 0,
            query: String::new(),
            vertical,
        }
    }

    pub fn filter(&mut self) {
        if self.query.trim().is_empty() {
            self.filtered_indices = (0..self.candidates.len()).collect();
        } else {
            let mut matcher = Matcher::new(Config::DEFAULT);
            let mut query_buf = Vec::new();
            let needle = Utf32Str::new(&self.query, &mut query_buf);

            let mut scored = Vec::new();
            for (idx, c) in self.candidates.iter().enumerate() {
                let haystack_str = format!("{} {} {}", c.index, c.name, c.harness);
                let mut haystack_buf = Vec::new();
                let haystack = Utf32Str::new(&haystack_str, &mut haystack_buf);
                if let Some(score) = matcher.fuzzy_match(haystack, needle) {
                    scored.push((score, idx));
                }
            }
            scored.sort_by_key(|a| std::cmp::Reverse(a.0));
            self.filtered_indices = scored.into_iter().map(|(_, idx)| idx).collect();
        }

        if self.filtered_indices.is_empty() {
            self.selected = 0;
        } else if self.selected >= self.filtered_indices.len() {
            self.selected = self.filtered_indices.len() - 1;
        }
    }

    pub fn select_next(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.selected = (self.selected + 1) % self.filtered_indices.len();
        }
    }

    pub fn select_prev(&mut self) {
        if !self.filtered_indices.is_empty() {
            if self.selected == 0 {
                self.selected = self.filtered_indices.len() - 1;
            } else {
                self.selected -= 1;
            }
        }
    }

    pub fn selected_candidate(&self) -> Option<&JoinCandidate> {
        self.filtered_indices
            .get(self.selected)
            .and_then(|&idx| self.candidates.get(idx))
    }
}

pub enum JoinPickerResult {
    Continue,
    Select(String),
    Cancel,
}

pub fn handle_key(key: KeyEvent, state: &mut JoinPickerState) -> JoinPickerResult {
    match key.code {
        KeyCode::Esc => JoinPickerResult::Cancel,
        KeyCode::Enter => {
            if let Some(c) = state.selected_candidate() {
                JoinPickerResult::Select(c.index.clone())
            } else {
                JoinPickerResult::Cancel
            }
        }
        KeyCode::Tab => {
            state.vertical = !state.vertical;
            JoinPickerResult::Continue
        }
        KeyCode::Up | KeyCode::Char('k')
            if key
                .modifiers
                .contains(crossterm::event::KeyModifiers::CONTROL) =>
        {
            state.select_prev();
            JoinPickerResult::Continue
        }
        KeyCode::Down | KeyCode::Char('j')
            if key
                .modifiers
                .contains(crossterm::event::KeyModifiers::CONTROL) =>
        {
            state.select_next();
            JoinPickerResult::Continue
        }
        KeyCode::Up => {
            state.select_prev();
            JoinPickerResult::Continue
        }
        KeyCode::Down => {
            state.select_next();
            JoinPickerResult::Continue
        }
        KeyCode::Backspace => {
            state.query.pop();
            state.filter();
            JoinPickerResult::Continue
        }
        KeyCode::Char(c) if c.is_ascii_digit() && state.query.is_empty() => {
            let digit_str = c.to_string();
            if let Some(c) = state.candidates.iter().find(|cand| cand.index == digit_str) {
                JoinPickerResult::Select(c.index.clone())
            } else {
                state.query.push(c);
                state.filter();
                JoinPickerResult::Continue
            }
        }
        KeyCode::Char(c) => {
            state.query.push(c);
            state.filter();
            JoinPickerResult::Continue
        }
        _ => JoinPickerResult::Continue,
    }
}

pub fn render(f: &mut Frame, area: Rect, state: &JoinPickerState) {
    let popup_width = 70.min(area.width.saturating_sub(4));
    let popup_height = 16.min(area.height.saturating_sub(2));

    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let split_mode = if state.vertical {
        "Vertical Split"
    } else {
        "Horizontal Split"
    };
    let title = format!(
        " Join Tab as Split (Tab: Toggle, Enter: Select) [{}] ",
        split_mode
    );
    let block = styled_block(&title, true);
    let inner_area = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(inner_area);

    let prompt_text = format!("> {}█", state.query);
    let search_bar = Paragraph::new(prompt_text).style(Style::default().fg(COLOR_CYAN));
    f.render_widget(search_bar, chunks[0]);

    let list_items: Vec<ListItem> = state
        .filtered_indices
        .iter()
        .enumerate()
        .map(|(display_idx, &real_idx)| {
            let c = &state.candidates[real_idx];
            let is_selected = display_idx == state.selected;

            let harness_display = if c.harness.is_empty() {
                "term".to_string()
            } else {
                c.harness.clone()
            };

            let prefix = if is_selected { "▶ " } else { "  " };
            let line = Line::from(vec![
                Span::styled(
                    prefix,
                    Style::default().fg(if is_selected { COLOR_GREEN } else { COLOR_FG }),
                ),
                Span::styled(
                    format!("[{}] ", c.index),
                    Style::default()
                        .fg(COLOR_YELLOW)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{}: ", harness_display),
                    Style::default().fg(COLOR_PINK),
                ),
                Span::styled(
                    c.name.clone(),
                    Style::default()
                        .fg(if is_selected { COLOR_GREEN } else { COLOR_FG })
                        .add_modifier(if is_selected {
                            Modifier::BOLD
                        } else {
                            Modifier::empty()
                        }),
                ),
            ]);

            let style = if is_selected {
                highlight_style()
            } else {
                base_style()
            };

            ListItem::new(line).style(style)
        })
        .collect();

    let list = List::new(list_items);
    let mut list_state = ListState::default();
    list_state.select(Some(state.selected));
    f.render_stateful_widget(list, chunks[1], &mut list_state);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_join_picker_state_filter_and_direct_digit() {
        let items = vec![
            (
                "1".to_string(),
                "main terminal".to_string(),
                "term".to_string(),
            ),
            (
                "2".to_string(),
                "claude session".to_string(),
                "claude".to_string(),
            ),
            ("3".to_string(), "agy review".to_string(), "agy".to_string()),
        ];

        let mut state = JoinPickerState::new(&items, false);
        assert_eq!(state.candidates.len(), 3);
        assert_eq!(state.filtered_indices.len(), 3);

        // Filter by text
        state.query = "claude".to_string();
        state.filter();
        assert_eq!(state.filtered_indices.len(), 1);
        assert_eq!(state.selected_candidate().unwrap().index, "2");

        // Key digit direct select
        state.query.clear();
        state.filter();
        let key_3 = KeyEvent::new(KeyCode::Char('3'), crossterm::event::KeyModifiers::NONE);
        match handle_key(key_3, &mut state) {
            JoinPickerResult::Select(idx) => assert_eq!(idx, "3"),
            _ => panic!("Expected Select"),
        }

        // Toggle vertical
        assert!(!state.vertical);
        let tab_key = KeyEvent::new(KeyCode::Tab, crossterm::event::KeyModifiers::NONE);
        let _ = handle_key(tab_key, &mut state);
        assert!(state.vertical);
    }
}
