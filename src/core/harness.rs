use crate::cli::ScopeArg;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{LazyLock, Mutex};
use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum AgentHarness {
    #[default]
    Antigravity,
    ClaudeCode,
    OpenCode,
    Codex,
    PlainTerminal,
    Custom(String),
}

impl AgentHarness {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Antigravity => "agy",
            Self::ClaudeCode => "claude",
            Self::OpenCode => "opencode",
            Self::Codex => "codex",
            Self::PlainTerminal => "term",
            Self::Custom(s) => s.as_str(),
        }
    }

    pub fn from_str_loose(s: &str) -> Self {
        let trimmed = s.trim();
        let lower = trimmed.to_lowercase();
        match lower.as_str() {
            "agy" | "antigravity" | "gemini" => Self::Antigravity,
            "claude" | "claudecode" | "claude-code" => Self::ClaudeCode,
            "opencode" | "open-code" => Self::OpenCode,
            "codex" | "openai" | "openai-codex" => Self::Codex,
            "term" | "terminal" | "shell" | "sh" | "bash" | "zsh" => Self::PlainTerminal,
            "" => Self::Antigravity,
            _ => Self::Custom(trimmed.to_string()),
        }
    }

    pub fn icon(&self) -> &str {
        match self {
            Self::Antigravity => "󰚩",
            Self::ClaudeCode => "󰊠",
            Self::OpenCode => "󰘐",
            Self::Codex => "󱚣",
            Self::PlainTerminal => "",
            Self::Custom(_) => "󰘑",
        }
    }

    pub fn default_command(&self) -> &str {
        match self {
            Self::Antigravity => "agy",
            Self::ClaudeCode => "claude",
            Self::OpenCode => "opencode",
            Self::Codex => "codex",
            Self::PlainTerminal => "$SHELL",
            Self::Custom(s) => s.as_str(),
        }
    }

    pub fn default_hotkey(&self) -> Option<char> {
        match self {
            Self::Antigravity => Some('n'),
            Self::ClaudeCode => Some('c'),
            Self::OpenCode => Some('o'),
            Self::Codex => None,
            Self::PlainTerminal => Some('t'),
            Self::Custom(_) => None,
        }
    }

    pub fn cyrillic_hotkey(&self) -> Option<char> {
        match self {
            Self::Antigravity => Some('т'),
            Self::ClaudeCode => Some('с'),
            Self::OpenCode => Some('щ'),
            Self::Codex => None,
            Self::PlainTerminal => Some('е'),
            Self::Custom(_) => None,
        }
    }
}

impl fmt::Display for AgentHarness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Default)]
pub struct HarnessLaunchContext {
    pub project: String,
    pub conversation_id: Option<String>,
    pub remote_control: bool,
    pub continue_flag: bool,
    pub args: Vec<String>,
    pub workdir: Option<PathBuf>,
    pub env: HashMap<String, String>,
}

pub trait HarnessDriver: Send + Sync {
    fn id(&self) -> &str;
    fn display_name(&self) -> &str;
    fn icon(&self) -> &str;
    fn default_command(&self) -> &str;
    fn default_hotkey(&self) -> Option<char>;
    fn cyrillic_hotkey(&self) -> Option<char>;
    fn build_command(&self, ctx: &HarnessLaunchContext) -> Command;
}

#[derive(Debug, Clone, Default)]
pub struct AntigravityDriver;

impl HarnessDriver for AntigravityDriver {
    fn id(&self) -> &str {
        "agy"
    }

    fn display_name(&self) -> &str {
        "Google Antigravity"
    }

    fn icon(&self) -> &str {
        "󰚩"
    }

    fn default_command(&self) -> &str {
        "agy"
    }

    fn default_hotkey(&self) -> Option<char> {
        Some('n')
    }

    fn cyrillic_hotkey(&self) -> Option<char> {
        Some('т')
    }

    fn build_command(&self, ctx: &HarnessLaunchContext) -> Command {
        let mut cmd = Command::new("agy");
        let forward_args = crate::build_agy_args(
            &ctx.project,
            &ctx.args,
            ctx.conversation_id.as_deref(),
            ctx.continue_flag,
            ctx.remote_control,
        );
        cmd.args(&forward_args);
        if let Some(ref dir) = ctx.workdir {
            cmd.current_dir(dir);
        }
        for (k, v) in &ctx.env {
            cmd.env(k, v);
        }
        cmd
    }
}

