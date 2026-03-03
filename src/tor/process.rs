use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use anyhow::Context;
use directories::ProjectDirs;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::watch;
use tracing::{info, warn};

#[derive(Debug)]
pub struct TorProcess {
    data_dir: PathBuf,
    socks_port: u16,
    control_port: u16,
    child: Child,
    boot_rx: watch::Receiver<bool>,
}

impl TorProcess {
    pub async fn start(tor_path: &str) -> anyhow::Result<Self> {
        let (socks_port, control_port) = (free_port().await?, free_port().await?);

        let data_dir = tor_data_dir()?;
        tokio::fs::create_dir_all(&data_dir).await.context("create tor data_dir failed")?;

        // tor --SocksPort 127.0.0.1:PORT --ControlPort 127.0.0.1:PORT --CookieAuthentication 1 --DataDirectory DIR --Log "notice stdout"
        let mut cmd = Command::new(tor_path);
        cmd.arg("--SocksPort").arg(format!("127.0.0.1:{}", socks_port))
            .arg("--ControlPort").arg(format!("127.0.0.1:{}", control_port))
            .arg("--CookieAuthentication").arg("1")
            .arg("--DataDirectory").arg(&data_dir)
            .arg("--Log").arg("notice stdout")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn().with_context(|| format!("failed to spawn tor: {}", tor_path))?;

        let stdout = child.stdout.take().context("tor stdout unavailable")?;
        let stderr = child.stderr.take().context("tor stderr unavailable")?;

        let (boot_tx, boot_rx) = watch::channel(false);

        // Read stdout for bootstrap
        let boot_tx2 = boot_tx.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if line.contains("Bootstrapped 100%") {
                    let _ = boot_tx2.send(true);
                }
            }
        });

        // Drain stderr (useful for debugging)
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                warn!("tor: {}", line);
            }
        });

        Ok(Self {
            data_dir,
            socks_port,
            control_port,
            child,
            boot_rx,
        })
    }

    pub fn socks_addr(&self) -> String {
        format!("127.0.0.1:{}", self.socks_port)
    }

    pub fn control_addr(&self) -> String {
        format!("127.0.0.1:{}", self.control_port)
    }

    pub fn cookie_path(&self) -> PathBuf {
        // Tor writes control_auth_cookie into its DataDirectory when CookieAuthentication=1. citeturn0search3turn0search20
        self.data_dir.join("control_auth_cookie")
    }

    pub async fn wait_bootstrap(&mut self, timeout: Duration) -> anyhow::Result<()> {
        // also wait for cookie to exist
        let cookie = self.cookie_path();

        let t0 = tokio::time::Instant::now();
        loop {
            if cookie.exists() && *self.boot_rx.borrow() {
                info!("Tor ready ✅ (socks={}, control={})", self.socks_port, self.control_port);
                return Ok(());
            }
            if t0.elapsed() > timeout {
                anyhow::bail!("Tor bootstrap timeout ({}s). Is tor installed and allowed to run?", timeout.as_secs());
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }

    pub async fn wait(&mut self) -> anyhow::Result<std::process::ExitStatus> {
        let status = self.child.wait().await.context("wait tor failed")?;
        Ok(status)
    }

    pub async fn kill(&mut self) -> anyhow::Result<()> {
        let _ = self.child.kill().await;
        Ok(())
    }
}

fn tor_data_dir() -> anyhow::Result<PathBuf> {
    let proj = ProjectDirs::from("br", "tcc", "onion_poc").context("ProjectDirs unavailable")?;
    let base = proj.data_local_dir();
    // unique per run
    let dir = base.join(format!("tor-{}", uuid::Uuid::new_v4()));
    Ok(dir)
}

async fn free_port() -> anyhow::Result<u16> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    Ok(listener.local_addr()?.port())
}
