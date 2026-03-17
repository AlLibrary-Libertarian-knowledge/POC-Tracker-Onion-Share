use std::fs::File;
use std::path::PathBuf;

use anyhow::Context;
use memmap2::Mmap;
use uuid::Uuid;

use crate::crypto::{content_hash_hex, encrypt_chunk, key_from_content_hash, FileKey};

#[derive(Clone)]
pub struct Share {
    pub file_id: Uuid,
    pub file_name: String,
    pub file_size: u64,
    pub chunk_size: usize,
    pub total_chunks: u64,
    pub key: FileKey,
    pub content_hash: String,
    pub chunk_hashes: Vec<String>,

    mmap: std::sync::Arc<Mmap>,
}

impl Share {
    pub fn new(file_path: PathBuf, chunk_size: usize) -> anyhow::Result<Self> {
        anyhow::ensure!(
            chunk_size >= 16 * 1024,
            "chunk_size too small (>= 16 KiB suggested)"
        );
        let f = File::open(&file_path)
            .with_context(|| format!("failed to open file: {}", file_path.display()))?;
        let meta = f.metadata().context("failed to read file metadata")?;
        anyhow::ensure!(meta.is_file(), "path is not a file");
        let file_size = meta.len();
        let total_chunks = if file_size == 0 {
            0
        } else {
            ((file_size as usize + chunk_size - 1) / chunk_size) as u64
        };

        let mmap = unsafe { Mmap::map(&f).context("mmap failed")? };

        let file_name = file_path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "shared.bin".to_string());

        let content_hash = content_hash_hex(&mmap[..]);
        let key = key_from_content_hash(&content_hash)?;
        let chunk_hashes = (0..total_chunks)
            .map(|idx| {
                let start = (idx as usize) * chunk_size;
                let end = ((idx as usize + 1) * chunk_size).min(mmap.len());
                content_hash_hex(&mmap[start..end])
            })
            .collect();

        Ok(Self {
            file_id: Uuid::new_v4(),
            file_name,
            file_size,
            chunk_size,
            total_chunks,
            key,
            content_hash,
            chunk_hashes,
            mmap: std::sync::Arc::new(mmap),
        })
    }

    pub fn chunk_plain(&self, chunk_index: u64) -> anyhow::Result<&[u8]> {
        anyhow::ensure!(chunk_index < self.total_chunks, "chunk out of range");
        let start = (chunk_index as usize) * self.chunk_size;
        let end = ((chunk_index as usize + 1) * self.chunk_size).min(self.mmap.len());
        Ok(&self.mmap[start..end])
    }

    pub fn chunk_cipher(&self, chunk_index: u64) -> anyhow::Result<Vec<u8>> {
        let pt = self.chunk_plain(chunk_index)?;
        encrypt_chunk(&self.key, self.file_id, chunk_index, pt)
    }
}
