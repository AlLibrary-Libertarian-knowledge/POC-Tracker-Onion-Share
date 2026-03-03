/// Task em background que gerencia Tor + servidor HTTP.
/// Recebe ControlMsg do TUI e envia AppEvent de volta.
use std::path::PathBuf;
use std::time::Duration;

use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::crypto;
use crate::server::ShareServerHandle;
use crate::wizard::app::{AppEvent, ControlMsg};

pub async fn run(
    event_tx: mpsc::Sender<AppEvent>,
    mut ctrl_rx: mpsc::Receiver<ControlMsg>,
    tor_path: String,
) {
    let mut server: Option<ShareServerHandle> = None;

    while let Some(msg) = ctrl_rx.recv().await {
        match msg {
            // ── Iniciar Tor ───────────────────────────────────────────────────
            ControlMsg::StartTor => {
                if server.is_some() {
                    let _ = event_tx
                        .send(AppEvent::TorStarted {
                            onion: server.as_ref().unwrap().onion_addr.clone(),
                        })
                        .await;
                    continue;
                }

                let tx = event_tx.clone();
                let tp = tor_path.clone();

                // Verifica se Tor está instalado; se não, tenta instalar
                let tor_bin = if std::process::Command::new(&tp)
                    .arg("--version")
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false)
                {
                    tp.clone()
                } else {
                    // Instalação automática
                    let installed = install_tor(&tx, &tp).await;
                    match installed {
                        Some(p) => p,
                        None => continue, // erro já enviado
                    }
                };

                // Inicia servidor
                let _ = tx
                    .send(AppEvent::InstallProgress(
                        0.9,
                        "Iniciando Onion Service...".into(),
                    ))
                    .await;

                match ShareServerHandle::start(&tor_bin).await {
                    Ok(handle) => {
                        let onion = handle.onion_addr.clone();

                        // Watch online count
                        let mut online_rx = handle.online_rx();
                        let tx2 = tx.clone();
                        tokio::spawn(async move {
                            while online_rx.changed().await.is_ok() {
                                // online count é lida pelo App.tick() via watch
                                let _ = tx2; // keep alive
                            }
                        });

                        let _ = tx
                            .send(AppEvent::TorStarted { onion })
                            .await;
                        server = Some(handle);
                    }
                    Err(e) => {
                        let _ = tx.send(AppEvent::TorError(e.to_string())).await;
                    }
                }
            }

            // ── Parar Tor ─────────────────────────────────────────────────────
            ControlMsg::StopTor => {
                if let Some(h) = server.take() {
                    h.stop().await;
                    let _ = event_tx.send(AppEvent::TorStopped).await;
                }
            }

            // ── Adicionar arquivo ─────────────────────────────────────────────
            ControlMsg::AddFile { path, chunk_size } => {
                if let Some(ref h) = server {
                    let key = crypto::random_key();
                    match h.add_file(path, chunk_size, key).await {
                        Ok(share) => {
                            let link = h.link_for(&share);
                            let _ = event_tx
                                .send(AppEvent::FileAdded {
                                    file_id: share.file_id,
                                    name: share.file_name.clone(),
                                    size: share.file_size,
                                    link,
                                })
                                .await;
                        }
                        Err(e) => {
                            let _ = event_tx.send(AppEvent::TorError(e.to_string())).await;
                        }
                    }
                } else {
                    let _ = event_tx
                        .send(AppEvent::TorError(
                            "Ative o OnionShare antes de compartilhar.".into(),
                        ))
                        .await;
                }
            }

            // ── Remover arquivo ───────────────────────────────────────────────
            ControlMsg::RemoveFile(id) => {
                if let Some(ref h) = server {
                    h.remove_file(id).await;
                    let _ = event_tx.send(AppEvent::FileRemoved(id)).await;
                }
            }
        }
    }

    // Cleanup quando o canal fecha
    if let Some(h) = server.take() {
        h.stop().await;
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Instalação automática do Tor por plataforma
// ─────────────────────────────────────────────────────────────────────────────

async fn install_tor(tx: &mpsc::Sender<AppEvent>, default_bin: &str) -> Option<String> {
    let _ = tx
        .send(AppEvent::InstallProgress(0.05, "Detectando plataforma...".into()))
        .await;

    #[cfg(target_os = "windows")]
    {
        return install_tor_windows(tx).await;
    }

    #[cfg(not(target_os = "windows"))]
    {
        install_tor_unix(tx, default_bin).await
    }
}

#[cfg(not(target_os = "windows"))]
async fn install_tor_unix(
    tx: &mpsc::Sender<AppEvent>,
    default_bin: &str,
) -> Option<String> {
    use super::installer;

    let _ = tx
        .send(AppEvent::InstallProgress(
            0.2,
            "Instalando via gerenciador de pacotes...".into(),
        ))
        .await;

    // Executa em thread bloqueante para não travar o runtime
    let result = tokio::task::spawn_blocking(installer::install_tor_unix).await;

    match result {
        Ok(installer::InstallResult::Ok(path)) => {
            let _ = tx
                .send(AppEvent::InstallDone {
                    ok: true,
                    tor_path: Some(path.clone()),
                })
                .await;
            Some(path)
        }
        Ok(installer::InstallResult::Err(e)) => {
            let _ = tx
                .send(AppEvent::InstallDone { ok: false, tor_path: None })
                .await;
            let _ = tx.send(AppEvent::TorError(e)).await;
            None
        }
        Err(e) => {
            let _ = tx.send(AppEvent::TorError(e.to_string())).await;
            None
        }
    }
}

#[cfg(target_os = "windows")]
async fn install_tor_windows(tx: &mpsc::Sender<AppEvent>) -> Option<String> {
    use super::installer;

    let (prog_tx, mut prog_rx) = tokio::sync::mpsc::channel::<f64>(32);

    // Spawn da task de progresso
    let tx2 = tx.clone();
    tokio::spawn(async move {
        while let Some(p) = prog_rx.recv().await {
            let msg = if p < 1.0 {
                format!("Baixando Tor Expert Bundle: {:.0}%", p * 100.0)
            } else {
                "Extraindo...".into()
            };
            let _ = tx2.send(AppEvent::InstallProgress(p * 0.8 + 0.1, msg)).await;
        }
    });

    match installer::install_tor_windows(prog_tx).await {
        Ok(path) => {
            let path_str = path.to_string_lossy().to_string();
            let _ = tx
                .send(AppEvent::InstallDone {
                    ok: true,
                    tor_path: Some(path_str.clone()),
                })
                .await;
            Some(path_str)
        }
        Err(e) => {
            let _ = tx
                .send(AppEvent::InstallDone { ok: false, tor_path: None })
                .await;
            let _ = tx.send(AppEvent::TorError(e.to_string())).await;
            None
        }
    }
}
