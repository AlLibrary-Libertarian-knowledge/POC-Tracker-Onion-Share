use std::path::PathBuf;

use anyhow::Context;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;

/// Minimal Tor control-port client:
/// - AUTHENTICATE <cookie-hex>
/// - ADD_ONION NEW:ED25519-V3 Port=80,127.0.0.1:<port>
/// - DEL_ONION <service_id>
pub struct TorControl {
    writer: OwnedWriteHalf,
    reader: BufReader<OwnedReadHalf>,
}

impl TorControl {
    pub async fn connect(control_addr: String, cookie_path: PathBuf) -> anyhow::Result<Self> {
        let stream = TcpStream::connect(&control_addr)
            .await
            .with_context(|| format!("connect control port {}", control_addr))?;

        let (read_half, write_half) = stream.into_split();

        let mut ctl = Self {
            writer: write_half,
            reader: BufReader::new(read_half),
        };

        let cookie = tokio::fs::read(&cookie_path)
            .await
            .with_context(|| format!("failed to read cookie file {}", cookie_path.display()))?;
        let cookie_hex = hex::encode(cookie);

        ctl.cmd(&format!("AUTHENTICATE {}", cookie_hex)).await?;
        Ok(ctl)
    }

    pub async fn add_onion(&mut self, local_port: u16) -> anyhow::Result<String> {
        // Map onion port 80 -> localhost:local_port
        let cmd = format!("ADD_ONION NEW:ED25519-V3 Port=80,127.0.0.1:{}", local_port);
        let lines = self.cmd(&cmd).await?;

        // Look for 250-ServiceID=...
        for l in lines {
            if let Some(rest) = l.strip_prefix("250-ServiceID=") {
                return Ok(rest.trim().to_string());
            }
        }
        anyhow::bail!("ADD_ONION did not return ServiceID");
    }

    pub async fn del_onion(&mut self, service_id: &str) -> anyhow::Result<()> {
        let _ = self.cmd(&format!("DEL_ONION {}", service_id)).await?;
        Ok(())
    }

    async fn cmd(&mut self, cmd: &str) -> anyhow::Result<Vec<String>> {
        self.writer
            .write_all(format!("{}\r\n", cmd).as_bytes())
            .await
            .context("control write failed")?;
        self.writer.flush().await.ok();

        self.read_response().await
    }

    async fn read_response(&mut self) -> anyhow::Result<Vec<String>> {
        let mut out = Vec::new();
        loop {
            let mut line = String::new();
            let n = self.reader.read_line(&mut line).await.context("control read failed")?;
            anyhow::ensure!(n > 0, "control port closed");

            let line = line.trim_end_matches(&['\r', '\n'][..]).to_string();
            out.push(line.clone());

            // End of response: status code + space (e.g. "250 OK")
            if line.len() >= 4 {
                let bytes = line.as_bytes();
                if bytes[3] == b' ' {
                    // error?
                    if bytes[0] == b'4' || bytes[0] == b'5' {
                        anyhow::bail!("tor control error: {:?}", out);
                    }
                    return Ok(out);
                }
            }
        }
    }
}
