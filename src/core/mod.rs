pub mod harness;
pub mod persistence;
pub mod project;
pub mod session;
pub mod tab_registry;
pub mod transcript;

pub use harness::AgentHarness;
pub use persistence::{
    delete_session_history_entry, find_missing_windows, list_session_history,
    prune_session_history, restore_session_from_history, restore_session_layout,
    restore_session_snapshot, save_named_session_layout, save_session_layout, session_history_dir,
    session_snapshot_path, sessions_dir, write_session_snapshot, LiveWindow, PaneSnapshot,
    SessionSnapshot, WindowSnapshot,
};
pub use project::{ProjectItem, ProjectScanner};
pub use session::{ConversationItem, SessionDb};
pub use tab_registry::TabRegistry;
pub use transcript::{TranscriptParser, TranscriptSummary};