#[derive(Debug, Clone, Default)]
pub struct ClaudeCodeDriver;

impl HarnessDriver for ClaudeCodeDriver {
    fn id(&self) -> &str {
        "claude"
    }

    fn display_name(&self) -> &str {
        "Claude Code"
    }

    fn icon(&self) -> &str {
        "󰊠"
    }

    fn default_command(&self) -> &str {
        "claude"
    }

    fn default_hotkey(&self) -> Option<char> {
        Some('c')
    }

    fn cyrillic_hotkey(&self) -> Option<char> {
        Some('с')
    }

    fn build_command(&self, ctx: &HarnessLaunchContext) -> Command {
        let mut cmd = Command::new("claude");
        cmd.args(&ctx.args);
        if let Some(ref dir) = ctx.workdir {
            cmd.current_dir(dir);
        }
        for (k, v) in &ctx.env {
            cmd.env(k, v);
        }
        cmd
    }
}

#[derive(Debug, Clone, Default)]
pub struct OpenCodeDriver;

impl HarnessDriver for OpenCodeDriver {
    fn id(&self) -> &str {
        "opencode"
    }

    fn display_name(&self) -> &str {
        "OpenCode"
    }

    fn icon(&self) -> &str {
        "󰘐"
    }

    fn default_command(&self) -> &str {
        "opencode"
    }

    fn default_hotkey(&self) -> Option<char> {
        Some('o')
    }

    fn cyrillic_hotkey(&self) -> Option<char> {
        Some('щ')
    }

    fn build_command(&self, ctx: &HarnessLaunchContext) -> Command {
        let mut cmd = Command::new("opencode");
        cmd.args(&ctx.args);
        if let Some(ref dir) = ctx.workdir {
            cmd.current_dir(dir);
        }
        for (k, v) in &ctx.env {
            cmd.env(k, v);
        }
        cmd
    }
}

#[derive(Debug, Clone, Default)]
pub struct CodexDriver;

impl HarnessDriver for CodexDriver {
    fn id(&self) -> &str {
        "codex"
    }

    fn display_name(&self) -> &str {
        "Codex"
    }

    fn icon(&self) -> &str {
        "󱚣"
    }

    fn default_command(&self) -> &str {
        "codex"
    }

    fn default_hotkey(&self) -> Option<char> {
        None
    }

    fn cyrillic_hotkey(&self) -> Option<char> {
        None
    }

    fn build_command(&self, ctx: &HarnessLaunchContext) -> Command {
        let mut cmd = Command::new("codex");
        cmd.args(&ctx.args);
        if let Some(ref dir) = ctx.workdir {
            cmd.current_dir(dir);
        }
        for (k, v) in &ctx.env {
            cmd.env(k, v);
        }
        cmd
    }
}

#[derive(Debug, Clone, Default)]
pub struct TerminalDriver;

impl HarnessDriver for TerminalDriver {
    fn id(&self) -> &str {
        "term"
    }

    fn display_name(&self) -> &str {
        "Plain Terminal"
    }

    fn icon(&self) -> &str {
        ""
    }

    fn default_command(&self) -> &str {
        "$SHELL"
    }

    fn default_hotkey(&self) -> Option<char> {
        Some('t')
    }

    fn cyrillic_hotkey(&self) -> Option<char> {
        Some('е')
    }

    fn build_command(&self, ctx: &HarnessLaunchContext) -> Command {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "sh".to_string());
        let mut cmd = Command::new(shell);
        if !ctx.args.is_empty() {
            cmd.args(&ctx.args);
        }
        if let Some(ref dir) = ctx.workdir {
            cmd.current_dir(dir);
        }
        for (k, v) in &ctx.env {
            cmd.env(k, v);
        }
        cmd
    }
}

#[derive(Debug, Clone)]
pub struct CustomDriver {
    pub id: String,
    pub display_name: String,
    pub icon: String,
    pub command: String,
    pub hotkey: Option<char>,
    pub cyrillic_hotkey: Option<char>,
    pub extra_args: Vec<String>,
    pub env: HashMap<String, String>,
}

impl CustomDriver {
    pub fn new(id: impl Into<String>, command: impl Into<String>) -> Self {
        let id_str = id.into();
        let cmd_str = command.into();
        Self {
            display_name: id_str.clone(),
            id: id_str,
            icon: "󰘑".to_string(),
            command: cmd_str,
            hotkey: None,
            cyrillic_hotkey: None,
            extra_args: Vec::new(),
            env: HashMap::new(),
        }
    }
}

