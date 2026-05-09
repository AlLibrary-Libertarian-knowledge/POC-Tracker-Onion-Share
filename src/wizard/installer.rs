/// Instalador automático do Tor por plataforma.
///
/// - Linux  → tenta apt-get / dnf / pacman / zypper
/// - macOS  → tenta brew
/// - Windows → baixa tor-expert-bundle via HTTPS e extrai tor.exe
///             Após instalar, adiciona o diretório ao PATH do usuário
///             (sem precisar de admin — usa HKCU no Registry)
#[cfg(target_os = "windows")]
use anyhow::{Context, Result};
use std::path::PathBuf;

/// Resultado da tentativa de instalação Unix.
#[derive(Clone)]
pub enum InstallResult {
    /// Tor disponível no caminho informado
    Ok(String),
    /// Falha com mensagem de erro
    Err(String),
}

// ─── URL do Tor Expert Bundle para Windows ─────────────────────────────────
// Formato correto: dist.torproject.org/torbrowser/{ver}/tor-expert-bundle-windows-x86_64-{ver}.tar.gz
// Nota: o nome do arquivo tem o OS antes da versão (não depois)
#[cfg(target_os = "windows")]
const TOR_WIN_VER: &str = "14.0.7";
#[cfg(target_os = "windows")]
const TOR_WIN_URL: &str = concat!(
    "https://dist.torproject.org/torbrowser/",
    "14.0.7/tor-expert-bundle-windows-x86_64-14.0.7.tar.gz"
);

// URL de fallback (espelho Arquivo)
#[cfg(target_os = "windows")]
const TOR_WIN_URL_FALLBACK: &str = concat!(
    "https://archive.torproject.org/tor-package-archive/torbrowser/",
    "14.0.7/tor-expert-bundle-windows-x86_64-14.0.7.tar.gz"
);


// ─────────────────────────────────────────────────────────────────────────────

/// Detecta se Tor está disponível. Verifica:
/// 1. Caminho passado explicitamente pelo config
/// 2. `tor` / `tor.exe` no PATH do sistema
/// 3. Bundle já extraído na pasta de dados do app
pub fn detect_tor(configured_path: &str) -> Option<String> {
    // 1. Caminho configurado
    if !configured_path.is_empty() && configured_path != "tor" {
        if try_run(configured_path) {
            return Some(configured_path.to_string());
        }
    }

    // 2. PATH do sistema
    let system_candidates: &[&str] = if cfg!(windows) {
        &["tor.exe", "tor"]
    } else {
        &["tor"]
    };
    for &bin in system_candidates {
        if try_run(bin) {
            return Some(bin.to_string());
        }
    }

    // 3. Bundle já extraído nas versões anteriores
    if let Some(p) = find_bundled_tor() {
        if try_run(p.to_str().unwrap_or("")) {
            return Some(p.to_string_lossy().to_string());
        }
    }

    None
}

