use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Clone, Parser, PartialEq, Eq)]
#[command(name = "sky", version, about = "Sky Code - Offline AI Coding Agent")]
pub struct Cli {
    #[arg(long, default_value = "cloud-apus-4-6")]
    pub model: String,

    #[arg(long, value_enum, default_value_t = PermissionMode::DangerFullAccess)]
    pub permission_mode: PermissionMode,

    #[arg(long)]
    pub config: Option<PathBuf>,

    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub output_format: OutputFormat,

    /// Use a named profile (isolates config under ~/.skycode-<name>)
    #[arg(long)]
    pub profile: Option<String>,

    /// Dev profile shortcut (equivalent to --profile dev)
    #[arg(long)]
    pub dev: bool,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Clone, Subcommand, PartialEq, Eq)]
pub enum Command {
    /// Read upstream TS sources and print extracted counts
    DumpManifests,
    /// Print the current bootstrap phase skeleton
    BootstrapPlan,
    /// Start the OAuth login flow
    Login,
    /// Clear saved OAuth credentials
    Logout,
    /// Run a non-interactive prompt and exit
    Prompt { prompt: Vec<String> },
    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Run system health check
    Doctor {
        /// Automatically fix issues when possible
        #[arg(long)]
        fix: bool,
    },
    /// Manage local models
    Models {
        #[command(subcommand)]
        action: ModelAction,
    },
}

#[derive(Debug, Clone, Subcommand, PartialEq, Eq)]
pub enum ConfigAction {
    /// Get config value (e.g., "model.name")
    Get { key: String },
    /// Set config value (e.g., "model.name" "sky-opus-4")
    Set { key: String, value: String },
    /// List all config values
    List,
    /// Validate config file
    Validate {
        #[arg(long)]
        fix: bool,
    },
    /// Reset config to defaults
    Reset {
        #[arg(long)]
        confirm: bool,
    },
    /// Open config file in $EDITOR
    Edit,
}

#[derive(Debug, Clone, Subcommand, PartialEq, Eq)]
pub enum ModelAction {
    /// List installed models
    List,
    /// Install a model from Ollama
    Install { name: String },
    /// Uninstall a model
    Uninstall { name: String },
    /// Install an entire preset pack
    InstallPack { preset: String },
    /// Create a model alias (e.g., angel0.1 -> qwen3:14b)
    CreateAlias { alias: String, source: String },
    /// List all model aliases
    ListAliases,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum PermissionMode {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    Json,
    Ndjson,
}

impl Cli {
    /// Get the effective profile name (resolves --dev shortcut)
    pub fn effective_profile(&self) -> Option<&str> {
        if self.dev {
            Some("dev")
        } else {
            self.profile.as_deref()
        }
    }

    /// Get profile-aware config directory path
    /// Default: ~/.skycode/
    /// With profile: ~/.skycode-<name>/
    pub fn config_dir(&self) -> Option<PathBuf> {
        let home = dirs::home_dir()?;
        let dir_name = match self.effective_profile() {
            Some(profile) => format!(".skycode-{}", profile),
            None => ".skycode".to_string(),
        };
        Some(home.join(dir_name))
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::{Cli, Command, OutputFormat, PermissionMode};

    #[test]
    fn parses_requested_flags() {
        let cli = Cli::parse_from([
            "sky",
            "--model",
            "cloud-haiku-4-5-20251213",
            "--permission-mode",
            "read-only",
            "--config",
            "/tmp/config.toml",
            "--output-format",
            "ndjson",
            "prompt",
            "hello",
            "world",
        ]);

        assert_eq!(cli.model, "cloud-haiku-4-5-20251213");
        assert_eq!(cli.permission_mode, PermissionMode::ReadOnly);
        assert_eq!(
            cli.config.as_deref(),
            Some(std::path::Path::new("/tmp/config.toml"))
        );
        assert_eq!(cli.output_format, OutputFormat::Ndjson);
        assert_eq!(
            cli.command,
            Some(Command::Prompt {
                prompt: vec!["hello".into(), "world".into()]
            })
        );
    }

    #[test]
    fn parses_login_and_logout_commands() {
        let login = Cli::parse_from(["sky", "login"]);
        assert_eq!(login.command, Some(Command::Login));

        let logout = Cli::parse_from(["sky", "logout"]);
        assert_eq!(logout.command, Some(Command::Logout));
    }

    #[test]
    fn defaults_to_danger_full_access_permission_mode() {
        let cli = Cli::parse_from(["sky"]);
        assert_eq!(cli.permission_mode, PermissionMode::DangerFullAccess);
    }

    #[test]
    fn parses_profile_flag() {
        let cli = Cli::parse_from(["sky", "--profile", "production"]);
        assert_eq!(cli.effective_profile(), Some("production"));
        
        let cfg_dir = cli.config_dir().unwrap();
        assert!(cfg_dir.to_string_lossy().contains(".skycode-production"));
    }

    #[test]
    fn dev_flag_sets_dev_profile() {
        let cli = Cli::parse_from(["sky", "--dev"]);
        assert_eq!(cli.effective_profile(), Some("dev"));
        
        let cfg_dir = cli.config_dir().unwrap();
        assert!(cfg_dir.to_string_lossy().contains(".skycode-dev"));
    }

    #[test]
    fn dev_flag_overrides_explicit_profile() {
        let cli = Cli::parse_from(["sky", "--profile", "production", "--dev"]);
        // --dev takes precedence
        assert_eq!(cli.effective_profile(), Some("dev"));
    }

    #[test]
    fn default_config_dir_without_profile() {
        let cli = Cli::parse_from(["sky"]);
        assert_eq!(cli.effective_profile(), None);
        
        let cfg_dir = cli.config_dir().unwrap();
        assert!(cfg_dir.to_string_lossy().ends_with(".skycode"));
        assert!(!cfg_dir.to_string_lossy().contains(".skycode-"));
    }
}
