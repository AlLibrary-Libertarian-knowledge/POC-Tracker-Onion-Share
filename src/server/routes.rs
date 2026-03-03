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
        .with_state(state)
}

// ─── Tipos de resposta ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub file_id: Uuid,
    pub file_name: String,
    pub file_size: u64,
    pub chunk_size: usize,
    pub total_chunks: u64,
    pub cipher: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub file_id: Uuid,
    pub file_name: String,
    pub file_size: u64,
    pub total_chunks: u64,
    pub cipher: String,
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

// ─── Handlers ─────────────────────────────────────────────────────────────

/// GET /files — lista todos os arquivos disponíveis (para clientes buscarem)
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
        })
        .collect();
    Json(entries)
}

/// GET /s/:id/manifest
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
    }))
}

/// GET /s/:id/chunk/:idx — chunk cifrado
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
    // Registra estatísticas sem bloquear a resposta
    tokio::spawn(async move {
        state.record_bytes(bytes_len).await;
    });

    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", "application/octet-stream".parse().unwrap());
    Ok((headers, Bytes::from(ct)).into_response())
}

/// POST /s/:id/register
async fn register(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<RegisterResponse>, StatusCode> {
    // Verifica se o arquivo existe
    {
        let shares = state.shares.lock().await;
        if !shares.contains_key(&id) {
            return Err(StatusCode::NOT_FOUND);
        }
    }
    let session_id = state.register().await;
    Ok(Json(RegisterResponse { session_id }))
}

/// POST /s/:id/ping
async fn ping(
    State(state): State<AppState>,
    Path(_id): Path<Uuid>,
    Json(req): Json<PingRequest>,
) -> StatusCode {
    state.ping(req.session_id).await;
    StatusCode::NO_CONTENT
}

/// GET /s/:id/presence
async fn presence(
    State(state): State<AppState>,
    Path(_id): Path<Uuid>,
) -> Json<PresenceResponse> {
    let online = state.online_count().await;
    Json(PresenceResponse { online })
}
