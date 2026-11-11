use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use nucleo_matcher::{Config, Matcher, Utf32Str};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use serde_json::Value;
use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
    process::Command,
};

use crate::cli::ScopeArg;
use crate::ui::theme::{
    base_style, highlight_style, muted_style, styled_block, COLOR_CYAN, COLOR_FG, COLOR_GREEN,
    COLOR_MUTED, COLOR_PINK, COLOR_PURPLE, COLOR_YELLOW,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CategoryFilter {
    All,
    OpenTabs,
    Antigravity,
    Claude,
    OpenCode,
}

impl CategoryFilter {
    pub fn all() -> &'static [CategoryFilter] {
        &[
            CategoryFilter::All,
            CategoryFilter::OpenTabs,
            CategoryFilter::Antigravity,
            CategoryFilter::Claude,
            CategoryFilter::OpenCode,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            CategoryFilter::All => "All",
            CategoryFilter::OpenTabs => "Open Tabs",
            CategoryFilter::Antigravity => "󰚩 agy",
            CategoryFilter::Claude => "󰊠 claude",
            CategoryFilter::OpenCode => "󰘐 opencode",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            CategoryFilter::All => CategoryFilter::OpenTabs,
            CategoryFilter::OpenTabs => CategoryFilter::Antigravity,
            CategoryFilter::Antigravity => CategoryFilter::Claude,
            CategoryFilter::Claude => CategoryFilter::OpenCode,
            CategoryFilter::OpenCode => CategoryFilter::All,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            CategoryFilter::All => CategoryFilter::OpenCode,
            CategoryFilter::OpenTabs => CategoryFilter::All,
            CategoryFilter::Antigravity => CategoryFilter::OpenTabs,
            CategoryFilter::Claude => CategoryFilter::Antigravity,
            CategoryFilter::OpenCode => CategoryFilter::Claude,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CollisionState {
    pub target_session: String,
    pub target_window_index: usize,
    pub conversation_id: String,
    pub harness: crate::core::AgentHarness,
    pub project: String,
}

#[derive(Debug, Clone)]
pub enum ItemKind {
    NewTab,
    OpenTab {
        session: String,
        window_index: usize,
        window_name: String,
        is_active: bool,
        conversation_id: Option<String>,
        pane_path: String,
        harness: crate::core::AgentHarness,
    },
    Historical {
        conversation_id: String,
        title: String,
        relative_time: String,
        project: String,
        step_count: i64,
        harness: crate::core::AgentHarness,
        last_modified_epoch: i64,
    },
}

#[derive(Debug, Clone)]
pub struct PickerCandidate {
    pub display_label: String,
    pub match_text: String,
    pub kind: ItemKind,
}

#[derive(Debug, Default, Clone)]
pub struct TranscriptInfo {
    pub initial_prompt: Option<String>,
    pub latest_turn: Option<String>,
    pub latest_type: Option<String>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_tokens: u64,
    pub tools_count: u64,
    pub tools_breakdown: HashMap<String, u64>,
}

pub struct PickerState {
    pub scope: ScopeArg,
    pub project_name: String,
    pub workdir: PathBuf,
    pub query: String,
    pub category: CategoryFilter,
    pub collision_prompt: Option<CollisionState>,
    pub open_convs: HashMap<String, (String, usize)>,
    pub all_candidates: Vec<PickerCandidate>,
    pub filtered_indices: Vec<usize>,
    pub selected: usize,
    pub transcript_cache: HashMap<String, TranscriptInfo>,
}

impl PickerState {
    pub fn new(scope: ScopeArg, project_name: String, workdir: PathBuf) -> Self {
        let mut state = Self {
            scope,
            project_name,
            workdir,
            query: String::new(),
            category: CategoryFilter::All,
            collision_prompt: None,
            open_convs: HashMap::new(),
            all_candidates: Vec::new(),
            filtered_indices: Vec::new(),
            selected: 0,
            transcript_cache: HashMap::new(),
        };
        state.reload_candidates();
        state
    }

    pub fn toggle_scope(&mut self) {
        self.scope = match self.scope {
            ScopeArg::Local => ScopeArg::Global,
            ScopeArg::Global => ScopeArg::Local,
        };
        self.reload_candidates();
    }

    pub fn category_count(&self, cat: CategoryFilter) -> usize {
        self.all_candidates
            .iter()
            .filter(|c| match cat {
                CategoryFilter::All => true,
                CategoryFilter::OpenTabs => matches!(c.kind, ItemKind::OpenTab { .. }),
                CategoryFilter::Antigravity => match &c.kind {
                    ItemKind::OpenTab { harness, .. } | ItemKind::Historical { harness, .. } => {
                        *harness == crate::core::AgentHarness::Antigravity
                    }
                    _ => false,
                },
                CategoryFilter::Claude => match &c.kind {
                    ItemKind::OpenTab { harness, .. } | ItemKind::Historical { harness, .. } => {
                        *harness == crate::core::AgentHarness::ClaudeCode
                    }
                    _ => false,
                },
                CategoryFilter::OpenCode => match &c.kind {
                    ItemKind::OpenTab { harness, .. } | ItemKind::Historical { harness, .. } => {
                        *harness == crate::core::AgentHarness::OpenCode
                    }
                    _ => false,
                },
            })
            .count()
    }

    pub fn reload_candidates(&mut self) {
        let _ = crate::heal_unbound_windows(&self.project_name);
        let mut candidates = Vec::new();

        // 1. New Tab
        candidates.push(PickerCandidate {
            display_label: "➕ [New Conversation Tab]".to_string(),
            match_text: "New Conversation Tab new tab fresh session".to_string(),
            kind: ItemKind::NewTab,
        });

        // 2. Open tmux tabs
        let open_tabs = load_open_tabs(self.scope, &self.project_name);
        let mut open_convs = HashMap::new();
        for tab in open_tabs {
            if let ItemKind::OpenTab {
                ref session,
                window_index,
                ref window_name,
                is_active,
                ref conversation_id,
                ref pane_path,
                ref harness,
            } = tab.kind
            {
                if let Some(ref cid) = conversation_id {
                    open_convs.insert(cid.clone(), (session.clone(), window_index));
                }
                let marker = if is_active { "▶ " } else { "  " };
                let turn_icon = if window_name.contains('●') {
                    "● "
                } else {
                    "  "
                };
                let h_icon = match harness {
                    crate::core::AgentHarness::Antigravity => "󰚩 ",
                    crate::core::AgentHarness::ClaudeCode => "󰊠 ",
                    crate::core::AgentHarness::OpenCode => "󰘐 ",
                    crate::core::AgentHarness::PlainTerminal => "> ",
                    _ => "# ",
                };
                let label = match self.scope {
                    ScopeArg::Local => format!(
                        "{}[Tab {}] {}{}{:<22} (open)",
                        marker, window_index, turn_icon, h_icon, window_name
                    ),
                    ScopeArg::Global => {
                        let clean_name = if let Some(stripped) = session.strip_prefix("agy-") {
                            if crate::core::session::is_uuid(stripped) {
                                std::path::Path::new(&pane_path)
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .filter(|n| !crate::core::session::is_uuid(n))
                                    .unwrap_or(stripped)
                            } else {
                                stripped
                            }
                        } else {
                            session
                        };
                        format!(
                            "{}[{}:{}] {}{}{:<18} (open)",
                            marker, clean_name, window_index, turn_icon, h_icon, window_name
                        )
                    }
                };
                let match_text = format!(
                    "{} {} {} {} {}",
                    session,
                    window_name,
                    conversation_id.as_deref().unwrap_or(""),
                    harness.as_str(),
                    h_icon,
                );
                candidates.push(PickerCandidate {
                    display_label: label,
                    match_text,
                    kind: tab.kind,
                });
            }
        }
        self.open_convs = open_convs;

        // 3. Historical conversations from multi-harness history
        let historical =
            crate::core::harness::load_all_historical_sessions(self.scope, &self.project_name, 50);
        for hist in historical {
            let h_badge = match hist.harness {
                crate::core::AgentHarness::Antigravity => "󰚩 agy",
                crate::core::AgentHarness::ClaudeCode => "󰊠 claude",
                crate::core::AgentHarness::OpenCode => "󰘐 opencode",
                crate::core::AgentHarness::PlainTerminal => "term",
                crate::core::AgentHarness::Codex => "codex",
                crate::core::AgentHarness::Custom(ref c) => c.as_str(),
            };

            let is_already_open = self.open_convs.contains_key(&hist.conversation_id);
            let open_tag = if is_already_open { " (open)" } else { "" };

            let label = match self.scope {
                ScopeArg::Local => format!(
                    "   [{:<8}] {:<28} ({}){}",
                    h_badge, hist.title, hist.timestamp_str, open_tag
                ),
                ScopeArg::Global => {
                    format!(
                        "   [{:<8}·{:<10}] {:<22} ({}){}",
                        h_badge, hist.project, hist.title, hist.timestamp_str, open_tag
                    )
                }
            };
            let match_text = format!(
                "{} {} {} {} {} {}",
                hist.title,
                hist.project,
                hist.timestamp_str,
                hist.conversation_id,
                hist.harness.as_str(),
                h_badge,
            );
            candidates.push(PickerCandidate {
                display_label: label,
                match_text,
                kind: ItemKind::Historical {
                    conversation_id: hist.conversation_id,
                    title: hist.title,
                    relative_time: hist.timestamp_str,
                    project: hist.project,
                    step_count: hist.step_count,
                    harness: hist.harness,
                    last_modified_epoch: hist.last_modified_epoch,
                },
            });
        }

        self.all_candidates = candidates;
        self.filter();
    }

    pub fn filter(&mut self) {
        let cat = self.category;
        let category_matching_indices: Vec<usize> = self
            .all_candidates
            .iter()
            .enumerate()
            .filter(|(_, c)| match cat {
                CategoryFilter::All => true,
                CategoryFilter::OpenTabs => {
                    matches!(c.kind, ItemKind::NewTab | ItemKind::OpenTab { .. })
                }
                CategoryFilter::Antigravity => match &c.kind {
                    ItemKind::NewTab => true,
                    ItemKind::OpenTab { harness, .. } | ItemKind::Historical { harness, .. } => {
                        *harness == crate::core::AgentHarness::Antigravity
                    }
                },
                CategoryFilter::Claude => match &c.kind {
                    ItemKind::NewTab => true,
                    ItemKind::OpenTab { harness, .. } | ItemKind::Historical { harness, .. } => {
                        *harness == crate::core::AgentHarness::ClaudeCode
                    }
                },
                CategoryFilter::OpenCode => match &c.kind {
                    ItemKind::NewTab => true,
                    ItemKind::OpenTab { harness, .. } | ItemKind::Historical { harness, .. } => {
                        *harness == crate::core::AgentHarness::OpenCode
                    }
                },
            })
            .map(|(i, _)| i)
            .collect();

        if self.query.trim().is_empty() {
            self.filtered_indices = category_matching_indices;
        } else {
            let mut matcher = Matcher::new(Config::DEFAULT);
            let mut query_buf = Vec::new();
            let needle = Utf32Str::new(&self.query, &mut query_buf);

            let mut scored = Vec::new();
            for &idx in &category_matching_indices {
                let candidate = &self.all_candidates[idx];
                let mut haystack_buf = Vec::new();
                let haystack = Utf32Str::new(&candidate.match_text, &mut haystack_buf);
                if let Some(score) = matcher.fuzzy_match(haystack, needle) {
                    scored.push((score, idx));
                }
            }

            // Sort descending by match score
            scored.sort_by_key(|a| std::cmp::Reverse(a.0));
            self.filtered_indices = scored.into_iter().map(|(_, idx)| idx).collect();
        }

        if self.selected >= self.filtered_indices.len() {
            self.selected = self.filtered_indices.len().saturating_sub(1);
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

    pub fn selected_candidate(&self) -> Option<&PickerCandidate> {
        self.filtered_indices
            .get(self.selected)
            .and_then(|&idx| self.all_candidates.get(idx))
    }

    pub fn get_or_load_transcript(&mut self, conv_id: &str) -> TranscriptInfo {
        if let Some(info) = self.transcript_cache.get(conv_id) {
            return info.clone();
        }

        let info = load_transcript(conv_id);
        self.transcript_cache
            .insert(conv_id.to_string(), info.clone());
        info
    }
}

pub enum PickerResult {
    Continue,
    Select(PickerCandidate),
    Cancel,
}

pub fn handle_key(key: KeyEvent, state: &mut PickerState) -> PickerResult {
    // If collision prompt is active, intercept input
    if let Some(col) = state.collision_prompt.clone() {
        match key.code {
            KeyCode::Char('s') | KeyCode::Char('S') | KeyCode::Enter => {
                state.collision_prompt = None;
                return PickerResult::Select(PickerCandidate {
                    display_label: format!("Switch to tab {}", col.target_window_index),
                    match_text: String::new(),
                    kind: ItemKind::OpenTab {
                        session: col.target_session,
                        window_index: col.target_window_index,
                        window_name: String::new(),
                        is_active: false,
                        conversation_id: Some(col.conversation_id),
                        pane_path: String::new(),
                        harness: col.harness,
                    },
                });
            }
            KeyCode::Char('f') | KeyCode::Char('F') => {
                state.collision_prompt = None;
                return PickerResult::Select(PickerCandidate {
                    display_label: format!("Fork conversation {}", col.conversation_id),
                    match_text: String::new(),
                    kind: ItemKind::Historical {
                        conversation_id: col.conversation_id,
                        title: String::new(),
                        relative_time: String::new(),
                        project: col.project,
                        step_count: 0,
                        harness: col.harness,
                        last_modified_epoch: 0,
                    },
                });
            }
            KeyCode::Esc => {
                state.collision_prompt = None;
                return PickerResult::Continue;
            }
            _ => return PickerResult::Continue,
        }
    }

    match key.code {
        KeyCode::Esc => PickerResult::Cancel,
        KeyCode::Enter => {
            if let Some(candidate) = state.selected_candidate() {
                if let ItemKind::Historical {
                    ref conversation_id,
                    ref project,
                    ref harness,
                    ..
                } = candidate.kind
                {
                    if let Some(&(ref open_session, open_idx)) =
                        state.open_convs.get(conversation_id)
                    {
                        state.collision_prompt = Some(CollisionState {
                            target_session: open_session.clone(),
                            target_window_index: open_idx,
                            conversation_id: conversation_id.clone(),
                            harness: harness.clone(),
                            project: project.clone(),
                        });
                        return PickerResult::Continue;
                    }
                }
                PickerResult::Select(candidate.clone())
            } else {
                PickerResult::Cancel
            }
        }
        KeyCode::Tab => {
            state.category = state.category.next();
            state.filter();
            PickerResult::Continue
        }
        KeyCode::BackTab => {
            state.category = state.category.prev();
            state.filter();
            PickerResult::Continue
        }
        KeyCode::Char('g')
        | KeyCode::Char('G')
        | KeyCode::Char('l')
        | KeyCode::Char('L')
        | KeyCode::Char('a')
            if key.modifiers.contains(KeyModifiers::CONTROL) =>
        {
            state.toggle_scope();
            PickerResult::Continue
        }
        KeyCode::Up | KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.select_prev();
            PickerResult::Continue
        }
        KeyCode::Down | KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.select_next();
            PickerResult::Continue
        }
        KeyCode::Up => {
            state.select_prev();
            PickerResult::Continue
        }
        KeyCode::Down => {
            state.select_next();
            PickerResult::Continue
        }
        KeyCode::PageUp => {
            for _ in 0..10 {
                state.select_prev();
            }
            PickerResult::Continue
        }
        KeyCode::PageDown => {
            for _ in 0..10 {
                state.select_next();
            }
            PickerResult::Continue
        }
        KeyCode::Backspace => {
            state.query.pop();
            state.filter();
            PickerResult::Continue
        }
        KeyCode::Char(c) => {
            state.query.push(c);
            state.filter();
            PickerResult::Continue
        }
        _ => PickerResult::Continue,
    }
}

pub fn render(f: &mut Frame, area: Rect, state: &mut PickerState) {
    let width = area.width.saturating_sub(4).min(140);
    let height = area.height.saturating_sub(2).min(45);

    let horiz_pad = (area.width.saturating_sub(width)) / 2;
    let vert_pad = (area.height.saturating_sub(height)) / 2;

    let modal_area = Rect {
        x: horiz_pad,
        y: vert_pad,
        width,
        height,
    };

    f.render_widget(Clear, modal_area);

    // Two-pane horizontal split: 45% left, 55% right
    let main_panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(modal_area);

    // --- LEFT PANE (45%) ---
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Categories bar (Tab / BackTab)
            Constraint::Length(3), // Search prompt input
            Constraint::Min(4),    // List of candidates
            Constraint::Length(1), // Shortcut hint footer
        ])
        .split(main_panes[0]);

    // Categories tab bar
    let mut cat_spans = Vec::new();
    for cat in CategoryFilter::all() {
        let is_active = *cat == state.category;
        let count = state.category_count(*cat);
        let label = format!("{} ({})", cat.label(), count);

        if is_active {
            cat_spans.push(Span::styled(
                format!("[ {} ]", label),
                Style::default()
                    .fg(COLOR_GREEN)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            cat_spans.push(Span::styled(
                format!("  {}  ", label),
                Style::default().fg(COLOR_MUTED),
            ));
        }
        cat_spans.push(Span::raw(" "));
    }
    let cat_p =
        Paragraph::new(Line::from(cat_spans)).block(styled_block("Categories (Tab)", false));
    f.render_widget(cat_p, left_chunks[0]);

    // Search bar
    let scope_tag = match state.scope {
        ScopeArg::Local => "Local",
        ScopeArg::Global => "Global Search",
    };
    let prompt_label = format!("⚡ [{}] > ", scope_tag);
    let search_block = styled_block("Switcher", true);
    let search_p = Paragraph::new(Line::from(vec![
        Span::styled(
            prompt_label,
            Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            &state.query,
            Style::default().fg(COLOR_FG).add_modifier(Modifier::BOLD),
        ),
        Span::styled("█", Style::default().fg(COLOR_PINK)),
    ]))
    .block(search_block)
    .style(base_style());
    f.render_widget(search_p, left_chunks[1]);

    // Candidate List
    let items: Vec<ListItem> = state
        .filtered_indices
        .iter()
        .enumerate()
        .map(|(display_idx, &item_idx)| {
            let candidate = &state.all_candidates[item_idx];
            let is_selected = display_idx == state.selected;

            let style = if is_selected {
                highlight_style()
            } else {
                base_style()
            };

            let line = match &candidate.kind {
                ItemKind::NewTab => Line::from(vec![
                    Span::styled("➕ ", Style::default().fg(COLOR_PURPLE)),
                    Span::styled(
                        &candidate.display_label,
                        if is_selected {
                            Style::default()
                                .fg(COLOR_GREEN)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(COLOR_PURPLE)
                        },
                    ),
                ]),
                ItemKind::OpenTab { .. } => Line::from(vec![Span::styled(
                    &candidate.display_label,
                    if is_selected {
                        Style::default()
                            .fg(COLOR_GREEN)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(COLOR_FG)
                    },
                )]),
                ItemKind::Historical { .. } => Line::from(vec![Span::styled(
                    &candidate.display_label,
                    if is_selected {
                        Style::default()
                            .fg(COLOR_YELLOW)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        muted_style()
                    },
                )]),
            };

            ListItem::new(line).style(style)
        })
        .collect();

    let list_block = styled_block("Tabs & Conversations", false);
    let list = List::new(items)
        .block(list_block)
        .highlight_style(highlight_style());

    let mut list_state = ListState::default();
    list_state.select(Some(state.selected));
    f.render_stateful_widget(list, left_chunks[2], &mut list_state);

    // Left Footer
    let scope_tag_clean = match state.scope {
        ScopeArg::Local => "Local",
        ScopeArg::Global => "Global",
    };
    let footer_text = format!(
        "[Scope: {} · Ctrl+G to toggle]  Tab: Category  Enter: Select  Esc: Cancel",
        scope_tag_clean
    );
    let left_footer = Paragraph::new(Line::from(vec![Span::styled(
        footer_text,
        Style::default().fg(COLOR_MUTED),
    )]))
    .style(base_style());
    f.render_widget(left_footer, left_chunks[3]);

    // --- RIGHT PANE (55%) Live Rich Transcript Preview ---
    let right_block = styled_block("Transcript Preview", false);
    let preview_inner = right_block.inner(main_panes[1]);
    f.render_widget(right_block, main_panes[1]);

    let selected_candidate = state.selected_candidate().cloned();
    if let Some(candidate) = selected_candidate {
        render_preview_content(f, preview_inner, &candidate, state);
    } else {
        let empty_p = Paragraph::new(Line::from(vec![Span::styled(
            "No item selected.",
            muted_style(),
        )]))
        .alignment(Alignment::Center);
        f.render_widget(empty_p, preview_inner);
    }

    // Modal dialog for collision prompt if active
    if let Some(ref col) = state.collision_prompt {
        let dialog_width = 68.min(modal_area.width.saturating_sub(4));
        let dialog_height = 8;
        let dialog_x = modal_area.x + (modal_area.width.saturating_sub(dialog_width)) / 2;
        let dialog_y = modal_area.y + (modal_area.height.saturating_sub(dialog_height)) / 2;
        let dialog_area = Rect {
            x: dialog_x,
            y: dialog_y,
            width: dialog_width,
            height: dialog_height,
        };

        f.render_widget(Clear, dialog_area);
        let block = styled_block("Session Already Open", true);

        let lines = vec![
            Line::raw(""),
            Line::from(vec![
                Span::styled(
                    " This conversation is already open in tab ",
                    Style::default().fg(COLOR_FG),
                ),
                Span::styled(
                    format!("#{}", col.target_window_index),
                    Style::default()
                        .fg(COLOR_YELLOW)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(" in session '{}'", col.target_session),
                    Style::default().fg(COLOR_MUTED),
                ),
            ]),
            Line::raw(""),
            Line::from(vec![
                Span::styled(
                    "   [S] ",
                    Style::default()
                        .fg(COLOR_GREEN)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("Switch to tab   ", Style::default().fg(COLOR_FG)),
                Span::styled(
                    "[F] ",
                    Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD),
                ),
                Span::styled("Fork new tab   ", Style::default().fg(COLOR_FG)),
                Span::styled(
                    "[Esc] ",
                    Style::default().fg(COLOR_PINK).add_modifier(Modifier::BOLD),
                ),
                Span::styled("Cancel", Style::default().fg(COLOR_FG)),
            ]),
        ];
        let p = Paragraph::new(lines)
            .block(block)
            .style(base_style())
            .alignment(Alignment::Center);
        f.render_widget(p, dialog_area);
    }
}

#[allow(clippy::vec_init_then_push)]
fn render_preview_content(
    f: &mut Frame,
    area: Rect,
    candidate: &PickerCandidate,
    state: &mut PickerState,
) {
    match &candidate.kind {
        ItemKind::NewTab => {
            let mut lines = Vec::new();
            lines.push(Line::from(vec![Span::styled(
                "➕ Launch New Conversation",
                Style::default()
                    .fg(COLOR_PURPLE)
                    .add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::raw(""));
            lines.push(meta_line("Project:", &state.project_name));
            lines.push(meta_line(
                "Working Dir:",
                &state.workdir.display().to_string(),
            ));
            lines.push(meta_line("Engine:", "Google Antigravity CLI (agy)"));
            lines.push(meta_line("Shortcut:", "Ctrl+Space c / с"));
            lines.push(Line::raw(""));
            lines.push(Line::from(vec![Span::styled(
                "┌── BEHAVIOR ──────────────────────────────────────────┐",
                Style::default().fg(COLOR_MUTED),
            )]));
            lines.push(Line::from(vec![Span::styled(
                "│  Spawns a fresh conversation in a new top tab.       │",
                Style::default().fg(COLOR_FG),
            )]));
            lines.push(Line::from(vec![Span::styled(
                "│  Title names immediately from your initial prompt.   │",
                Style::default().fg(COLOR_FG),
            )]));
            lines.push(Line::from(vec![Span::styled(
                "│  Running turn status displays live in status bar (●). │",
                Style::default().fg(COLOR_FG),
            )]));
            lines.push(Line::from(vec![Span::styled(
                "└──────────────────────────────────────────────────────┘",
                Style::default().fg(COLOR_MUTED),
            )]));

            let p = Paragraph::new(lines)
                .style(base_style())
                .wrap(Wrap { trim: false });
            f.render_widget(p, area);
        }

        ItemKind::OpenTab {
            session,
            window_index,
            window_name,
            conversation_id,
            pane_path,
            harness,
            ..
        } => {
            let mut lines = Vec::new();
            lines.push(Line::from(vec![Span::styled(
                format!("🗂️  Tab #{} {}", window_index, window_name),
                Style::default()
                    .fg(COLOR_GREEN)
                    .add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::raw(""));
            let clean_proj = if let Some(stripped) = session.strip_prefix("agy-") {
                if crate::core::session::is_uuid(stripped) {
                    std::path::Path::new(pane_path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .filter(|n| !crate::core::session::is_uuid(n))
                        .unwrap_or(stripped)
                } else {
                    stripped
                }
            } else {
                session
            };
            lines.push(meta_line("Harness:", harness.as_str()));
            lines.push(meta_line("Project:", clean_proj));
            lines.push(meta_line("Session:", session));
            lines.push(meta_line("Window Index:", &window_index.to_string()));
            lines.push(meta_line("Path:", pane_path));

            if let Some(ref cid) = conversation_id {
                lines.push(meta_line("Conversation ID:", cid));
                if *harness == crate::core::AgentHarness::Antigravity {
                    let t_info = state.get_or_load_transcript(cid);
                    append_transcript_lines(&mut lines, &t_info, cid);
                }
            } else {
                lines.push(meta_line(
                    "Status:",
                    "Active Tab (no bound conversation ID)",
                ));
            }

            let p = Paragraph::new(lines)
                .style(base_style())
                .wrap(Wrap { trim: false });
            f.render_widget(p, area);
        }

        ItemKind::Historical {
            conversation_id,
            title,
            relative_time,
            project,
            step_count,
            harness,
            ..
        } => {
            let mut lines = Vec::new();
            lines.push(Line::from(vec![Span::styled(
                format!("💬 {}", title),
                Style::default()
                    .fg(COLOR_PURPLE)
                    .add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::raw(""));
            lines.push(meta_line("Harness:", harness.as_str()));
            lines.push(meta_line("Conversation ID:", conversation_id));
            lines.push(meta_line("Project:", project));
            lines.push(meta_line("Last Active:", relative_time));
            lines.push(meta_line("Step Count:", &format!("{} steps", step_count)));

            if *harness == crate::core::AgentHarness::Antigravity {
                let t_info = state.get_or_load_transcript(conversation_id);
                append_transcript_lines(&mut lines, &t_info, conversation_id);
            }

            let p = Paragraph::new(lines)
                .style(base_style())
                .wrap(Wrap { trim: false });
            f.render_widget(p, area);
        }
    }
}

fn append_transcript_lines(lines: &mut Vec<Line<'static>>, info: &TranscriptInfo, _cid: &str) {
    lines.push(Line::raw(""));
    let total_tokens = info.input_tokens + info.output_tokens + info.cache_tokens;
    lines.push(Line::from(vec![
        Span::styled(
            "Tokens: ",
            Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("Input: {} ", format_number(info.input_tokens)),
            Style::default().fg(COLOR_GREEN),
        ),
        Span::styled(
            format!("| Output: {} ", format_number(info.output_tokens)),
            Style::default().fg(COLOR_GREEN),
        ),
        Span::styled(
            format!("| Cache: {} ", format_number(info.cache_tokens)),
            Style::default().fg(COLOR_GREEN),
        ),
        Span::styled(
            format!("(Total: {})", format_number(total_tokens)),
            Style::default().fg(COLOR_YELLOW),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "Tools: ",
            Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{} executed", info.tools_count),
            Style::default().fg(COLOR_FG),
        ),
    ]));
    lines.push(Line::raw(""));

    // Initial Prompt block
    lines.push(Line::from(vec![Span::styled(
        "━━━ Initial User Prompt ━━━━━━━━━━━━━━━━━━━━━━━━━",
        Style::default().fg(COLOR_CYAN),
    )]));
    if let Some(ref prompt) = info.initial_prompt {
        let clean = prompt.replace('\n', " ");
        let truncated = if clean.len() > 300 {
            format!("{}...", &clean[..300])
        } else {
            clean
        };
        lines.push(Line::from(vec![Span::styled(
            truncated,
            Style::default().fg(COLOR_FG),
        )]));
    } else {
        lines.push(Line::from(vec![Span::styled(
            "(No initial prompt recorded)",
            muted_style(),
        )]));
    }
    lines.push(Line::raw(""));

    // Latest Turn block
    lines.push(Line::from(vec![Span::styled(
        format!(
            "━━━ Latest Assistant Turn [{}] ━━━━━━━━━━━━━",
            info.latest_type.as_deref().unwrap_or("RESPONSE")
        ),
        Style::default().fg(COLOR_CYAN),
    )]));
    if let Some(ref turn) = info.latest_turn {
        let clean = turn.replace('\n', " ");
        let truncated = if clean.len() > 400 {
            format!("{}...", &clean[..400])
        } else {
            clean
        };
        lines.push(Line::from(vec![Span::styled(
            truncated,
            Style::default().fg(COLOR_FG),
        )]));
    } else {
        lines.push(Line::from(vec![Span::styled(
            "(No assistant response yet)",
            muted_style(),
        )]));
    }
    lines.push(Line::raw(""));

    // Executed Tools Breakdown
    if !info.tools_breakdown.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "━━━ Executed Tools Breakdown ━━━━━━━━━━━━━━━━━━━━",
            Style::default().fg(COLOR_CYAN),
        )]));
        let mut sorted_tools: Vec<(&String, &u64)> = info.tools_breakdown.iter().collect();
        sorted_tools.sort_by(|a, b| b.1.cmp(a.1));
        for (name, count) in sorted_tools.into_iter().take(8) {
            lines.push(Line::from(vec![
                Span::styled("  ● ", Style::default().fg(COLOR_GREEN)),
                Span::styled(
                    format!("{:<20}", name),
                    Style::default().fg(COLOR_FG).add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!(": {}", count), Style::default().fg(COLOR_YELLOW)),
            ]));
        }
    }
}

fn meta_line(label: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {:<18}", label), Style::default().fg(COLOR_CYAN)),
        Span::styled(value.to_string(), Style::default().fg(COLOR_FG)),
    ])
}

fn format_number(num: u64) -> String {
    if num >= 1_000_000 {
        format!("{:.1}M", num as f64 / 1_000_000.0)
    } else if num >= 1_000 {
        format!("{:.1}k", num as f64 / 1_000.0)
    } else {
        num.to_string()
    }
}

/// Query active tmux windows
fn load_open_tabs(scope: ScopeArg, project: &str) -> Vec<PickerCandidate> {
    let mut items = Vec::new();
    let format = "#{session_name}\t#{window_index}\t#{window_name}\t#{@conversation_id}\t#{window_active}\t#{pane_current_path}\t#{?#{@harness},#{@harness},#{pane_current_command}}";

    let output = Command::new("tmux")
        .args(["list-windows", "-a", "-F", format])
        .output();

    if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 6 {
                let session = parts[0].to_string();
                let win_idx: usize = parts[1].parse().unwrap_or(0);
                let win_name = parts[2].to_string();
                let conv_id = if parts[3].is_empty() || parts[3] == "-" {
                    None
                } else {
                    Some(parts[3].to_string())
                };
                let is_active = parts[4] == "1";
                let pane_path = parts[5].to_string();
                let harness_str = parts.get(6).copied().unwrap_or("").trim();
                let harness = crate::core::AgentHarness::from_str_loose(harness_str);

                if scope == ScopeArg::Local {
                    let matches_session = session == project
                        || session == format!("agy-{}", project)
                        || session == format!("claude-{}", project)
                        || session == format!("opencode-{}", project)
                        || session.ends_with(&format!("-{}", project));

                    let matches_path = !project.is_empty()
                        && (pane_path.ends_with(project)
                            || pane_path.contains(&format!("/{}/", project)));

                    if !matches_session && !matches_path {
                        continue;
                    }
                }

                items.push(PickerCandidate {
                    display_label: String::new(),
                    match_text: String::new(),
                    kind: ItemKind::OpenTab {
                        session,
                        window_index: win_idx,
                        window_name: win_name,
                        is_active,
                        conversation_id: conv_id,
                        pane_path,
                        harness,
                    },
                });
            }
        }
    }
    items
}

/// Parse transcript JSONL from ~/.gemini/antigravity-cli/brain/<id>/.system_generated/logs/transcript.jsonl
fn load_transcript(conv_id: &str) -> TranscriptInfo {
    let mut info = TranscriptInfo::default();
    let brain_dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".gemini/antigravity-cli/brain");

    let p1 = brain_dir
        .join(conv_id)
        .join(".system_generated/logs/transcript.jsonl");
    let p2 = brain_dir
        .join(conv_id)
        .join(".system_generated/logs/transcript_full.jsonl");

    let path = if p1.exists() {
        p1
    } else if p2.exists() {
        p2
    } else {
        return info;
    };

    if let Ok(file) = File::open(path) {
        let reader = BufReader::new(file);
        for line in reader.lines().map_while(Result::ok) {
            if let Ok(v) = serde_json::from_str::<Value>(&line) {
                let item_type = v.get("type").and_then(|t| t.as_str()).unwrap_or("");
                let source = v.get("source").and_then(|s| s.as_str()).unwrap_or("");

                // Initial prompt
                if info.initial_prompt.is_none()
                    && (item_type == "USER_INPUT" || source == "USER_EXPLICIT")
                {
                    if let Some(content) = v.get("content").and_then(|c| c.as_str()) {
                        let cleaned = clean_prompt(content);
                        if !cleaned.is_empty() {
                            info.initial_prompt = Some(cleaned);
                        }
                    }
                }

                // Latest turn
                if item_type == "PLANNER_RESPONSE" {
                    info.latest_type = Some("PLANNER".to_string());
                    if let Some(content) = v.get("content").and_then(|c| c.as_str()) {
                        if !content.trim().is_empty() {
                            info.latest_turn = Some(content.to_string());
                        }
                    } else if let Some(thinking) = v.get("thinking").and_then(|th| th.as_str()) {
                        if !thinking.trim().is_empty() {
                            info.latest_turn = Some(thinking.to_string());
                        }
                    }
                }

                // Tokens
                if let Some(inp) = v.get("input_tokens").and_then(|t| t.as_u64()) {
                    info.input_tokens += inp;
                }
                if let Some(out) = v.get("output_tokens").and_then(|t| t.as_u64()) {
                    info.output_tokens += out;
                }
                if let Some(cch) = v.get("cache_read_tokens").and_then(|t| t.as_u64()) {
                    info.cache_tokens += cch;
                }

                // Tool calls
                if let Some(calls) = v.get("tool_calls").and_then(|c| c.as_array()) {
                    for call in calls {
                        info.tools_count += 1;
                        let t_name = call
                            .get("name")
                            .or_else(|| call.get("toolAction"))
                            .and_then(|n| n.as_str())
                            .unwrap_or("tool");
                        *info.tools_breakdown.entry(t_name.to_string()).or_insert(0) += 1;
                    }
                }
            }
        }
    }

    info
}

fn clean_prompt(raw: &str) -> String {
    let mut s = raw.to_string();
    if let Some(start) = s.find("<USER_REQUEST>") {
        s = s[start + "<USER_REQUEST>".len()..].to_string();
    }
    if let Some(end) = s.find("</USER_REQUEST>") {
        s = s[..end].to_string();
    }
    s.trim().to_string()
}

/// Execute selection outside or inside tmux
pub fn execute_selection(
    candidate: &PickerCandidate,
    project: &str,
    workdir: &std::path::Path,
) -> anyhow::Result<()> {
    match &candidate.kind {
        ItemKind::NewTab => {
            if std::env::var("TMUX").is_ok() {
                let _ = Command::new("tmux")
                    .args([
                        "new-window",
                        "-c",
                        &workdir.display().to_string(),
                        "-n",
                        "+ new",
                        &format!("hx run --project '{}'", project),
                    ])
                    .status();
            } else {
                let session_name = format!("agy-{}", project);
                let config = crate::config::Config::load().unwrap_or_default();
                let mut run_cmd = format!("hx run --project '{}'", project);
                if config.general.remote_control {
                    run_cmd.push_str(" --remote-control");
                }
                crate::tmux::TmuxDriver::ensure_session(
                    &session_name,
                    workdir,
                    &config,
                    Some(&run_cmd),
                    Some("+ new"),
                )?;
                crate::tmux::TmuxDriver::attach_session(&session_name)?;
            }
        }
        ItemKind::OpenTab {
            session,
            window_index,
            ..
        } => {
            if std::env::var("TMUX").is_ok() {
                let _ = Command::new("tmux")
                    .args([
                        "switch-client",
                        "-t",
                        &format!("{}:{}", session, window_index),
                    ])
                    .status();
                let _ = Command::new("tmux")
                    .args([
                        "select-window",
                        "-t",
                        &format!("{}:{}", session, window_index),
                    ])
                    .status();
            } else {
                crate::tmux::TmuxDriver::attach_session(session)?;
            }
        }
        ItemKind::Historical {
            conversation_id,
            project: target_project,
            harness,
            ..
        } => {
            let effective_project = if target_project.is_empty()
                || target_project == "main"
                || crate::core::session::is_uuid(target_project)
            {
                if !project.is_empty() && !crate::core::session::is_uuid(project) {
                    project.to_string()
                } else if let Some(dir_name) = workdir.file_name().and_then(|n| n.to_str()) {
                    dir_name.to_string()
                } else {
                    "main".to_string()
                }
            } else {
                target_project.to_string()
            };

            let run_cmd = match harness {
                crate::core::AgentHarness::Antigravity => {
                    format!(
                        "hx run -H agy --project '{}' --conversation '{}'",
                        effective_project, conversation_id
                    )
                }
                crate::core::AgentHarness::ClaudeCode => {
                    format!(
                        "hx run -H claude --project '{}' -- --resume '{}'",
                        effective_project, conversation_id
                    )
                }
                crate::core::AgentHarness::OpenCode => {
                    format!(
                        "hx run -H opencode --project '{}' -- --session '{}'",
                        effective_project, conversation_id
                    )
                }
                crate::core::AgentHarness::PlainTerminal => {
                    format!("hx run -H term --project '{}'", effective_project)
                }
                crate::core::AgentHarness::Codex => {
                    format!("hx run -H codex --project '{}'", effective_project)
                }
                crate::core::AgentHarness::Custom(ref c) => {
                    format!("hx run -H '{}' --project '{}'", c, effective_project)
                }
            };

            let win_name = match harness {
                crate::core::AgentHarness::Antigravity => "agy",
                crate::core::AgentHarness::ClaudeCode => "claude",
                crate::core::AgentHarness::OpenCode => "opencode",
                crate::core::AgentHarness::PlainTerminal => "term",
                crate::core::AgentHarness::Codex => "codex",
                crate::core::AgentHarness::Custom(ref c) => c.as_str(),
            };

            if std::env::var("TMUX").is_ok() {
                let _ = Command::new("tmux")
                    .args([
                        "new-window",
                        "-c",
                        &workdir.display().to_string(),
                        "-n",
                        win_name,
                        &run_cmd,
                    ])
                    .status();
            } else {
                let session_name = format!("agy-{}", effective_project);
                let config = crate::config::Config::load().unwrap_or_default();
                let mut full_cmd = run_cmd;
                if config.general.remote_control
                    && *harness == crate::core::AgentHarness::Antigravity
                {
                    full_cmd.push_str(" --remote-control");
                }
                crate::tmux::TmuxDriver::ensure_session(
                    &session_name,
                    workdir,
                    &config,
                    Some(&full_cmd),
                    Some(win_name),
                )?;
                crate::tmux::TmuxDriver::attach_session(&session_name)?;
            }
        }
    }
    Ok(())
}