fn try_run(bin: &str) -> bool {
    std::process::Command::new(bin)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Retorna o tor.exe do bundle já extraído (se existir)
pub fn find_bundled_tor() -> Option<PathBuf> {
    let bundle_dir = crate::config::AppConfig::tor_data_dir().ok()?;
    let exe = if cfg!(windows) { "tor.exe" } else { "tor" };

    // Locais conhecidos após extração
    let candidates = [
        bundle_dir.join("tor").join(exe),
        bundle_dir.join("Tor").join(exe),
        bundle_dir.join("Browser").join("TorBrowser").join("Tor").join(exe),
        bundle_dir.join(exe),
    ];
    for p in &candidates {
        if p.exists() {
            return Some(p.clone());
        }
    }

    // Busca recursiva
    for entry in walkdir(&bundle_dir) {
        if entry.file_name().map(|f| f == exe).unwrap_or(false) {
            return Some(entry);
        }
    }
    None
}

// ─────────────────────────────────────────────────────────────────────────────
// Linux / macOS
// ─────────────────────────────────────────────────────────────────────────────

pub fn install_tor_unix() -> InstallResult {
    let cmd_args: Option<Vec<&str>> = if cfg!(target_os = "macos") {
        Some(vec!["brew", "install", "tor"])
    } else {
        if which("apt-get") {
            Some(vec!["sudo", "apt-get", "install", "-y", "tor"])
        } else if which("dnf") {
            Some(vec!["sudo", "dnf", "install", "-y", "tor"])
        } else if which("pacman") {
            Some(vec!["sudo", "pacman", "-S", "--noconfirm", "tor"])
        } else if which("zypper") {
            Some(vec!["sudo", "zypper", "install", "-y", "tor"])
        } else {
            None
        }
    };

    let Some(args) = cmd_args else {
        return InstallResult::Err(
            "Nenhum gerenciador de pacotes encontrado.\nInstale manualmente: https://www.torproject.org".into(),
        );
    };

    let status = std::process::Command::new(args[0])
        .args(&args[1..])
        .status();

    match status {
        Ok(s) if s.success() => InstallResult::Ok("tor".into()),
        Ok(s) => InstallResult::Err(format!("Instalação falhou (código {})", s)),
        Err(e) => InstallResult::Err(format!("Erro: {}", e)),
    }
}

fn which(bin: &str) -> bool {
    std::process::Command::new("which")
        .arg(bin)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

// ─────────────────────────────────────────────────────────────────────────────
// Windows — download + extração + PATH do usuário
// ─────────────────────────────────────────────────────────────────────────────

/// Baixa e extrai o Tor expert bundle para Windows.
/// Tenta URL principal (dist.torproject.org); se falhar, tenta fallback (archive).
/// Após extrair, adiciona o diretório do tor.exe ao PATH do usuário (sem admin).
#[cfg(target_os = "windows")]
pub async fn install_tor_windows(
    progress_tx: tokio::sync::mpsc::Sender<f64>,
) -> Result<PathBuf> {
    let bundle_dir = crate::config::AppConfig::tor_data_dir()?;
    tokio::fs::create_dir_all(&bundle_dir).await?;

    // ── Se já temos o bundle de uma instalação anterior, usa direto ──────────
    if let Some(existing) = find_bundled_tor() {
        let _ = progress_tx.send(1.0).await;
        return Ok(existing);
    }

    // ── 1. Download (tenta URL principal, depois fallback) ───────────────────
    let _ = progress_tx.send(0.05).await;

    let buf = download_with_fallback(&progress_tx).await
        .context("Falha ao baixar Tor Expert Bundle. Verifique sua conexão.")?;

    let _ = progress_tx.send(0.75).await;

    // ── 2. Extrai tar.gz ─────────────────────────────────────────────────────
    {
        let buf_clone = buf.clone();
        let bundle_dir_clone = bundle_dir.clone();
        tokio::task::spawn_blocking(move || -> Result<()> {
            let gz = flate2::read::GzDecoder::new(std::io::Cursor::new(&buf_clone));
            let mut archive = tar::Archive::new(gz);
            archive.unpack(&bundle_dir_clone).context("falha ao extrair arquivo")?;
            Ok(())
        })
        .await
        .context("thread de extração falhou")?
        .context("extração do tar.gz falhou")?;
    }

    let _ = progress_tx.send(0.90).await;

    // ── 3. Localiza tor.exe ──────────────────────────────────────────────────
    let tor_exe = find_bundled_tor()
        .context("tor.exe não encontrado após extração do bundle")?;

    // ── 4. Adiciona ao PATH do usuário (sem admin, usa HKCU) ─────────────────
    if let Some(tor_dir) = tor_exe.parent() {
        if let Err(e) = add_to_user_path(tor_dir) {
            // Falha não é fatal — o app usa o caminho absoluto
            eprintln!("Aviso: não foi possível adicionar ao PATH: {}", e);
        }
    }

    let _ = progress_tx.send(1.0).await;
    Ok(tor_exe)
}

/// Tenta baixar da URL principal; se receber erro HTTP ou falha de conexão,
/// tenta o mirror de arquivo.
#[cfg(target_os = "windows")]
async fn download_with_fallback(
    progress_tx: &tokio::sync::mpsc::Sender<f64>,
) -> Result<bytes::Bytes> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300)) // 5 min timeout
        .user_agent("onion-poc/0.3 (https://github.com/DJmesh/onion_poc)")
        .build()
        .context("falha ao criar cliente HTTP")?;

    // Tenta URL principal
    let _ = progress_tx.send(0.1).await;
    match try_download(&client, TOR_WIN_URL).await {
        Ok(b) => return Ok(b),
        Err(e) => {
            eprintln!("URL principal falhou ({}), tentando fallback...", e);
        }
    }

    // Fallback
    let _ = progress_tx.send(0.15).await;
    try_download(&client, TOR_WIN_URL_FALLBACK).await
        .context("Ambas as URLs falharam")
}

#[cfg(target_os = "windows")]
async fn try_download(client: &reqwest::Client, url: &str) -> Result<bytes::Bytes> {
    let resp = client
        .get(url)
        .send()
        .await
        .context("falha na conexão")?
        .error_for_status()
        .context("servidor retornou erro HTTP")?;

    let bytes = resp.bytes().await.context("falha ao ler corpo")?;

    // Verifica se parece um tar.gz válido (magic: 1f 8b)
    if bytes.len() < 2 || bytes[0] != 0x1f || bytes[1] != 0x8b {
        anyhow::bail!(
            "Arquivo baixado não é um .tar.gz válido ({} bytes, magic={:02x}{:02x})",
            bytes.len(), bytes.get(0).copied().unwrap_or(0), bytes.get(1).copied().unwrap_or(0)
        );
    }

    Ok(bytes)
}


/// Adiciona um diretório ao PATH do usuário atual no Windows Registry
/// usando HKEY_CURRENT_USER\Environment — não requer privilégios de admin.
#[cfg(target_os = "windows")]
fn add_to_user_path(dir: &std::path::Path) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;

    let dir_str = dir.to_string_lossy().to_string();

    // Lê o PATH atual do usuário via winreg-free abordagem (cmd /c)
    let current = std::process::Command::new("cmd")
        .args(["/C", "echo %PATH%"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    // Já está no PATH?
    if current.contains(&dir_str) {
        return Ok(());
    }

    // Adiciona ao PATH do usuário via REG ADD
    let new_path = format!("{};{}", dir_str, current);
    let status = std::process::Command::new("reg")
        .args([
            "add",
            r"HKCU\Environment",
            "/v", "PATH",
            "/t", "REG_EXPAND_SZ",
            "/d", &new_path,
            "/f",
        ])
        .status()
        .context("falha ao executar 'reg add'")?;

    if !status.success() {
        anyhow::bail!("reg add saiu com status: {}", status);
    }

    // Notifica o Windows do change (sem admin)
    // Não bloqueia em caso de falha
    let _ = std::process::Command::new("cmd")
        .args(["/C", "setx", "PATH", &new_path])
        .output();

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Utilitários
// ─────────────────────────────────────────────────────────────────────────────

fn walkdir(dir: &PathBuf) -> Vec<PathBuf> {
    let mut results = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let path = e.path();
            if path.is_dir() {
                results.extend(walkdir(&path));
            } else {
                results.push(path);
            }
        }
    }
    results
}
