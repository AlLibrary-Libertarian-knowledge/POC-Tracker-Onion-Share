pub mod routes;
pub mod state;

use std::path::PathBuf;
use std::time::Duration;

use anyhow::Context;
use axum::Router;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::sync::{oneshot, watch};
use tracing::{info, warn};

use crate::crypto::FileKey;
use crate::link::ShareLink;
use crate::share::Share;
use crate::tor::{TorControl, TorProcess};
use state::AppState;

// ─────────────────────────────────────────────────────────────────────────────
// Handle para o servidor em execução (modo TUI)
// ─────────────────────────────────────────────────────────────────────────────

pub struct ShareServerHandle {
    pub state: AppState,
    pub onion_addr: String,   // "<id>.onion"
    pub local_port: u16,
    stop_tx: oneshot::Sender<()>,
    server_task: tokio::task::JoinHandle<anyhow::Result<()>>,
    tor_proc: TorProcess,
    tor_ctl: TorControl,
    service_id: String,
}

impl ShareServerHandle {
    /// Inicia o servidor HTTP + Tor, retorna quando o Onion Service está pronto.
    pub async fn start(tor_path: &str) -> anyhow::Result<Self> {
        // 1. Bind do listener local
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .context("bind failed")?;
        let local_addr = listener.local_addr()?;
        let local_port = local_addr.port();

        let app_state = AppState::new();
        let app: Router = routes::router(app_state.clone());

        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let server_task = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async move {
                    let _ = shutdown_rx.await;
                })
                .await
                .map_err(|e| anyhow::anyhow!(e))
        });

        // 2. Sobe o Tor
        let mut tor = TorProcess::start(tor_path).await?;
        tor.wait_bootstrap(Duration::from_secs(90)).await?;
        let mut ctl = TorControl::connect(tor.control_addr(), tor.cookie_path()).await?;
        let service_id = ctl.add_onion(local_port).await?;
        let onion_addr = format!("{}.onion", service_id);

        info!("Onion service ready: {}", onion_addr);

        Ok(Self {
            state: app_state,
            onion_addr,
            local_port,
            stop_tx: shutdown_tx,
            server_task,
            tor_proc: tor,
            tor_ctl: ctl,
            service_id,
        })
    }

    /// Adiciona um arquivo ao servidor.
    pub async fn add_file(
        &self,
        file_path: PathBuf,
        chunk_size: usize,
        key: FileKey,
    ) -> anyhow::Result<Share> {
        let share = Share::new(file_path, chunk_size, key)?;
        self.state.add_share(share.clone()).await;
        Ok(share)
    }

    /// Remove um arquivo do servidor.
    pub async fn remove_file(&self, file_id: uuid::Uuid) {
        self.state.remove_share(file_id).await;
    }

    /// Link para um arquivo específico.
    pub fn link_for(&self, share: &Share) -> String {
        ShareLink {
            onion: self.onion_addr.clone(),
            file_id: share.file_id,
            key: share.key,
        }
        .to_string()
    }

    /// Watch receiver para contagem online (clone para o TUI).
    pub fn online_rx(&self) -> watch::Receiver<usize> {
        self.state.online_rx.clone()
    }

    /// Endereço SOCKS local do Tor
    pub fn socks_addr(&self) -> String {
        self.tor_proc.socks_addr()
    }

    /// Para o servidor e o Tor.
    pub async fn stop(mut self) {
        let _ = self.tor_ctl.del_onion(&self.service_id).await;
        let _ = self.stop_tx.send(());
        let _ = self.server_task.await;
        let _ = self.tor_proc.kill().await;
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Modo CLI legado: share (mantido para compatibilidade)
// ─────────────────────────────────────────────────────────────────────────────

pub async fn run_share_server(share: Share, tor_path: String) -> anyhow::Result<()> {
    let handle = ShareServerHandle::start(&tor_path).await?;
    let link = handle.link_for(&share);

    handle.state.add_share(share.clone()).await;

    info!("ONION READY ✅");
    info!(
        "File: {} ({} bytes) chunks={} chunk_size={}",
        share.file_name, share.file_size, share.total_chunks, share.chunk_size
    );
    info!("Share link:\n{}", link);

    // Espera Ctrl+C
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            warn!("Ctrl+C — encerrando...");
        }
    }

    handle.stop().await;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Modo CLI legado: join (mantido para compatibilidade)
// ─────────────────────────────────────────────────────────────────────────────

pub async fn run_join_client(
    link: ShareLink,
    out_dir: PathBuf,
    tor_path: String,
) -> anyhow::Result<()> {
    tokio::fs::create_dir_all(&out_dir)
        .await
        .context("failed to create --out dir")?;

    let mut tor = TorProcess::start(&tor_path).await?;
    tor.wait_bootstrap(Duration::from_secs(90)).await?;

    let socks = tor.socks_addr();
    let proxy =
        reqwest::Proxy::all(format!("socks5h://{}", socks)).context("invalid socks proxy")?;
    let client = reqwest::Client::builder()
        .proxy(proxy)
        .build()
        .context("reqwest build failed")?;

    let base = format!("http://{}/s/{}", link.onion, link.file_id);

    // Manifesto
    let manifest: routes::Manifest = client
        .get(format!("{}/manifest", base))
        .send()
        .await
        .context("manifest request failed")?
        .error_for_status()?
        .json()
        .await?;

    info!(
        "Conectado ✅ — {} ({} bytes), {} chunks",
        manifest.file_name, manifest.file_size, manifest.total_chunks
    );

    // Registro de presença
    let reg: routes::RegisterResponse = client
        .post(format!("{}/register", base))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let session_id = reg.session_id;

    // Heartbeat
    let ping_client = client.clone();
    let ping_url = format!("{}/ping", base);
    let ping_task = tokio::spawn(async move {
        loop {
            let _ = ping_client
                .post(&ping_url)
                .json(&routes::PingRequest { session_id })
                .send()
                .await;
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    });

    // Presença em paralelo
    let presence_client = client.clone();
    let presence_url = format!("{}/presence", base);
    let presence_task = tokio::spawn(async move {
        loop {
            if let Ok(r) = presence_client.get(&presence_url).send().await {
                if let Ok(r) = r.error_for_status() {
                    if let Ok(p) = r.json::<routes::PresenceResponse>().await {
                        println!("👥 online agora: {}", p.online);
                    }
                }
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });

    // Download
    let out_path = out_dir.join(&manifest.file_name);
    let mut out_file = tokio::fs::File::create(&out_path)
        .await
        .context("failed to create output file")?;

    for idx in 0..manifest.total_chunks {
        let ct = client
            .get(format!("{}/chunk/{}", base, idx))
            .send()
            .await
            .with_context(|| format!("chunk {} request failed", idx))?
            .error_for_status()?
            .bytes()
            .await?;

        let pt = crate::crypto::decrypt_chunk(&link.key, link.file_id, idx, &ct)?;
        out_file.write_all(&pt).await?;

        if idx % 8 == 0 || idx + 1 == manifest.total_chunks {
            println!("⬇️  {}/{} chunks", idx + 1, manifest.total_chunks);
        }
    }

    out_file.flush().await?;
    info!("Salvo em: {}", out_path.display());

    ping_task.abort();
    presence_task.abort();
    let _ = tor.kill().await;
    Ok(())
}
