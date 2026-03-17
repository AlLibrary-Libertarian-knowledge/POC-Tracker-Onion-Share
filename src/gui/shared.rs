use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use uuid::Uuid;

use crate::tracker_proto::NetworkLobby;

// ─────────────────────────────────────────────────────────────────────────────
// Comandos da GUI → background
// ─────────────────────────────────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub enum GuiControl {
    StartTor,
    StopTor,
    AddFile(PathBuf),
    RemoveFile(Uuid),
    DownloadItem(String, PathBuf),
    RefreshTracker,
}

// ─────────────────────────────────────────────────────────────────────────────
// Estado de Download
// ─────────────────────────────────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub struct DownloadState {
    pub id: Uuid,
    pub _link: String,
    pub name: String,
    pub progress: f32,
    pub bytes_downloaded: u64,
    pub total_bytes: u64,
    pub status: String,
    pub is_done: bool,
    pub error: Option<String>,
    pub speed_bytes_per_sec: u64,
    pub eta_seconds: Option<u64>,
    pub start_time: Option<Instant>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Arquivo compartilhado (informações para a GUI)
// ─────────────────────────────────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub struct SharedFileInfo {
    pub file_id: Uuid,
    pub name: String,
    pub size: u64,
    pub link: String,
    pub content_hash: String,
    pub downloads: u64,
    pub _added_at: Instant,
}

// ─────────────────────────────────────────────────────────────────────────────
// Estado de inicialização do Tor
// ─────────────────────────────────────────────────────────────────────────────
#[derive(Debug, Clone, PartialEq)]
pub enum TorInitState {
    Idle,
    Starting { progress: f32, message: String },
    Ready,
    Error(String),
    Installing { progress: f32, message: String },
}

impl Default for TorInitState {
    fn default() -> Self {
        Self::Idle
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Rede (Tracker)
// Tipos compartilhados em crate::tracker_proto
// ─────────────────────────────────────────────────────────────────────────────

// ─────────────────────────────────────────────────────────────────────────────
// Estado compartilhado (GUI lê + background escreve)
// ─────────────────────────────────────────────────────────────────────────────
#[derive(Debug, Default)]
pub struct SharedState {
    // Rede
    pub tor_active: bool,
    pub onion_addr: Option<String>,
    pub tor_socks_addr: Option<String>,
    pub online_now: usize,
    pub total_sessions: u64,
    pub total_bytes: u64,
    pub chunks_served: u64,
    pub start_time: Option<Instant>,
    pub tor_init: TorInitState,

    // Arquivos
    pub shared_files: Vec<SharedFileInfo>,

    // Downloads
    pub active_downloads: Vec<DownloadState>,

    // Lobby global (preenchido se o tracker estiver ativo)
    pub global_lobby: NetworkLobby,

    // Nós conhecidos (Onion Addresses) para o Gossip
    pub known_peers: Vec<String>,

    // Fila de comandos (GUI escreve, background consome)
    pub control_queue: Vec<GuiControl>,
}

impl SharedState {
    pub fn uptime_str(&self) -> String {
        let secs = self.start_time.map(|t| t.elapsed().as_secs()).unwrap_or(0);
        format!(
            "{:02}:{:02}:{:02}",
            secs / 3600,
            (secs % 3600) / 60,
            secs % 60
        )
    }

    pub fn fmt_bytes(b: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;
        if b >= GB {
            format!("{:.2} GB", b as f64 / GB as f64)
        } else if b >= MB {
            format!("{:.2} MB", b as f64 / MB as f64)
        } else if b >= KB {
            format!("{:.1} KB", b as f64 / KB as f64)
        } else {
            format!("{} B", b)
        }
    }

    pub fn fmt_duration(secs: u64) -> String {
        if secs >= 3600 {
            format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
        } else if secs >= 60 {
            format!("{}m {}s", secs / 60, secs % 60)
        } else {
            format!("{}s", secs)
        }
    }
}

pub type SharedStateRef = Arc<Mutex<SharedState>>;
