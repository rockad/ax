use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentHarness {
    Antigravity,
    ClaudeCode,
    OpenCode,
    Codex,
    PlainTerminal,
    Custom(String),
}

impl Default for AgentHarness {
    fn default() -> Self {
        Self::Antigravity
    }
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
            Self::Codex => Some('x'),
            Self::PlainTerminal => Some('t'),
            Self::Custom(_) => None,
        }
    }

    pub fn cyrillic_hotkey(&self) -> Option<char> {
        match self {
            Self::Antigravity => Some('т'),
            Self::ClaudeCode => Some('с'),
            Self::OpenCode => Some('щ'),
            Self::Codex => Some('ч'),
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
        Some('x')
    }

    fn cyrillic_hotkey(&self) -> Option<char> {
        Some('ч')
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
        assert_eq!(
            AgentHarness::from_str_loose("codex"),
            AgentHarness::Codex
        );
        assert_eq!(
            AgentHarness::from_str_loose("openai"),
            AgentHarness::Codex
        );
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
        assert_eq!(AgentHarness::Custom("my-bot".to_string()).as_str(), "my-bot");
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
        assert_eq!(AgentHarness::Custom("tool".to_string()).default_command(), "tool");

        assert_eq!(AgentHarness::Antigravity.default_hotkey(), Some('n'));
        assert_eq!(AgentHarness::ClaudeCode.default_hotkey(), Some('c'));
        assert_eq!(AgentHarness::OpenCode.default_hotkey(), Some('o'));
        assert_eq!(AgentHarness::Codex.default_hotkey(), Some('x'));
        assert_eq!(AgentHarness::PlainTerminal.default_hotkey(), Some('t'));
        assert_eq!(AgentHarness::Custom("bot".to_string()).default_hotkey(), None);

        assert_eq!(AgentHarness::Antigravity.cyrillic_hotkey(), Some('т'));
        assert_eq!(AgentHarness::ClaudeCode.cyrillic_hotkey(), Some('с'));
        assert_eq!(AgentHarness::OpenCode.cyrillic_hotkey(), Some('щ'));
        assert_eq!(AgentHarness::Codex.cyrillic_hotkey(), Some('ч'));
        assert_eq!(AgentHarness::PlainTerminal.cyrillic_hotkey(), Some('е'));
        assert_eq!(AgentHarness::Custom("bot".to_string()).cyrillic_hotkey(), None);
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
}
