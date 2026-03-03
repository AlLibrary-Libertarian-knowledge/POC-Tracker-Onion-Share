use anyhow::Context;
use url::Url;
use uuid::Uuid;

use crate::crypto::{FileKey, key_from_b64url, key_to_b64url};

#[derive(Clone, Debug)]
pub struct ShareLink {
    pub onion: String,   // "<id>.onion"
    pub file_id: Uuid,
    pub key: FileKey,
}

impl ShareLink {
    /// Format: opoc://<service>.onion/s/<uuid>#<key_b64url>
    pub fn to_string(&self) -> String {
        format!(
            "opoc://{}/s/{}#{}",
            self.onion,
            self.file_id,
            key_to_b64url(&self.key),
        )
    }

    pub fn parse(s: &str) -> anyhow::Result<Self> {
        let url = Url::parse(s).context("invalid link URL")?;
        anyhow::ensure!(url.scheme() == "opoc", "link must start with opoc://");

        let host = url.host_str().context("link missing host")?.to_string();
        anyhow::ensure!(host.ends_with(".onion"), "host must be a .onion");

        let segs = url
            .path_segments()
            .context("link missing path")?
            .collect::<Vec<_>>();

        // expected: /s/<uuid>
        anyhow::ensure!(segs.len() == 2 && segs[0] == "s", "link path must be /s/<file_id>");
        let file_id = Uuid::parse_str(segs[1]).context("invalid file_id (uuid)")?;

        let fragment = url.fragment().context("link missing #<key> fragment")?;
        let key = key_from_b64url(fragment)?;

        Ok(Self { onion: host, file_id, key })
    }
}
