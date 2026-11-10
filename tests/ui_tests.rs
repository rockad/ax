use ax::cli::ScopeArg;
use ax::ui::helper::{self, HelperState};
use ax::ui::palette::{self, PaletteAction, PaletteState};
use ax::ui::picker::{self, PickerState};
use ax::ui::theme::{self, COLOR_BG, COLOR_GREEN, PL_LEFT, PL_RIGHT};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use std::path::PathBuf;

#[test]
fn test_theme_tokens_and_pills() {
    let p = theme::pill("active", COLOR_GREEN, COLOR_BG);
    assert_eq!(p.spans.len(), 3);
    assert_eq!(p.spans[0].content, PL_LEFT);
    assert_eq!(p.spans[1].content, " active ");
    assert_eq!(p.spans[2].content, PL_RIGHT);

    let block = theme::styled_block("Test", true);
    // Block is created without panics
    let _ = block;
}

#[test]
fn test_helper_modal_render_and_scroll() {
    let backend = TestBackend::new(80, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut state = HelperState::new();

    terminal
        .draw(|f| helper::render(f, f.area(), &state))
        .unwrap();

    let buffer = terminal.backend().buffer();
    let rendered_text: String = buffer.content().iter().map(|c| c.symbol()).collect();

    // Verify key sections and bilingual bindings are rendered
    assert!(rendered_text.contains("Keyboard Shortcuts"));
    assert!(rendered_text.contains("TAB NAVIGATION & MOVEMENT"));
    assert!(rendered_text.contains("Ctrl+Shift+Left / Right"));
    assert!(rendered_text.contains("CONVERSATION & WORKTREE LIFECYCLE"));
    assert!(rendered_text.contains("Ctrl+Space 1..9"));
    assert!(rendered_text.contains("Ctrl+Space c"));
    assert!(rendered_text.contains("Ctrl+Space s"));

    // Test scrolling logic
    assert_eq!(state.scroll, 0);
    let down_key = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
    assert!(!helper::handle_key(down_key, &mut state));
    assert_eq!(state.scroll, 1);

    // Scroll down further to see Modals & Global Controls
    state.scroll = 10;
    terminal
        .draw(|f| helper::render(f, f.area(), &state))
        .unwrap();
    let scrolled_text: String = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect();
    assert!(scrolled_text.contains("Ctrl+Space z"));
    assert!(scrolled_text.contains("Ctrl+Space p"));

    state.scroll = 1;
    let up_key = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
    assert!(!helper::handle_key(up_key, &mut state));
    assert_eq!(state.scroll, 0);

    // Esc closes helper
    let esc_key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
    assert!(helper::handle_key(esc_key, &mut state));
}

#[test]
fn test_palette_modal_render_and_fuzzy_filter() {
    let backend = TestBackend::new(70, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut state = PaletteState::new();

    assert_eq!(state.items.len(), 33);

    terminal
        .draw(|f| palette::render(f, f.area(), &state))
        .unwrap();

    let buffer = terminal.backend().buffer();
    let rendered_text: String = buffer.content().iter().map(|c| c.symbol()).collect();
    assert!(rendered_text.contains("Command Palette"));
    assert!(rendered_text.contains("Action >"));

    // Test typing and filtering for "ren"
    let char_r = KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE);
    let char_e = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE);
    let char_n = KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE);
    palette::handle_key(char_r, &mut state);
    palette::handle_key(char_e, &mut state);
    palette::handle_key(char_n, &mut state);

    assert_eq!(state.query, "ren");
    assert!(!state.filtered_indices.is_empty());
    assert_eq!(state.selected_action(), Some(PaletteAction::RenameTab));

    // Test query for "restore" finds RestoreSession
    state.query.clear();
    for c in "restore".chars() {
        palette::handle_key(
            KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
            &mut state,
        );
    }
    assert_eq!(state.selected_action(), Some(PaletteAction::RestoreSession));

    // Test query for "history" finds SessionHistory
    state.query.clear();
    for c in "history".chars() {
        palette::handle_key(
            KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
            &mut state,
        );
    }
    assert_eq!(state.selected_action(), Some(PaletteAction::SessionHistory));

    // Test query for "reload" / "sync" finds ReloadActiveSession
    state.query.clear();
    for c in "reload".chars() {
        palette::handle_key(
            KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
            &mut state,
        );
    }
    assert_eq!(
        state.selected_action(),
        Some(PaletteAction::ReloadActiveSession)
    );

    // Test query for "horizontal" finds SplitHorizontal
    state.query.clear();
    for c in "horizontal".chars() {
        palette::handle_key(
            KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
            &mut state,
        );
    }
    assert_eq!(
        state.selected_action(),
        Some(PaletteAction::SplitHorizontal)
    );

    // Test query for "vertical" finds SplitVertical
    state.query.clear();
    for c in "vertical".chars() {
        palette::handle_key(
            KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
            &mut state,
        );
    }
    assert_eq!(state.selected_action(), Some(PaletteAction::SplitVertical));

    // Test query for "zoom" finds TogglePaneZoom
    state.query.clear();
    for c in "zoom".chars() {
        palette::handle_key(
            KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
            &mut state,
        );
    }
    assert_eq!(state.selected_action(), Some(PaletteAction::TogglePaneZoom));

    // Test query for "copy" / "scroll" finds CopyMode
    state.query.clear();
    for c in "copy".chars() {
        palette::handle_key(
            KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
            &mut state,
        );
    }
    assert_eq!(state.selected_action(), Some(PaletteAction::CopyMode));

    // Test query for "quit" / "exit" finds QuitKeepTabs
    state.query.clear();
    for c in "quit".chars() {
        palette::handle_key(
            KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
            &mut state,
        );
    }
    assert_eq!(state.selected_action(), Some(PaletteAction::QuitKeepTabs));

    state.query.clear();
    for c in "exit".chars() {
        palette::handle_key(
            KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
            &mut state,
        );
    }
    assert_eq!(state.selected_action(), Some(PaletteAction::QuitKeepTabs));

    // Test query for "kill" / "terminate" finds KillSession
    state.query.clear();
    for c in "kill".chars() {
        palette::handle_key(
            KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
            &mut state,
        );
    }
    assert_eq!(state.selected_action(), Some(PaletteAction::KillSession));

    // Test query for "bind" finds BindConversation
    state.query.clear();
    for c in "bind".chars() {
        palette::handle_key(
            KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
            &mut state,
        );
    }
    assert_eq!(
        state.selected_action(),
        Some(PaletteAction::BindConversation)
    );

    // Test query for "terminal" finds NewTerminalTab
    state.query.clear();
    for c in "terminal".chars() {
        palette::handle_key(
            KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
            &mut state,
        );
    }
    assert_eq!(state.selected_action(), Some(PaletteAction::NewTerminalTab));

    // Test query for "main" finds SwitchToMain
    state.query.clear();
    for c in "main".chars() {
        palette::handle_key(
            KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
            &mut state,
        );
    }
    assert_eq!(state.selected_action(), Some(PaletteAction::SwitchToMain));

    // Test query for "claude" finds NewClaudeSession
    state.query.clear();
    for c in "claude".chars() {
        palette::handle_key(
            KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
            &mut state,
        );
    }
    assert_eq!(
        state.selected_action(),
        Some(PaletteAction::NewClaudeSession)
    );

    // Test query for "antigravity" finds NewAntigravitySession
    state.query.clear();
    for c in "antigravity".chars() {
        palette::handle_key(
            KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
            &mut state,
        );
    }
    assert_eq!(
        state.selected_action(),
        Some(PaletteAction::NewAntigravitySession)
    );

    // Test query for "move left" finds MoveTabLeft
    state.query.clear();
    for c in "left".chars() {
        palette::handle_key(
            KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
            &mut state,
        );
    }
    assert_eq!(state.selected_action(), Some(PaletteAction::MoveTabLeft));

    // Test query for "move right" finds MoveTabRight
    state.query.clear();
    for c in "right".chars() {
        palette::handle_key(
            KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
            &mut state,
        );
    }
    assert_eq!(state.selected_action(), Some(PaletteAction::MoveTabRight));

    // Test query for "join" finds JoinTab
    state.query.clear();
    for c in "join".chars() {
        palette::handle_key(
            KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
            &mut state,
        );
    }
    assert_eq!(state.selected_action(), Some(PaletteAction::JoinTab));

    // Test query for "unsplit" finds UnsplitPane
    state.query.clear();
    for c in "unsplit".chars() {
        palette::handle_key(
            KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
            &mut state,
        );
    }
    assert_eq!(state.selected_action(), Some(PaletteAction::UnsplitPane));
}

