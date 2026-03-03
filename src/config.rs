use anyhow::Context;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone)]
pub struct AppConfig {
    /// Usuário aceitou os termos de uso
    pub terms_accepted: bool,
    /// Caminho para o binário tor (vazio = usa "tor" do PATH)
    pub tor_path: String,
    /// ID anônimo deste nó
    pub node_id: String,
    /// URL do servidor tracker (padrão local para teste)
    pub tracker_url: String,
    /// Se true, arquivos compartilhados vão para o lobby público
    pub share_publicly: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            terms_accepted: false,
            tor_path: String::new(),
            node_id: uuid::Uuid::new_v4().to_string(),
            tracker_url: "http://3phps2siiwstimug2mipw7tlizdvdmfydjf5clb7phujg4yfnkrh56qd.onion"
                .to_string(),
            share_publicly: true,
        }
    }
}

impl AppConfig {
    fn config_path() -> Option<PathBuf> {
        ProjectDirs::from("br", "tcc", "onion_poc").map(|d| d.config_dir().join("config.json"))
    }

    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            return Self::default();
        };
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::config_path().context("no config dir")?;
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    pub fn tor_bin(&self) -> &str {
        if self.tor_path.is_empty() {
            "tor"
        } else {
            &self.tor_path
        }
    }

    pub fn tor_available(&self) -> bool {
        std::process::Command::new(self.tor_bin())
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    pub fn effective_tor_path(&self) -> String {
        self.tor_bin().to_string()
    }

    /// Diretório de dados para o Tor bundled (Windows)
    pub fn tor_data_dir() -> anyhow::Result<PathBuf> {
        ProjectDirs::from("br", "tcc", "onion_poc")
            .map(|d| d.data_local_dir().join("tor_bundle"))
            .context("no data dir")
    }
}
