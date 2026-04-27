use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkyConfig {
    pub model: ModelConfig,
    pub bridge: BridgeConfig,
    pub ollama: OllamaConfig,
    pub permissions: PermissionsConfig,
    #[serde(default)]
    pub advanced: AdvancedConfig,
    #[serde(default)]
    pub model_pack: ModelPackConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub name: String,
    pub max_tokens: u32,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeConfig {
    pub url: String,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    pub url: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionsConfig {
    pub mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedConfig {
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default = "default_context_window")]
    pub context_window: u32,
    #[serde(default = "default_auto_save")]
    pub auto_save_sessions: bool,
}

/// Model pack preset for one-click installation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPreset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub models: Vec<String>,
    pub roles: Vec<String>,
    pub min_ram_gb: u32,
    pub min_vram_gb: u32,
}

/// Model alias mapping for custom names
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelAlias {
    pub alias: String,
    pub source_model: String,
}

/// Model pack configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPackConfig {
    #[serde(default)]
    pub presets: Vec<ModelPreset>,
    #[serde(default)]
    pub aliases: Vec<ModelAlias>,
    #[serde(default)]
    pub active_preset: Option<String>,
}

impl Default for SkyConfig {
    fn default() -> Self {
        Self {
            model: ModelConfig {
                name: "cloud-apus-4-6".to_string(),
                max_tokens: 32000,
                temperature: 0.7,
            },
            bridge: BridgeConfig {
                url: "http://localhost:4000".to_string(),
                timeout_seconds: 30,
            },
            ollama: OllamaConfig {
                url: "http://localhost:11434".to_string(),
                model: "llama3.1:8b".to_string(),
            },
            permissions: PermissionsConfig {
                mode: "danger-full-access".to_string(),
            },
            advanced: AdvancedConfig::default(),
            model_pack: ModelPackConfig::default(),
        }
    }
}

impl Default for ModelPackConfig {
    fn default() -> Self {
        Self {
            presets: default_model_presets(),
            aliases: Vec::new(),
            active_preset: None,
        }
    }
}

/// Built-in model presets shipped with SkyCode.
/// These are merged into the config when no presets are found on disk.
pub fn default_model_presets() -> Vec<ModelPreset> {
    vec![
        ModelPreset {
            id: "coding-pro".to_string(),
            name: "Coding Pro".to_string(),
            description: "High-performance coding stack: Qwen3 14B for reasoning, Qwen2.5-Coder for completions, Codestral for FIM.".to_string(),
            models: vec![
                "qwen3:14b".to_string(),
                "qwen2.5-coder:14b".to_string(),
                "codestral:latest".to_string(),
            ],
            roles: vec!["chat".to_string(), "completion".to_string(), "fim".to_string()],
            min_ram_gb: 32,
            min_vram_gb: 16,
        },
        ModelPreset {
            id: "reasoning-heavy".to_string(),
            name: "Reasoning Heavy".to_string(),
            description: "Deep reasoning stack: DeepSeek-R1 32B for complex problems, DeepSeek-Coder-V2 for code generation.".to_string(),
            models: vec![
                "deepseek-r1:32b".to_string(),
                "deepseek-coder-v2:16b".to_string(),
            ],
            roles: vec!["chat".to_string(), "reasoning".to_string(), "completion".to_string()],
            min_ram_gb: 64,
            min_vram_gb: 24,
        },
        ModelPreset {
            id: "balanced".to_string(),
            name: "Balanced".to_string(),
            description: "Balanced everyday stack: Llama4 Scout for chat, Qwen3 14B for reasoning, Codestral for fast completions.".to_string(),
            models: vec![
                "llama4:scout".to_string(),
                "qwen3:14b".to_string(),
                "codestral:latest".to_string(),
            ],
            roles: vec!["chat".to_string(), "completion".to_string()],
            min_ram_gb: 24,
            min_vram_gb: 12,
        },
    ]
}

fn apply_default_model_preset_fallback(config: &mut SkyConfig) {
    // Legacy configs may have an empty preset list; backfill built-ins for GUI/CLI packs.
    if config.model_pack.presets.is_empty() {
        config.model_pack.presets = default_model_presets();
    }
}

impl Default for AdvancedConfig {
    fn default() -> Self {
        Self {
            log_level: "info".to_string(),
            context_window: 8192,
            auto_save_sessions: true,
        }
    }
}

fn default_temperature() -> f32 {
    0.7
}
fn default_timeout() -> u32 {
    30
}
fn default_log_level() -> String {
    "info".to_string()
}
fn default_context_window() -> u32 {
    8192
}
fn default_auto_save() -> bool {
    true
}

/// Configuration manager with profile support
pub struct ConfigManager {
    profile: Option<String>,
}