impl HarnessDriver for CustomDriver {
    fn id(&self) -> &str {
        &self.id
    }

    fn display_name(&self) -> &str {
        &self.display_name
    }

    fn icon(&self) -> &str {
        &self.icon
    }

    fn default_command(&self) -> &str {
        &self.command
    }

    fn default_hotkey(&self) -> Option<char> {
        self.hotkey
    }

    fn cyrillic_hotkey(&self) -> Option<char> {
        self.cyrillic_hotkey
    }

    fn build_command(&self, ctx: &HarnessLaunchContext) -> Command {
        let mut cmd = Command::new(&self.command);
        cmd.args(&self.extra_args);
        cmd.args(&ctx.args);
        if let Some(ref dir) = ctx.workdir {
            cmd.current_dir(dir);
        }
        for (k, v) in &self.env {
            cmd.env(k, v);
        }
        for (k, v) in &ctx.env {
            cmd.env(k, v);
        }
        cmd
    }
}

pub struct HarnessRegistry {
    drivers: HashMap<String, Box<dyn HarnessDriver>>,
}

impl Default for HarnessRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl HarnessRegistry {
    pub fn new() -> Self {
        let mut reg = Self {
            drivers: HashMap::new(),
        };
        reg.register(Box::new(AntigravityDriver));
        reg.register(Box::new(ClaudeCodeDriver));
        reg.register(Box::new(OpenCodeDriver));
        reg.register(Box::new(CodexDriver));
        reg.register(Box::new(TerminalDriver));
        reg
    }

    pub fn new_with_config(config: &crate::config::Config) -> Self {
        let mut reg = Self::new();
        for (name, h_cfg) in &config.harnesses {
            let cmd = h_cfg.command.clone().unwrap_or_else(|| name.clone());
            let mut driver = CustomDriver::new(name.clone(), cmd);
            if let Some(ref disp) = h_cfg.display_name {
                driver.display_name = disp.clone();
            }
            if let Some(ref ic) = h_cfg.icon {
                driver.icon = ic.clone();
            }
            driver.hotkey = h_cfg.hotkey;
            driver.cyrillic_hotkey = h_cfg.cyrillic_hotkey;
            driver.extra_args = h_cfg.args.clone();
            driver.env = h_cfg.env.clone();
            reg.register(Box::new(driver));
        }
        reg
    }

    pub fn register(&mut self, driver: Box<dyn HarnessDriver>) {
        self.drivers.insert(driver.id().to_string(), driver);
    }

    pub fn get(&self, id: &str) -> Option<&dyn HarnessDriver> {
        self.drivers.get(id).map(|b| b.as_ref())
    }

    pub fn get_for_harness(&self, harness: &AgentHarness) -> Option<&dyn HarnessDriver> {
        self.get(harness.as_str())
    }

    pub fn drivers(&self) -> impl Iterator<Item = &dyn HarnessDriver> {
        self.drivers.values().map(|b| b.as_ref())
    }
}

