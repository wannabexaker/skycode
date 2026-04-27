use anyhow::{Context, Result};
use std::process::Command;

/// Doctor check result
#[derive(Debug, Clone)]
pub struct CheckResult {
    pub name: String,
    pub status: CheckStatus,
    pub message: String,
    pub fix_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckStatus {
    Ok,
    Warning,
    Error,
}

impl CheckResult {
    pub fn ok(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: CheckStatus::Ok,
            message: message.into(),
            fix_available: false,
        }
    }

    pub fn warning(
        name: impl Into<String>,
        message: impl Into<String>,
        fix_available: bool,
    ) -> Self {
        Self {
            name: name.into(),
            status: CheckStatus::Warning,
            message: message.into(),
            fix_available,
        }
    }

    pub fn error(name: impl Into<String>, message: impl Into<String>, fix_available: bool) -> Self {
        Self {
            name: name.into(),
            status: CheckStatus::Error,
            message: message.into(),
            fix_available,
        }
    }
}

/// System health checker
pub struct Doctor {
    auto_fix: bool,
}

impl Doctor {
    pub fn new(auto_fix: bool) -> Self {
        Self { auto_fix }
    }

    /// Run all diagnostic checks
    pub fn run_all_checks(&self) -> Vec<CheckResult> {
        vec![
            self.check_home_env(),
            self.check_config_file(),
            self.check_ollama_running(),
            self.check_ollama_version(),
            self.check_skybridge_port(),
            self.check_base_url_config(),
            self.check_model_availability(),
        ]
    }