#[test]
fn test_picker_modal_render_and_scope_toggle() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut state = PickerState::new(ScopeArg::Local, "ax".to_string(), PathBuf::from("/tmp"));

    terminal
        .draw(|f| picker::render(f, f.area(), &mut state))
        .unwrap();

    let buffer = terminal.backend().buffer();
    let rendered_text: String = buffer.content().iter().map(|c| c.symbol()).collect();
    assert!(rendered_text.contains("Switcher"));
    assert!(rendered_text.contains("Tabs & Conversations"));
    assert!(rendered_text.contains("[Local]"));

    // Toggle scope to Global
    state.toggle_scope();
    assert_eq!(state.scope, ScopeArg::Global);

    terminal
        .draw(|f| picker::render(f, f.area(), &mut state))
        .unwrap();

    let buffer2 = terminal.backend().buffer();
    let rendered_text2: String = buffer2.content().iter().map(|c| c.symbol()).collect();
    assert!(rendered_text2.contains("[Global Search]"));
}

#[test]
fn test_picker_category_cycling_and_filtering() {
    use ax::core::AgentHarness;
    use ax::ui::picker::{handle_key, CategoryFilter, ItemKind, PickerCandidate};

    let mut state = PickerState::new(ScopeArg::Local, "ax".to_string(), PathBuf::from("/tmp"));
    state.all_candidates = vec![
        PickerCandidate {
            display_label: "Tab 1".to_string(),
            match_text: "Tab 1 agy".to_string(),
            kind: ItemKind::OpenTab {
                session: "agy-ax".to_string(),
                window_index: 1,
                window_name: "agy".to_string(),
                is_active: true,
                conversation_id: Some("conv-1".to_string()),
                pane_path: "/tmp".to_string(),
                harness: AgentHarness::Antigravity,
            },
        },
        PickerCandidate {
            display_label: "Tab 2".to_string(),
            match_text: "Tab 2 claude".to_string(),
            kind: ItemKind::OpenTab {
                session: "claude-ax".to_string(),
                window_index: 2,
                window_name: "claude".to_string(),
                is_active: false,
                conversation_id: Some("conv-2".to_string()),
                pane_path: "/tmp".to_string(),
                harness: AgentHarness::ClaudeCode,
            },
        },
        PickerCandidate {
            display_label: "Hist 1".to_string(),
            match_text: "Hist 1 opencode".to_string(),
            kind: ItemKind::Historical {
                conversation_id: "conv-3".to_string(),
                title: "opencode task".to_string(),
                relative_time: "5m ago".to_string(),
                project: "ax".to_string(),
                step_count: 3,
                harness: AgentHarness::OpenCode,
                last_modified_epoch: 1000,
            },
        },
    ];

    assert_eq!(state.category, CategoryFilter::All);
    state.filter();
    assert_eq!(state.filtered_indices.len(), 3);

    // Press Tab -> OpenTabs
    let tab_key = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
    handle_key(tab_key, &mut state);
    assert_eq!(state.category, CategoryFilter::OpenTabs);
    assert_eq!(state.filtered_indices.len(), 2);

    // Press Tab -> Antigravity
    handle_key(tab_key, &mut state);
    assert_eq!(state.category, CategoryFilter::Antigravity);
    assert_eq!(state.filtered_indices.len(), 1);

    // Press Tab -> Claude
    handle_key(tab_key, &mut state);
    assert_eq!(state.category, CategoryFilter::Claude);
    assert_eq!(state.filtered_indices.len(), 1);

    // Press Tab -> OpenCode
    handle_key(tab_key, &mut state);
    assert_eq!(state.category, CategoryFilter::OpenCode);
    assert_eq!(state.filtered_indices.len(), 1);

    // Press BackTab -> Claude
    let backtab_key = KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE);
    handle_key(backtab_key, &mut state);
    assert_eq!(state.category, CategoryFilter::Claude);

    // Press Ctrl+G -> Scope toggle
    let ctrl_g_key = KeyEvent::new(KeyCode::Char('g'), KeyModifiers::CONTROL);
    handle_key(ctrl_g_key, &mut state);
    assert_eq!(state.scope, ScopeArg::Global);
}