impl ConfigManager {
    pub fn new(profile: Option<String>) -> Self {
        Self { profile }
    }

    /// Get config directory path based on active profile
    pub fn config_dir(&self) -> Result<PathBuf> {
        let home = dirs::home_dir().context("Could not determine home directory")?;

        let dir_name = match &self.profile {
            Some(name) => format!(".skycode-{}", name),
            None => ".skycode".to_string(),
        };

        Ok(home.join(dir_name))
    }

    /// Get config file path
    pub fn config_file_path(&self) -> Result<PathBuf> {
        Ok(self.config_dir()?.join("config.toml"))
    }

    /// Load config from file (or return defaults if missing).
    /// If the loaded config has no presets, the built-in default presets are merged in.
    pub fn load(&self) -> Result<SkyConfig> {
        let path = self.config_file_path()?;

        if !path.exists() {
            return Ok(SkyConfig::default());
        }

        let content = fs::read_to_string(&path).context("Failed to read config file")?;

        let mut config: SkyConfig =
            toml::from_str(&content).context("Failed to parse config file")?;

        apply_default_model_preset_fallback(&mut config);

        Ok(config)
    }

    /// Save config to file
    pub fn save(&self, config: &SkyConfig) -> Result<()> {
        let dir = self.config_dir()?;
        fs::create_dir_all(&dir).context("Failed to create config directory")?;

        let path = self.config_file_path()?;
        let content = toml::to_string_pretty(config).context("Failed to serialize config")?;

        fs::write(&path, content).context("Failed to write config file")?;

        Ok(())
    }

    /// Get nested config value by dot-notation key
    pub fn get_value(&self, key: &str) -> Result<String> {
        let config = self.load()?;

        match key {
            "model.name" => Ok(config.model.name),
            "model.max_tokens" => Ok(config.model.max_tokens.to_string()),
            "model.temperature" => Ok(config.model.temperature.to_string()),
            "bridge.url" => Ok(config.bridge.url),
            "bridge.timeout_seconds" => Ok(config.bridge.timeout_seconds.to_string()),
            "ollama.url" => Ok(config.ollama.url),
            "ollama.model" => Ok(config.ollama.model),
            "permissions.mode" => Ok(config.permissions.mode),
            "advanced.log_level" => Ok(config.advanced.log_level),
            "advanced.context_window" => Ok(config.advanced.context_window.to_string()),
            "advanced.auto_save_sessions" => Ok(config.advanced.auto_save_sessions.to_string()),
            _ => Err(anyhow::anyhow!("Unknown config key: {}", key)),
        }
    }

    /// Set nested config value by dot-notation key
    pub fn set_value(&self, key: &str, value: &str) -> Result<()> {
        let mut config = self.load()?;

        match key {
            "model.name" => config.model.name = value.to_string(),
            "model.max_tokens" => {
                config.model.max_tokens = value
                    .parse()
                    .context("max_tokens must be a positive integer")?
            }
            "model.temperature" => {
                let temp: f32 = value.parse().context("temperature must be a number")?;
                if !(0.0..=2.0).contains(&temp) {
                    return Err(anyhow::anyhow!("temperature must be between 0.0 and 2.0"));
                }
                config.model.temperature = temp;
            }
            "bridge.url" => config.bridge.url = value.to_string(),
            "bridge.timeout_seconds" => {
                config.bridge.timeout_seconds = value
                    .parse()
                    .context("timeout_seconds must be a positive integer")?
            }
            "ollama.url" => config.ollama.url = value.to_string(),
            "ollama.model" => config.ollama.model = value.to_string(),
            "permissions.mode" => {
                validate_permission_mode(value)?;
                config.permissions.mode = value.to_string();
            }
            "advanced.log_level" => {
                validate_log_level(value)?;
                config.advanced.log_level = value.to_string();
            }
            "advanced.context_window" => {
                config.advanced.context_window = value
                    .parse()
                    .context("context_window must be a positive integer")?
            }
            "advanced.auto_save_sessions" => {
                config.advanced.auto_save_sessions = value
                    .parse()
                    .context("auto_save_sessions must be true or false")?
            }
            _ => return Err(anyhow::anyhow!("Unknown config key: {}", key)),
        }

        self.save(&config)?;
        Ok(())
    }

    /// List all config values as TOML
    pub fn list(&self) -> Result<String> {
        let config = self.load()?;
        toml::to_string_pretty(&config).context("Failed to serialize config")
    }