    /// Check if HOME environment variable is set
    fn check_home_env(&self) -> CheckResult {
        match std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
            Ok(home) => CheckResult::ok("HOME environment", format!("✅ HOME set to: {}", home)),
            Err(_) => {
                if self.auto_fix {
                    if let Ok(userprofile) = std::env::var("USERPROFILE") {
                        std::env::set_var("HOME", &userprofile);
                        CheckResult::warning(
                            "HOME environment",
                            format!("⚠️  HOME was missing, auto-fixed to: {}", userprofile),
                            true,
                        )
                    } else {
                        CheckResult::error(
                            "HOME environment",
                            "❌ HOME not set and USERPROFILE unavailable",
                            false,
                        )
                    }
                } else {
                    CheckResult::error(
                        "HOME environment",
                        "❌ HOME not set. Fix: Set $env:HOME = $env:USERPROFILE",
                        true,
                    )
                }
            }
        }
    }

    /// Check if config file exists
    fn check_config_file(&self) -> CheckResult {
        use crate::config::ConfigManager;

        let manager = ConfigManager::new(None);
        match manager.config_file_path() {
            Ok(path) => {
                if path.exists() {
                    CheckResult::ok("Config file", format!("✅ Found: {}", path.display()))
                } else if self.auto_fix {
                    match manager.save(&crate::config::SkyConfig::default()) {
                        Ok(_) => CheckResult::warning(
                            "Config file",
                            format!("⚠️  Missing config, created: {}", path.display()),
                            true,
                        ),
                        Err(e) => CheckResult::error(
                            "Config file",
                            format!("❌ Failed to create config: {}", e),
                            false,
                        ),
                    }
                } else {
                    CheckResult::warning(
                        "Config file",
                        format!(
                            "⚠️  Config missing: {}. Run: sky config list",
                            path.display()
                        ),
                        true,
                    )
                }
            }
            Err(e) => CheckResult::error(
                "Config file",
                format!("❌ Cannot determine config path: {}", e),
                false,
            ),
        }
    }

    /// Check if Ollama is running
    fn check_ollama_running(&self) -> CheckResult {
        let ollama_url = std::env::var("OLLAMA_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:11434".to_string());

        // Try to connect to Ollama's /api/tags endpoint
        let check_url = format!("{}/api/tags", ollama_url);

        match self.check_http_endpoint(&check_url, 5) {
            Ok(_) => CheckResult::ok("Ollama service", format!("✅ Running at {}", ollama_url)),
            Err(_) => CheckResult::error(
                "Ollama service",
                format!(
                    "❌ Not responding at {}. Start Ollama: ollama serve",
                    ollama_url
                ),
                false,
            ),
        }
    }

    /// Check Ollama version
    fn check_ollama_version(&self) -> CheckResult {
        if let Ok(output) = Command::new("ollama").arg("--version").output() {
            let version_str = String::from_utf8_lossy(&output.stdout);
            let version = version_str.trim();

            if version.contains("0.20") || version.contains("0.1") {
                CheckResult::warning(
                    "Ollama version",
                    format!(
                        "⚠️  Old version detected: {}. Upgrade recommended: ollama update",
                        version
                    ),
                    false,
                )
            } else if !version.is_empty() {
                CheckResult::ok("Ollama version", format!("✅ {}", version))
            } else {
                CheckResult::warning(
                    "Ollama version",
                    "⚠️  Could not detect version".to_string(),
                    false,
                )
            }
        } else {
            CheckResult::warning(
                "Ollama version",
                "⚠️  'ollama' command not found in PATH".to_string(),
                false,
            )
        }
    }

    /// Check if SkyBridge port is available
    fn check_skybridge_port(&self) -> CheckResult {
        let bridge_url = std::env::var("FILANTHROPIC_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:4000".to_string());

        if bridge_url.starts_with("https://api.") {
            return CheckResult::ok(
                "SkyBridge",
                "✅ Using cloud API (no bridge needed)".to_string(),
            );
        }

        match self.check_http_endpoint(&bridge_url, 2) {
            Ok(_) => CheckResult::ok("SkyBridge", format!("✅ Responding at {}", bridge_url)),
            Err(_) => CheckResult::error(
                "SkyBridge",
                format!(
                    "❌ Not responding at {}. Start SkyBridge: cd skybridge && cargo run --release",
                    bridge_url
                ),
                false,
            ),
        }
    }

    /// Check base URL configuration
    fn check_base_url_config(&self) -> CheckResult {
        match std::env::var("FILANTHROPIC_BASE_URL") {
            Ok(url) => {
                if url.starts_with("https://api.") {
                    CheckResult::warning(
                        "Base URL config",
                        "⚠️  Using cloud API (requires API key)".to_string(),
                        false,
                    )
                } else {
                    CheckResult::ok("Base URL config", format!("✅ Offline mode: {}", url))
                }
            }
            Err(_) => CheckResult::warning(
                "Base URL config",
                "⚠️  FILANTHROPIC_BASE_URL not set. Set to http://localhost:4000 for offline mode"
                    .to_string(),
                true,
            ),
        }
    }

    /// Check if model is available in Ollama
    fn check_model_availability(&self) -> CheckResult {
        use crate::config::ConfigManager;

        let manager = ConfigManager::new(None);
        let model_name = match manager.load() {
            Ok(config) => config.ollama.model,
            Err(_) => "llama3.1:8b".to_string(),
        };

        let ollama_url = std::env::var("OLLAMA_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:11434".to_string());

        let check_url = format!("{}/api/tags", ollama_url);

        match self.check_http_endpoint(&check_url, 5) {
            Ok(response_text) => {
                if response_text.contains(&model_name) {
                    CheckResult::ok(
                        "Model availability",
                        format!("✅ Model '{}' is available", model_name),
                    )
                } else {
                    CheckResult::error(
                        "Model availability",
                        format!(
                            "❌ Model '{}' not found. Pull it: ollama pull {}",
                            model_name, model_name
                        ),
                        false,
                    )
                }
            }
            Err(_) => CheckResult::warning(
                "Model availability",
                "⚠️  Cannot check models (Ollama not responding)".to_string(),
                false,
            ),
        }
    }

    /// Helper: Check if HTTP endpoint responds
    fn check_http_endpoint(&self, url: &str, timeout_secs: u64) -> Result<String> {
        use std::time::Duration;

        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .context("Failed to create HTTP client")?;

        let response = client
            .get(url)
            .send()
            .context(format!("Failed to connect to {}", url))?;

        response.text().context("Failed to read response body")
    }
}

/// Format check results for display
pub fn format_check_results(results: &[CheckResult]) -> String {
    let mut output = String::new();
    output.push_str("🏥 SkyCode Health Check\n");
    output.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\n");

    let mut ok_count = 0;
    let mut warning_count = 0;
    let mut error_count = 0;

    for result in results {
        match result.status {
            CheckStatus::Ok => ok_count += 1,
            CheckStatus::Warning => warning_count += 1,
            CheckStatus::Error => error_count += 1,
        }

        output.push_str(&format!("{}\n  {}\n\n", result.name, result.message));
    }

    output.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    output.push_str(&format!(
        "Summary: {} OK, {} warnings, {} errors\n",
        ok_count, warning_count, error_count
    ));

    if error_count > 0 {
        output.push_str("\n❌ System is NOT ready. Fix errors above.\n");
    } else if warning_count > 0 {
        output.push_str("\n⚠️  System may work with warnings.\n");
    } else {
        output.push_str("\n✅ All systems operational!\n");
    }

    output
}
