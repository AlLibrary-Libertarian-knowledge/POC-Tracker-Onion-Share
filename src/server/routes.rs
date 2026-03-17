use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::server::state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/s/:id/manifest", get(manifest))
        .route("/s/:id/chunk/:idx", get(chunk))
        .route("/s/:id/register", post(register))
        .route("/s/:id/ping", post(ping))
        .route("/s/:id/presence", get(presence))
        .route("/files", get(list_files))
        .route("/network/gossip", get(gossip))
        .with_state(state)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub file_id: Uuid,
    pub file_name: String,
    pub file_size: u64,
    pub chunk_size: usize,
    pub total_chunks: u64,
    pub cipher: String,
    pub content_hash: String,
    pub chunk_hashes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub file_id: Uuid,
    pub file_name: String,
    pub file_size: u64,
    pub total_chunks: u64,
    pub cipher: String,
    pub content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterResponse {
    pub session_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingRequest {
    pub session_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceResponse {
    pub online: usize,
}

async fn list_files(State(state): State<AppState>) -> Json<Vec<FileEntry>> {
    let shares = state.shares.lock().await;
    let entries = shares
        .values()
        .map(|s| FileEntry {
            file_id: s.file_id,
            file_name: s.file_name.clone(),
            file_size: s.file_size,
            total_chunks: s.total_chunks,
            cipher: "XChaCha20-Poly1305".into(),
            content_hash: s.content_hash.clone(),
        })
        .collect();
    Json(entries)
}

async fn manifest(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Manifest>, StatusCode> {
    let share = {
        let shares = state.shares.lock().await;
        shares.get(&id).cloned().ok_or(StatusCode::NOT_FOUND)?
    };
    Ok(Json(Manifest {
        file_id: share.file_id,
        file_name: share.file_name.clone(),
        file_size: share.file_size,
        chunk_size: share.chunk_size,
        total_chunks: share.total_chunks,
        cipher: "XChaCha20-Poly1305".into(),
        content_hash: share.content_hash.clone(),
        chunk_hashes: share.chunk_hashes.clone(),
    }))
}

async fn chunk(
    State(state): State<AppState>,
    Path((id, idx)): Path<(Uuid, u64)>,
) -> Result<Response, StatusCode> {
    let share = {
        let shares = state.shares.lock().await;
        shares.get(&id).cloned().ok_or(StatusCode::NOT_FOUND)?
    };
    if idx >= share.total_chunks {
        return Err(StatusCode::NOT_FOUND);
    }
    let ct = share
        .chunk_cipher(idx)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let bytes_len = ct.len() as u64;
    tokio::spawn(async move {
        state.record_bytes(bytes_len).await;
    });

    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", "application/octet-stream".parse().unwrap());
    Ok((headers, Bytes::from(ct)).into_response())
}

async fn register(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<RegisterResponse>, StatusCode> {
    {
        let shares = state.shares.lock().await;
        if !shares.contains_key(&id) {
            return Err(StatusCode::NOT_FOUND);
        }
    }
    let session_id = state.register().await;
    Ok(Json(RegisterResponse { session_id }))
}

async fn ping(
    State(state): State<AppState>,
    Path(_id): Path<Uuid>,
    Json(req): Json<PingRequest>,
) -> StatusCode {
    state.ping(req.session_id).await;
    StatusCode::NO_CONTENT
}

async fn presence(
    State(state): State<AppState>,
    Path(_id): Path<Uuid>,
) -> Json<PresenceResponse> {
    let online = state.online_count().await;
    Json(PresenceResponse { online })
}

async fn gossip(State(state): State<AppState>) -> Json<crate::tracker_proto::GossipMessage> {
    let shares = state.shares.lock().await;
    let onion = state.onion_addr.lock().await.clone().unwrap_or_default();
    let files = shares
        .values()
        .map(|s| crate::tracker_proto::AnnouncedFile {
            file_id: s.file_id,
            name: s.file_name.clone(),
            size: s.file_size,
            link: format!("http://{}/s/{}/manifest", onion, s.file_id), // Gera o link completo
            content_hash: s.content_hash.clone(),
        })
        .collect();

    Json(crate::tracker_proto::GossipMessage {
        node_id: state.node_id.clone(),
        onion,
        files,
        known_peers: Vec::new(), // Por enquanto
    })
}
