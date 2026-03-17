use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{watch, Mutex};
use uuid::Uuid;

use crate::share::Share;

// ─────────────────────────────────────────────────────────────────────────────

#[derive(Default, Clone, Debug)]
pub struct GlobalStats {
    pub total_sessions: u64,
    pub total_bytes_sent: u64,
    pub chunks_served: u64,
}

// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct AppState {
    /// Arquivos atualmente compartilhados (chave = file_id)
    pub shares: Arc<Mutex<HashMap<Uuid, Share>>>,
    /// Sessões de presença ativas (session_id → last_seen)
    sessions: Arc<Mutex<HashMap<Uuid, Instant>>>,
    pub ttl: Duration,
    /// Estatísticas globais acumuladas
    pub stats: Arc<Mutex<GlobalStats>>,
    /// Broadcast da contagem online (TUI consome via .subscribe())
    online_tx: Arc<watch::Sender<usize>>,
    pub online_rx: watch::Receiver<usize>,
    pub node_id: String,
    pub onion_addr: Arc<Mutex<Option<String>>>,
}

impl AppState {
    pub fn new(node_id: String) -> Self {
        let (online_tx, online_rx) = watch::channel(0usize);
        Self {
            shares: Arc::new(Mutex::new(HashMap::new())),
            sessions: Arc::new(Mutex::new(HashMap::new())),
            ttl: Duration::from_secs(6),
            stats: Arc::new(Mutex::new(GlobalStats::default())),
            online_tx: Arc::new(online_tx),
            online_rx,
            node_id,
            onion_addr: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn add_share(&self, share: Share) {
        self.shares.lock().await.insert(share.file_id, share);
    }

    pub async fn remove_share(&self, id: Uuid) {
        self.shares.lock().await.remove(&id);
    }

    pub async fn register(&self) -> Uuid {
        let id = Uuid::new_v4();
        {
            let mut map = self.sessions.lock().await;
            map.insert(id, Instant::now());
            let n = map.len();
            let _ = self.online_tx.send(n);
        }
        {
            let mut s = self.stats.lock().await;
            s.total_sessions += 1;
        }
        id
    }

    pub async fn ping(&self, sid: Uuid) {
        let mut map = self.sessions.lock().await;
        map.insert(sid, Instant::now());
        let n = map.len();
        let _ = self.online_tx.send(n);
    }

    pub async fn online_count(&self) -> usize {
        let now = Instant::now();
        let mut map = self.sessions.lock().await;
        map.retain(|_, t| now.duration_since(*t) <= self.ttl);
        let n = map.len();
        let _ = self.online_tx.send(n);
        n
    }

    pub async fn record_bytes(&self, bytes: u64) {
        let mut s = self.stats.lock().await;
        s.chunks_served += 1;
        s.total_bytes_sent += bytes;
    }
}