#[test]
fn test_picker_collision_prompt_flow() {
    use ax::core::AgentHarness;
    use ax::ui::picker::{handle_key, ItemKind, PickerCandidate, PickerResult};

    let mut state = PickerState::new(ScopeArg::Local, "ax".to_string(), PathBuf::from("/tmp"));
    state
        .open_convs
        .insert("conv-active".to_string(), ("agy-ax".to_string(), 2));

    state.all_candidates = vec![PickerCandidate {
        display_label: "Hist Active".to_string(),
        match_text: "Hist Active".to_string(),
        kind: ItemKind::Historical {
            conversation_id: "conv-active".to_string(),
            title: "Active task".to_string(),
            relative_time: "just now".to_string(),
            project: "ax".to_string(),
            step_count: 5,
            harness: AgentHarness::Antigravity,
            last_modified_epoch: 2000,
        },
    }];
    state.filter();

    // Select candidate with Enter -> triggers collision prompt
    let enter_key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
    let res = handle_key(enter_key, &mut state);
    assert!(matches!(res, PickerResult::Continue));
    assert!(state.collision_prompt.is_some());
    let prompt = state.collision_prompt.as_ref().unwrap();
    assert_eq!(prompt.target_window_index, 2);
    assert_eq!(prompt.conversation_id, "conv-active");

    // Press S -> switch to open tab
    let s_key = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE);
    let switch_res = handle_key(s_key, &mut state);
    assert!(state.collision_prompt.is_none());
    match switch_res {
        PickerResult::Select(candidate) => match candidate.kind {
            ItemKind::OpenTab { window_index, .. } => assert_eq!(window_index, 2),
            _ => panic!("Expected OpenTab selection"),
        },
        _ => panic!("Expected Select result"),
    }
}
