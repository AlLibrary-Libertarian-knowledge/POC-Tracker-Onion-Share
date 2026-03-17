/// Background manager para a GUI.
/// Roda em thread separada com runtime Tokio próprio.
/// Lê GuiControl da fila shared.control_queue e atualiza shared.* com resultados.
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::Context;
use crate::config::AppConfig;
use crate::server::ShareServerHandle;
use crate::wizard::installer;

use super::shared::{GuiControl, SharedFileInfo, SharedStateRef, TorInitState};
use crate::tracker_proto::{AnnouncedFile, NetworkFile, NetworkLobby, WsClientMessage, WsServerMessage};
use futures_util::{SinkExt, StreamExt, future::Either};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::client_async_with_config;
use std::net::SocketAddr;

pub fn run_blocking(shared: SharedStateRef, tor_path: String) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    rt.block_on(run(shared, tor_path));
}

async fn run(shared: SharedStateRef, initial_tor_path: String) {
    let mut server: Option<ShareServerHandle> = None;
    let mut tor_path = initial_tor_path;

    // --- Task de Descoberta / Tracker (WebSocket + fallback manual HTTP) ---
    let tracker_shared = shared.clone();
    tokio::spawn(async move {
        tracker_ws_loop(tracker_shared).await;
    });

    loop {
        // Drena a fila de controle
        let cmds: Vec<GuiControl> = {
            let mut s = shared.lock().unwrap();
            std::mem::take(&mut s.control_queue)
        };

        for cmd in cmds {
            match cmd {
                GuiControl::StartTor => {
                    if server.is_some() {
                        // Já está rodando — apenas notifica
                        let onion = server.as_ref().unwrap().onion_addr.clone();
                        let mut s = shared.lock().unwrap();
                        s.tor_active = true;
                        s.onion_addr = Some(onion);
                        s.tor_init = TorInitState::Ready;
                        continue;
                    }

                    // Verifica / instala Tor — atualiza tor_path se instalou de novo
                    match ensure_tor(&shared, &tor_path).await {
                        Some(resolved_bin) => {
                            // Persiste o caminho resolvido no config (importante no Windows)
                            if resolved_bin != tor_path {
                                tor_path = resolved_bin.clone();
                                let mut cfg = AppConfig::load();
                                cfg.tor_path = resolved_bin.clone();
                                let _ = cfg.save();
                            }

                            // Sobe servidor
                            {
                                let mut s = shared.lock().unwrap();
                                s.tor_init = TorInitState::Starting {
                                    progress: 0.82,
                                    message: "Criando Onion Service…".into(),
                                };
                            }

                            match ShareServerHandle::start(&resolved_bin).await {
                                Ok(handle) => {
                                    let onion = handle.onion_addr.clone();
                                    let shared2 = shared.clone();
                                    let mut online_rx = handle.online_rx();
                                    tokio::spawn(async move {
                                        while online_rx.changed().await.is_ok() {
                                            let count = *online_rx.borrow();
                                            shared2.lock().unwrap().online_now = count;
                                        }
                                    });
                                    {
                                        let mut s = shared.lock().unwrap();
                                        s.tor_active = true;
                                        s.onion_addr = Some(onion);
                                        s.tor_socks_addr = Some(handle.socks_addr());
                                        s.start_time = Some(Instant::now());
                                        s.tor_init = TorInitState::Ready;
                                    }
                                    server = Some(handle);
                                }
                                Err(e) => {
                                    let mut s = shared.lock().unwrap();
                                    s.tor_init = TorInitState::Error(format!(
                                        "Falha ao iniciar Tor: {}.\n\
                                         Verifique se o Tor está instalado: \
                                         https://www.torproject.org",
                                        e
                                    ));
                                }
                            }
                        }
                        None => {} // erro já registrado em ensure_tor
                    }
                }

                GuiControl::StopTor => {
                    if let Some(h) = server.take() {
                        h.stop().await;
                    }
                    let mut s = shared.lock().unwrap();
                    s.tor_active = false;
                    s.onion_addr = None;
                    s.tor_socks_addr = None;
                    s.start_time = None;
                    s.online_now = 0;
                    s.tor_init = TorInitState::Idle;
                }

                GuiControl::AddFile(path) => {
                    if let Some(ref h) = server {
                        match h.add_file(path, 256 * 1024).await {
                            Ok(share) => {
                                let link = h.link_for(&share);
                                shared.lock().unwrap().shared_files.push(SharedFileInfo {
                                    file_id: share.file_id,
                                    name: share.file_name.clone(),
                                    size: share.file_size,
                                    link,
                                    content_hash: share.content_hash.clone(),
                                    downloads: 0,
                                    _added_at: Instant::now(),
                                });
                            }
                            Err(e) => {
                                shared.lock().unwrap().tor_init =
                                    TorInitState::Error(format!("Erro ao adicionar arquivo: {}", e));
                            }
                        }
                    }
                }

                GuiControl::RemoveFile(id) => {
                    if let Some(ref h) = server {
                        h.remove_file(id).await;
                    }
                    shared.lock().unwrap().shared_files.retain(|f| f.file_id != id);
                }

                GuiControl::DownloadItem(link_str, out_dir) => {
                    let socks_addr = shared.lock().unwrap().tor_socks_addr.clone();
                    if let Some(socks) = socks_addr {
                        spawn_download_task(shared.clone(), link_str, out_dir, socks);
                    }
                }

                GuiControl::RefreshTracker => {
                    let ts = shared.clone();
                    tokio::spawn(async move {
                        sync_tracker(ts).await;
                    });
                }
            }
        }

        // Atualiza stats do servidor
        if let Some(ref h) = server {
            let stats = h.state.stats.lock().await;
            let mut s = shared.lock().unwrap();
            s.total_sessions = stats.total_sessions;
            s.total_bytes = stats.total_bytes_sent;
            s.chunks_served = stats.chunks_served;
        }

        tokio::time::sleep(Duration::from_millis(300)).await;
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Garante que Tor está disponível (detecta / instala se necessário)
// Retorna Some(caminho) se pronto, None se falhou (erro gravado no SharedState)
// ─────────────────────────────────────────────────────────────────────────────

async fn ensure_tor(shared: &SharedStateRef, configured_path: &str) -> Option<String> {
    // 1. Detecta (PATH, config, bundle existente)
    if let Some(found) = installer::detect_tor(configured_path) {
        return Some(found);
    }

    // 2. Precisa instalar
    {
        shared.lock().unwrap().tor_init = TorInitState::Installing {
            progress: 0.05,
            message: "Preparando instalação do Tor…".into(),
        };
    }

    #[cfg(target_os = "windows")]
    {
        return ensure_tor_windows(shared).await;
    }

    #[cfg(not(target_os = "windows"))]
    {
        return ensure_tor_unix(shared).await;
    }
}

#[cfg(target_os = "windows")]
async fn ensure_tor_windows(shared: &SharedStateRef) -> Option<String> {
    let (prog_tx, mut prog_rx) = tokio::sync::mpsc::channel::<f64>(32);
    let shared2 = shared.clone();

    // Watcher de progresso
    tokio::spawn(async move {
        while let Some(p) = prog_rx.recv().await {
            shared2.lock().unwrap().tor_init = TorInitState::Installing {
                progress: p as f32,
                message: if p < 0.75 {
                    format!("Baixando Tor Expert Bundle: {:.0}%", p * 100.0)
                } else {
                    "Extraindo arquivos…".into()
                },
            };
        }
    });

    match installer::install_tor_windows(prog_tx).await {
        Ok(path) => {
            let path_str = path.to_string_lossy().to_string();
            {
                let mut s = shared.lock().unwrap();
                s.tor_init = TorInitState::Starting {
                    progress: 0.80,
                    message: format!("Tor instalado em: {}", &path_str),
                };
            }
            Some(path_str)
        }
        Err(e) => {
            shared.lock().unwrap().tor_init = TorInitState::Error(format!(
                "❌ Falha ao instalar Tor automaticamente.\n\n\
                 Erro: {}\n\n\
                 Solução manual:\n\
                 1. Baixe o Tor Expert Bundle em: https://www.torproject.org/download/tor/\n\
                 2. Extraia o arquivo\n\
                 3. Adicione a pasta ao PATH do sistema\n\
                 4. Reinicie o onion-poc",
                e
            ));
            None
        }
    }
}

#[cfg(not(target_os = "windows"))]
async fn ensure_tor_unix(shared: &SharedStateRef) -> Option<String> {
    {
        shared.lock().unwrap().tor_init = TorInitState::Installing {
            progress: 0.30,
            message: "Instalando via gerenciador de pacotes…".into(),
        };
    }

    let result = tokio::task::spawn_blocking(installer::install_tor_unix).await;

    match result {
        Ok(installer::InstallResult::Ok(path)) => {
            shared.lock().unwrap().tor_init = TorInitState::Starting {
                progress: 0.80,
                message: "Tor instalado com sucesso!".into(),
            };
            Some(path)
        }
        Ok(installer::InstallResult::Err(msg)) => {
            shared.lock().unwrap().tor_init = TorInitState::Error(format!(
                "❌ {}\n\nInstale manualmente: https://www.torproject.org",
                msg
            ));
            None
        }
        Err(_) => {
            shared.lock().unwrap().tor_init = TorInitState::Error(
                "❌ Erro interno ao instalar Tor.".into(),
            );
            None
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Task de Download (não bloqueia bg.rs)
// ─────────────────────────────────────────────────────────────────────────────
fn spawn_download_task(
    shared: SharedStateRef,
    link_str: String,
    out_dir: PathBuf,
    socks_addr: String,
) {
    let dl_id = uuid::Uuid::new_v4();
    {
        let mut s = shared.lock().unwrap();
        s.active_downloads.push(crate::gui::shared::DownloadState {
            id: dl_id,
            _link: link_str.clone(),
            name: "Aguardando...".into(),
            progress: 0.0,
            bytes_downloaded: 0,
            total_bytes: 0,
            status: "Conectando ao tracker/peer...".into(),
            is_done: false,
            error: None,
            speed_bytes_per_sec: 0,
            start_time: None,
        });
    }

    tokio::spawn(async move {
        macro_rules! update {
            ($p:expr, $bd:expr, $tb:expr, $n:expr, $st:expr, $err:expr, $done:expr, $spd:expr, $start:expr) => {
                if let Some(dl) = shared.lock().unwrap().active_downloads.iter_mut().find(|d| d.id == dl_id) {
                    if let Some(p) = $p { dl.progress = p; }
                    if let Some(bd) = $bd { dl.bytes_downloaded = bd; }
                    if let Some(tb) = $tb { dl.total_bytes = tb; }
                    if let Some(n) = $n { dl.name = n; }
                    if let Some(st) = $st { dl.status = st; }
                    if let Some(err) = $err { dl.error = Some(err); dl.is_done = true; }
                    if let Some(done) = $done { dl.is_done = done; }
                    if let Some(spd) = $spd { dl.speed_bytes_per_sec = spd; }
                    if let Some(start) = $start { dl.start_time = Some(start); }
                }
            };
        }

        let parsed = match crate::link::parse_any(&link_str) {
            Ok(p) => p,
            Err(e) => {
                update!(None, None, None, None, Some("Erro no link".into()), Some(e.to_string()), None, None, None);
                return;
            }
        };

        let start_t = std::time::Instant::now();
        update!(Some(0.0), Some(0), Some(0), None, Some("Preparando download...".into()), None, None, Some(0), Some(start_t));

        let result: anyhow::Result<()> = async {
            match parsed {
                crate::link::ParsedLink::Direct(link) => {
                    let client = build_http_client(&format!("http://{}", link.onion), Some(socks_addr.clone()))?;
                    let base = format!("http://{}/s/{}", link.onion, link.file_id);
                    let manifest: crate::server::routes::Manifest = client
                        .get(format!("{}/manifest", base))
                        .send()
                        .await?
                        .error_for_status()?
                        .json()
                        .await?;

                    update!(Some(0.0), Some(0), Some(manifest.file_size), Some(manifest.file_name.clone()), Some("Baixando direto do peer...".into()), None, None, Some(0), None);
                    tokio::fs::create_dir_all(&out_dir).await.ok();
                    let out_path = out_dir.join(&manifest.file_name);
                    let mut out_file = tokio::fs::File::create(&out_path).await?;
                    let mut downloaded = 0u64;
                    for idx in 0..manifest.total_chunks {
                        let ct = client
                            .get(format!("{}/chunk/{}", base, idx))
                            .send()
                            .await?
                            .error_for_status()?
                            .bytes()
                            .await?;
                        let pt = crate::crypto::decrypt_chunk(&link.key, link.file_id, idx, &ct)?;
                        tokio::io::AsyncWriteExt::write_all(&mut out_file, &pt).await?;
                        downloaded += pt.len() as u64;
                        let prg = (idx + 1) as f32 / manifest.total_chunks.max(1) as f32;
                        let elapsed = start_t.elapsed().as_secs_f64().max(0.001);
                        let speed = (downloaded as f64 / elapsed) as u64;
                        update!(Some(prg), Some(downloaded), Some(manifest.file_size), None, Some("Baixando direto do peer...".into()), None, None, Some(speed), None);
                    }
                    tokio::io::AsyncWriteExt::flush(&mut out_file).await?;
                }
                crate::link::ParsedLink::Swarm(swarm) => {
                    #[derive(serde::Deserialize)]
                    struct SwarmLookupResponse { file: Option<NetworkFile> }

                    let tracker_client = build_http_client(&swarm.tracker_url, Some(socks_addr.clone()))?;
                    let lookup: SwarmLookupResponse = tracker_client
                        .get(format!("{}/swarm/{}", swarm.tracker_url.trim_end_matches('/'), swarm.content_hash))
                        .send()
                        .await?
                        .error_for_status()?
                        .json()
                        .await?;
                    let network_file = lookup.file.context("arquivo não encontrado no swarm do tracker")?;
                    anyhow::ensure!(!network_file.peers.is_empty(), "nenhum peer disponível para esse hash");

                    let peer_client = if let Some(first_peer) = network_file.peers.first() {
                        build_http_client(&format!("http://{}", first_peer.onion), Some(socks_addr.clone()))?
                    } else {
                        anyhow::bail!("nenhum peer disponível")
                    };

                    let first_peer = network_file.peers[0].clone();
                    let base = format!("http://{}/s/{}", first_peer.onion, first_peer.file_id);
                    let manifest: crate::server::routes::Manifest = peer_client
                        .get(format!("{}/manifest", base))
                        .send()
                        .await?
                        .error_for_status()?
                        .json()
                        .await?;

                    let key = crate::crypto::key_from_content_hash(&network_file.content_hash)?;
                    update!(Some(0.0), Some(0), Some(manifest.file_size), Some(manifest.file_name.clone()), Some(format!("Baixando via swarm de {} peers...", network_file.peer_count)), None, None, Some(0), None);
                    tokio::fs::create_dir_all(&out_dir).await.ok();
                    let out_path = out_dir.join(&manifest.file_name);

                    let mut join_set = tokio::task::JoinSet::new();
                    let concurrency = network_file.peers.len().clamp(2, 8);
                    let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(concurrency));

                    for idx in 0..manifest.total_chunks {
                        let permit = sem.clone().acquire_owned().await?;
                        let peers = network_file.peers.clone();
                        let client = build_http_client(&format!("http://{}", peers[0].onion), Some(socks_addr.clone()))?;
                        let key = key;
                        join_set.spawn(async move {
                            let _permit = permit;
                            for offset in 0..peers.len() {
                                let peer = &peers[(idx as usize + offset) % peers.len()];
                                let base = format!("http://{}/s/{}", peer.onion, peer.file_id);
                                let fetched = client
                                    .get(format!("{}/chunk/{}", base, idx))
                                    .send()
                                    .await;
                                if let Ok(resp) = fetched {
                                    if let Ok(ok_resp) = resp.error_for_status() {
                                        if let Ok(bytes) = ok_resp.bytes().await {
                                            if let Ok(pt) = crate::crypto::decrypt_chunk(&key, peer.file_id, idx, &bytes) {
                                                return Ok::<(u64, Vec<u8>), anyhow::Error>((idx, pt));
                                            }
                                        }
                                    }
                                }
                            }
                            anyhow::bail!("não foi possível baixar o chunk {} de nenhum peer", idx)
                        });
                    }

                    let mut chunks: Vec<Option<Vec<u8>>> = vec![None; manifest.total_chunks as usize];
                    let mut downloaded = 0u64;
                    while let Some(res) = join_set.join_next().await {
                        let (idx, pt) = res??;
                        downloaded += pt.len() as u64;
                        chunks[idx as usize] = Some(pt);
                        let done_chunks = chunks.iter().filter(|c| c.is_some()).count() as u64;
                        let prg = done_chunks as f32 / manifest.total_chunks.max(1) as f32;
                        let elapsed = start_t.elapsed().as_secs_f64().max(0.001);
                        let speed = (downloaded as f64 / elapsed) as u64;
                        update!(Some(prg), Some(downloaded), Some(manifest.file_size), None, Some(format!("Baixando via swarm de {} peers...", network_file.peer_count)), None, None, Some(speed), None);
                    }

                    let mut out_file = tokio::fs::File::create(&out_path).await?;
                    for chunk in chunks.into_iter() {
                        let chunk = chunk.context("faltou chunk no download swarm")?;
                        tokio::io::AsyncWriteExt::write_all(&mut out_file, &chunk).await?;
                    }
                    tokio::io::AsyncWriteExt::flush(&mut out_file).await?;
                }
            }
            Ok(())
        }.await;

        match result {
            Ok(()) => update!(Some(1.0), None, None, None, Some("Concluído!".into()), None, Some(true), Some(0), None),
            Err(e) => update!(None, None, None, None, Some("Falha no download".into()), Some(e.to_string()), None, Some(0), None),
        }
    });
}

fn build_http_client(base_url: &str, socks_addr: Option<String>) -> anyhow::Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder();
    if base_url.contains(".onion") {
        if let Some(socks) = socks_addr {
            builder = builder.proxy(reqwest::Proxy::all(format!("socks5h://{}", socks))?);
        }
    }
    Ok(builder.build()?)
}

fn tracker_ws_url(tracker_url: &str) -> String {
    if tracker_url.starts_with("https://") {
        tracker_url.replacen("https://", "wss://", 1) + "/ws"
    } else {
        tracker_url.replacen("http://", "ws://", 1) + "/ws"
    }
}

async fn tracker_ws_loop(shared: SharedStateRef) {
    loop {
        let (tor_active, onion_addr, _socks_addr, tracker_url) = {
            let s = shared.lock().unwrap();
            let cfg = AppConfig::load();
            (
                s.tor_active,
                s.onion_addr.clone(),
                s.tor_socks_addr.clone(),
                cfg.tracker_url,
            )
        };

        if !tor_active || onion_addr.is_none() {
            tokio::time::sleep(Duration::from_secs(2)).await;
            continue;
        }

        let socks_addr_str = _socks_addr.clone();
        let tracker_url_clone = tracker_url.clone();
        let ws_url = tracker_ws_url(tracker_url_clone.trim_end_matches('/'));

        let ws_conn: anyhow::Result<Either<_, _>> = async {
            if tracker_url_clone.contains(".onion") {
                if let Some(ref socks) = socks_addr_str {
                    let socks_socket: SocketAddr = socks.parse()?;
                    let url = url::Url::parse(&ws_url)?;
                    let host = url.host_str().context("no host")?;
                    let port = url.port().unwrap_or(80);
                    let stream = tokio_socks::tcp::Socks5Stream::connect(socks_socket, (host, port)).await?;
                    let (ws, _resp) = client_async_with_config(ws_url.clone(), stream, None).await?;
                    Ok(Either::Left(ws))
                } else {
                    anyhow::bail!("Tor desativado para tracker onion")
                }
            } else {
                let (ws, _resp) = tokio_tungstenite::connect_async(ws_url.clone()).await?;
                Ok(Either::Right(ws))
            }
        }.await;

        match ws_conn {
            Ok(ws_stream) => {
                tracing::info!("Conectado ao Tracker WebSocket: {}", ws_url);
                let _ = ws_comm_loop(ws_stream, shared.clone()).await;
            }
            Err(e) => {
                tracing::warn!("Erro ao conectar no Tracker WebSocket ({}): {}", ws_url, e);
                let _ = sync_tracker(shared.clone()).await;
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
        }
    }
}

async fn ws_comm_loop<S>(mut ws_stream: S, shared: SharedStateRef) -> anyhow::Result<()>
where S: SinkExt<Message, Error = tokio_tungstenite::tungstenite::Error> + StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin
{
    let mut interval = tokio::time::interval(Duration::from_secs(5));
    loop {
        tokio::select! {
            _ = interval.tick() => {
                let payload = {
                    let s = shared.lock().unwrap();
                    let cfg = AppConfig::load();
                    if !s.tor_active {
                        None
                    } else {
                        let files = if cfg.share_publicly {
                            s.shared_files.iter().map(|f| AnnouncedFile {
                                file_id: f.file_id,
                                name: f.name.clone(),
                                size: f.size,
                                link: f.link.clone(),
                                content_hash: f.content_hash.clone(),
                            }).collect()
                        } else {
                            Vec::new()
                        };
                        s.onion_addr.clone().map(|onion| WsClientMessage::Announce {
                            node_id: cfg.node_id,
                            onion,
                            files,
                        })
                    }
                };
                if let Some(msg) = payload {
                    let text = serde_json::to_string(&msg)?;
                    ws_stream.send(Message::Text(text.into())).await?;
                }
            }
            incoming = ws_stream.next() => {
                match incoming {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(WsServerMessage::Lobby { lobby }) = serde_json::from_str::<WsServerMessage>(&text) {
                            shared.lock().unwrap().global_lobby = lobby;
                        }
                    }
                    Some(Ok(Message::Ping(bytes))) => {
                        let _ = ws_stream.send(Message::Pong(bytes)).await;
                    }
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

async fn sync_tracker(tracker_shared: SharedStateRef) {
    let (tor_active, socks_addr, tracker_url) = {
        let s = tracker_shared.lock().unwrap();
        let cfg = AppConfig::load();
        (s.tor_active, s.tor_socks_addr.clone(), cfg.tracker_url)
    };

    if !tor_active {
        return;
    }

    let tracker_url = tracker_url.trim_end_matches('/').to_string();
    if let Ok(client) = build_http_client(&tracker_url, socks_addr) {
        if let Ok(res) = client.get(format!("{}/lobby", tracker_url)).send().await {
            if let Ok(lobby) = res.json::<NetworkLobby>().await {
                tracker_shared.lock().unwrap().global_lobby = lobby;
            }
        }
    }
}
