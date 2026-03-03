/// Background manager para a GUI.
/// Roda em thread separada com runtime Tokio próprio.
/// Lê GuiControl da fila shared.control_queue e atualiza shared.* com resultados.
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::config::AppConfig;
use crate::crypto;
use crate::server::ShareServerHandle;
use crate::wizard::installer;

use super::shared::{GuiControl, NetworkFile, NetworkLobby, SharedFileInfo, SharedStateRef, TorInitState};

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

    // --- Task de Descoberta / Tracker (P2P Lobby) ---
    let tracker_shared = shared.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        loop {
            interval.tick().await;
            sync_tracker(tracker_shared.clone()).await;
        }
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
                        let key = crypto::random_key();
                        match h.add_file(path, 256 * 1024, key).await {
                            Ok(share) => {
                                let link = h.link_for(&share);
                                shared.lock().unwrap().shared_files.push(SharedFileInfo {
                                    file_id: share.file_id,
                                    name: share.file_name.clone(),
                                    size: share.file_size,
                                    link,
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
            status: "Conectando ao OnionShare...".into(),
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

        let link = match crate::link::ShareLink::parse(&link_str) {
            Ok(l) => l,
            Err(e) => {
                update!(None, None, None, None, Some("Erro no link".into()), Some(e.to_string()), None, None, None);
                return;
            }
        };

        let proxy = match reqwest::Proxy::all(format!("socks5h://{}", socks_addr)) {
            Ok(p) => p,
            Err(e) => {
                update!(None, None, None, None, Some("Erro de proxy".into()), Some(e.to_string()), None, None, None);
                return;
            }
        };

        let client = match reqwest::Client::builder().proxy(proxy).build() {
            Ok(c) => c,
            Err(e) => {
                update!(None, None, None, None, Some("Erro cliente".into()), Some(e.to_string()), None, None, None);
                return;
            }
        };

        let base = format!("http://{}/s/{}", link.onion, link.file_id);

        let manifest: crate::server::routes::Manifest = match client.get(format!("{}/manifest", base)).send().await {
            Ok(r) => match r.error_for_status() {
                Ok(res) => match res.json().await {
                    Ok(m) => m,
                    Err(e) => { update!(None, None, None, None, Some("Erro manifest".into()), Some(e.to_string()), None, None, None); return; }
                },
                Err(e) => { update!(None, None, None, None, Some("Arquivo não encontrado".into()), Some(e.to_string()), None, None, None); return; }
            },
            Err(e) => { update!(None, None, None, None, Some("Tor falhou".into()), Some(e.to_string()), None, None, None); return; }
        };

        let start_t = std::time::Instant::now();
        update!(Some(0.0), Some(0), Some(manifest.file_size), Some(manifest.file_name.clone()), Some("Baixando...".into()), None, None, Some(0), Some(start_t));

        let out_path = out_dir.join(&manifest.file_name);
        tokio::fs::create_dir_all(&out_dir).await.ok();
        let mut out_file = match tokio::fs::File::create(&out_path).await {
            Ok(f) => f,
            Err(e) => { update!(None, None, None, None, Some("Erro ao criar arquivo".into()), Some(e.to_string()), None, None, None); return; }
        };

        let mut bd = 0;
        let mut last_speed_update = std::time::Instant::now();
        let mut bytes_since_last_update = 0;
        let mut speed = 0;

        for idx in 0..manifest.total_chunks {
            let ct = match client.get(format!("{}/chunk/{}", base, idx)).send().await {
                Ok(r) => match r.error_for_status() {
                    Ok(res) => match res.bytes().await {
                        Ok(b) => b,
                        Err(e) => { update!(None, None, None, None, Some("Erro bytes".into()), Some(e.to_string()), None, None, None); return; }
                    },
                    Err(e) => { update!(None, None, None, None, Some("Erro HTTP ao baixar".into()), Some(e.to_string()), None, None, None); return; }
                },
                Err(e) => { update!(None, None, None, None, Some("Conexão caiu".into()), Some(e.to_string()), None, None, None); return; }
            };

            let pt = match crate::crypto::decrypt_chunk(&link.key, link.file_id, idx, &ct) {
                Ok(pt) => pt,
                Err(e) => { update!(None, None, None, None, Some("Erro de Criptografia".into()), Some(e.to_string()), None, None, None); return; }
            };

            if let Err(e) = tokio::io::AsyncWriteExt::write_all(&mut out_file, &pt).await {
                update!(None, None, None, None, Some("Erro disco".into()), Some(e.to_string()), None, None, None);
                return;
            }

            let chunk_len = pt.len() as u64;
            bd += chunk_len;
            bytes_since_last_update += chunk_len;

            let elapsed = last_speed_update.elapsed().as_secs_f64();
            if elapsed >= 1.0 {
                speed = (bytes_since_last_update as f64 / elapsed) as u64;
                last_speed_update = std::time::Instant::now();
                bytes_since_last_update = 0;
            }

            let prg = (idx + 1) as f32 / manifest.total_chunks as f32;
            update!(Some(prg), Some(bd), None, None, Some("Baixando...".into()), None, None, Some(speed), None);
        }

        let _ = tokio::io::AsyncWriteExt::flush(&mut out_file).await;
        let _ = tokio::io::AsyncWriteExt::flush(&mut out_file).await;
        update!(Some(1.0), Some(manifest.file_size), None, None, Some("Concluído!".into()), None, Some(true), Some(0), None);
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// Sincroniza ativamente com o Tracker (Ping & Fetch)
// ─────────────────────────────────────────────────────────────────────────────
async fn sync_tracker(tracker_shared: SharedStateRef) {
    let (tor_active, socks_addr, node_id, tracker_url, files) = {
        let s = tracker_shared.lock().unwrap();
        let cfg = crate::config::AppConfig::load();

        let p_files = if cfg.share_publicly {
            s.shared_files
                .iter()
                .map(|f| NetworkFile {
                    name: f.name.clone(),
                    size: f.size,
                    link: f.link.clone(),
                })
                .collect()
        } else {
            vec![]
        };

        (
            s.tor_active,
            s.tor_socks_addr.clone(),
            cfg.node_id,
            cfg.tracker_url,
            p_files,
        )
    };

    if !tor_active {
        return;
    }

    // Se for .onion, usa o proxy Socks5h senão usa cliente normal
    let client = if tracker_url.contains(".onion") {
        if let Some(socks) = socks_addr {
            if let Ok(proxy) = reqwest::Proxy::all(format!("socks5h://{}", socks)) {
                reqwest::Client::builder()
                    .proxy(proxy)
                    .build()
                    .unwrap_or_default()
            } else {
                reqwest::Client::new()
            }
        } else {
            reqwest::Client::new()
        }
    } else {
        reqwest::Client::new()
    };

    #[derive(serde::Serialize)]
    struct PingReq {
        node_id: String,
        files: Vec<NetworkFile>,
    }

    // Ping Tracker (Avisa: Estou online e tenho esses arquivos)
    let _ = client
        .post(format!("{}/ping", tracker_url))
        .json(&PingReq {
            node_id: node_id.clone(),
            files: files.clone(),
        })
        .send()
        .await;

    // Busca Arquivos Globalmente
    if let Ok(res) = client.get(format!("{}/lobby", tracker_url)).send().await {
        if let Ok(lobby) = res.json::<NetworkLobby>().await {
            let mut s = tracker_shared.lock().unwrap();
            s.global_lobby = lobby;
        }
    }
}
