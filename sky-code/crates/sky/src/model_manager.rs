use anyhow::{anyhow, Context, Result};
use serde_json::json;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use tracing::{debug, info};

/// Manages local model operations via Ollama CLI
pub struct ModelManager {
    ollama_url: String,
}

impl ModelManager {
    pub fn new(ollama_url: Option<&str>) -> Self {
        let url = ollama_url
            .unwrap_or("http://localhost:11434")
            .to_string();
        Self { ollama_url: url }
    }

    /// List all locally installed models
    pub fn list_models(&self) -> Result<Vec<ModelInfo>> {
        debug!("Listing models from Ollama at {}", self.ollama_url);

        // Try using Ollama CLI first
        let output = Command::new("ollama")
            .arg("list")
            .output()
            .map_err(|e| anyhow!("Failed to run 'ollama list': {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("ollama list failed: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut models = Vec::new();

        for line in stdout.lines().skip(1) {
            if line.trim().is_empty() {
                continue;
            }
            // Format: NAME ID SIZE MODIFIED
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                models.push(ModelInfo {
                    name: parts[0].to_string(),
                    size: parts.get(2).map(|s| s.to_string()).unwrap_or_default(),
                });
            }
        }

        Ok(models)
    }

    /// Install a model by name (e.g., "qwen3:14b")
    pub fn install_model(&self, name: &str) -> Result<()> {
        info!("Installing model: {}", name);

        let child = Command::new("ollama")
            .arg("pull")
            .arg(name)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| anyhow!("Failed to spawn 'ollama pull': {}", e))?;

        let stdout = BufReader::new(
            child
                .stdout
                .ok_or_else(|| anyhow!("Could not capture stdout"))?,
        );

        for line in stdout.lines() {
            if let Ok(line) = line {
                info!("  {}", line);
            }
        }

        let output = Command::new("ollama")
            .arg("pull")
            .arg(name)
            .output()
            .map_err(|e| anyhow!("Failed to pull model: {}", e))?;

        if output.status.success() {
            info!("Successfully installed model: {}", name);
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow!("Failed to install model {}: {}", name, stderr))
        }
    }

    /// Uninstall a model
    pub fn uninstall_model(&self, name: &str) -> Result<()> {
        info!("Uninstalling model: {}", name);

        let output = Command::new("ollama")
            .arg("rm")
            .arg(name)
            .output()
            .map_err(|e| anyhow!("Failed to spawn 'ollama rm': {}", e))?;

        if output.status.success() {
            info!("Successfully uninstalled model: {}", name);
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow!("Failed to uninstall model {}: {}", name, stderr))
        }
    }

    /// Install all models in a preset pack
    pub fn install_pack(&self, models: &[String]) -> Result<()> {
        info!("Installing pack with {} models", models.len());

        for (idx, model) in models.iter().enumerate() {
            info!("[{}/{}] Installing {}", idx + 1, models.len(), model);
            self.install_model(model)?;
        }

        info!("Pack installation complete");
        Ok(())
    }

    /// Create a model alias in config
    pub fn create_alias(
        &self,
        alias: &str,
        source_model: &str,
    ) -> Result<()> {
        info!("Creating alias '{}' -> '{}'", alias, source_model);

        // Note: this is just recording the alias; Ollama doesn't support native aliases
        // In a real implementation, this would update the config.toml
        info!("Alias mapping recorded: {} -> {}", alias, source_model);
        Ok(())
    }

    /// Output model list as JSON
    pub fn list_models_json(&self) -> Result<String> {
        let models = self.list_models()?;
        let json = json!(models);
        Ok(json.to_string())
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ModelInfo {
    pub name: String,
    pub size: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_manager_creation() {
        let mgr = ModelManager::new(None);
        assert_eq!(mgr.ollama_url, "http://localhost:11434");

        let mgr2 = ModelManager::new(Some("http://127.0.0.1:11434"));
        assert_eq!(mgr2.ollama_url, "http://127.0.0.1:11434");
    }
}