    /// Validate config and optionally fix common issues
    pub fn validate(&self, fix: bool) -> Result<Vec<String>> {
        let mut issues = Vec::new();
        let mut config = self.load()?;
        let mut fixed = false;

        // Validate temperature
        if !(0.0..=2.0).contains(&config.model.temperature) {
            if fix {
                issues.push(format!(
                    "Fixed invalid temperature {} → 0.7",
                    config.model.temperature
                ));
                config.model.temperature = 0.7;
                fixed = true;
            } else {
                issues.push(format!(
                    "Invalid temperature {} (must be 0.0-2.0)",
                    config.model.temperature
                ));
            }
        }

        // Validate permission mode
        if let Err(e) = validate_permission_mode(&config.permissions.mode) {
            if fix {
                issues.push(format!(
                    "Fixed invalid permission mode '{}' → 'danger-full-access'",
                    config.permissions.mode
                ));
                config.permissions.mode = "danger-full-access".to_string();
                fixed = true;
            } else {
                issues.push(format!("{}", e));
            }
        }

        // Validate log level
        if let Err(e) = validate_log_level(&config.advanced.log_level) {
            if fix {
                issues.push(format!(
                    "Fixed invalid log level '{}' → 'info'",
                    config.advanced.log_level
                ));
                config.advanced.log_level = "info".to_string();
                fixed = true;
            } else {
                issues.push(format!("{}", e));
            }
        }

        // Save fixed config
        if fixed {
            self.save(&config)?;
        }

        Ok(issues)
    }

    /// Reset config to defaults
    pub fn reset(&self) -> Result<()> {
        let config = SkyConfig::default();
        self.save(&config)?;
        Ok(())
    }
}

fn validate_permission_mode(mode: &str) -> Result<()> {
    match mode {
        "read-only" | "workspace-write" | "danger-full-access" => Ok(()),
        _ => Err(anyhow::anyhow!(
            "Invalid permission mode '{}' (expected: read-only, workspace-write, or danger-full-access)",
            mode
        )),
    }
}

fn validate_log_level(level: &str) -> Result<()> {
    match level {
        "error" | "warn" | "info" | "debug" | "trace" => Ok(()),
        _ => Err(anyhow::anyhow!(
            "Invalid log level '{}' (expected: error, warn, info, debug, or trace)",
            level
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SkyConfig::default();
        assert_eq!(config.model.name, "cloud-apus-4-6");
        assert_eq!(config.model.max_tokens, 32000);
        assert_eq!(config.model.temperature, 0.7);
        assert_eq!(config.bridge.url, "http://localhost:4000");
        assert_eq!(config.ollama.url, "http://localhost:11434");
        assert_eq!(config.permissions.mode, "danger-full-access");
    }

    #[test]
    fn test_validate_permission_mode() {
        assert!(validate_permission_mode("read-only").is_ok());
        assert!(validate_permission_mode("workspace-write").is_ok());
        assert!(validate_permission_mode("danger-full-access").is_ok());
        assert!(validate_permission_mode("invalid").is_err());
    }

    #[test]
    fn test_validate_log_level() {
        assert!(validate_log_level("error").is_ok());
        assert!(validate_log_level("warn").is_ok());
        assert!(validate_log_level("info").is_ok());
        assert!(validate_log_level("debug").is_ok());
        assert!(validate_log_level("trace").is_ok());
        assert!(validate_log_level("invalid").is_err());
    }

    #[test]
    fn test_config_serialization() {
        let config = SkyConfig::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        assert!(toml_str.contains("[model]"));
        assert!(toml_str.contains("[bridge]"));
        assert!(toml_str.contains("[ollama]"));
    }

    #[test]
    fn test_default_model_pack_presets_present() {
        let config = SkyConfig::default();
        let ids: Vec<&str> = config
            .model_pack
            .presets
            .iter()
            .map(|p| p.id.as_str())
            .collect();

        assert!(ids.contains(&"coding-pro"));
        assert!(ids.contains(&"reasoning-heavy"));
        assert!(ids.contains(&"balanced"));
        assert_eq!(config.model_pack.presets.len(), 3);
    }

    #[test]
    fn test_model_preset_fallback_applies_when_empty() {
        let mut config = SkyConfig::default();
        config.model_pack.presets.clear();
        assert!(config.model_pack.presets.is_empty());

        apply_default_model_preset_fallback(&mut config);

        assert_eq!(config.model_pack.presets.len(), 3);
    }

    #[test]
    fn test_model_preset_fallback_does_not_override_custom_presets() {
        let mut config = SkyConfig::default();
        config.model_pack.presets = vec![ModelPreset {
            id: "custom".to_string(),
            name: "Custom".to_string(),
            description: "User-defined".to_string(),
            models: vec!["my-model:latest".to_string()],
            roles: vec!["chat".to_string()],
            min_ram_gb: 8,
            min_vram_gb: 0,
        }];

        apply_default_model_preset_fallback(&mut config);

        assert_eq!(config.model_pack.presets.len(), 1);
        assert_eq!(config.model_pack.presets[0].id, "custom");
    }
}