/// Synchronize conversation title to backing harness stores (Antigravity SQLite, Claude JSON, OpenCode SQLite)
pub fn sync_conversation_title(
    harness: &AgentHarness,
    conversation_id: &str,
    title: &str,
) -> anyhow::Result<()> {
    match harness {
        AgentHarness::Antigravity => {
            let db_path = dirs::home_dir()
                .unwrap_or_default()
                .join(".gemini/antigravity-cli/conversation_summaries.db");
            if db_path.exists() {
                if let Ok(conn) = rusqlite::Connection::open(&db_path) {
                    let _ = conn.execute(
                        "UPDATE conversation_summaries SET title = ?1, last_modified_time = datetime('now') WHERE conversation_id = ?2;",
                        rusqlite::params![title, conversation_id],
                    );
                    if crate::core::session::table_exists(&conn, "conversations") {
                        let _ = conn.execute(
                            "UPDATE conversations SET title = ?1, updated_at = strftime('%s','now') WHERE conversation_id = ?2;",
                            rusqlite::params![title, conversation_id],
                        );
                    }
                }
            }
        }
        AgentHarness::ClaudeCode => {
            let projects_dir = dirs::home_dir()
                .unwrap_or_default()
                .join(".claude/projects");
            if projects_dir.exists() {
                if let Ok(entries) = std::fs::read_dir(&projects_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() {
                            let sess_dir = path.join(conversation_id);
                            let jsonl_file = path.join(format!("{}.jsonl", conversation_id));
                            if sess_dir.exists() || jsonl_file.exists() {
                                let _ = std::fs::create_dir_all(&sess_dir);
                                let custom_title_file = sess_dir.join("custom-title.json");
                                let payload = serde_json::json!({ "customTitle": title });
                                if let Ok(json_str) = serde_json::to_string(&payload) {
                                    let _ = std::fs::write(&custom_title_file, json_str);
                                }
                                break;
                            }
                        }
                    }
                }
            }
        }
        AgentHarness::OpenCode => {
            let db_path = dirs::home_dir()
                .unwrap_or_default()
                .join(".local/share/opencode/opencode.db");
            if db_path.exists() {
                if let Ok(conn) = rusqlite::Connection::open(&db_path) {
                    let _ = conn.execute(
                        "UPDATE session_v2 SET title = ?1, time_updated = strftime('%s','now')*1000 WHERE id = ?2;",
                        rusqlite::params![title, conversation_id],
                    );
                }
            }
        }
        _ => {}
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoricalSession {
    pub harness: AgentHarness,
    pub conversation_id: String,
    pub title: String,
    pub project: String,
    pub timestamp_str: String,
    pub last_modified_epoch: i64,
    pub step_count: i64,
}

pub trait HarnessHistoryProvider: Send + Sync {
    fn load_conversations(
        &self,
        scope: ScopeArg,
        project: &str,
        limit: usize,
    ) -> Vec<HistoricalSession>;
}

pub struct AntigravityHistory;

impl HarnessHistoryProvider for AntigravityHistory {
    fn load_conversations(
        &self,
        scope: ScopeArg,
        project: &str,
        limit: usize,
    ) -> Vec<HistoricalSession> {
        let mut items = Vec::new();
        let db_path = dirs::home_dir()
            .unwrap_or_default()
            .join(".gemini/antigravity-cli/conversation_summaries.db");
        if !db_path.exists() {
            return items;
        }

        if let Ok(conn) = rusqlite::Connection::open_with_flags(
            &db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        ) {
            let hygiene =
                crate::core::session::session_hygiene_conditions(&conn, "conversation_summaries");
            let hygiene_and = if hygiene.is_empty() {
                String::new()
            } else {
                format!(" AND {}", hygiene.join(" AND "))
            };
            let hygiene_where = if hygiene.is_empty() {
                String::new()
            } else {
                format!("WHERE {}", hygiene.join(" AND "))
            };

            let step_count_col = if crate::core::session::table_has_column(
                &conn,
                "conversation_summaries",
                "step_count",
            ) {
                "step_count"
            } else {
                "0 AS step_count"
            };

            let sql = match scope {
                ScopeArg::Global => {
                    format!(
                        "SELECT conversation_id,
                                COALESCE(NULLIF(title, ''), NULLIF(preview, ''), 'Untitled'),
                                strftime('%m-%d %H:%M', last_modified_time),
                                COALESCE(NULLIF(project_id, ''), 'main'),
                                {},
                                workspace_uris,
                                strftime('%s', last_modified_time)
                         FROM conversation_summaries
                         {}
                         ORDER BY last_modified_time DESC
                         LIMIT ?;",
                        step_count_col, hygiene_where
                    )
                }
                ScopeArg::Local => {
                    format!(
                        "SELECT conversation_id,
                                COALESCE(NULLIF(title, ''), NULLIF(preview, ''), 'Untitled'),
                                strftime('%m-%d %H:%M', last_modified_time),
                                COALESCE(NULLIF(project_id, ''), 'main'),
                                {},
                                workspace_uris,
                                strftime('%s', last_modified_time)
                         FROM conversation_summaries
                         WHERE (workspace_uris LIKE ? OR project_id = ? OR ? = ''){hygiene_and}
                         ORDER BY last_modified_time DESC
                         LIMIT ?;",
                        step_count_col
                    )
                }
            };

            if scope == ScopeArg::Global {
                if let Ok(mut stmt) = conn.prepare(&sql) {
                    let rows = stmt.query_map([limit as i64], |row| {
                        let conv_id: String = row.get(0)?;
                        let title: String = row.get(1)?;
                        let rel_time: String = row.get(2)?;
                        let raw_proj: String = row.get(3)?;
                        let steps: i64 = row.get(4)?;
                        let uris: Option<String> = row.get(5).ok();
                        let epoch_str: Option<String> = row.get(6).ok();
                        let epoch = epoch_str.and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
                        let proj = crate::core::session::resolve_project_name(
                            Some(&raw_proj),
                            uris.as_deref(),
                            Some(project),
                        );
                        Ok(HistoricalSession {
                            harness: AgentHarness::Antigravity,
                            conversation_id: conv_id,
                            title,
                            project: proj,
                            timestamp_str: rel_time,
                            last_modified_epoch: epoch,
                            step_count: steps,
                        })
                    });
                    if let Ok(mapped) = rows {
                        items.extend(mapped.flatten());
                    }
                }
            } else {
                let filter = format!("%{}%", project);
                if let Ok(mut stmt) = conn.prepare(&sql) {
                    let rows = stmt.query_map(
                        rusqlite::params![filter, project, project, limit as i64],
                        |row| {
                            let conv_id: String = row.get(0)?;
                            let title: String = row.get(1)?;
                            let rel_time: String = row.get(2)?;
                            let raw_proj: String = row.get(3)?;
                            let steps: i64 = row.get(4)?;
                            let uris: Option<String> = row.get(5).ok();
                            let epoch_str: Option<String> = row.get(6).ok();
                            let epoch = epoch_str.and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
                            let proj = crate::core::session::resolve_project_name(
                                Some(&raw_proj),
                                uris.as_deref(),
                                Some(project),
                            );
                            Ok(HistoricalSession {
                                harness: AgentHarness::Antigravity,
                                conversation_id: conv_id,
                                title,
                                project: proj,
                                timestamp_str: rel_time,
                                last_modified_epoch: epoch,
                                step_count: steps,
                            })
                        },
                    );
                    if let Ok(mapped) = rows {
                        items.extend(mapped.flatten());
                    }
                }
            }
        }
        items
    }
}

static CLAUDE_TITLE_CACHE: LazyLock<Mutex<HashMap<PathBuf, (SystemTime, String)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn extract_claude_jsonl_title(path: &Path) -> Option<String> {
    let file = std::fs::File::open(path).ok()?;
    let reader = BufReader::new(file);
    for line in reader.lines().map_while(Result::ok).take(30) {
        if line.contains("\"type\":\"user\"") {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) {
                if let Some(msg) = v.get("message") {
                    if let Some(content) = msg.get("content") {
                        if let Some(s) = content.as_str() {
                            let first_line = s.lines().next().unwrap_or("").trim();
                            if !first_line.is_empty() {
                                return Some(first_line.chars().take(40).collect());
                            }
                        } else if let Some(arr) = content.as_array() {
                            for block in arr {
                                if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                                    let first_line = text.lines().next().unwrap_or("").trim();
                                    if !first_line.is_empty() {
                                        return Some(first_line.chars().take(40).collect());
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

#[derive(Default)]
pub struct ClaudeHistory;

impl HarnessHistoryProvider for ClaudeHistory {
    fn load_conversations(
        &self,
        scope: ScopeArg,
        project: &str,
        limit: usize,
    ) -> Vec<HistoricalSession> {
        let mut items = Vec::new();
        let projects_dir = dirs::home_dir()
            .unwrap_or_default()
            .join(".claude/projects");
        if !projects_dir.exists() {
            return items;
        }

        let entries = match std::fs::read_dir(&projects_dir) {
            Ok(e) => e,
            Err(_) => return items,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let dir_name = entry.file_name().to_string_lossy().to_string();
            let clean_proj = dir_name.rsplit('-').next().unwrap_or(&dir_name).to_string();

            if scope == ScopeArg::Local
                && !clean_proj.eq_ignore_ascii_case(project)
                && !dir_name.to_lowercase().contains(&project.to_lowercase())
            {
                continue;
            }

            if let Ok(sub_entries) = std::fs::read_dir(&path) {
                for sub in sub_entries.flatten() {
                    let sub_path = sub.path();
                    let name = sub.file_name().to_string_lossy().to_string();
                    if name.ends_with(".jsonl") && name != "history.jsonl" {
                        let conv_id = name.trim_end_matches(".jsonl").to_string();
                        if conv_id.is_empty() {
                            continue;
                        }

                        let mtime = sub
                            .metadata()
                            .and_then(|m| m.modified())
                            .unwrap_or(SystemTime::UNIX_EPOCH);
                        let epoch = mtime
                            .duration_since(SystemTime::UNIX_EPOCH)
                            .map(|d| d.as_secs() as i64)
                            .unwrap_or(0);
                        let rel_time = crate::core::session::format_relative_time(epoch);

                        // Check custom-title.json first
                        let custom_title_file = path.join(&conv_id).join("custom-title.json");
                        let title = if custom_title_file.exists() {
                            if let Ok(content) = std::fs::read_to_string(&custom_title_file) {
                                serde_json::from_str::<serde_json::Value>(&content)
                                    .ok()
                                    .and_then(|v| {
                                        v.get("customTitle")
                                            .and_then(|s| s.as_str())
                                            .map(|s| s.to_string())
                                    })
                                    .unwrap_or_else(|| {
                                        format!("Claude ({})", &conv_id[..conv_id.len().min(8)])
                                    })
                            } else {
                                format!("Claude ({})", &conv_id[..conv_id.len().min(8)])
                            }
                        } else {
                            let cached = {
                                let lock = CLAUDE_TITLE_CACHE.lock().unwrap();
                                lock.get(&sub_path).cloned()
                            };

                            if let Some((cached_mtime, cached_t)) = cached {
                                if cached_mtime == mtime {
                                    cached_t
                                } else if let Some(extracted) =
                                    extract_claude_jsonl_title(&sub_path)
                                {
                                    let mut lock = CLAUDE_TITLE_CACHE.lock().unwrap();
                                    lock.insert(sub_path.clone(), (mtime, extracted.clone()));
                                    extracted
                                } else {
                                    format!("Claude ({})", &conv_id[..conv_id.len().min(8)])
                                }
                            } else if let Some(extracted) = extract_claude_jsonl_title(&sub_path) {
                                let mut lock = CLAUDE_TITLE_CACHE.lock().unwrap();
                                lock.insert(sub_path.clone(), (mtime, extracted.clone()));
                                extracted
                            } else {
                                format!("Claude ({})", &conv_id[..conv_id.len().min(8)])
                            }
                        };

                        items.push(HistoricalSession {
                            harness: AgentHarness::ClaudeCode,
                            conversation_id: conv_id,
                            title,
                            project: clean_proj.clone(),
                            timestamp_str: rel_time,
                            last_modified_epoch: epoch,
                            step_count: 0,
                        });
                    }
                }
            }
        }

        items.sort_by_key(|a| std::cmp::Reverse(a.last_modified_epoch));
        items.truncate(limit);
        items
    }
}

pub struct OpenCodeHistory;

impl HarnessHistoryProvider for OpenCodeHistory {
    fn load_conversations(
        &self,
        scope: ScopeArg,
        project: &str,
        limit: usize,
    ) -> Vec<HistoricalSession> {
        let mut items = Vec::new();
        let db_path = dirs::home_dir()
            .unwrap_or_default()
            .join(".local/share/opencode/opencode.db");
        if !db_path.exists() {
            return items;
        }

        if let Ok(conn) = rusqlite::Connection::open_with_flags(
            &db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        ) {
            if !crate::core::session::table_exists(&conn, "session_v2") {
                return items;
            }

            let sql = match scope {
                ScopeArg::Global => {
                    "SELECT id,
                            COALESCE(NULLIF(title, ''), 'Untitled Session'),
                            directory,
                            project_id,
                            CAST(time_updated / 1000 AS INTEGER) AS epoch_sec
                     FROM session_v2
                     WHERE parent_id IS NULL
                     ORDER BY time_updated DESC
                     LIMIT ?1;"
                }
                ScopeArg::Local => {
                    "SELECT id,
                            COALESCE(NULLIF(title, ''), 'Untitled Session'),
                            directory,
                            project_id,
                            CAST(time_updated / 1000 AS INTEGER) AS epoch_sec
                     FROM session_v2
                     WHERE parent_id IS NULL AND (directory LIKE ?1 OR project_id = ?2 OR ?3 = '')
                     ORDER BY time_updated DESC
                     LIMIT ?4;"
                }
            };

            if scope == ScopeArg::Global {
                if let Ok(mut stmt) = conn.prepare(sql) {
                    let rows = stmt.query_map([limit as i64], |row| {
                        let id: String = row.get(0)?;
                        let title: String = row.get(1)?;
                        let dir: String = row.get(2)?;
                        let pid: String = row.get(3)?;
                        let epoch: i64 = row.get(4)?;
                        let proj = Path::new(&dir)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .filter(|n| !n.is_empty())
                            .unwrap_or(&pid)
                            .to_string();
                        let rel_time = crate::core::session::format_relative_time(epoch);
                        Ok(HistoricalSession {
                            harness: AgentHarness::OpenCode,
                            conversation_id: id,
                            title,
                            project: proj,
                            timestamp_str: rel_time,
                            last_modified_epoch: epoch,
                            step_count: 0,
                        })
                    });
                    if let Ok(mapped) = rows {
                        items.extend(mapped.flatten());
                    }
                }
            } else {
                let filter = format!("%{}%", project);
                if let Ok(mut stmt) = conn.prepare(sql) {
                    let rows = stmt.query_map(
                        rusqlite::params![filter, project, project, limit as i64],
                        |row| {
                            let id: String = row.get(0)?;
                            let title: String = row.get(1)?;
                            let dir: String = row.get(2)?;
                            let pid: String = row.get(3)?;
                            let epoch: i64 = row.get(4)?;
                            let proj = Path::new(&dir)
                                .file_name()
                                .and_then(|n| n.to_str())
                                .filter(|n| !n.is_empty())
                                .unwrap_or(&pid)
                                .to_string();
                            let rel_time = crate::core::session::format_relative_time(epoch);
                            Ok(HistoricalSession {
                                harness: AgentHarness::OpenCode,
                                conversation_id: id,
                                title,
                                project: proj,
                                timestamp_str: rel_time,
                                last_modified_epoch: epoch,
                                step_count: 0,
                            })
                        },
                    );
                    if let Ok(mapped) = rows {
                        items.extend(mapped.flatten());
                    }
                }
            }
        }

        items
    }
}

/// Load and merge historical conversations across Antigravity, Claude Code, and OpenCode
pub fn load_all_historical_sessions(
    scope: ScopeArg,
    project: &str,
    limit: usize,
) -> Vec<HistoricalSession> {
    let mut all = Vec::new();

    let agy = AntigravityHistory;
    all.extend(agy.load_conversations(scope, project, limit));

    let claude = ClaudeHistory;
    all.extend(claude.load_conversations(scope, project, limit));

    let opencode = OpenCodeHistory;
    all.extend(opencode.load_conversations(scope, project, limit));

    all.sort_by_key(|a| std::cmp::Reverse(a.last_modified_epoch));
    all.truncate(limit);
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_harness_strings_and_loose_parsing() {
        assert_eq!(
            AgentHarness::from_str_loose("agy"),
            AgentHarness::Antigravity
        );
        assert_eq!(
            AgentHarness::from_str_loose("antigravity"),
            AgentHarness::Antigravity
        );
        assert_eq!(
            AgentHarness::from_str_loose("claude"),
            AgentHarness::ClaudeCode
        );
        assert_eq!(
            AgentHarness::from_str_loose("claudecode"),
            AgentHarness::ClaudeCode
        );
        assert_eq!(
            AgentHarness::from_str_loose("opencode"),
            AgentHarness::OpenCode
        );
        assert_eq!(
            AgentHarness::from_str_loose("open-code"),
            AgentHarness::OpenCode
        );
        assert_eq!(AgentHarness::from_str_loose("codex"), AgentHarness::Codex);
        assert_eq!(AgentHarness::from_str_loose("openai"), AgentHarness::Codex);
        assert_eq!(
            AgentHarness::from_str_loose("term"),
            AgentHarness::PlainTerminal
        );
        assert_eq!(
            AgentHarness::from_str_loose("terminal"),
            AgentHarness::PlainTerminal
        );
        assert_eq!(
            AgentHarness::from_str_loose("shell"),
            AgentHarness::PlainTerminal
        );
        assert_eq!(
            AgentHarness::from_str_loose("custom-agent"),
            AgentHarness::Custom("custom-agent".to_string())
        );

        assert_eq!(AgentHarness::Antigravity.as_str(), "agy");
        assert_eq!(AgentHarness::ClaudeCode.as_str(), "claude");
        assert_eq!(AgentHarness::OpenCode.as_str(), "opencode");
        assert_eq!(AgentHarness::Codex.as_str(), "codex");
        assert_eq!(AgentHarness::PlainTerminal.as_str(), "term");
        assert_eq!(
            AgentHarness::Custom("my-bot".to_string()).as_str(),
            "my-bot"
        );
    }

    #[test]
    fn test_agent_harness_icons_and_commands() {
        assert_eq!(AgentHarness::Antigravity.icon(), "󰚩");
        assert_eq!(AgentHarness::ClaudeCode.icon(), "󰊠");
        assert_eq!(AgentHarness::OpenCode.icon(), "󰘐");
        assert_eq!(AgentHarness::Codex.icon(), "󱚣");
        assert_eq!(AgentHarness::PlainTerminal.icon(), "");
        assert_eq!(AgentHarness::Custom("custom".to_string()).icon(), "󰘑");

        assert_eq!(AgentHarness::Antigravity.default_command(), "agy");
        assert_eq!(AgentHarness::ClaudeCode.default_command(), "claude");
        assert_eq!(AgentHarness::OpenCode.default_command(), "opencode");
        assert_eq!(AgentHarness::Codex.default_command(), "codex");
        assert_eq!(AgentHarness::PlainTerminal.default_command(), "$SHELL");
        assert_eq!(
            AgentHarness::Custom("tool".to_string()).default_command(),
            "tool"
        );

        assert_eq!(AgentHarness::Antigravity.default_hotkey(), Some('n'));
        assert_eq!(AgentHarness::ClaudeCode.default_hotkey(), Some('c'));
        assert_eq!(AgentHarness::OpenCode.default_hotkey(), Some('o'));
        assert_eq!(AgentHarness::Codex.default_hotkey(), None);
        assert_eq!(AgentHarness::PlainTerminal.default_hotkey(), Some('t'));
        assert_eq!(
            AgentHarness::Custom("bot".to_string()).default_hotkey(),
            None
        );

        assert_eq!(AgentHarness::Antigravity.cyrillic_hotkey(), Some('т'));
        assert_eq!(AgentHarness::ClaudeCode.cyrillic_hotkey(), Some('с'));
        assert_eq!(AgentHarness::OpenCode.cyrillic_hotkey(), Some('щ'));
        assert_eq!(AgentHarness::Codex.cyrillic_hotkey(), None);
        assert_eq!(AgentHarness::PlainTerminal.cyrillic_hotkey(), Some('е'));
        assert_eq!(
            AgentHarness::Custom("bot".to_string()).cyrillic_hotkey(),
            None
        );
    }

    #[test]
    fn test_harness_registry_builtins_and_custom() {
        let mut registry = HarnessRegistry::new();
        assert!(registry.get("agy").is_some());
        assert!(registry.get("claude").is_some());
        assert!(registry.get("opencode").is_some());
        assert!(registry.get("codex").is_some());
        assert!(registry.get("term").is_some());
        assert!(registry.get("nonexistent").is_none());

        let mut custom = CustomDriver::new("evaluator", "eval-cli");
        custom.display_name = "Model Evaluator".to_string();
        custom.icon = "󰙨".to_string();
        custom.hotkey = Some('e');
        custom.cyrillic_hotkey = Some('у');
        registry.register(Box::new(custom));

        let driver = registry.get("evaluator").expect("custom driver found");
        assert_eq!(driver.id(), "evaluator");
        assert_eq!(driver.display_name(), "Model Evaluator");
        assert_eq!(driver.icon(), "󰙨");
        assert_eq!(driver.default_command(), "eval-cli");
        assert_eq!(driver.default_hotkey(), Some('e'));
        assert_eq!(driver.cyrillic_hotkey(), Some('у'));

        let ctx = HarnessLaunchContext {
            args: vec!["--check".to_string()],
            ..Default::default()
        };
        let cmd = driver.build_command(&ctx);
        assert_eq!(cmd.get_program(), "eval-cli");
    }

    #[test]
    fn test_sync_conversation_title_plain_terminal_noop() {
        let res = sync_conversation_title(&AgentHarness::PlainTerminal, "test-id", "New Title");
        assert!(res.is_ok());
    }

    #[test]
    fn test_load_all_historical_sessions_structure() {
        let sessions = load_all_historical_sessions(ScopeArg::Local, "nonexistent-proj-xyz", 10);
        // Returns valid vec without panics
        assert!(sessions.len() <= 10);
    }
}
